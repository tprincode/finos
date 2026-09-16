//! Trends weekly capture, profit suggestion, distributions, tax monitor.

use crate::contracts::{
    TrendsDistributionAccountTotal, TrendsDistributionBody, TrendsDistributionLine,
    TrendsDistributionSection, TrendsOverviewBody, TrendsTaxMonitorBody, TrendsWeekCaptureBody,
    TrendsWeekPoint, TrendsWeekSourceRecord,
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

pub async fn resolve_capture_as_of(
    canonical: &dyn Canonical,
    requested: Option<&str>,
) -> Result<String, PlatformError> {
    if let Some(raw) = requested.map(str::trim).filter(|s| !s.is_empty()) {
        let day = if raw.len() >= 10 { &raw[..10] } else { raw };
        return Ok(day.to_string());
    }
    let sources = canonical.trends_week_list().await?;
    let saved: Vec<String> = sources.iter().map(|w| w.period_end.clone()).collect();
    let today = chrono::Local::now().date_naive();
    Ok(
        financial_domain::trends::first_unpopulated_saturday(&saved, today)
            .format("%Y-%m-%d")
            .to_string(),
    )
}

pub async fn suggested_acct9_proxy(
    canonical: &dyn Canonical,
    period_end: &str,
) -> Result<Option<i64>, PlatformError> {
    let accounts = canonical.account_list().await?;
    let Some(acct9) = accounts
        .iter()
        .find(|a| financial_domain::income_plan::map_control_account(&a.name) == Some("Account 9"))
    else {
        return Ok(None);
    };
    let basis = canonical.basis_get().await?;
    let securities = canonical.security_list().await?;
    let mut equity_mv = 0i64;
    let mut saw_non_cash = false;
    for lot in &basis.lots {
        if lot.account_id != acct9.account_id || lot.remaining_quantity_minor <= 0 {
            continue;
        }
        let Some(sec) = securities.iter().find(|s| s.security_id == lot.security_id) else {
            continue;
        };
        if financial_domain::current_price::is_cash_par_symbol(&sec.symbol) {
            continue;
        }
        saw_non_cash = true;
        let quotes = canonical
            .price_quote_list(lot.security_id)
            .await
            .unwrap_or_default();
        let obs: Vec<financial_domain::current_price::QuoteObservation> = quotes
            .into_iter()
            .map(|q| financial_domain::current_price::QuoteObservation {
                price_minor: q.price_minor,
                scale: q.scale,
                accepted: q.validation_status.eq_ignore_ascii_case("accepted"),
                as_of: q.as_of_at,
            })
            .collect();
        let Some((px, scale)) =
            financial_domain::current_price::select_price_on_or_before(&obs, period_end)
        else {
            return Ok(None);
        };
        equity_mv += financial_domain::calculator::plan_payment_cents(
            lot.remaining_quantity_minor,
            lot.quantity_scale,
            px,
            scale,
        );
    }
    if !saw_non_cash {
        return Ok(Some(0));
    }
    Ok(Some(financial_domain::trends::acct9_etf_last_price_minor(
        equity_mv,
    )))
}

pub async fn trends_week_capture_view(
    canonical: &dyn Canonical,
    as_of_date: &str,
    week_income_minor: i64,
) -> Result<TrendsWeekCaptureBody, PlatformError> {
    let as_of = financial_domain::trends::parse_iso_date(as_of_date)
        .ok_or_else(|| PlatformError::new("bad_date", format!("invalid asOfDate {as_of_date}")))?;
    let week = financial_domain::trends::trends_period_for_capture(as_of);
    let period_start = week.start.format("%Y-%m-%d").to_string();
    let period_end = week.end.format("%Y-%m-%d").to_string();
    let id = financial_domain::week::week_id_containing(as_of);
    let sources = canonical.trends_week_list().await?;
    let current = sources.iter().find(|w| w.period_end == period_end).cloned();
    let prior = sources
        .iter()
        .rev()
        .find(|w| w.period_end.as_str() < period_end.as_str())
        .cloned();
    let suggested_profit = suggested_profit_for_week(canonical, &period_start, &period_end).await?;
    let suggested_acct9 = suggested_acct9_proxy(canonical, &period_end).await?;
    let accounts = canonical.account_list().await?;
    let snaps = canonical.account_balance_snapshot_list().await?;
    // Prefer balances already on snapshots for this period_end
    let mut car = None;
    let mut income = None;
    let mut health = None;
    let mut roth = None;
    let mut speculation = None;
    let mut acct9 = None;
    let mut car_cash = None;
    let mut health_cash = None;
    let mut roth_cash = None;
    let mut speculation_cash = None;
    for snap in &snaps {
        if snap.period_end != period_end {
            continue;
        }
        let Some(account) = accounts.iter().find(|a| a.account_id == snap.account_id) else {
            continue;
        };
        if account.name.eq_ignore_ascii_case("speculation") {
            speculation = Some(snap.balance_minor);
            speculation_cash = snap.cash_minor;
            continue;
        }
        match financial_domain::income_plan::map_control_account(&account.name) {
            Some("Car") => {
                car = Some(snap.balance_minor);
                car_cash = snap.cash_minor;
            }
            Some("Income") => income = Some(snap.balance_minor),
            Some("Health") => {
                health = Some(snap.balance_minor);
                health_cash = snap.cash_minor;
            }
            Some("Roth") => {
                roth = Some(snap.balance_minor);
                roth_cash = snap.cash_minor;
            }
            Some("Account 9") => {
                acct9 = Some(snap.balance_minor);
            }
            _ => {}
        }
    }
    let mut missing = Vec::new();
    if let Some(ref c) = current {
        if c.fidelity_total_minor == 0 && c.schwab_total_minor == 0 {
            missing.push("fidelity_or_schwab".into());
        }
    }
    let acct9_balance_minor = acct9.or_else(|| current.as_ref().map(|c| c.schwab_total_minor));
    let today = chrono::Local::now().date_naive();
    let saved: Vec<String> = sources.iter().map(|w| w.period_end.clone()).collect();
    let first_unpopulated_start = financial_domain::trends::first_unpopulated_saturday(&saved, today)
        .format("%Y-%m-%d")
        .to_string();
    let chooser_saturdays =
        financial_domain::trends::chooser_saturdays(
            financial_domain::trends::first_unpopulated_saturday(&saved, today),
            today,
        );
    Ok(TrendsWeekCaptureBody {
        period_start: period_start.clone(),
        period_end: period_end.clone(),
        week_year: id.year,
        week_number: id.number,
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
        acct9_balance_minor,
        car_cash_minor: car_cash,
        health_cash_minor: health_cash,
        roth_cash_minor: roth_cash,
        speculation_cash_minor: speculation_cash,
        suggested_profit_minor: suggested_profit,
        suggested_monthly_divs_minor: week_income_minor,
        suggested_acct9_etf_proxy_minor: suggested_acct9,
        first_unpopulated_start,
        chooser_saturdays,
        populated_period_ends: saved,
        missing_required: missing,
        scale: 2,
        cash_references: crate::cash_management::cash_references_for_week(canonical, &period_end)
            .await
            .unwrap_or_default(),
    })
}

pub async fn save_trends_week(
    canonical: &dyn Canonical,
    record: TrendsWeekSourceRecord,
    balances: &[(&str, Option<i64>, Option<i64>)],
    allow_closed_edit: bool,
    week_income_minor: i64,
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
        let suggested =
            suggested_profit_for_week(canonical, &to_save.period_start, &to_save.period_end)
                .await?;
        if suggested != 0 {
            to_save.profit_minor = suggested;
        }
    }
    if to_save.monthly_divs_minor == 0 && week_income_minor != 0 {
        to_save.monthly_divs_minor = week_income_minor;
    }
    // Slice 1b: blank ETF total stays 0 — do not invent last-price 70% proxy on save.
    let accounts = canonical.account_list().await?;
    let mut ids = std::collections::HashMap::new();
    for a in &accounts {
        ids.insert(a.name.clone(), a.account_id);
    }
    let mut pairs = Vec::new();
    for (name, bal, cash) in balances {
        let Some(balance_minor) = *bal else { continue };
        let Some(account_id) = ids.get(*name).copied() else {
            continue;
        };
        pairs.push((account_id, balance_minor, *cash));
    }
    canonical
        .trends_week_save_with_balances(to_save.clone(), pairs)
        .await?;
    trends_week_capture_view(canonical, &to_save.period_end, week_income_minor).await
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
    let mut federal = 0i64;
    let mut state = 0i64;
    let mut net = 0i64;
    for a in &activities {
        if !financial_domain::trends::is_non_roi_distribution(&a.activity_type) {
            continue;
        }
        if !a.occurred_on.starts_with(year) {
            continue;
        }
        let account = accounts.iter().find(|x| x.account_id == a.account_id);
        let name = account
            .map(|x| x.name.clone())
            .unwrap_or_else(|| "unknown".into());
        let kind = account
            .map(|x| x.kind.clone())
            .unwrap_or_default();
        let section = financial_domain::cash_management::cash_tax_section(&a.activity_type, &kind);
        let line_net = financial_domain::cash_management::net_minor(
            a.amount_minor,
            a.federal_withholding_minor,
            a.state_withholding_minor,
        );
        gross += a.amount_minor;
        federal += a.federal_withholding_minor;
        state += a.state_withholding_minor;
        net += line_net;
        lines.push(TrendsDistributionLine {
            activity_type: a.activity_type.clone(),
            account_name: name,
            amount_minor: a.amount_minor,
            occurred_on: a.occurred_on.clone(),
            scale: a.scale,
            federal_withholding_minor: a.federal_withholding_minor,
            state_withholding_minor: a.state_withholding_minor,
            net_minor: line_net,
            account_kind: kind,
            tax_section: section.id().into(),
        });
    }
    lines.sort_by(|a, b| a.occurred_on.cmp(&b.occurred_on));
    let mut account_map: std::collections::BTreeMap<
        String,
        TrendsDistributionAccountTotal,
    > = std::collections::BTreeMap::new();
    let mut section_map: std::collections::BTreeMap<
        financial_domain::cash_management::CashTaxSection,
        (i64, i64),
    > = std::collections::BTreeMap::new();
    for line in &lines {
        let section = financial_domain::cash_management::cash_tax_section(
            &line.activity_type,
            &line.account_kind,
        );
        let entry = account_map
            .entry(line.account_name.clone())
            .or_insert(TrendsDistributionAccountTotal {
                account_name: line.account_name.clone(),
                account_kind: line.account_kind.clone(),
                tax_section: section.id().into(),
                gross_minor: 0,
                net_minor: 0,
            });
        entry.gross_minor += line.amount_minor;
        entry.net_minor += line.net_minor;
        let slot = section_map.entry(section).or_insert((0, 0));
        slot.0 += line.amount_minor;
        slot.1 += line.net_minor;
    }
    let account_totals = account_map.into_values().collect();
    let sections = section_map
        .into_iter()
        .map(|(section, (gross_minor, net_minor))| TrendsDistributionSection {
            id: section.id().into(),
            label: section.label().into(),
            tax_note: section.tax_note().into(),
            gross_minor,
            net_minor,
        })
        .collect();
    Ok(TrendsDistributionBody {
        gross_minor: gross,
        federal_withholding_minor: federal,
        state_withholding_minor: state,
        net_minor: net,
        lines,
        account_totals,
        sections,
        scale: 2,
    })
}

