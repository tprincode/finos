//! Live cash SOT = money-market lot. Friday lock = account_balance_snapshot.cash_minor.
//! Integrity: pile == fold(last snapshot with cash, journal after that Friday through as-of).

use uuid::Uuid;

use crate::contracts::CashIntegrityBody;
use crate::ports::canonical::Canonical;
use crate::ports::platform::PlatformError;
use financial_domain::cash_management::cash_journal_delta_cents;
use financial_domain::money::to_usd_cents;

/// Prior snapshot cash + journal and dividends with occurred_on in (snapshot_end, as_of_end].
pub async fn cash_fold_through(
    canonical: &dyn Canonical,
    account_id: Uuid,
    as_of_end: &str,
) -> Result<Option<(String, i64, i64)>, PlatformError> {
    let end = if as_of_end.len() >= 10 {
        &as_of_end[..10]
    } else {
        as_of_end
    };
    let snaps = canonical.account_balance_snapshot_list().await?;
    let mut prior: Option<(String, i64)> = None;
    for snap in &snaps {
        if snap.account_id != account_id || snap.period_end.as_str() >= end {
            continue;
        }
        let Some(cash) = snap.cash_minor else {
            continue;
        };
        let take = prior
            .as_ref()
            .map(|(pe, _)| snap.period_end.as_str() > pe.as_str())
            .unwrap_or(true);
        if take {
            prior = Some((snap.period_end.clone(), cash));
        }
    }
    let Some((prior_end, prior_cash)) = prior else {
        return Ok(None);
    };
    let after = if prior_end.len() >= 10 {
        prior_end[..10].to_string()
    } else {
        prior_end.clone()
    };
    let activity = canonical
        .activity_list_in_range(Some(account_id), &after, end)
        .await
        .unwrap_or_default();
    let mut journal_delta = 0_i64;
    for row in &activity {
        // Range helpers are inclusive on both ends; skip the prior Friday itself.
        if row.occurred_on.as_str() <= after.as_str() {
            continue;
        }
        if row.occurred_on.as_str() > end {
            continue;
        }
        journal_delta += cash_journal_delta_cents(&row.activity_type, row.amount_minor, row.scale);
    }
    let dividends = canonical
        .dividend_list_in_range(Some(account_id), &after, end)
        .await
        .unwrap_or_default();
    let mut dividend_minor = 0_i64;
    for row in &dividends {
        if row.occurred_on.as_str() <= after.as_str() {
            continue;
        }
        if row.occurred_on.as_str() > end {
            continue;
        }
        dividend_minor += to_usd_cents(row.amount_minor, row.scale);
    }
    Ok(Some((
        after,
        prior_cash,
        prior_cash + dividend_minor + journal_delta,
    )))
}

pub async fn integrity_for_account(
    canonical: &dyn Canonical,
    account_id: Uuid,
    as_of_end: &str,
) -> Result<CashIntegrityBody, PlatformError> {
    let account = canonical.account_get(account_id).await?;
    let pile = crate::cash_pile::pile_for_account(canonical, account_id).await?;
    let pile_minor = pile.as_ref().map(|p| p.dollars_minor);
    let fold = cash_fold_through(canonical, account_id, as_of_end).await?;
    let (snapshot_on, snapshot_minor, fold_minor) = match fold {
        Some((on, snap, folded)) => (Some(on), Some(snap), Some(folded)),
        None => (None, None, None),
    };
    let gap_minor = match (pile_minor, fold_minor) {
        (Some(p), Some(f)) => Some(p - f),
        _ => None,
    };
    let ok = match gap_minor {
        Some(0) => true,
        Some(_) => false,
        // No Friday lock yet — cannot verify; do not block cart complete.
        None => true,
    };
    Ok(CashIntegrityBody {
        account_id,
        account_name: account.name,
        as_of_end: if as_of_end.len() >= 10 {
            as_of_end[..10].to_string()
        } else {
            as_of_end.to_string()
        },
        pile_minor,
        snapshot_minor,
        snapshot_on,
        fold_minor,
        gap_minor,
        ok,
    })
}

pub async fn integrity_get(
    canonical: &dyn Canonical,
    account_id: Option<Uuid>,
    as_of_end: String,
) -> Result<Vec<CashIntegrityBody>, PlatformError> {
    let accounts = canonical.account_list().await?;
    let names = financial_domain::cash_management::CASH_ADJUST_ACCOUNT_NAMES;
    let mut out = Vec::new();
    for name in names {
        let Some(account) = accounts.iter().find(|a| a.name == *name) else {
            continue;
        };
        if let Some(want) = account_id {
            if account.account_id != want {
                continue;
            }
        }
        out.push(integrity_for_account(canonical, account.account_id, &as_of_end).await?);
    }
    // Account listed as "9" in some seeds
    if account_id.is_some() && out.is_empty() {
        if let Some(id) = account_id {
            out.push(integrity_for_account(canonical, id, &as_of_end).await?);
        }
    }
    Ok(out)
}
