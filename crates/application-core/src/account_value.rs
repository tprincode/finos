//! Home / Trends account-value reconstruction (live last price, risk, weekly actuals).

use crate::contracts::{
    AccountIncomePointBody, AccountMarketValueDailyRecord, AccountRecord, AccountValueHomeBody,
    AccountValuePointBody, AccountValueSeriesBody, PositionCharacteristicRecord,
    PositionDetailsBody, PriceQuoteBody, RiskGroupValueBody, RiskSymbolValueBody,
    RiskValueHomeBody, RiskValuePointBody, TrendsWeekPoint,
};
use crate::ports::canonical::Canonical;
use crate::queries::{account_trends_weeks, is_data_account, position_details_summary};

pub(crate) async fn record_account_value_snapshot(
    canonical: &dyn crate::ports::canonical::Canonical,
    as_of: &str,
) -> Result<AccountValueHomeBody, crate::ports::platform::PlatformError> {
    let details =
        position_details_summary(canonical, &serde_json::json!({ "asOfDate": as_of })).await?;
    let accounts = canonical.account_list().await?;
    let captured_at = chrono::Utc::now().to_rfc3339();
    let mut live: Vec<(String, String, Option<i64>)> = Vec::new();
    for acct in accounts.iter().filter(|a| is_data_account(&a.name)) {
        let mv = details
            .account_totals
            .iter()
            .find(|t| t.account_id == acct.account_id)
            .map(|t| t.market_value_minor)
            .unwrap_or(Some(0));
        live.push((acct.account_id.to_string(), acct.name.clone(), mv));
        canonical
            .account_market_value_daily_upsert(AccountMarketValueDailyRecord {
                snapshot_id: uuid::Uuid::new_v4().to_string(),
                account_id: acct.account_id.to_string(),
                account_name: acct.name.clone(),
                as_of: as_of.to_string(),
                market_value_minor: mv,
                market_value_complete: mv.is_some(),
                scale: 2,
                captured_at: captured_at.clone(),
            })
            .await?;
    }
    let fid_rows: Vec<(String, Option<i64>)> = live
        .iter()
        .map(|(_, name, mv)| (name.clone(), *mv))
        .collect();
    let (fid_mv, fid_ok) = financial_domain::account_value::fidelity_total_from_accounts(&fid_rows);
    let (sch_mv, sch_ok) = financial_domain::account_value::schwab_total_from_accounts(&fid_rows);
    canonical
        .account_market_value_daily_upsert(AccountMarketValueDailyRecord {
            snapshot_id: uuid::Uuid::new_v4().to_string(),
            account_id: financial_domain::account_value::FIDELITY_TOTAL_ID.to_string(),
            account_name: financial_domain::account_value::FIDELITY_TOTAL_NAME.to_string(),
            as_of: as_of.to_string(),
            market_value_minor: fid_mv,
            market_value_complete: fid_ok,
            scale: 2,
            captured_at: captured_at.clone(),
        })
        .await?;
    canonical
        .account_market_value_daily_upsert(AccountMarketValueDailyRecord {
            snapshot_id: uuid::Uuid::new_v4().to_string(),
            account_id: financial_domain::account_value::SCHWAB_TOTAL_ID.to_string(),
            account_name: financial_domain::account_value::SCHWAB_TOTAL_NAME.to_string(),
            as_of: as_of.to_string(),
            market_value_minor: sch_mv,
            market_value_complete: sch_ok,
            scale: 2,
            captured_at: captured_at.clone(),
        })
        .await?;
    let characteristics = canonical.position_characteristic_list().await?;
    let live_risk = live_risk_from_details(&details, &characteristics);
    persist_risk_snapshot(canonical, as_of, &captured_at, &live_risk).await?;
    account_value_home_view(canonical, as_of).await
}

