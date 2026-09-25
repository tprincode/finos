//! Coverage: forward plan income versus forward planned withdrawals.
//! Year = amount per period × periods still ahead in the next 12 months.
//! Week = that year / 52. Month = that year / 12.

use std::collections::HashMap;

use crate::contracts::{
    CashCoverageBody, CashCoverageExpenseLine, CashCoverageIncomeLine, CashCoverageRow,
    PlanHistoryRecord,
};
use crate::ports::canonical::Canonical;
use crate::ports::platform::PlatformError;
use chrono::{Months, NaiveDate};
use financial_domain::trends::parse_iso_date;
use uuid::Uuid;

const COVERAGE_BOOKS: &[&str] = &[
    "Income",
    "FI Roth",
    "Car",
    "Health",
    "Account 9",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum PeriodKind {
    Week,
    Month,
    Year,
}

fn parse_kind(raw: &str) -> PeriodKind {
    match raw.trim().to_ascii_lowercase().as_str() {
        "month" => PeriodKind::Month,
        "year" => PeriodKind::Year,
        _ => PeriodKind::Week,
    }
}

fn kind_label(kind: PeriodKind) -> &'static str {
    match kind {
        PeriodKind::Week => "week",
        PeriodKind::Month => "month",
        PeriodKind::Year => "year",
    }
}

fn scale_year(year: i64, kind: PeriodKind) -> i64 {
    match kind {
        PeriodKind::Week => year / 52,
        PeriodKind::Month => year / 12,
        PeriodKind::Year => year,
    }
}

fn is_deposit_kind(kind: &str) -> bool {
    kind.eq_ignore_ascii_case("Deposit") || kind.eq_ignore_ascii_case("SSA")
}

fn cadence_norm(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "week" | "weekly" => "weekly".into(),
        "month" | "monthly" => "monthly".into(),
        "year" | "yearly" | "annual" => "annual".into(),
        "one-time" | "onetime" | "once" => "one-time".into(),
        other => other.into(),
    }
}

fn iso(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

fn parse_day(raw: &str) -> Option<NaiveDate> {
    let raw = raw.trim();
    if raw.len() < 10 {
        return None;
    }
    parse_iso_date(&raw[..10])
}

/// Periods of this cadence from `as_of` through 12 months, clipped by start and stop.
/// An open-ended weekly plan is 52. A stop inside the year counts only the periods still left.
pub fn periods_ahead(cadence: &str, as_of: NaiveDate, start_on: &str, stop_on: &str) -> i64 {
    let horizon = as_of.checked_add_months(Months::new(12)).unwrap_or(as_of);
    let start = parse_day(start_on)
        .filter(|d| *d > as_of)
        .unwrap_or(as_of);
    let stop = parse_day(stop_on).unwrap_or(horizon);
    if stop < start {
        return 0;
    }
    let end = stop.min(horizon);
    let open = parse_day(stop_on).map(|d| d >= horizon).unwrap_or(true) && start <= as_of;
    match cadence_norm(cadence).as_str() {
        "weekly" if open => 52,
        "monthly" if open => 12,
        "annual" if open => 1,
        "weekly" => ((end - start).num_days() / 7).max(0),
        "monthly" => month_steps(start, end),
        "annual" => i64::from(end > start),
        _ => 0,
    }
}

fn month_steps(start: NaiveDate, end: NaiveDate) -> i64 {
    let mut n = 0i64;
    let mut cursor = start;
    while cursor <= end && n < 12 {
        n += 1;
        cursor = cursor
            .checked_add_months(Months::new(1))
            .unwrap_or(cursor);
        if cursor == start {
            break;
        }
    }
    n
}

fn current_plan<'a>(
    plans: &'a [PlanHistoryRecord],
    security_id: Uuid,
    as_of: &str,
) -> Option<&'a PlanHistoryRecord> {
    plans
        .iter()
        .filter(|p| {
            p.security_id == security_id
                && p.effective_from.as_str() <= as_of
                && (p.effective_to.is_empty() || p.effective_to.as_str() >= as_of)
        })
        .max_by(|a, b| a.effective_from.cmp(&b.effective_from))
}

struct YearBucket {
    income: i64,
    income_known: bool,
    expense: i64,
}

