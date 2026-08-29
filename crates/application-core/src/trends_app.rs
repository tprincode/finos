//! Trends weekly capture, profit suggestion, distributions, tax monitor.

use crate::contracts::{
    TrendsDistributionBody, TrendsDistributionLine, TrendsOverviewBody, TrendsTaxMonitorBody,
    TrendsWeekCaptureBody, TrendsWeekPoint, TrendsWeekSourceRecord,
};
use crate::ports::canonical::Canonical;
use crate::ports::platform::PlatformError;

pub async fn suggested_profit_for_week(
    canonical: &dyn Canonical,
    period_start: &str,
    period_end: &str,
) -> Result<i64, PlatformError> {
    let activities = canonical.activity_list().await?;
    let mut total = 0i64;
    for a in &activities {
        if !financial_domain::trends::is_trends_profit_activity(&a.activity_type) {
            continue;
        }
        if !financial_domain::income_plan::occurred_in_week(
            &a.occurred_on,
            period_start,
            period_end,
        ) {
            continue;
        }
        total += a.amount_minor;
    }
    Ok(total)
}

pub async fn suggested_acct9_proxy(
    canonical: &dyn Canonical,
) -> Result<Option<i64>, PlatformError> {
    let accounts = canonical.account_list().await?;
    let Some(acct9) = accounts.iter().find(|a| {
        financial_domain::income_plan::map_control_account(&a.name) == Some("Account 9")
    }) else {
        return Ok(None);
    };
    let basis = canonical.basis_get().await?;
    let securities = canonical.security_list().await?;
    let mut cash_par = 0i64;
    let mut other = 0i64;
    for lot in &basis.lots {
        if lot.account_id != acct9.account_id || lot.remaining_quantity_minor <= 0 {
            continue;
        }
        let Some(sec) = securities.iter().find(|s| s.security_id == lot.security_id) else {
            continue;
        };
        if financial_domain::current_price::is_cash_par_symbol(&sec.symbol) {
            cash_par += lot.remaining_tax_minor;
        } else {
            other += lot.remaining_tax_minor;
        }
    }
    Ok(Some(financial_domain::trends::acct9_classified_liquid_minor(
        cash_par, other,
    )))
}

pub async fn trends_week_capture_view(
    canonical: &dyn Canonical,
    as_of_date: &str,
) -> Result<TrendsWeekCaptureBody, PlatformError> {
    let as_of = financial_domain::trends::parse_iso_date(as_of_date).ok_or_else(|| {
        PlatformError::new("bad_date", format!("invalid asOfDate {as_of_date}"))
    })?;
    let week = financial_domain::trends::trends_period_for_capture(as_of);
    let period_start = week.start.format("%Y-%m-%d").to_string();
    let period_end = week.end.format("%Y-%m-%d").to_string();
    let sources = canonical.trends_week_list().await?;
    let current = sources.iter().find(|w| w.period_end == period_end).cloned();
    let prior = sources
        .iter()
        .rev()
        .find(|w| w.period_end.as_str() < period_end.as_str())
        .cloned();
    let plan = canonical.income_plan_get().await.unwrap_or(
        crate::contracts::IncomePlanBody {
            planned_minor: 0,
            actual_minor: 0,
            scale: 2,
        },
    );
    let suggested_profit =
        suggested_profit_for_week(canonical, &period_start, &period_end).await?;
    let suggested_acct9 = suggested_acct9_proxy(canonical).await?;
    let accounts = canonical.account_list().await?;
    let snaps = canonical.account_balance_snapshot_list().await?;
    let bal = |control: &str| -> Option<i64> {
        let account = accounts.iter().find(|a| {
            if control == "Speculation" {
                a.name.eq_ignore_ascii_case("speculation")
            } else if control == "Roth" {
                financial_domain::income_plan::map_control_account(&a.name) == Some("Roth")
            } else {
                financial_domain::income_plan::map_control_account(&a.name) == Some(control)
                    || a.name.eq_ignore_ascii_case(control)
            }
        })?;
        snaps
            .iter()
            .filter(|s| s.account_id == account.account_id && s.period_end == period_end)
            .map(|s| s.balance_minor)
            .next()
            .or_else(|| {
                // fall back to latest for that account at this period from current week row maps
                None
            })
    };
    // Prefer balances already on snapshots for this period_end
    let mut car = None;
    let mut income = None;
    let mut health = None;
    let mut roth = None;
    let mut speculation = None;
    for snap in &snaps {
        if snap.period_end != period_end {
            continue;
        }
        let Some(account) = accounts.iter().find(|a| a.account_id == snap.account_id) else {
            continue;
        };
        if account.name.eq_ignore_ascii_case("speculation") {
            speculation = Some(snap.balance_minor);
            continue;
        }
        match financial_domain::income_plan::map_control_account(&account.name) {
            Some("Car") => car = Some(snap.balance_minor),
            Some("Income") => income = Some(snap.balance_minor),
            Some("Health") => health = Some(snap.balance_minor),
            Some("Roth") => roth = Some(snap.balance_minor),
            _ => {}
        }
    }
    let _ = bal;
    let mut missing = Vec::new();
    if let Some(ref c) = current {
        if c.fidelity_total_minor == 0 && c.schwab_total_minor == 0 {
            missing.push("fidelity_or_schwab".into());
        }
    } else {
        missing.push("week_not_saved".into());
    }
    Ok(TrendsWeekCaptureBody {
        period_start,
        period_end,
        captured_at: chrono::Utc::now().to_rfc3339(),
        closed: current.as_ref().map(|c| c.closed).unwrap_or(false),
        exists: current.is_some(),
        prior,
        current,
        car_balance_minor: car,
        income_balance_minor: income,
        health_balance_minor: health,
        roth_balance_minor: roth,
        speculation_balance_minor: speculation,
        suggested_profit_minor: suggested_profit,
        suggested_monthly_divs_minor: plan.planned_minor,
        suggested_acct9_etf_proxy_minor: suggested_acct9,
        missing_required: missing,
        scale: 2,
    })
}