fn prior_iso_day(as_of: &str) -> Option<String> {
    let day = if as_of.len() >= 10 {
        &as_of[..10]
    } else {
        as_of
    };
    let parsed = chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()?;
    Some((parsed - chrono::Duration::days(1)).to_string())
}

fn overlay_live_point(
    mut points: Vec<AccountValuePointBody>,
    as_of: &str,
    current: Option<i64>,
) -> Vec<AccountValuePointBody> {
    if let Some(point) = points.iter_mut().find(|p| p.as_of == as_of) {
        point.market_value_minor = current;
        point.market_value_complete = current.is_some();
    } else {
        points.push(AccountValuePointBody {
            as_of: as_of.to_string(),
            market_value_minor: current,
            market_value_complete: current.is_some(),
        });
    }
    if let Some(yesterday) = prior_iso_day(as_of) {
        if !points.iter().any(|p| p.as_of == yesterday) {
            points.push(AccountValuePointBody {
                as_of: yesterday,
                market_value_minor: None,
                market_value_complete: false,
            });
        }
    }
    points.sort_by(|a, b| a.as_of.cmp(&b.as_of));
    points
}

struct LiveRisk {
    groups: Vec<RiskGroupValueBody>,
    total: Option<i64>,
    complete: bool,
}

fn live_risk_from_details(
    details: &PositionDetailsBody,
    characteristics: &[PositionCharacteristicRecord],
) -> LiveRisk {
    use std::collections::BTreeMap;
    let risk_by_sec: std::collections::HashMap<_, _> = characteristics
        .iter()
        .map(|c| (c.security_id, c.risk_tier.as_str()))
        .collect();
    let mut by_symbol: BTreeMap<String, (String, Option<i64>, bool)> = BTreeMap::new();
    for line in &details.positions {
        if !is_data_account(&line.account_name) {
            continue;
        }
        let tier = financial_domain::account_value::risk_bucket(
            risk_by_sec.get(&line.security_id).copied().unwrap_or(""),
        )
        .to_string();
        let entry = by_symbol
            .entry(line.symbol.clone())
            .or_insert((tier.clone(), Some(0), true));
        entry.0 = tier;
        match line.market_value_minor {
            Some(mv) => {
                if let Some(cur) = entry.1.as_mut() {
                    *cur += mv;
                }
            }
            None => {
                entry.1 = None;
                entry.2 = false;
            }
        }
    }
    let mut grouped: BTreeMap<&str, Vec<RiskSymbolValueBody>> = BTreeMap::new();
    for tier in financial_domain::account_value::RISK_TIERS {
        grouped.insert(tier, Vec::new());
    }
    for (symbol, (tier, mv, complete)) in by_symbol {
        let key = financial_domain::account_value::risk_bucket(&tier);
        grouped.entry(key).or_default().push(RiskSymbolValueBody {
            symbol,
            risk_tier: key.to_string(),
            market_value_minor: mv,
            market_value_complete: complete,
        });
    }
    let mut groups = Vec::new();
    for tier in financial_domain::account_value::RISK_TIERS {
        let mut symbols = grouped.remove(tier).unwrap_or_default();
        symbols.sort_by(|a, b| {
            a.symbol
                .to_ascii_uppercase()
                .cmp(&b.symbol.to_ascii_uppercase())
        });
        let values: Vec<Option<i64>> = symbols.iter().map(|s| s.market_value_minor).collect();
        let (current, complete) = financial_domain::account_value::risk_bucket_total(&values);
        groups.push(RiskGroupValueBody {
            risk_tier: tier.to_string(),
            current_minor: current,
            current_complete: complete,
            symbols,
        });
    }
    let totals: Vec<Option<i64>> = groups.iter().map(|g| g.current_minor).collect();
    let (total, totals_ok) = financial_domain::account_value::risk_bucket_total(&totals);
    LiveRisk {
        complete: totals_ok && groups.iter().all(|g| g.current_complete),
        groups,
        total,
    }
}