pub async fn cash_coverage_get(
    canonical: &dyn Canonical,
    as_of: &str,
    period: &str,
) -> Result<CashCoverageBody, PlatformError> {
    let kind = parse_kind(period);
    let as_of_d = parse_iso_date(as_of)
        .ok_or_else(|| PlatformError::new("bad_date", format!("invalid asOfDate {as_of}")))?;
    let horizon = as_of_d
        .checked_add_months(Months::new(12))
        .unwrap_or(as_of_d);

    let basis = canonical.basis_get().await?;
    let accounts = canonical.account_list().await?;
    let securities = canonical.security_list().await?;
    let plans = canonical.plan_history_list().await.unwrap_or_default();
    let elements = canonical.cash_element_list().await.unwrap_or_default();
    let occurrences = canonical.planned_occurrence_list().await.unwrap_or_default();

    let mut buckets: HashMap<&str, YearBucket> = HashMap::new();
    for book in COVERAGE_BOOKS {
        buckets.insert(book, YearBucket { income: 0, income_known: false, expense: 0 });
    }
    let mut income_lines: Vec<CashCoverageIncomeLine> = Vec::new();
    let mut expense_lines: Vec<CashCoverageExpenseLine> = Vec::new();

    let mut grouped: HashMap<(String, String), (i64, i64)> = HashMap::new();
    for lot in &basis.lots {
        if lot.remaining_quantity_minor <= 0 {
            continue;
        }
        let Some(account) = accounts.iter().find(|a| a.account_id == lot.account_id) else {
            continue;
        };
        let Some(book) = COVERAGE_BOOKS
            .iter()
            .copied()
            .find(|book| {
                financial_domain::cash_management::book_matches_income_plan_account(
                    book,
                    &account.name,
                )
            })
        else {
            continue;
        };
        let symbol = securities
            .iter()
            .find(|s| s.security_id == lot.security_id)
            .map(|s| s.symbol.clone())
            .unwrap_or_else(|| "—".into());
        let Some(plan) = current_plan(&plans, lot.security_id, as_of) else {
            income_lines.push(CashCoverageIncomeLine {
                account: book.into(),
                symbol,
                per_period_minor: None,
                periods: None,
                year_minor: None,
            });
            continue;
        };
        if plan.planning_periods_per_year == 0 || plan.amount_per_share_minor == 0 {
            income_lines.push(CashCoverageIncomeLine {
                account: book.into(),
                symbol,
                per_period_minor: None,
                periods: None,
                year_minor: None,
            });
            continue;
        }
        let per = financial_domain::calculator::plan_payment_cents(
            lot.remaining_quantity_minor,
            lot.quantity_scale,
            plan.amount_per_share_minor,
            plan.amount_scale,
        );
        let periods = i64::from(plan.planning_periods_per_year);
        let entry = grouped.entry((book.to_string(), symbol)).or_insert((0, periods));
        entry.0 += per;
        entry.1 = periods;
    }
    for ((book, symbol), (per, periods)) in grouped {
        let year = per.saturating_mul(periods);
        if let Some(bucket) = buckets.get_mut(book.as_str()) {
            bucket.income += year;
            bucket.income_known = true;
        }
        income_lines.push(CashCoverageIncomeLine {
            account: book,
            symbol,
            per_period_minor: Some(per),
            periods: Some(periods),
            year_minor: Some(year),
        });
    }
    income_lines.sort_by(|a, b| a.account.cmp(&b.account).then(a.symbol.cmp(&b.symbol)));

    for elem in &elements {
        let Some(book) =
            financial_domain::cash_management::register_book_label(&elem.account)
        else {
            continue;
        };
        if !COVERAGE_BOOKS.contains(&book) || is_deposit_kind(&elem.kind) {
            continue;
        }
        let cadence = cadence_norm(&elem.cadence);
        if cadence == "one-time" {
            continue;
        }
        let periods = periods_ahead(&cadence, as_of_d, &elem.start_on, &elem.stop_on);
        if periods <= 0 || elem.amount_minor == 0 {
            continue;
        }
        let year = elem.amount_minor.abs().saturating_mul(periods);
        if let Some(bucket) = buckets.get_mut(book) {
            bucket.expense += year;
        }
        let name = if elem.note.trim().is_empty() {
            elem.kind.clone()
        } else {
            elem.note.clone()
        };
        expense_lines.push(CashCoverageExpenseLine {
            account: book.into(),
            name,
            cadence,
            per_period_minor: elem.amount_minor.abs(),
            periods,
            year_minor: year,
        });
    }
    for o in &occurrences {
        if o.is_cancelled {
            continue;
        }
        let Some(on) = parse_day(&o.occurred_on) else {
            continue;
        };
        if on < as_of_d || on > horizon {
            continue;
        }
        let Some(elem) = elements.iter().find(|e| e.element_id == o.element_id) else {
            continue;
        };
        if cadence_norm(&elem.cadence) != "one-time" || is_deposit_kind(&elem.kind) {
            continue;
        }
        let Some(book) =
            financial_domain::cash_management::register_book_label(&o.account)
        else {
            continue;
        };
        if !COVERAGE_BOOKS.contains(&book) {
            continue;
        }
        let amount = o.amount_minor.abs();
        if amount == 0 {
            continue;
        }
        if let Some(bucket) = buckets.get_mut(book) {
            bucket.expense += amount;
        }
        expense_lines.push(CashCoverageExpenseLine {
            account: book.into(),
            name: if elem.note.trim().is_empty() {
                format!("One-time {}", o.occurred_on)
            } else {
                format!("{} {}", elem.note, o.occurred_on)
            },
            cadence: "one-time".into(),
            per_period_minor: amount,
            periods: 1,
            year_minor: amount,
        });
    }
    expense_lines.sort_by(|a, b| a.account.cmp(&b.account).then(a.name.cmp(&b.name)));

    let mut rows = Vec::with_capacity(COVERAGE_BOOKS.len() + 1);
    let mut tot_income = 0i64;
    let mut any_income = false;
    let mut tot_expense = 0i64;
    for book in COVERAGE_BOOKS {
        let bucket = buckets.get(book).expect("book");
        let income = bucket.income_known.then_some(bucket.income);
        let expense = bucket.expense;
        let income_shown = income.map(|v| scale_year(v, kind));
        let expense_shown = scale_year(expense, kind);
        let margin = income_shown.map(|inc| inc - expense_shown);
        if let Some(v) = income {
            tot_income += v;
            any_income = true;
        }
        tot_expense += expense;
        rows.push(CashCoverageRow {
            account: (*book).into(),
            plan_income_minor: income_shown,
            plan_expense_minor: Some(expense_shown),
            plan_minor: margin,
            average_minor: None,
            actual_income_minor: None,
            actual_expense_minor: None,
            actual_minor: None,
            declared_minor: None,
            lookback_plan_minor: None,
            delta_minor: margin,
            variance_bps: None,
        });
    }
    let tot_income = any_income.then_some(scale_year(tot_income, kind));
    let tot_expense = scale_year(tot_expense, kind);
    rows.push(CashCoverageRow {
        account: "Total".into(),
        plan_income_minor: tot_income,
        plan_expense_minor: Some(tot_expense),
        plan_minor: tot_income.map(|inc| inc - tot_expense),
        average_minor: None,
        actual_income_minor: None,
        actual_expense_minor: None,
        actual_minor: None,
        declared_minor: None,
        lookback_plan_minor: None,
        delta_minor: tot_income.map(|inc| inc - tot_expense),
        variance_bps: None,
    });

    let start = iso(as_of_d);
    let end = iso(horizon);
    Ok(CashCoverageBody {
        as_of_date: as_of.to_string(),
        period: kind_label(kind).into(),
        period_start: start.clone(),
        period_end: end.clone(),
        lookback_start: start,
        lookback_end: end,
        rows,
        income_lines,
        expense_lines,
        scale: 2,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_weekly_is_52_and_monthly_is_12() {
        let as_of = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();
        assert_eq!(periods_ahead("weekly", as_of, "", ""), 52);
        assert_eq!(periods_ahead("monthly", as_of, "", ""), 12);
        assert_eq!(periods_ahead("annual", as_of, "", ""), 1);
    }

    #[test]
    fn monthly_year_divided_by_52_matches_three_thirteenths() {
        let year = 85_000i64 * 12;
        assert_eq!(year / 52, 19_615);
        assert_eq!(20_000i64 * 12 / 52, 4_615);
    }

    #[test]
    fn stopped_plan_counts_only_periods_still_left() {
        let as_of = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();
        assert_eq!(periods_ahead("weekly", as_of, "", "2026-09-01"), 0);
        let left = periods_ahead("weekly", as_of, "", "2026-12-18");
        assert!(left > 0 && left < 52, "{left}");
    }
}
