//! Trends weekly capture helpers (Sat–Fri periods).

use chrono::NaiveDate;

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
    )
}

/// Account 9 liquid proxy until position liquidity classifications exist (T7 interim):
/// cash-par symbols at face; all other open Account-9 lot tax basis × 70%.
pub fn acct9_classified_liquid_minor(
    cash_par_minor: i64,
    other_tax_basis_minor: i64,
) -> i64 {
    cash_par_minor + (other_tax_basis_minor * 70) / 100
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
    }
}