struct QuoteLine {
    account_id: String,
    account_name: String,
    symbol: String,
    qty: i64,
    qty_scale: u8,
    first_opened_on: Option<String>,
    qty_events: Vec<(String, i64)>,
    risk_idx: usize,
    cash_par: bool,
    quotes: Vec<financial_domain::current_price::QuoteObservation>,
}

impl QuoteLine {
    fn qty_on(&self, day: &str) -> i64 {
        financial_domain::lot::remaining_qty_on_day(
            day,
            self.first_opened_on.as_deref(),
            self.qty,
            &self.qty_events,
        )
    }
}

fn risk_idx_for(tier: &str) -> usize {
    match financial_domain::account_value::risk_bucket(tier) {
        financial_domain::account_value::RISK_FOUNDATION => 0,
        financial_domain::account_value::RISK_CORE => 1,
        financial_domain::account_value::RISK_ON => 2,
        _ => 3,
    }
}

fn mark_line_on_day(line: &QuoteLine, day: &str) -> Option<i64> {
    let (px, scale) = if line.cash_par {
        (
            financial_domain::current_price::CASH_PAR_MINOR,
            financial_domain::current_price::CASH_PAR_SCALE,
        )
    } else {
        financial_domain::current_price::select_price_on_or_before(&line.quotes, day)?
    };
    Some(financial_domain::calculator::plan_payment_cents(
        line.qty_on(day),
        line.qty_scale,
        px,
        scale,
    ))
}

fn mark_line_on_exact_day(line: &QuoteLine, day: &str) -> Option<i64> {
    let (px, scale) = if line.cash_par {
        (
            financial_domain::current_price::CASH_PAR_MINOR,
            financial_domain::current_price::CASH_PAR_SCALE,
        )
    } else {
        financial_domain::current_price::select_price_on_day(&line.quotes, day)?
    };
    Some(financial_domain::calculator::plan_payment_cents(
        line.qty_on(day),
        line.qty_scale,
        px,
        scale,
    ))
}

async fn live_quote_lines(
    canonical: &dyn Canonical,
    details: &PositionDetailsBody,
    characteristics: &[PositionCharacteristicRecord],
) -> Result<Vec<QuoteLine>, crate::ports::platform::PlatformError> {
    let ch_by_sec: std::collections::HashMap<_, _> =
        characteristics.iter().map(|c| (c.security_id, c)).collect();
    let mut events_by_sec: std::collections::HashMap<
        uuid::Uuid,
        Vec<(String, i64)>,
    > = std::collections::HashMap::new();
    if let Ok(events) = canonical.holding_qty_event_list().await {
        for ev in events {
            events_by_sec
                .entry(ev.security_id)
                .or_default()
                .push((ev.occurred_on, ev.remaining_quantity_minor));
        }
    }
    let mut lines = Vec::new();
    for line in &details.positions {
        if !is_data_account(&line.account_name) {
            continue;
        }
        let ch = ch_by_sec.get(&line.security_id);
        let quotes = canonical
            .price_quote_list(line.security_id)
            .await
            .unwrap_or_default();
        let obs = quotes
            .into_iter()
            .map(
                |q: PriceQuoteBody| financial_domain::current_price::QuoteObservation {
                    price_minor: q.price_minor,
                    scale: q.scale,
                    accepted: q.validation_status.eq_ignore_ascii_case("accepted"),
                    as_of: q.as_of_at,
                },
            )
            .collect();
        let qty_events = events_by_sec
            .get(&line.security_id)
            .cloned()
            .unwrap_or_default();
        let first_opened_on = qty_events
            .iter()
            .map(|(on, _)| on.clone())
            .min();
        lines.push(QuoteLine {
            account_id: line.account_id.to_string(),
            account_name: line.account_name.clone(),
            symbol: line.symbol.clone(),
            qty: line.remaining_quantity_minor,
            qty_scale: line.quantity_scale,
            first_opened_on,
            qty_events,
            risk_idx: risk_idx_for(ch.map(|c| c.risk_tier.as_str()).unwrap_or("")),
            cash_par: financial_domain::current_price::uses_cash_par(
                ch.map(|c| c.div_type.as_str()).unwrap_or(""),
                &line.symbol,
            ),
            quotes: obs,
        });
    }
    Ok(lines)
}

