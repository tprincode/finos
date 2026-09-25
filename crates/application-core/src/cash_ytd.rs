//! Slice 4 YTD actual + remaining plan by Register book or tax type.

use std::collections::HashMap;

use crate::contracts::{CashYtdBody, CashYtdRow};
use crate::ports::canonical::Canonical;
use crate::ports::platform::PlatformError;
use crate::week_ahead::{ensure_horizon, ensure_seed};
use chrono::Datelike;
use financial_domain::cash_management::{
    book_matches_income_plan_account, is_cash_adjust_type, is_cash_distribution_type,
    job_1099_year_amounts, register_book_label, register_ledger_account, REGISTER_BOOKS,
};
use financial_domain::trends::parse_iso_date;

/// Tax-type YTD is reportable income. Car cash-outs stay on the Account view
/// only — Car tax year is ROC (nontaxable) + ordinary dividends on Tax Planning.
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

async fn income_plan_remaining_by_book(
    canonical: &dyn Canonical,
    start: &str,
    end: &str,
    as_of: &str,
) -> Result<HashMap<&'static str, i64>, PlatformError> {
    let start_d = parse_iso_date(start)
        .ok_or_else(|| PlatformError::new("bad_date", format!("invalid start {start}")))?;
    let end_d = parse_iso_date(end)
        .ok_or_else(|| PlatformError::new("bad_date", format!("invalid end {end}")))?;
    if start_d > end_d {
        return Ok(HashMap::new());
    }
    let mut by_book: HashMap<&'static str, i64> = HashMap::new();
    let views = crate::queries::income_plan_week_views_in_range(canonical, start, end, as_of).await?;
    for view in &views {
        for pos in &view.positions {
            if !pos.plan_known || pos.planned_minor == 0 {
                continue;
            }
            if pos.pay_on.is_empty() || pos.pay_on.as_str() < start || pos.pay_on.as_str() > end {
                continue;
            }
            for book in REGISTER_BOOKS {
                if *book == "SSA_2026" {
                    continue;
                }
                let day_sum: i64 = pos
                    .accounts
                    .iter()
                    .filter(|a| {
                        a.plan_known && book_matches_income_plan_account(book, &a.account_name)
                    })
                    .map(|a| a.planned_minor)
                    .sum();
                if day_sum > 0 {
                    *by_book.entry(*book).or_insert(0) += day_sum;
                }
            }
        }
    }
    Ok(by_book)
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
    let remaining_start = next_day(as_of)?;
    let plan_remaining =
        income_plan_remaining_by_book(canonical, &remaining_start, dec31, as_of).await?;
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
            if o.confirmed_at.is_none()
                && o.occurred_on.as_str() > as_of
                && o.occurred_on.as_str() <= dec31
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
        if let Some(plan) = plan_remaining.get(book) {
            remaining += *plan;
            remaining_known = true;
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
                let actual = roll.map(|r| r.actual).unwrap_or(0);
                let remaining = roll.and_then(|r| r.remaining);
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
