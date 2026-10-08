//! Return-of-capital research and MAGI split. Unknown stays unknown; ROC is not MAGI.

use crate::schedule::{remaining_year_payments, DateOverride};

pub const ROC_PCT_SCALE: u8 = 2;

/// Convert a stored ROC minor between decimal scales (800 @1 = 8000 @2 = 80%).
pub fn rescale_roc_pct(minor: i64, from_scale: u8, to_scale: u8) -> i64 {
    if from_scale == to_scale {
        return minor;
    }
    if from_scale < to_scale {
        minor.saturating_mul(10i64.pow(u32::from(to_scale - from_scale)))
    } else {
        minor / 10i64.pow(u32::from(from_scale - to_scale))
    }
}

pub fn roc_pcts_equal(left: i64, left_scale: u8, right: i64, right_scale: u8) -> bool {
    rescale_roc_pct(left, left_scale, ROC_PCT_SCALE)
        == rescale_roc_pct(right, right_scale, ROC_PCT_SCALE)
}

/// Same calendar month (`YYYY-MM`) for two ISO dates. Used by the once-per-month ROC gate.
pub fn same_calendar_month(left: &str, right: &str) -> bool {
    let l = left.trim();
    let r = right.trim();
    if l.len() < 7 || r.len() < 7 {
        return false;
    }
    &l[..7] == &r[..7]
}