fn live_days_from_quotes(
    lines: &[QuoteLine],
    history: &[AccountMarketValueDailyRecord],
    as_of: &str,
) -> Vec<String> {
    use std::collections::BTreeSet;
    let mut days = BTreeSet::new();
    days.insert(as_of.to_string());
    if let Some(yesterday) = prior_iso_day(as_of) {
        days.insert(yesterday);
    }
    for row in history {
        if row.as_of.as_str() <= as_of {
            days.insert(row.as_of.clone());
        }
    }
    for line in lines {
        for q in &line.quotes {
            let day = financial_domain::current_price::quote_day(&q.as_of).to_string();
            if day.as_str() <= as_of {
                days.insert(day);
            }
        }
    }
    days.into_iter().collect()
}

fn points_from_quote_marks(
    lines: &[QuoteLine],
    days: &[String],
    as_of: &str,
    current: Option<i64>,
    include: impl Fn(&QuoteLine) -> bool,
) -> Vec<AccountValuePointBody> {
    let mut points = Vec::new();
    for day in days {
        let values: Vec<Option<i64>> = lines
            .iter()
            .filter(|line| include(line))
            .map(|line| mark_line_on_day(line, day))
            .collect();
        if values.is_empty() && day != as_of {
            continue;
        }
        let (mv, ok) = financial_domain::account_value::risk_bucket_total(&values);
        // Incomplete historical marks are cash/known names only — not the account total.
        if !ok && day.as_str() != as_of {
            continue;
        }
        points.push(AccountValuePointBody {
            as_of: day.clone(),
            market_value_minor: mv,
            market_value_complete: ok,
        });
    }
    overlay_live_point(points, as_of, current)
}

fn quote_mark_days(lines: &[QuoteLine], as_of: &str) -> Vec<String> {
    use std::collections::BTreeSet;
    let mut days = BTreeSet::new();
    for line in lines {
        if line.cash_par {
            continue;
        }
        for q in &line.quotes {
            if !q.accepted {
                continue;
            }
            let day = financial_domain::current_price::quote_day(&q.as_of);
            if day.len() == 10 && day <= as_of {
                days.insert(day.to_string());
            }
        }
    }
    days.into_iter().collect()
}

fn has_session_quote_coverage(lines: &[QuoteLine], day: &str) -> bool {
    use std::collections::BTreeSet;
    let mut priced = BTreeSet::new();
    let mut marked = BTreeSet::new();
    for line in lines {
        if line.cash_par {
            continue;
        }
        priced.insert(line.symbol.as_str());
        if financial_domain::current_price::select_price_on_day(&line.quotes, day).is_some() {
            marked.insert(line.symbol.as_str());
        }
    }
    !priced.is_empty() && marked.len() * 5 >= priced.len() * 4
}