pub async fn save_trends_week(
    canonical: &dyn Canonical,
    record: TrendsWeekSourceRecord,
    balances: &[(&str, Option<i64>)],
    allow_closed_edit: bool,
) -> Result<TrendsWeekCaptureBody, PlatformError> {
    let mut preserve_closed = false;
    if let Some(existing) = canonical.trends_week_get(record.period_end.clone()).await? {
        if existing.closed && !allow_closed_edit {
            return Err(PlatformError::new(
                "trends_week_closed",
                "week is closed; use TrendsWeekCorrect",
            ));
        }
        if existing.closed && allow_closed_edit {
            preserve_closed = true;
        }
    }
    let mut to_save = record;
    if preserve_closed {
        to_save.closed = true;
    }
    // T2: if profit zero and ledger has value, use ledger.
    if to_save.profit_minor == 0 {
        let suggested = suggested_profit_for_week(
            canonical,
            &to_save.period_start,
            &to_save.period_end,
        )
        .await?;
        if suggested != 0 {
            to_save.profit_minor = suggested;
        }
    }
    // T2: freeze Monthly DIVS from Income Plan when not provided.
    if to_save.monthly_divs_minor == 0 {
        if let Ok(plan) = canonical.income_plan_get().await {
            to_save.monthly_divs_minor = plan.planned_minor;
        }
    }
    // T7: if acct9 etf blank/zero, suggest from lots.
    if to_save.acct9_etf_value_minor == 0 {
        if let Some(proxy) = suggested_acct9_proxy(canonical).await? {
            to_save.acct9_etf_value_minor = proxy;
        }
    }
    canonical.trends_week_upsert(to_save.clone()).await?;
    let accounts = canonical.account_list().await?;
    let mut ids = std::collections::HashMap::new();
    for a in &accounts {
        ids.insert(a.name.clone(), a.account_id);
    }
    for (name, bal) in balances {
        let Some(balance_minor) = bal else { continue };
        let Some(account_id) = ids.get(*name).copied() else {
            continue;
        };
        canonical
            .account_balance_snapshot_upsert(
                account_id,
                to_save.period_end.clone(),
                *balance_minor,
                to_save.scale,
                to_save.captured_at.clone(),
            )
            .await?;
    }
    trends_week_capture_view(canonical, &to_save.period_end).await
}

pub fn overview_from_weeks(weeks: &[TrendsWeekPoint]) -> TrendsOverviewBody {
    let last = weeks.last();
    TrendsOverviewBody {
        fid_sch_combined_minor: last.map(|w| w.fid_sch_combined_minor),
        wk_to_wk_change_minor: last.map(|w| w.wk_to_wk_change_minor),
        profit_minor: last.map(|w| w.profit_minor),
        monthly_divs_minor: last.map(|w| w.monthly_divs_minor),
        div_delta_minor: last.map(|w| w.div_delta_minor),
        total_cash_minor: last.map(|w| w.total_cash_minor),
        scale: 2,
    }
}

pub async fn distributions_ytd(
    canonical: &dyn Canonical,
    as_of: &str,
) -> Result<TrendsDistributionBody, PlatformError> {
    let year = &as_of[..4.min(as_of.len())];
    let accounts = canonical.account_list().await?;
    let activities = canonical.activity_list().await?;
    let mut lines = Vec::new();
    let mut gross = 0i64;
    for a in &activities {
        if !financial_domain::trends::is_non_roi_distribution(&a.activity_type) {
            continue;
        }
        if !a.occurred_on.starts_with(year) {
            continue;
        }
        let name = accounts
            .iter()
            .find(|x| x.account_id == a.account_id)
            .map(|x| x.name.clone())
            .unwrap_or_else(|| "unknown".into());
        gross += a.amount_minor;
        lines.push(TrendsDistributionLine {
            activity_type: a.activity_type.clone(),
            account_name: name,
            amount_minor: a.amount_minor,
            occurred_on: a.occurred_on.clone(),
            scale: a.scale,
        });
    }
    lines.sort_by(|a, b| a.occurred_on.cmp(&b.occurred_on));
    Ok(TrendsDistributionBody {
        gross_minor: gross,
        lines,
        scale: 2,
    })
}

pub async fn tax_monitor(
    canonical: &dyn Canonical,
) -> Result<TrendsTaxMonitorBody, PlatformError> {
    let threshold = canonical
        .aca_threshold_get(2026, 2, "US-contiguous".into())
        .await?;
    let tax = canonical.tax_projection_get().await.ok();
    let projected = tax
        .as_ref()
        .map(|t| t.actual_included_ytd.amount_minor);
    let (warning, gap) = match (projected, threshold) {
        (Some(p), Some((th, _))) => (p > th, Some(p - th)),
        _ => (false, None),
    };
    Ok(TrendsTaxMonitorBody {
        federal_withholding_minor: 0,
        projected_liability_minor: projected,
        gap_minor: gap,
        warning,
        aca_threshold_minor: threshold.map(|(t, _)| t),
        aca_coverage_year: Some(2026),
        note: "Planning warnings only; final forms/return remain authoritative.".into(),
        scale: 2,
    })
}
