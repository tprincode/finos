//! Calculator Plan is owner-controlled per share. It is not cash and not a declaration.

use chrono::{Duration, NaiveDate};

/// One locked cadence: label and period count are the same fact (TR-C-5).
/// There is no default. Empty is unidentified. `None` means the position does not pay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentCadence {
    Weekly,
    Monthly,
    Quarterly,
    None,
}

impl PaymentCadence {
    /// Parse a description (`Weekly` / `None`) or a period count (`52` / `12` / `4`).
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "weekly" | "52" => Some(Self::Weekly),
            "monthly" | "12" => Some(Self::Monthly),
            "quarterly" | "4" => Some(Self::Quarterly),
            "none" => Some(Self::None),
            _ => None,
        }
    }

    pub fn parse_periods(periods: u8) -> Option<Self> {
        match periods {
            52 => Some(Self::Weekly),
            12 => Some(Self::Monthly),
            4 => Some(Self::Quarterly),
            _ => None,
        }
    }

    pub fn periods(self) -> Option<u8> {
        match self {
            Self::Weekly => Some(52),
            Self::Monthly => Some(12),
            Self::Quarterly => Some(4),
            Self::None => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Weekly => "Weekly",
            Self::Monthly => "Monthly",
            Self::Quarterly => "Quarterly",
            Self::None => "None",
        }
    }
}

/// Normalized planning periods from the locked cadence. None means no 52/12/4 schedule.
pub fn periods_from_frequency(frequency: &str) -> Option<u8> {
    PaymentCadence::parse(frequency).and_then(PaymentCadence::periods)
}

/// Owner-tagged "does not pay". Empty/unidentified is not this — those still list until cadence is set.
pub fn is_non_paying(frequency: &str) -> bool {
    matches!(PaymentCadence::parse(frequency), Some(PaymentCadence::None))
}

/// Infer Weekly / Monthly / Quarterly from paid declaration dates and/or an issuer page label.
/// Returns `None` (unknown) when history cannot support 52 / 12 / 4 — never invents a cadence.
/// Owner does not type frequency; Process A persists the suggestion when present.
pub fn infer_payment_cadence(
    payment_periods: &[&str],
    page_label: Option<&str>,
) -> Option<PaymentCadence> {
    if let Some(label) = page_label.map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(c) = PaymentCadence::parse(label) {
            if c.periods().is_some() {
                return Some(c);
            }
        }
        let lower = label.to_ascii_lowercase();
        if lower.contains("weekly") {
            return Some(PaymentCadence::Weekly);
        }
        if lower.contains("monthly") {
            return Some(PaymentCadence::Monthly);
        }
        if lower.contains("quarterly") {
            return Some(PaymentCadence::Quarterly);
        }
    }

    let mut dates: Vec<NaiveDate> = payment_periods
        .iter()
        .filter_map(|raw| {
            let s = raw.trim();
            if s.len() < 10 {
                return None;
            }
            NaiveDate::parse_from_str(&s[..10], "%Y-%m-%d").ok()
        })
        .collect();
    dates.sort_unstable();
    dates.dedup();
    if dates.len() < 2 {
        return None;
    }
    let mut gaps: Vec<i64> = dates
        .windows(2)
        .map(|w| (w[1] - w[0]).num_days().abs())
        .collect();
    gaps.sort_unstable();
    let med = gaps[gaps.len() / 2];
    // ~weekly (≤10d), ~monthly (≤40d), ~quarterly (≤110d). Wider gaps stay unknown.
    if med <= 10 {
        Some(PaymentCadence::Weekly)
    } else if med <= 40 {
        Some(PaymentCadence::Monthly)
    } else if med <= 110 {
        Some(PaymentCadence::Quarterly)
    } else {
        None
    }
}

/// Position dollars for one period, in USD cents (scale 2).
pub fn plan_payment_cents(
    quantity_minor: i64,
    quantity_scale: u8,
    plan_minor: i64,
    plan_scale: u8,
) -> i64 {
    if quantity_minor == 0 || plan_minor == 0 {
        return 0;
    }
    let num = (quantity_minor as i128) * (plan_minor as i128) * 100;
    let den = 10i128.pow((quantity_scale as u32) + (plan_scale as u32));
    if den == 0 {
        return 0;
    }
    let q = num / den;
    let r = (num % den).abs();
    let rounded = if r * 2 >= den {
        q + if num >= 0 { 1 } else { -1 }
    } else {
        q
    };
    rounded as i64
}