fn reconstruct_live_risk_on_day(lines: &[QuoteLine], day: &str) -> Option<LiveRisk> {
    use std::collections::BTreeMap;
    let mut any_exact_equity = false;
    let mut by_symbol: BTreeMap<String, (usize, Option<i64>, bool)> = BTreeMap::new();
    for line in lines {
        let exact = mark_line_on_exact_day(line, day);
        if !line.cash_par && exact.is_some() {
            any_exact_equity = true;
        }
        let entry = by_symbol
            .entry(line.symbol.clone())
            .or_insert((line.risk_idx, Some(0), true));
        entry.0 = line.risk_idx;
        match exact {
            Some(mv) => {
                if let Some(cur) = entry.1.as_mut() {
                    *cur += mv;
                }
            }
            None => {
                entry.1 = None;
                entry.2 = false;
            }
        }
    }
    if !any_exact_equity {
        return None;
    }
    let mut groups = Vec::new();
    for (idx, tier) in financial_domain::account_value::RISK_TIERS
        .iter()
        .enumerate()
    {
        let mut symbols: Vec<RiskSymbolValueBody> = by_symbol
            .iter()
            .filter(|(_, (risk_idx, _, _))| *risk_idx == idx)
            .map(|(symbol, (_, mv, complete))| RiskSymbolValueBody {
                symbol: symbol.clone(),
                risk_tier: (*tier).to_string(),
                market_value_minor: *mv,
                market_value_complete: *complete,
            })
            .collect();
        symbols.sort_by(|a, b| {
            a.symbol
                .to_ascii_uppercase()
                .cmp(&b.symbol.to_ascii_uppercase())
        });
        let values: Vec<Option<i64>> = symbols.iter().map(|s| s.market_value_minor).collect();
        let (current, complete) = financial_domain::account_value::risk_bucket_total(&values);
        groups.push(RiskGroupValueBody {
            risk_tier: (*tier).to_string(),
            current_minor: current,
            current_complete: complete,
            symbols,
        });
    }
    let totals: Vec<Option<i64>> = groups.iter().map(|g| g.current_minor).collect();
    let (total, totals_ok) = financial_domain::account_value::risk_bucket_total(&totals);
    Some(LiveRisk {
        complete: totals_ok && groups.iter().all(|g| g.current_complete),
        groups,
        total,
    })
}

async fn backfill_risk_captures_from_quotes(
    canonical: &dyn Canonical,
    history: Vec<AccountMarketValueDailyRecord>,
    quote_lines: &[QuoteLine],
    as_of: &str,
) -> Result<Vec<AccountMarketValueDailyRecord>, crate::ports::platform::PlatformError> {
    let captured_at = chrono::Utc::now().to_rfc3339();
    let mut wrote = false;
    for day in quote_mark_days(quote_lines, as_of) {
        if day == as_of {
            continue;
        }
        if !has_session_quote_coverage(quote_lines, &day) {
            continue;
        }
        let Some(live) = reconstruct_live_risk_on_day(quote_lines, &day) else {
            continue;
        };
        persist_risk_snapshot(canonical, &day, &captured_at, &live).await?;
        wrote = true;
    }
    if wrote {
        canonical.account_market_value_daily_list().await
    } else {
        Ok(history)
    }
}

fn stored_risk_on_day(
    history: &[AccountMarketValueDailyRecord],
    day: &str,
    series_id: &str,
) -> (Option<i64>, bool) {
    history
        .iter()
        .filter(|row| row.account_id == series_id && row.as_of == day)
        .max_by(|a, b| a.captured_at.cmp(&b.captured_at))
        .map(|row| (row.market_value_minor, row.market_value_complete))
        .unwrap_or((None, false))
}

fn risk_capture_days(
    history: &[AccountMarketValueDailyRecord],
    as_of: &str,
    quote_lines: &[QuoteLine],
) -> Vec<String> {
    use std::collections::BTreeSet;
    let mut days = BTreeSet::new();
    days.insert(as_of.to_string());
    for row in history {
        if financial_domain::account_value::parse_risk_series_id(&row.account_id).is_some()
            && row.as_of.as_str() <= as_of
            && (row.as_of == as_of || has_session_quote_coverage(quote_lines, &row.as_of))
        {
            days.insert(row.as_of.clone());
        }
    }
    days.into_iter().collect()
}