pub async fn tax_monitor(
    canonical: &dyn Canonical,
    as_of: &str,
) -> Result<TrendsTaxMonitorBody, PlatformError> {
    let threshold = canonical
        .aca_threshold_get(2026, 2, "US-contiguous".into())
        .await?;
    let tax = canonical.tax_projection_get().await.ok();
    let projected = tax.as_ref().map(|t| t.actual_included_ytd.amount_minor);
    let (warning, gap) = match (projected, threshold) {
        (Some(p), Some((th, _))) => (p > th, Some(p - th)),
        _ => (false, None),
    };
    let year = &as_of[..4.min(as_of.len())];
    let activities = canonical.activity_list().await?;
    let federal_withholding_minor = activities
        .iter()
        .filter(|a| financial_domain::trends::is_non_roi_distribution(&a.activity_type))
        .filter(|a| a.occurred_on.starts_with(year))
        .map(|a| a.federal_withholding_minor)
        .sum();
    Ok(TrendsTaxMonitorBody {
        federal_withholding_minor,
        projected_liability_minor: projected,
        gap_minor: gap,
        warning,
        aca_threshold_minor: threshold.map(|(t, _)| t),
        aca_coverage_year: Some(2026),
        note: "Planning warnings only; final forms/return remain authoritative.".into(),
        scale: 2,
    })
}