fn parse_day(raw: &str) -> Option<NaiveDate> {
    let day = if raw.len() >= 10 { &raw[..10] } else { raw };
    NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()
}

/// Whether this Sat–Fri week is an expected pay week from last actual + normalized periods.
/// Weekly positions are expected every week. Missing last actual for monthly/quarterly → not scheduled
/// (unknown schedule, not a zero Plan).
pub fn expected_in_week(
    last_actual_on: Option<&str>,
    periods_per_year: u8,
    week_start: &str,
    week_end: &str,
) -> bool {
    let Some(start) = parse_day(week_start) else {
        return false;
    };
    let Some(end) = parse_day(week_end) else {
        return false;
    };
    if periods_per_year == 52 {
        return true;
    }
    let step_days = match periods_per_year {
        12 => 30,
        4 => 91,
        _ => return false,
    };
    let Some(last) = last_actual_on.and_then(parse_day) else {
        return false;
    };
    let step = Duration::days(step_days);
    let mut d = last;
    let earliest = start - Duration::days(400);
    while d > earliest {
        d -= step;
    }
    let latest = end + Duration::days(400);
    while d <= latest {
        if d >= start && d <= end {
            return true;
        }
        d += step;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cadence_is_one_value_label_or_periods() {
        assert_eq!(PaymentCadence::parse("Weekly").unwrap().periods(), Some(52));
        assert_eq!(PaymentCadence::parse("52").unwrap().label(), "Weekly");
        assert_eq!(PaymentCadence::parse("monthly").unwrap().periods(), Some(12));
        assert_eq!(PaymentCadence::parse("12").unwrap().label(), "Monthly");
        assert_eq!(PaymentCadence::parse("Quarterly").unwrap().periods(), Some(4));
        assert_eq!(PaymentCadence::parse("4").unwrap().label(), "Quarterly");
        assert_eq!(PaymentCadence::parse("None").unwrap().label(), "None");
        assert_eq!(PaymentCadence::parse("None").unwrap().periods(), None);
        assert!(PaymentCadence::parse("").is_none());
        assert!(PaymentCadence::parse_periods(0).is_none());
        assert!(periods_from_frequency("").is_none());
        assert!(periods_from_frequency("None").is_none());
        assert!(is_non_paying("None"));
        assert!(!is_non_paying(""));
        assert!(!is_non_paying("Quarterly"));
    }

    #[test]
    fn crf_plan_payment_matches_qty_times_plan() {
        // 1045.2 * 0.1168 = 122.07936 → $122.08
        assert_eq!(plan_payment_cents(10_452, 1, 1_168, 4), 12_208);
    }

    #[test]
    fn weekly_every_week_monthly_needs_anchor() {
        assert!(expected_in_week(None, 52, "2026-08-22", "2026-08-28"));
        assert!(!expected_in_week(None, 12, "2026-08-22", "2026-08-28"));
        assert!(expected_in_week(
            Some("2026-07-31"),
            12,
            "2026-08-29",
            "2026-09-04"
        ));
        assert!(!expected_in_week(
            Some("2026-07-31"),
            12,
            "2026-08-22",
            "2026-08-28"
        ));
    }

    #[test]
    fn infer_monthly_from_seven_monthly_pays_or_page_label() {
        let periods = [
            "2026-01-31",
            "2026-02-28",
            "2026-03-31",
            "2026-04-30",
            "2026-05-30",
            "2026-06-30",
            "2026-07-31",
        ];
        assert_eq!(
            infer_payment_cadence(&periods, None),
            Some(PaymentCadence::Monthly)
        );
        assert_eq!(
            infer_payment_cadence(&[], Some("Distribution Frequency: Monthly")),
            Some(PaymentCadence::Monthly)
        );
        assert_eq!(infer_payment_cadence(&["2026-01-15"], None), None);
        assert_eq!(
            infer_payment_cadence(&["2026-01-01", "2026-07-01"], None),
            None,
            "semi-annual gap is not 52/12/4"
        );
    }
}
