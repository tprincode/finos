//! Income Plan week grid and Dashboard burndown account identity (domain docs + BR-X).
//! Plan amounts are not inferred. Missing plan stays unknown — never zero.

use chrono::{Datelike, Duration, Months, NaiveDate};

use crate::calculator::PaymentCadence;
use crate::plan::PlanAmountWindow;
use crate::week::{week_containing, CanonicalWeek};

pub const CONTROL_ACCOUNTS: [&str; 5] = ["Income", "Health", "Roth", "Account 9", "Car"];
pub const BURNDOWN_ACCOUNTS: [&str; 4] = ["Income", "Car", "Health", "Roth"];

/// Pattern A chips that start on. Session-only; never persisted as owner identity.
pub const DEFAULT_ACCOUNTS_ON: [&str; 5] = ["Income", "Car", "Health", "FI Roth", "9"];
/// Pattern A chips that start off.
pub const DEFAULT_ACCOUNTS_OFF: [&str; 3] = ["Speculation", "Energy", "Robinhood"];
/// Table 1 account-detail order (plan block then actual block).
pub const TABLE1_ACCOUNT_ORDER: [&str; 5] = ["Income", "Health", "FI Roth", "9", "Car"];
pub const WEEK_COUNT_DEFAULT: u32 = 6;
pub const WEEK_COUNT_MAX: u32 = 26;
/// Scale-2 cents: past actual within this of plan is green, not amber.
pub const TABLE2_OK_TOLERANCE_MINOR: i64 = 100;

/// Percent of plan as scale-2 (10000 = 100.00%). None if plan unknown or plan is 0 — never 0%.
pub fn pct_of_plan_minor(actual_minor: i64, plan_known: bool, planned_minor: i64) -> Option<i64> {
    if !plan_known || planned_minor == 0 {
        return None;
    }
    Some(actual_minor.saturating_mul(10_000) / planned_minor)
}

/// Inclusive start of a performance range. `None` means unbounded (`all`).
pub fn performance_range_start(as_of: NaiveDate, range: &str) -> Result<Option<NaiveDate>, &'static str> {
    match range {
        "30d" => Ok(Some(as_of - Duration::days(30))),
        "60d" => Ok(Some(as_of - Duration::days(60))),
        "90d" => Ok(Some(as_of - Duration::days(90))),
        "1m" => as_of
            .checked_sub_months(Months::new(1))
            .map(Some)
            .ok_or("range_overflow"),
        "2m" => as_of
            .checked_sub_months(Months::new(2))
            .map(Some)
            .ok_or("range_overflow"),
        "3m" => as_of
            .checked_sub_months(Months::new(3))
            .map(Some)
            .ok_or("range_overflow"),
        "ytd" => Ok(NaiveDate::from_ymd_opt(as_of.year(), 1, 1)),
        "all" => Ok(None),
        _ => Err("unknown_range"),
    }
}

/// Map an account name in the data file onto the Income Plan control grid, if it belongs.
pub fn map_control_account(name: &str) -> Option<&'static str> {
    let n = name.trim().to_ascii_lowercase();
    if n == "income" {
        return Some("Income");
    }
    if n.contains("health") || n == "hsa" {
        return Some("Health");
    }
    if n.contains("roth") {
        return Some("Roth");
    }
    if n.contains("account 9") || n == "9" {
        return Some("Account 9");
    }
        if n == "car" || n.starts_with("car ") || n.ends_with(" car") {
        return Some("Car");
    }
    None
}

/// Income Plan selectable identity, including default-off accounts.
/// Dashboard burndown still uses [`map_control_account`].
pub fn map_income_plan_account(name: &str) -> Option<&'static str> {
    if let Some(control) = map_control_account(name) {
        return Some(control);
    }
    let n = name.trim().to_ascii_lowercase();
    if n == "speculation" {
        return Some("Speculation");
    }
    if n == "energy" {
        return Some("Energy");
    }
    if n == "robinhood" {
        return Some("Robinhood");
    }
    None
}

/// Owner-facing Table 1 / chip label for a control or optional account key.
pub fn display_account_label(control: &str) -> String {
    match control {
        "Roth" => "FI Roth".into(),
        "Account 9" => "9".into(),
        other => other.to_string(),
    }
}

