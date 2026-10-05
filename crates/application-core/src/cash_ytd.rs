//! Slice 4 YTD actual + remaining plan by Register book or tax type.

use std::collections::HashMap;

use crate::contracts::{CashYtdBody, CashYtdRow};
use crate::ports::canonical::Canonical;
use crate::ports::platform::PlatformError;
use crate::week_ahead::{ensure_horizon, ensure_seed};
use chrono::Datelike;
use financial_domain::cash_management::{
    is_cash_adjust_type, is_cash_distribution_type, job_1099_year_amounts, register_book_label,
    register_ledger_account, REGISTER_BOOKS,
};
use financial_domain::trends::parse_iso_date;

/// Tax-type YTD is posted cash. Remaining plan is the cash-element plan.
/// Dividend income plan is not a withdrawal and is not added here.
/// Car cash-outs stay on the Account view only — Car tax year is ROC and
/// ordinary dividends on Tax Planning.
const TAX_ROWS: &[(&str, &str)] = &[
    ("IRA ordinary", "Income"),
    ("SSA", "SSA_2026"),
    ("Roth", "FI Roth"),
    ("Health (not MAGI)", "Health"),
];

fn sat_of(as_of: &str) -> Result<String, PlatformError> {
    let day = parse_iso_date(as_of)
        .ok_or_else(|| PlatformError::new("bad_date", format!("invalid asOfDate {as_of}")))?;
    Ok(financial_domain::trends::trends_period_for_capture(day)
        .start
        .format("%Y-%m-%d")
        .to_string())
}

fn year_bounds(as_of: &str) -> Result<(String, String), PlatformError> {
    let day = parse_iso_date(as_of)
        .ok_or_else(|| PlatformError::new("bad_date", format!("invalid asOfDate {as_of}")))?;
    let year = day.year();
    Ok((format!("{year}-01-01"), format!("{year}-12-31")))
}

fn next_day(as_of: &str) -> Result<String, PlatformError> {
    let day = parse_iso_date(as_of)
        .ok_or_else(|| PlatformError::new("bad_date", format!("invalid asOfDate {as_of}")))?;
    let next = day
        .succ_opt()
        .ok_or_else(|| PlatformError::new("bad_date", format!("no day after {as_of}")))?;
    Ok(next.format("%Y-%m-%d").to_string())
}

fn eoy(actual: i64, remaining: Option<i64>) -> Option<i64> {
    remaining.map(|r| actual + r)
}

struct BookRollup {
    actual: i64,
    remaining: Option<i64>,
}

async fn rollups(
    canonical: &dyn Canonical,
    jan1: &str,
    as_of: &str,
    dec31: &str,
) -> Result<HashMap<&'static str, BookRollup>, PlatformError> {
    let accounts = canonical.account_list().await?;
    let occs = canonical.planned_occurrence_list().await?;
    let activities = canonical
        .activity_list_in_range(None, jan1, dec31)
        .await
        .unwrap_or_default();
    let mut out = HashMap::new();
    for book in REGISTER_BOOKS {
        let ledger = register_ledger_account(book);
        let ledger_id = accounts
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case(ledger))
            .map(|a| a.account_id);
        let mut actual = 0i64;
        let mut remaining = 0i64;
        let mut remaining_known = false;
        let mut withdrawn_on = std::collections::HashSet::<String>::new();
        if let Some(account_id) = ledger_id {
            for act in &activities {
                if act.account_id != account_id || !is_cash_distribution_type(&act.activity_type) {
                    continue;
                }
                let on = act.occurred_on.get(..10).unwrap_or(act.occurred_on.as_str());
                if on >= jan1 && on <= dec31 {
                    withdrawn_on.insert(on.to_string());
                }
            }
        }
        for o in &occs {
            if register_book_label(&o.account) != Some(*book) {
                continue;
            }
            if o.note.eq_ignore_ascii_case("dividend") {
                continue;
            }
            if o.confirmed_at.is_some()
                && o.occurred_on.as_str() >= jan1
                && o.occurred_on.as_str() <= as_of
            {
                actual += o.amount_minor.abs();
            }
            let on = o.occurred_on.get(..10).unwrap_or(o.occurred_on.as_str());
            if o.confirmed_at.is_none() && on >= as_of && on <= dec31 && !withdrawn_on.contains(on)
            {
                remaining += o.amount_minor.abs();
                remaining_known = true;
            }
        }
        if let Some(account_id) = ledger_id {
            for act in &activities {
                if act.account_id != account_id {
                    continue;
                }
                if act.occurred_on.as_str() < jan1 || act.occurred_on.as_str() > as_of {
                    continue;
                }
                if act.idempotency_key.starts_with("week-ahead-") {
                    continue;
                }
                if is_cash_adjust_type(&act.activity_type) {
                    continue;
                }
                if !is_cash_distribution_type(&act.activity_type) {
                    continue;
                }
                actual += act.amount_minor.abs();
            }
        }
        out.insert(
            *book,
            BookRollup {
                actual,
                remaining: if remaining_known { Some(remaining) } else { None },
            },
        );
    }
    Ok(out)
}

