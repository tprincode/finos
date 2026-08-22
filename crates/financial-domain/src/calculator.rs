//! Calculator Plan is owner-controlled per share. It is not cash and not a declaration.

use chrono::{Duration, NaiveDate};

/// Normalized planning periods: Weekly=52, Monthly=12, Quarterly=4 (TR-C-5).
pub fn periods_from_frequency(frequency: &str) -> Option<u8> {
    match frequency.trim().to_ascii_lowercase().as_str() {
        "weekly" => Some(52),
        "monthly" => Some(12),
        "quarterly" => Some(4),
        "none" | "" => None,
        _ => None,
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
}
