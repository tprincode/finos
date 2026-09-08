//! Return-of-capital research and MAGI split. Unknown stays unknown; ROC is not MAGI.

use crate::schedule::{remaining_year_payments, DateOverride};

pub const ROC_PCT_SCALE: u8 = 2;

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
}
