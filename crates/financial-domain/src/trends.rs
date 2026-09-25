//! Trends weekly capture helpers (Sat–Fri periods).

use chrono::{Duration, NaiveDate};

use crate::week::{week_containing, CanonicalWeek};

/// Resolve the reporting week for a capture date (Saturday capture → prior Friday end).
pub fn trends_period_for_capture(as_of: NaiveDate) -> CanonicalWeek {
    week_containing(as_of)
}

pub fn parse_iso_date(raw: &str) -> Option<NaiveDate> {
    let day = if raw.len() >= 10 { &raw[..10] } else { raw };
    NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()
}

/// Activity types included in weekly realized Profit (TR-AC-04 subset available today).
pub fn is_trends_profit_activity(activity_type: &str) -> bool {
    matches!(
        activity_type.to_ascii_lowercase().as_str(),
        "dividend" | "covered_call_premium" | "csp_premium" | "day_trade" | "lot_sale"
    )
}

pub fn is_non_roi_distribution(activity_type: &str) -> bool {
    matches!(
        activity_type,
        "IRA_Distribution" | "Withdrawal" | "Form_1099" | "SSA" | "Roth_Distribution"
            | "HSA_Withdrawal"
    )
}

/// Seed `Form_1099` rows are tax-year 2025 ROC guidance. They never apply to the current year.
pub fn is_prior_year_1099(activity_type: &str) -> bool {
    activity_type.eq_ignore_ascii_case("Form_1099")
}

/// Current-year Cash Management cash-out types. Prior-year 1099 is excluded.
pub fn is_current_year_cash_distribution(activity_type: &str) -> bool {
    is_non_roi_distribution(activity_type) && !is_prior_year_1099(activity_type)
}

/// Account 9 liquid proxy until position liquidity classifications exist (T7 interim):
/// cash-par symbols at face; all other open Account-9 lot tax basis × 70%.
pub fn acct9_classified_liquid_minor(
    cash_par_minor: i64,
    other_tax_basis_minor: i64,
) -> i64 {
    cash_par_minor + (other_tax_basis_minor * 70) / 100
}

/// 70% of Account 9 non-cash last-price market value. Cash-par is typed cash, not this proxy.
pub fn acct9_etf_last_price_minor(non_cash_last_price_minor: i64) -> i64 {
    (non_cash_last_price_minor * 70) / 100
}

/// Saturday of the first Sat–Fri week in the last 52 weeks (through `today`) with no saved Friday.
/// If every week in that window is saved, returns this week's Saturday.
pub fn first_unpopulated_saturday(saved_period_ends: &[String], today: NaiveDate) -> NaiveDate {
    let today_sat = week_containing(today).start;
    let start = today_sat - Duration::days(52 * 7);
    let mut sat = start;
    while sat <= today_sat {
        let fri = (sat + Duration::days(6)).format("%Y-%m-%d").to_string();
        if !saved_period_ends.iter().any(|end| end == &fri) {
            return sat;
        }
        sat += Duration::days(7);
    }
    today_sat
}

/// Saturdays from the first unpopulated week through this week (more current only).
pub fn chooser_saturdays(first_unpopulated: NaiveDate, today: NaiveDate) -> Vec<String> {
    let today_sat = week_containing(today).start;
    let mut sat = if first_unpopulated > today_sat {
        today_sat
    } else {
        first_unpopulated
    };
    let mut out = Vec::new();
    while sat <= today_sat {
        out.push(sat.format("%Y-%m-%d").to_string());
        sat += Duration::days(7);
    }
    if out.is_empty() {
        out.push(today_sat.format("%Y-%m-%d").to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friday_period_end() {
        let fri = NaiveDate::from_ymd_opt(2026, 8, 21).unwrap();
        let w = trends_period_for_capture(fri);
        assert_eq!(w.end, fri);
        assert_eq!(w.start, NaiveDate::from_ymd_opt(2026, 8, 15).unwrap());
    }

    #[test]
    fn acct9_proxy_math() {
        assert_eq!(acct9_classified_liquid_minor(10_000, 20_000), 10_000 + 14_000);
        assert_eq!(acct9_etf_last_price_minor(20_000), 14_000);
        assert_eq!(
            acct9_etf_last_price_minor(1_000_000),
            700_000,
            "G2: typed ETF $10,000 → 70% is $7,000"
        );
    }

    #[test]
    fn first_unpopulated_is_first_gap_then_this_week() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 13).unwrap();
        let saved = vec!["2026-08-21".to_string(), "2026-08-14".to_string()];
        let first = first_unpopulated_saturday(&saved, today);
        assert_eq!(first, NaiveDate::from_ymd_opt(2025, 9, 13).unwrap());

        let weeks: Vec<String> = (0..=52)
            .map(|i| {
                let sat = NaiveDate::from_ymd_opt(2025, 9, 13).unwrap() + Duration::days(7 * i);
                (sat + Duration::days(6)).format("%Y-%m-%d").to_string()
            })
            .collect();
        let after_full_window = first_unpopulated_saturday(&weeks, today);
        assert_eq!(after_full_window, NaiveDate::from_ymd_opt(2026, 9, 12).unwrap());

        let through_aug21: Vec<String> = (0..=51)
            .filter_map(|i| {
                let sat = NaiveDate::from_ymd_opt(2025, 9, 13).unwrap() + Duration::days(7 * i);
                let fri = sat + Duration::days(6);
                if fri <= NaiveDate::from_ymd_opt(2026, 8, 21).unwrap() {
                    Some(fri.format("%Y-%m-%d").to_string())
                } else {
                    None
                }
            })
            .collect();
        let first_after_seed = first_unpopulated_saturday(&through_aug21, today);
        assert_eq!(first_after_seed, NaiveDate::from_ymd_opt(2026, 8, 22).unwrap());
        let choices = chooser_saturdays(first_after_seed, today);
        assert_eq!(choices.first().map(String::as_str), Some("2026-08-22"));
        assert_eq!(choices.last().map(String::as_str), Some("2026-09-12"));
    }
}