/// Runtime ROC HTTP is once per calendar month unless forced (Reevaluate / ticket / owner paste).
/// No prior fetch → due. Same `YYYY-MM` as last `roc-19a1` run → skip.
pub fn roc_monthly_fetch_due(last_fetch_as_of: Option<&str>, as_of: &str, force: bool) -> bool {
    if force {
        return true;
    }
    match last_fetch_as_of.map(str::trim).filter(|s| !s.is_empty()) {
        None => true,
        Some(last) => !same_calendar_month(last, as_of),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RocSuggestion {
    pub roc_pct_minor: Option<i64>,
    pub scale: u8,
    pub source: String,
    pub complete: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CashSplit {
    pub ordinary_minor: Option<i64>,
    pub roc_minor: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemainingYearPlan {
    pub remaining_periods: i64,
    pub total_minor: i64,
    pub ordinary_minor: Option<i64>,
    pub roc_minor: Option<i64>,
    pub magi_uncertain_ordinary_minor: Option<i64>,
}

/// Current-year 19a-1 estimate. Owner may override; missing is not 0%.
pub fn suggest_current_year(
    roc_pct_minor: i64,
    scale: u8,
    source: String,
    established_how: String,
) -> RocSuggestion {
    RocSuggestion {
        roc_pct_minor: Some(roc_pct_minor),
        scale,
        source,
        complete: true,
        reason: format!("{established_how}; owner may override before MAGI uses it"),
    }
}

/// Latest actual year percent is the system research default. Missing is not 0%.
pub fn suggest_from_history(
    actuals_newest_first: &[Option<i64>],
    scale: u8,
) -> RocSuggestion {
    let latest = actuals_newest_first.iter().copied().flatten().next();
    match latest {
        Some(pct) => RocSuggestion {
            roc_pct_minor: Some(pct),
            scale,
            source: "prior-year-actual".into(),
            complete: true,
            reason: "latest stored actual year percent; owner must confirm before MAGI uses it"
                .into(),
        },
        None => RocSuggestion {
            roc_pct_minor: None,
            scale,
            source: "miss".into(),
            complete: false,
            reason: "no 19a-1/1099 year percent; unknown is not 0%".into(),
        },
    }
}

/// Split one cash amount. Unknown ROC leaves both sides unknown.
pub fn split_cash(amount_minor: i64, roc_pct_minor: Option<i64>, pct_scale: u8) -> CashSplit {
    let Some(pct) = roc_pct_minor else {
        return CashSplit {
            ordinary_minor: None,
            roc_minor: None,
        };
    };
    let den = 10i64.pow(pct_scale as u32).saturating_mul(100);
    if den == 0 {
        return CashSplit {
            ordinary_minor: None,
            roc_minor: None,
        };
    }
    let roc = (amount_minor.saturating_mul(pct)) / den;
    CashSplit {
        ordinary_minor: Some(amount_minor.saturating_sub(roc)),
        roc_minor: Some(roc),
    }
}

pub fn remaining_year_end(as_of: &str) -> String {
    let year = as_of.get(..4).unwrap_or("2026");
    format!("{year}-12-31")
}

pub fn remaining_year_plan(
    as_of: &str,
    periods_per_year: u8,
    quantity_minor: i64,
    quantity_scale: u8,
    plan_per_share_minor: i64,
    plan_scale: u8,
    roc_pct_minor: Option<i64>,
    pct_scale: u8,
    latest_payment_period: Option<&str>,
    overrides: &[DateOverride],
) -> Option<RemainingYearPlan> {
    let schedule = remaining_year_payments(
        as_of,
        latest_payment_period,
        periods_per_year,
        quantity_minor,
        quantity_scale,
        plan_per_share_minor,
        plan_scale,
        overrides,
    );
    if !schedule.known {
        return None;
    }
    let remaining_periods = schedule.remaining_periods?;
    let total = schedule.year_to_go_minor?;
    let split = split_cash(total, roc_pct_minor, pct_scale);
    Some(RemainingYearPlan {
        remaining_periods,
        total_minor: total,
        ordinary_minor: split.ordinary_minor,
        roc_minor: split.roc_minor,
        magi_uncertain_ordinary_minor: split.ordinary_minor,
    })
}

pub fn magi_remaining_source(security_id: &str, year: &str) -> String {
    format!("{security_id}:roc-plan:{year}:ordinary-remaining")
}

pub fn magi_actual_source(security_id: &str, occurred_on: &str) -> String {
    format!("{security_id}:roc-actual:{occurred_on}:ordinary")
}

pub fn tax_year(as_of: &str) -> String {
    as_of.get(..4).unwrap_or("2026").to_string()
}

pub fn account_is_car_magi(account_name: &str) -> bool {
    account_name.trim().eq_ignore_ascii_case("car")
}

/// Planning copy for Cash Management. 2025 1099 is prior-year ROC guidance only.
pub const CAR_ROC_PLAN_NOTE: &str =
    "Planning estimate. A 2025 1099 is prior-year ROC guidance for the ETF — it is not current-year tax.";

pub const CAR_TAX_UNKNOWN_NOTE: &str =
    "Car 2026 tax stays unknown until the 1099 process (April 2027).";

pub const YTD_ROC_NO_PAYMENTS: &str = "no Car payments this year yet";
pub const YTD_ROC_ESTIMATE_BLANK: &str =
    "Position.roc_pct_2026_estimate blank on names that paid";
pub const YTD_ROC_NEEDS_RESEARCH: &str = "needs_roc_research still set";
pub const YTD_ROC_UNTIL_1099: &str = "2026_actual empty until 1099 (expected)";

pub fn ytd_roc_mixed_reason(symbols: &[String]) -> String {
    let listed = if symbols.is_empty() {
        String::new()
    } else {
        format!(" — {}", symbols.join(", "))
    };
    format!("mixed: some names estimated, some unclassified{listed}")
}

/// First owner-facing reason when YTD ROC cannot be computed. Never invent $0.
pub fn first_ytd_roc_unknown_reason(
    no_payments: bool,
    all_paid_estimate_blank: bool,
    needs_research: bool,
    actual_empty: bool,
    mixed: bool,
    mixed_symbols: &[String],
) -> Option<String> {
    if no_payments {
        return Some(YTD_ROC_NO_PAYMENTS.into());
    }
    if all_paid_estimate_blank {
        return Some(YTD_ROC_ESTIMATE_BLANK.into());
    }
    if needs_research {
        return Some(YTD_ROC_NEEDS_RESEARCH.into());
    }
    if actual_empty {
        return Some(YTD_ROC_UNTIL_1099.into());
    }
    if mixed {
        return Some(ytd_roc_mixed_reason(mixed_symbols));
    }
    None
}

pub const NO_ASSIGNED_LOT_SALES_NOTE: &str =
    "No assigned lot sales. Long-term and short-term stay unknown.";

pub const UNDATED_LOT_SALE_NOTE: &str =
    "A lot sale is dated before its lot. Long-term and short-term stay unknown until it is fixed.";

/// Any unknown operand keeps the sum unknown (unknown ≠ 0).
pub fn sum_known(left: Option<i64>, right: Option<i64>) -> Option<i64> {
    match (left, right) {
        (Some(a), Some(b)) => Some(a.saturating_add(b)),
        _ => None,
    }
}

/// 1040 caps the net capital loss deducted against income at $3,000 a year.
/// Minor units, scale 2.
pub const NET_CAPITAL_LOSS_LIMIT_MINOR: i64 = 300_000;

/// Net capital gain split into what MAGI may take this year and what carries forward.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetCapitalGain {
    /// Every part added up, gains and losses, uncapped. This is what the rows show.
    pub net_minor: i64,
    /// The slice MAGI takes. A net loss is limited to `NET_CAPITAL_LOSS_LIMIT_MINOR`.
    pub magi_minor: i64,
    /// Loss left over after the limit, negative. Zero when the net is a gain.
    pub carryforward_minor: i64,
}

/// A net gain passes through whole. A net loss over the limit gives MAGI
/// `-NET_CAPITAL_LOSS_LIMIT_MINOR` and carries the rest forward.
pub fn net_capital_gain_for_magi(parts: &[i64]) -> NetCapitalGain {
    let net_minor = parts.iter().fold(0i64, |sum, part| sum.saturating_add(*part));
    let magi_minor = net_minor.max(-NET_CAPITAL_LOSS_LIMIT_MINOR);
    NetCapitalGain {
        net_minor,
        magi_minor,
        carryforward_minor: net_minor - magi_minor,
    }
}

/// After an actual, remaining planned ordinary is reduced by that period's ordinary slice.
pub fn remaining_after_actual(
    planned_ordinary_remaining: i64,
    actual_ordinary: i64,
) -> i64 {
    planned_ordinary_remaining.saturating_sub(actual_ordinary).max(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_history_is_unknown_not_zero() {
        let s = suggest_from_history(&[None, None], 2);
        assert!(!s.complete);
        assert_eq!(s.roc_pct_minor, None);
    }

    #[test]
    fn current_year_19a1_is_the_system_percent() {
        let s = suggest_current_year(
            10_000,
            2,
            "19a-1".into(),
            "current distribution 19a-1 estimate".into(),
        );
        assert_eq!(s.roc_pct_minor, Some(10_000));
        assert!(s.complete);
        assert!(s.reason.contains("19a-1"));
    }

    #[test]
    fn latest_actual_is_the_research_default() {
        let s = suggest_from_history(&[Some(7_030), Some(6_100)], 2);
        assert_eq!(s.roc_pct_minor, Some(7_030));
        assert!(s.complete);
    }

    #[test]
    fn unknown_roc_does_not_split_to_zero() {
        let split = split_cash(10_000, None, 2);
        assert_eq!(split.ordinary_minor, None);
        assert_eq!(split.roc_minor, None);
    }

    #[test]
    fn seventy_percent_roc_leaves_thirty_ordinary() {
        let split = split_cash(10_000, Some(7_000), 2);
        assert_eq!(split.roc_minor, Some(7_000));
        assert_eq!(split.ordinary_minor, Some(3_000));
    }

    #[test]
    fn remaining_year_uses_plan_times_qty() {
        let plan = remaining_year_plan(
            "2026-08-22",
            12,
            100,
            0,
            10_000,
            4,
            Some(7_000),
            2,
            Some("2026-07-01"),
            &[],
        )
        .unwrap();
        assert_eq!(plan.remaining_periods, 5);
        assert_eq!(plan.ordinary_minor.unwrap() + plan.roc_minor.unwrap(), plan.total_minor);
    }

    #[test]
    fn actual_reduces_planned_remaining() {
        assert_eq!(remaining_after_actual(9_000, 3_000), 6_000);
    }

    #[test]
    fn car_is_magi_eligible() {
        assert!(account_is_car_magi("Car"));
        assert!(!account_is_car_magi("Income"));
    }

    #[test]
    fn sum_known_stays_unknown_when_either_side_is() {
        assert_eq!(sum_known(Some(100), Some(40)), Some(140));
        assert_eq!(sum_known(Some(100), None), None);
        assert_eq!(sum_known(None, Some(40)), None);
    }

    #[test]
    fn a_net_capital_gain_reaches_magi_whole() {
        // $1,200.00 long + $300.00 short, scale 2.
        let net = net_capital_gain_for_magi(&[120_000, 30_000]);
        assert_eq!(net.net_minor, 150_000);
        assert_eq!(net.magi_minor, 150_000);
        assert_eq!(net.carryforward_minor, 0);
    }

    #[test]
    fn a_net_loss_under_the_limit_reaches_magi_whole() {
        // $1,800.00 loss is under the $3,000.00 limit.
        let net = net_capital_gain_for_magi(&[-200_000, 20_000]);
        assert_eq!(net.net_minor, -180_000);
        assert_eq!(net.magi_minor, -180_000);
        assert_eq!(net.carryforward_minor, 0);
    }

    #[test]
    fn a_net_loss_over_the_limit_stops_at_three_thousand_and_carries_the_rest() {
        // $10,000.00 loss: MAGI takes $3,000.00, $7,000.00 carries forward.
        let net = net_capital_gain_for_magi(&[-1_000_000]);
        assert_eq!(net.net_minor, -1_000_000);
        assert_eq!(net.magi_minor, -NET_CAPITAL_LOSS_LIMIT_MINOR);
        assert_eq!(net.magi_minor, -300_000);
        assert_eq!(net.carryforward_minor, -700_000);
    }

    #[test]
    fn a_gain_offsets_the_loss_before_the_limit_applies() {
        // $10,000.00 loss netted against a $9,500.00 gain is a $500.00 loss.
        let net = net_capital_gain_for_magi(&[-1_000_000, 950_000]);
        assert_eq!(net.net_minor, -50_000);
        assert_eq!(net.magi_minor, -50_000);
        assert_eq!(net.carryforward_minor, 0);
    }

    #[test]
    fn exactly_the_limit_carries_nothing() {
        let net = net_capital_gain_for_magi(&[-300_000]);
        assert_eq!(net.magi_minor, -300_000);
        assert_eq!(net.carryforward_minor, 0);
    }

    #[test]
    fn first_ytd_roc_reason_is_priority_order() {
        assert_eq!(
            first_ytd_roc_unknown_reason(true, true, true, true, true, &["HAKY".into()]).as_deref(),
            Some(YTD_ROC_NO_PAYMENTS)
        );
        assert_eq!(
            first_ytd_roc_unknown_reason(false, true, true, true, true, &[]).as_deref(),
            Some(YTD_ROC_ESTIMATE_BLANK)
        );
        assert_eq!(
            first_ytd_roc_unknown_reason(false, false, true, true, true, &[]).as_deref(),
            Some(YTD_ROC_NEEDS_RESEARCH)
        );
        assert_eq!(
            first_ytd_roc_unknown_reason(false, false, false, true, true, &[]).as_deref(),
            Some(YTD_ROC_UNTIL_1099)
        );
        assert_eq!(
            first_ytd_roc_unknown_reason(false, false, false, false, true, &["HAKY".into()]),
            Some("mixed: some names estimated, some unclassified — HAKY".into())
        );
    }

    #[test]
    fn rescale_treats_one_decimal_80_as_two_decimal_80() {
        assert_eq!(rescale_roc_pct(800, 1, 2), 8_000);
        assert!(roc_pcts_equal(800, 1, 8_000, 2));
        assert!(!roc_pcts_equal(8_000, 2, 7_500, 2));
    }

    #[test]
    fn roc_monthly_fetch_once_per_calendar_month() {
        assert!(roc_monthly_fetch_due(None, "2026-10-02", false));
        assert!(roc_monthly_fetch_due(Some("2026-09-15"), "2026-10-02", false));
        assert!(!roc_monthly_fetch_due(Some("2026-10-01"), "2026-10-15", false));
        assert!(roc_monthly_fetch_due(Some("2026-10-01"), "2026-10-15", true));
        assert!(same_calendar_month("2026-10-01", "2026-10-31"));
        assert!(!same_calendar_month("2026-09-30", "2026-10-01"));
    }
}