fn live_risk_point(day: &str, live: &LiveRisk) -> RiskValuePointBody {
    let live_vals = |tier: &str| -> (Option<i64>, bool) {
        live.groups
            .iter()
            .find(|g| g.risk_tier == tier)
            .map(|g| (g.current_minor, g.current_complete))
            .unwrap_or((Some(0), true))
    };
    let foundation = live_vals(financial_domain::account_value::RISK_FOUNDATION);
    let core = live_vals(financial_domain::account_value::RISK_CORE);
    let risk_on = live_vals(financial_domain::account_value::RISK_ON);
    let undecided = live_vals(financial_domain::account_value::RISK_UNDECIDED);
    let (total, ok) = financial_domain::account_value::risk_bucket_total(&[
        foundation.0,
        core.0,
        risk_on.0,
        undecided.0,
    ]);
    RiskValuePointBody {
        as_of: day.to_string(),
        foundation_minor: foundation.0,
        core_minor: core.0,
        risk_on_minor: risk_on.0,
        undecided_minor: undecided.0,
        total_minor: total,
        market_value_complete: foundation.1 && core.1 && risk_on.1 && undecided.1 && ok,
    }
}

fn risk_points_from_captures(
    history: &[AccountMarketValueDailyRecord],
    as_of: &str,
    live: &LiveRisk,
    quote_lines: &[QuoteLine],
) -> Vec<RiskValuePointBody> {
    risk_capture_days(history, as_of, quote_lines)
        .into_iter()
        .map(|day| {
            if day == as_of {
                return live_risk_point(&day, live);
            }
            let foundation = stored_risk_on_day(
                history,
                &day,
                financial_domain::account_value::RISK_FOUNDATION_ID,
            );
            let core =
                stored_risk_on_day(history, &day, financial_domain::account_value::RISK_CORE_ID);
            let risk_on =
                stored_risk_on_day(history, &day, financial_domain::account_value::RISK_ON_ID);
            let undecided = stored_risk_on_day(
                history,
                &day,
                financial_domain::account_value::RISK_UNDECIDED_ID,
            );
            let (total, ok) = financial_domain::account_value::risk_bucket_total(&[
                foundation.0,
                core.0,
                risk_on.0,
                undecided.0,
            ]);
            RiskValuePointBody {
                as_of: day,
                foundation_minor: foundation.0,
                core_minor: core.0,
                risk_on_minor: risk_on.0,
                undecided_minor: undecided.0,
                total_minor: total,
                market_value_complete: foundation.1 && core.1 && risk_on.1 && undecided.1 && ok,
            }
        })
        .collect()
}

async fn persist_risk_snapshot(
    canonical: &dyn Canonical,
    as_of: &str,
    captured_at: &str,
    live: &LiveRisk,
) -> Result<(), crate::ports::platform::PlatformError> {
    for group in &live.groups {
        let id = financial_domain::account_value::risk_series_id(&group.risk_tier);
        canonical
            .account_market_value_daily_upsert(AccountMarketValueDailyRecord {
                snapshot_id: uuid::Uuid::new_v4().to_string(),
                account_id: id.to_string(),
                account_name: financial_domain::account_value::risk_series_name(&group.risk_tier),
                as_of: as_of.to_string(),
                market_value_minor: group.current_minor,
                market_value_complete: group.current_complete,
                scale: 2,
                captured_at: captured_at.to_string(),
            })
            .await?;
    }
    Ok(())
}

fn income_points_for_account(
    account_name: &str,
    buckets: &std::collections::BTreeMap<(String, chrono::NaiveDate), i64>,
) -> Vec<AccountIncomePointBody> {
    let mut points: Vec<AccountIncomePointBody> = buckets
        .iter()
        .filter(|((name, _), _)| name == account_name)
        .map(|((_, friday), amount)| AccountIncomePointBody {
            as_of: friday.format("%Y-%m-%d").to_string(),
            income_minor: Some(*amount),
        })
        .collect();
    points.sort_by(|a, b| a.as_of.cmp(&b.as_of));
    points
}

