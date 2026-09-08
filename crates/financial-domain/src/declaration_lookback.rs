//! Declaration lookback: retrieve first; inception only when paid count is under 12.

use chrono::{Datelike, NaiveDate};

use crate::calculator::PaymentCadence;

/// Full paid-declaration target. At or above this, inception is N/A.
pub const DECLARATION_LOOKBACK_TARGET: u8 = 12;

/// Result of checking collected paid declarations after a retrieve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LookbackValidation {
    /// Paid count meets the full target — inception was not consulted.
    Complete,
    /// Paid count is under 12 but matches periods since inception (optional field).
    CompleteViaInception { paid: u8, expected: u8 },
    /// Under 12 and no inception — set inception only for rare too-new names, else adapter miss.
    ShortWithoutInception { paid: u8 },
    /// Under 12 with inception, but still fewer than periods since inception allow.
    ShortWithInception { paid: u8, expected: u8 },
}

/// After retrieve: if `paid_count >= 12`, ok and inception is N/A.
/// Only when under 12 does optional inception decide completeness.
pub fn validate_paid_lookback(
    paid_count: u8,
    inception_on: &str,
    as_of: &str,
    payment_frequency: &str,
) -> LookbackValidation {
    if paid_count >= DECLARATION_LOOKBACK_TARGET {
        return LookbackValidation::Complete;
    }
    let inception = inception_on.trim();
    if inception.is_empty() {
        return LookbackValidation::ShortWithoutInception { paid: paid_count };
    }
    let expected = expected_from_inception(inception, as_of, payment_frequency);
    if paid_count >= expected {
        LookbackValidation::CompleteViaInception {
            paid: paid_count,
            expected,
        }
    } else {
        LookbackValidation::ShortWithInception {
            paid: paid_count,
            expected,
        }
    }
}

/// Periods available since inception, capped at [`DECLARATION_LOOKBACK_TARGET`].
/// Empty/invalid inception returns the full target (caller should only use this when paid &lt; 12).
pub fn expected_from_inception(
    inception_on: &str,
    as_of: &str,
    payment_frequency: &str,
) -> u8 {
    let Some(periods) = PaymentCadence::parse(payment_frequency).and_then(PaymentCadence::periods)
    else {
        return DECLARATION_LOOKBACK_TARGET;
    };
    expected_from_inception_periods(inception_on, as_of, periods)
}

fn expected_from_inception_periods(
    inception_on: &str,
    as_of: &str,
    periods_per_year: u8,
) -> u8 {
    if periods_per_year == 0 {
        return DECLARATION_LOOKBACK_TARGET;
    }
    let Some(inception) = parse_iso_date(inception_on) else {
        return DECLARATION_LOOKBACK_TARGET;
    };
    let Some(as_of_d) = parse_iso_date(as_of) else {
        return DECLARATION_LOOKBACK_TARGET;
    };
    if as_of_d <= inception {
        return 0;
    }
    let elapsed = periods_elapsed(inception, as_of_d, periods_per_year);
    elapsed.min(u32::from(DECLARATION_LOOKBACK_TARGET)) as u8
}

/// Prefer [`validate_paid_lookback`]. Kept for UI labels when paid is already known short.
pub fn expected_declaration_lookback(
    inception_on: &str,
    as_of: &str,
    payment_frequency: &str,
) -> u8 {
    if inception_on.trim().is_empty() {
        return DECLARATION_LOOKBACK_TARGET;
    }
    expected_from_inception(inception_on, as_of, payment_frequency)
}

fn parse_iso_date(raw: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d").ok()
}

/// First `YYYY-MM-DD` in search-hit title/snippet. Empty hits = search miss.
pub fn inception_on_from_search_hits(hits: &[(&str, &str)]) -> Option<String> {
    for (title, snippet) in hits {
        let text = format!("{title} {snippet}");
        if let Some(date) = first_iso_date(&text) {
            return Some(date);
        }
    }
    None
}

pub fn first_iso_date(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() < 10 {
        return None;
    }
    for i in 0..=chars.len() - 10 {
        let s: String = chars[i..i + 10].iter().collect();
        if NaiveDate::parse_from_str(&s, "%Y-%m-%d").is_ok() {
            return Some(s);
        }
    }
    None
}

fn periods_elapsed(inception: NaiveDate, as_of: NaiveDate, periods_per_year: u8) -> u32 {
    let days = (as_of - inception).num_days().max(0) as u32;
    match periods_per_year {
        52 => days / 7,
        12 => month_span(inception, as_of),
        4 => month_span(inception, as_of) / 3,
        _ => {
            let step = (365u32).div_ceil(u32::from(periods_per_year)).max(1);
            days / step
        }
    }
}

fn month_span(from: NaiveDate, to: NaiveDate) -> u32 {
    let mut months =
        (to.year() - from.year()) * 12 + (to.month() as i32 - from.month() as i32);
    if to.day() < from.day() {
        months -= 1;
    }
    months.max(0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twelve_or_more_skips_inception() {
        assert_eq!(
            validate_paid_lookback(12, "", "2026-08-27", "Monthly"),
            LookbackValidation::Complete
        );
        assert_eq!(
            validate_paid_lookback(12, "2026-02-27", "2026-08-27", "Monthly"),
            LookbackValidation::Complete
        );
    }

    #[test]
    fn under_twelve_without_inception_is_short() {
        assert_eq!(
            validate_paid_lookback(6, "", "2026-08-27", "Monthly"),
            LookbackValidation::ShortWithoutInception { paid: 6 }
        );
    }

    #[test]
    fn under_twelve_with_inception_can_complete() {
        assert_eq!(
            validate_paid_lookback(6, "2026-02-27", "2026-08-27", "Monthly"),
            LookbackValidation::CompleteViaInception {
                paid: 6,
                expected: 6
            }
        );
    }

    #[test]
    fn under_twelve_with_inception_still_short() {
        assert_eq!(
            validate_paid_lookback(3, "2026-02-27", "2026-08-27", "Monthly"),
            LookbackValidation::ShortWithInception {
                paid: 3,
                expected: 6
            }
        );
    }

    #[test]
    fn six_months_weekly_caps_expected_at_twelve() {
        assert_eq!(
            expected_from_inception("2026-02-27", "2026-08-27", "Weekly"),
            12
        );
    }

    #[test]
    fn search_hits_yield_first_iso_date() {
        assert_eq!(
            inception_on_from_search_hits(&[("HAKY launched 2026-02-01", "ETF")]),
            Some("2026-02-01".into())
        );
        assert_eq!(inception_on_from_search_hits(&[]), None);
    }
}