/// Map a chip / filter label onto the internal account key.
pub fn account_key_from_display(label: &str) -> Option<&'static str> {
    map_income_plan_account(label).or_else(|| {
        let n = label.trim().to_ascii_lowercase();
        if n == "fi roth" || n == "roth" {
            Some("Roth")
        } else if n == "9" || n == "account 9" {
            Some("Account 9")
        } else {
            None
        }
    })
}

pub fn is_default_on_account(display: &str) -> bool {
    DEFAULT_ACCOUNTS_ON
        .iter()
        .any(|n| n.eq_ignore_ascii_case(display.trim()))
}

pub fn clamp_week_count(n: i64) -> u32 {
    n.clamp(0, WEEK_COUNT_MAX as i64) as u32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridWeekKind {
    Closed,
    InProgress,
    Future,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridWeek {
    pub week: CanonicalWeek,
    pub kind: GridWeekKind,
}

/// Visible Pattern A weeks: N closed historical, the in-progress week, N future.
/// In-progress is never counted as future.
pub fn visible_grid_weeks(as_of: NaiveDate, historical: u32, future: u32) -> Vec<GridWeek> {
    let hist = historical.min(WEEK_COUNT_MAX);
    let fut = future.min(WEEK_COUNT_MAX);
    let current = week_containing(as_of);
    let mut out = Vec::with_capacity((hist + fut + 1) as usize);
    for i in (1..=hist).rev() {
        let end = current.end - Duration::days(7 * i as i64);
        let start = end - Duration::days(6);
        out.push(GridWeek {
            week: CanonicalWeek { start, end },
            kind: GridWeekKind::Closed,
        });
    }
    let in_progress_closed = current.end < as_of;
    out.push(GridWeek {
        week: current,
        kind: if in_progress_closed {
            GridWeekKind::Closed
        } else {
            GridWeekKind::InProgress
        },
    });
    for i in 1..=fut {
        let end = current.end + Duration::days(7 * i as i64);
        let start = end - Duration::days(6);
        out.push(GridWeek {
            week: CanonicalWeek { start, end },
            kind: GridWeekKind::Future,
        });
    }
    out
}

/// Actual / Difference are known only after the Friday has passed.
pub fn actuals_known(kind: GridWeekKind) -> bool {
    matches!(kind, GridWeekKind::Closed)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Table2Tone {
    Empty,
    Future,
    Ok,
    Variance,
    Miss,
}

/// Table 2 cell color. Table 1 account rows must not use Miss.
pub fn table2_cell_tone(
    planned_minor: Option<i64>,
    actual_minor: Option<i64>,
    kind: GridWeekKind,
) -> Table2Tone {
    let plan = planned_minor.filter(|p| *p != 0);
    let paid = actual_minor.filter(|a| *a != 0);
    if plan.is_none() && paid.is_none() {
        return Table2Tone::Empty;
    }
    if !actuals_known(kind) {
        return Table2Tone::Future;
    }
    if plan.is_some() && paid.is_none() {
        return Table2Tone::Miss;
    }
    let plan_amt = planned_minor.unwrap_or(0);
    let actual_amt = actual_minor.unwrap_or(0);
    if (actual_amt - plan_amt).abs() <= TABLE2_OK_TOLERANCE_MINOR {
        Table2Tone::Ok
    } else {
        Table2Tone::Variance
    }
}

pub fn cadence_group(frequency: &str) -> &'static str {
    match PaymentCadence::parse(frequency) {
        Some(PaymentCadence::Weekly) => "Weekly",
        Some(PaymentCadence::Monthly) => "Monthly",
        Some(PaymentCadence::Quarterly) => "Quarterly",
        Some(PaymentCadence::None) => "Other",
        None => "Other",
    }
}

pub fn is_income_cash_activity(activity_type: &str) -> bool {
    matches!(
        activity_type.trim().to_ascii_lowercase().as_str(),
        "dividend" | "interest"
    )
}

/// Weekdays (Mon–Fri) used to treat an issuer declaration as current.
pub const DECLARATION_CURRENT_TRADING_DAYS: u32 = 5;

pub fn parse_iso_date(raw: &str) -> Option<NaiveDate> {
    let day = raw.get(..10)?;
    NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()
}

pub fn subtract_trading_days(as_of: NaiveDate, n: u32) -> NaiveDate {
    let mut day = as_of;
    let mut left = n;
    while left > 0 {
        day -= Duration::days(1);
        if day.weekday().number_from_monday() <= 5 {
            left -= 1;
        }
    }
    day
}

/// Current when `entered_at` is on or after `as_of` minus five weekdays.
/// Unparseable dates are not current. A later `entered_at` than `as_of` counts as current.
pub fn declaration_is_current(entered_at: &str, as_of: &str) -> bool {
    let Some(entered) = parse_iso_date(entered_at) else {
        return false;
    };
    let Some(as_of_day) = parse_iso_date(as_of) else {
        return false;
    };
    if entered > as_of_day {
        return true;
    }
    entered >= subtract_trading_days(as_of_day, DECLARATION_CURRENT_TRADING_DAYS)
}

/// Last Update is the last successful declaration-collector run date. Failed is blank.
pub fn last_update_success(last_run_ok: Option<bool>, last_run_at: &str) -> Option<String> {
    if last_run_ok != Some(true) {
        return None;
    }
    let day = last_run_at.trim();
    let day = if day.len() >= 10 { &day[..10] } else { day };
    if day.is_empty() {
        None
    } else {
        Some(day.to_string())
    }
}

/// Plan $/share in effect on a pay date. `effective_to` is inclusive ("through that day").
/// Overlapping windows keep the later `effective_from`. Calculator still uses exclusive `plan_amount_as_of`.
pub fn plan_amount_on_pay_date(
    windows: &[PlanAmountWindow<'_>],
    pay_on: &str,
) -> Option<(i64, u8)> {
    let pay = if pay_on.len() >= 10 {
        &pay_on[..10]
    } else {
        pay_on.trim()
    };
    if pay.is_empty() {
        return None;
    }
    windows
        .iter()
        .filter(|w| {
            let from = if w.effective_from.len() >= 10 {
                &w.effective_from[..10]
            } else {
                w.effective_from
            };
            if from > pay {
                return false;
            }
            let to = w.effective_to.trim();
            if to.is_empty() {
                return true;
            }
            let to = if to.len() >= 10 { &to[..10] } else { to };
            pay <= to
        })
        .max_by_key(|w| w.effective_from)
        .map(|w| (w.amount_per_share_minor, w.amount_scale))
}

/// (actual − plan) / plan as scale-2 percent. None when plan is unknown or 0.
pub fn delta_to_plan_pct_minor(actual_minor: i64, plan_known: bool, planned_minor: i64) -> Option<i64> {
    if !plan_known || planned_minor == 0 {
        return None;
    }
    Some((actual_minor - planned_minor).saturating_mul(10_000) / planned_minor)
}

/// Table 1 column ids. Must never include last_update.
pub fn table1_column_ids<'a>(year_label: &'a str, week_ends: &'a [String]) -> Vec<&'a str> {
    let mut cols = Vec::with_capacity(2 + week_ends.len());
    cols.push("label");
    cols.push(year_label);
    for w in week_ends {
        cols.push(w.as_str());
    }
    cols
}

pub fn is_burndown_account(control: &str) -> bool {
    BURNDOWN_ACCOUNTS.contains(&control)
}

pub fn occurred_in_week(occurred_on: &str, start: &str, end: &str) -> bool {
    let day = if occurred_on.len() >= 10 {
        &occurred_on[..10]
    } else {
        occurred_on
    };
    day >= start && day <= end
}

fn year_month(raw: &str) -> Option<&str> {
    let day = if raw.len() >= 10 { &raw[..10] } else { raw };
    if day.len() >= 7 {
        Some(&day[..7])
    } else {
        None
    }
}

/// Issuer declaration belongs on this week's payable: period in the Sat–Fri week.
/// Monthly leftover-ex may match the same year-month as the week's pay-on.
/// Weekly (52) never clones a prior-week payable onto this week's forecast pay-on.
pub fn declaration_belongs_in_week(
    payment_period: &str,
    week_start: &str,
    week_end: &str,
    pay_on: &str,
    periods_per_year: u8,
) -> bool {
    if occurred_in_week(payment_period, week_start, week_end) {
        return true;
    }
    if periods_per_year == 52 {
        return false;
    }
    if pay_on.trim().is_empty() || !occurred_in_week(pay_on, week_start, week_end) {
        return false;
    }
    year_month(payment_period) == year_month(pay_on)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn maps_fi_roth_and_excludes_speculation() {
        assert_eq!(map_control_account("FI Roth"), Some("Roth"));
        assert_eq!(map_control_account("For the CAR"), Some("Car"));
        assert_eq!(map_control_account("Speculation"), None);
        assert_eq!(map_income_plan_account("Speculation"), Some("Speculation"));
        assert_eq!(map_income_plan_account("Energy"), Some("Energy"));
        assert_eq!(map_income_plan_account("Robinhood"), Some("Robinhood"));
        assert_eq!(display_account_label("Roth"), "FI Roth");
        assert_eq!(display_account_label("Account 9"), "9");
        assert_eq!(account_key_from_display("9"), Some("Account 9"));
        assert_eq!(account_key_from_display("FI Roth"), Some("Roth"));
        assert!(!is_burndown_account("Account 9"));
        assert!(is_burndown_account("Roth"));
        assert!(!DEFAULT_ACCOUNTS_ON.iter().any(|n| *n == "Speculation"));
    }

    #[test]
    fn grid_weeks_keep_in_progress_out_of_future_count() {
        let as_of = NaiveDate::from_ymd_opt(2026, 9, 8).unwrap();
        let weeks = visible_grid_weeks(as_of, 6, 6);
        assert_eq!(weeks.len(), 13);
        assert_eq!(weeks[6].week.end, NaiveDate::from_ymd_opt(2026, 9, 11).unwrap());
        assert_eq!(weeks[6].kind, GridWeekKind::InProgress);
        assert_eq!(weeks[5].kind, GridWeekKind::Closed);
        assert_eq!(weeks[5].week.end, NaiveDate::from_ymd_opt(2026, 9, 4).unwrap());
        assert_eq!(weeks[7].kind, GridWeekKind::Future);
        assert_eq!(weeks[7].week.end, NaiveDate::from_ymd_opt(2026, 9, 18).unwrap());
        assert!(!actuals_known(GridWeekKind::InProgress));
        assert!(!actuals_known(GridWeekKind::Future));
    }

    #[test]
    fn plan_lock_through_friday_uses_inclusive_to() {
        let v1 = PlanAmountWindow {
            effective_from: "2026-01-01",
            effective_to: "2026-08-14",
            amount_per_share_minor: 15,
            amount_scale: 2,
        };
        let v2 = PlanAmountWindow {
            effective_from: "2026-08-15",
            effective_to: "",
            amount_per_share_minor: 17,
            amount_scale: 2,
        };
        let windows = [v1, v2];
        assert_eq!(plan_amount_on_pay_date(&windows, "2026-08-14"), Some((15, 2)));
        assert_eq!(plan_amount_on_pay_date(&windows, "2026-08-21"), Some((17, 2)));
    }

    #[test]
    fn table2_empty_is_not_miss_and_failed_last_update_is_blank() {
        assert_eq!(
            table2_cell_tone(None, None, GridWeekKind::Closed),
            Table2Tone::Empty
        );
        assert_eq!(
            table2_cell_tone(Some(1500), None, GridWeekKind::Closed),
            Table2Tone::Miss
        );
        assert_eq!(
            table2_cell_tone(Some(1500), Some(1500), GridWeekKind::Closed),
            Table2Tone::Ok
        );
        assert_eq!(
            table2_cell_tone(Some(1500), Some(1400), GridWeekKind::Closed),
            Table2Tone::Ok
        );
        assert_eq!(
            table2_cell_tone(Some(1500), Some(1300), GridWeekKind::Closed),
            Table2Tone::Variance
        );
        assert_eq!(
            table2_cell_tone(Some(1700), None, GridWeekKind::Future),
            Table2Tone::Future
        );
        assert_eq!(cadence_group(""), "Other");
        assert_eq!(cadence_group("Weekly"), "Weekly");
        assert!(is_income_cash_activity("interest"));
        assert_eq!(last_update_success(Some(false), "2026-09-01"), None);
        assert_eq!(
            last_update_success(Some(true), "2026-09-01T12:00:00Z").as_deref(),
            Some("2026-09-01")
        );
        let week_ends = ["2026-08-14".to_string(), "2026-08-21".to_string()];
        let cols = table1_column_ids("2026", &week_ends);
        assert_eq!(cols[0], "label");
        assert!(!cols.iter().any(|c| c.contains("last") || *c == "last_update"));
    }

    #[test]
    fn declaration_current_uses_five_weekdays_not_calendar_days() {
        let as_of = NaiveDate::from_ymd_opt(2026, 9, 9).unwrap();
        assert_eq!(
            subtract_trading_days(as_of, DECLARATION_CURRENT_TRADING_DAYS),
            NaiveDate::from_ymd_opt(2026, 9, 2).unwrap()
        );
        assert!(declaration_is_current("2026-09-02", "2026-09-09"));
        assert!(declaration_is_current("2026-09-08T15:00:00Z", "2026-09-09"));
        assert!(!declaration_is_current("2026-09-01", "2026-09-09"));
        assert!(!declaration_is_current("", "2026-09-09"));
        assert!(declaration_is_current("2026-09-10", "2026-09-09"));
        let friday = NaiveDate::from_ymd_opt(2026, 9, 4).unwrap();
        assert_eq!(
            subtract_trading_days(friday, DECLARATION_CURRENT_TRADING_DAYS),
            NaiveDate::from_ymd_opt(2026, 8, 28).unwrap()
        );
    }

    #[test]
    fn week_filter_uses_iso_dates() {
        assert!(occurred_in_week("2026-08-17", "2026-08-15", "2026-08-21"));
        assert!(!occurred_in_week("2026-08-14", "2026-08-15", "2026-08-21"));
    }

    #[test]
    fn declaration_follows_week_payable_not_a_named_example() {
        assert!(declaration_belongs_in_week(
            "2026-08-31",
            "2026-08-29",
            "2026-09-04",
            "2026-08-31",
            12
        ));
        assert!(declaration_belongs_in_week(
            "2026-08-31",
            "2026-08-22",
            "2026-08-28",
            "2026-08-28",
            12
        ));
        assert!(!declaration_belongs_in_week(
            "2026-07-31",
            "2026-08-29",
            "2026-09-04",
            "2026-08-31",
            12
        ));
    }

    #[test]
    fn weekly_does_not_clone_last_payable_onto_forecast_friday() {
        assert!(declaration_belongs_in_week(
            "2026-09-15",
            "2026-09-12",
            "2026-09-18",
            "2026-09-15",
            52
        ));
        assert!(!declaration_belongs_in_week(
            "2026-09-11",
            "2026-09-12",
            "2026-09-18",
            "2026-09-18",
            52
        ));
        assert!(declaration_belongs_in_week(
            "2026-09-11",
            "2026-09-05",
            "2026-09-11",
            "2026-09-11",
            52
        ));
    }

    #[test]
    fn pct_of_plan_is_none_when_unknown_or_zero_never_zero_percent() {
        assert_eq!(pct_of_plan_minor(1_650, false, 0), None);
        assert_eq!(pct_of_plan_minor(0, true, 0), None);
        assert_eq!(pct_of_plan_minor(3_500, true, 3_500), Some(10_000));
        assert_eq!(pct_of_plan_minor(3_850, true, 3_500), Some(11_000));
        assert_eq!(pct_of_plan_minor(0, true, 3_500), Some(0));
    }

    #[test]
    fn performance_range_start_covers_days_months_ytd_all() {
        let as_of = NaiveDate::from_ymd_opt(2026, 8, 31).unwrap();
        assert_eq!(
            performance_range_start(as_of, "30d").unwrap(),
            Some(NaiveDate::from_ymd_opt(2026, 8, 1).unwrap())
        );
        assert_eq!(
            performance_range_start(as_of, "1m").unwrap(),
            Some(NaiveDate::from_ymd_opt(2026, 7, 31).unwrap())
        );
        assert_eq!(
            performance_range_start(as_of, "ytd").unwrap(),
            Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap())
        );
        assert_eq!(performance_range_start(as_of, "all").unwrap(), None);
        assert!(performance_range_start(as_of, "nope").is_err());
    }
}