fn income_points_for_custodian(
    custodian: &str,
    buckets: &std::collections::BTreeMap<(String, chrono::NaiveDate), i64>,
) -> Vec<AccountIncomePointBody> {
    let mut by_friday: std::collections::BTreeMap<chrono::NaiveDate, i64> =
        std::collections::BTreeMap::new();
    for ((name, friday), amount) in buckets {
        if financial_domain::account_value::account_custodian(name) == custodian {
            *by_friday.entry(*friday).or_insert(0) += *amount;
        }
    }
    by_friday
        .into_iter()
        .map(|(friday, amount)| AccountIncomePointBody {
            as_of: friday.format("%Y-%m-%d").to_string(),
            income_minor: Some(amount),
        })
        .collect()
}

async fn weekly_actual_buckets(
    canonical: &dyn crate::ports::canonical::Canonical,
    accounts: &[AccountRecord],
    as_of: &str,
) -> std::collections::BTreeMap<(String, chrono::NaiveDate), i64> {
    let as_of_day = financial_domain::week::parse_iso_day(as_of)
        .unwrap_or_else(|| chrono::Local::now().date_naive());
    let mut buckets = std::collections::BTreeMap::<(String, chrono::NaiveDate), i64>::new();
    let mut add = |account_name: &str, occurred_on: &str, amount_minor: i64| {
        if let Some(friday) =
            financial_domain::account_value::closed_week_friday(occurred_on, as_of_day)
        {
            *buckets
                .entry((account_name.to_string(), friday))
                .or_insert(0) += amount_minor;
        }
    };
    if let Ok(div) = canonical.dividend_get().await {
        for actual in &div.actuals {
            if let Some(acct) = accounts.iter().find(|a| a.account_id == actual.account_id) {
                add(&acct.name, &actual.occurred_on, actual.amount_minor);
            }
        }
    }
    if let Ok(activities) = canonical.activity_list().await {
        for act in activities {
            if !financial_domain::income_plan::is_income_cash_activity(&act.activity_type)
                || !act.activity_type.eq_ignore_ascii_case("interest")
            {
                continue;
            }
            if let Some(acct) = accounts.iter().find(|a| a.account_id == act.account_id) {
                add(&acct.name, &act.occurred_on, act.amount_minor);
            }
        }
    }
    buckets
}

fn trends_points_for(name: &str, weeks: &[TrendsWeekPoint]) -> Vec<AccountValuePointBody> {
    weeks
        .iter()
        .filter_map(|week| {
            financial_domain::account_value::trends_balance_for_account(
                name,
                week.fidelity_total_minor,
                week.schwab_total_minor,
                week.income_balance_minor,
                week.car_balance_minor,
                week.health_balance_minor,
                week.roth_balance_minor,
                week.speculation_balance_minor,
            )
            .map(|market_value_minor| AccountValuePointBody {
                as_of: week.period_end.clone(),
                market_value_minor: Some(market_value_minor),
                market_value_complete: true,
            })
        })
        .collect()
}