pub async fn cash_ytd_get(
    canonical: &dyn Canonical,
    as_of: &str,
    view: &str,
) -> Result<CashYtdBody, PlatformError> {
    let view = match view.trim().to_ascii_lowercase().as_str() {
        "tax" | "tax_type" | "taxtype" => "tax",
        _ => "account",
    };
    let (jan1, dec31) = year_bounds(as_of)?;
    let sat = sat_of(as_of)?;
    ensure_seed(canonical, &sat).await?;
    let remaining_start = next_day(as_of)?;
    if remaining_start.as_str() <= dec31.as_str() {
        ensure_horizon(canonical, &remaining_start, &dec31).await?;
    }
    let books = rollups(canonical, &jan1, as_of, &dec31).await?;
    let rows = if view == "tax" {
        let mut rows: Vec<CashYtdRow> = TAX_ROWS
            .iter()
            .map(|(label, book)| {
                let roll = books.get(book);
                let mut actual = roll.map(|r| r.actual).unwrap_or(0);
                let mut remaining = roll.and_then(|r| r.remaining);
                // One IRA figure: Income plus Account 9. Speculation has no book here;
                // Tax Planning adds it and it is zero when that account has no withdrawal.
                if *book == "Income" {
                    if let Some(nine) = books.get("Account 9") {
                        actual += nine.actual;
                        remaining = match (remaining, nine.remaining) {
                            (Some(a), Some(b)) => Some(a + b),
                            (Some(a), None) => Some(a),
                            (None, Some(b)) => Some(b),
                            (None, None) => None,
                        };
                    }
                }
                CashYtdRow {
                    label: (*label).into(),
                    actual_minor: actual,
                    remaining_minor: remaining,
                    eoy_minor: eoy(actual, remaining),
                }
            })
            .collect();
        let (job_ytd, job_remain) = job_1099_year_amounts(as_of);
        rows.push(CashYtdRow {
            label: "1099 job".into(),
            actual_minor: job_ytd,
            remaining_minor: Some(job_remain),
            eoy_minor: eoy(job_ytd, Some(job_remain)),
        });
        rows
    } else {
        REGISTER_BOOKS
            .iter()
            .map(|book| {
                let roll = books.get(book);
                let actual = roll.map(|r| r.actual).unwrap_or(0);
                let remaining = roll.and_then(|r| r.remaining);
                CashYtdRow {
                    label: (*book).into(),
                    actual_minor: actual,
                    remaining_minor: remaining,
                    eoy_minor: eoy(actual, remaining),
                }
            })
            .collect()
    };
    Ok(CashYtdBody {
        as_of_date: as_of.to_string(),
        view: view.into(),
        rows,
        scale: 2,
    })
}