pub(crate) async fn account_value_home_view(
    canonical: &dyn crate::ports::canonical::Canonical,
    as_of: &str,
) -> Result<AccountValueHomeBody, crate::ports::platform::PlatformError> {
    let _ = crate::production_seed::ensure_holding_qty_history(canonical).await;
    let details =
        position_details_summary(canonical, &serde_json::json!({ "asOfDate": as_of })).await?;
    let accounts = canonical.account_list().await?;
    let history = canonical.account_market_value_daily_list().await?;
    let weeks = account_trends_weeks(canonical).await?;
    let characteristics = canonical.position_characteristic_list().await?;
    let live_risk = live_risk_from_details(&details, &characteristics);
    let quote_lines = live_quote_lines(canonical, &details, &characteristics).await?;
    let history =
        backfill_risk_captures_from_quotes(canonical, history, &quote_lines, as_of).await?;
    let mark_days = live_days_from_quotes(&quote_lines, &history, as_of);
    let risk_points = risk_points_from_captures(&history, as_of, &live_risk, &quote_lines);
    let income_buckets = weekly_actual_buckets(canonical, &accounts, as_of).await;
    let mut series = Vec::new();
    let mut live_rows: Vec<(String, Option<i64>)> = Vec::new();
    for acct in accounts.iter().filter(|a| is_data_account(&a.name)) {
        let current = details
            .account_totals
            .iter()
            .find(|t| t.account_id == acct.account_id)
            .map(|t| t.market_value_minor)
            .unwrap_or(Some(0));
        live_rows.push((acct.name.clone(), current));
        let account_id = acct.account_id.to_string();
        let points = points_from_quote_marks(&quote_lines, &mark_days, as_of, current, |line| {
            line.account_id == account_id
        });
        series.push(AccountValueSeriesBody {
            account_id: acct.account_id.to_string(),
            account_name: acct.name.clone(),
            custodian: financial_domain::account_value::account_custodian(&acct.name).to_string(),
            current_minor: current,
            current_complete: current.is_some(),
            points,
            trends_points: trends_points_for(&acct.name, &weeks),
            income_points: if financial_domain::account_value::shows_weekly_actuals(&acct.name) {
                income_points_for_account(&acct.name, &income_buckets)
            } else {
                Vec::new()
            },
            scale: 2,
        });
    }
    series.sort_by(|a, b| {
        a.account_name
            .to_ascii_lowercase()
            .cmp(&b.account_name.to_ascii_lowercase())
    });
    let (fid_mv, fid_ok) =
        financial_domain::account_value::fidelity_total_from_accounts(&live_rows);
    let (sch_mv, sch_ok) = financial_domain::account_value::schwab_total_from_accounts(&live_rows);
    Ok(AccountValueHomeBody {
        as_of: as_of.to_string(),
        accounts: series,
        fidelity: AccountValueSeriesBody {
            account_id: financial_domain::account_value::FIDELITY_TOTAL_ID.to_string(),
            account_name: financial_domain::account_value::FIDELITY_TOTAL_NAME.to_string(),
            custodian: "Fidelity".into(),
            current_minor: fid_mv,
            current_complete: fid_ok,
            points: points_from_quote_marks(
                &quote_lines,
                &mark_days,
                as_of,
                fid_mv,
                |line| financial_domain::account_value::is_fidelity_holdings_account(&line.account_name),
            ),
            trends_points: trends_points_for(
                financial_domain::account_value::FIDELITY_TOTAL_NAME,
                &weeks,
            ),
            income_points: income_points_for_custodian("Fidelity", &income_buckets),
            scale: 2,
        },
        schwab: AccountValueSeriesBody {
            account_id: financial_domain::account_value::SCHWAB_TOTAL_ID.to_string(),
            account_name: financial_domain::account_value::SCHWAB_TOTAL_NAME.to_string(),
            custodian: "Schwab".into(),
            current_minor: sch_mv,
            current_complete: sch_ok,
            points: points_from_quote_marks(
                &quote_lines,
                &mark_days,
                as_of,
                sch_mv,
                |line| financial_domain::account_value::is_schwab_holdings_account(&line.account_name),
            ),
            trends_points: trends_points_for(
                financial_domain::account_value::SCHWAB_TOTAL_NAME,
                &weeks,
            ),
            income_points: income_points_for_custodian("Schwab", &income_buckets),
            scale: 2,
        },
        risk: RiskValueHomeBody {
            current_total_minor: live_risk.total,
            current_complete: live_risk.complete,
            groups: live_risk.groups,
            points: risk_points,
            scale: 2,
        },
        weeks,
        note: "Solid line is live holdings (qty Ã— last price). Dashed line is stored Trends weeks. Dotted line is weekly actuals on the Friday week-end. Missing stays unknown."
            .into(),
        scale: 2,
    })
}
