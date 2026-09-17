//! Auto last-price fetch window: weekdays 09:00–16:00 America/New_York,
//! skipped when the last price retrieve_run is under four hours old
//! (ok, fail, or started — Restart must not start another auto fetch).

use chrono::{DateTime, Datelike, Duration, NaiveDateTime, TimeZone, Timelike, Weekday};
use chrono_tz::America::New_York;
use chrono_tz::Tz;

pub const LAST_PRICE_AUTO_WINDOW: &str = "weekday 9-4 Eastern";
pub const LAST_PRICE_FRESH_HOURS: i64 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LastPriceAutoSkip {
    Weekend,
    OutsideHours,
    FreshUnderFourHours,
}

impl LastPriceAutoSkip {
    pub fn reason(self) -> &'static str {
        match self {
            Self::Weekend | Self::OutsideHours => "outside weekday 9–4 Eastern",
            Self::FreshUnderFourHours => "last refresh under 4 hours",
        }
    }
}

pub fn now_eastern() -> DateTime<Tz> {
    chrono::Utc::now().with_timezone(&New_York)
}

pub fn parse_run_stamp_et(stamp: &str) -> Option<DateTime<Tz>> {
    let trimmed = stamp.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(utc) = DateTime::parse_from_rfc3339(trimmed) {
        return Some(utc.with_timezone(&New_York));
    }
    let naive = NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S"))
        .ok()?;
    New_York.from_local_datetime(&naive).single()
}

/// Clock-only window (weekend / hours). Pass a retrieve_run stamp for the 4-hour cooldown.
pub fn last_price_auto_window(now: DateTime<Tz>) -> Result<(), LastPriceAutoSkip> {
    auto_last_price_allowed(now, None)
}

pub fn last_price_auto_window_now() -> Result<(), LastPriceAutoSkip> {
    last_price_auto_window(now_eastern())
}

/// Weekday 9–4 Eastern plus 4-hour cooldown from any price retrieve_run stamp.
pub fn last_price_auto_window_with_last(last: Option<&str>) -> Result<(), LastPriceAutoSkip> {
    auto_last_price_allowed(now_eastern(), last.and_then(parse_run_stamp_et))
}

pub fn auto_last_price_allowed(
    now: DateTime<Tz>,
    last_ok: Option<DateTime<Tz>>,
) -> Result<(), LastPriceAutoSkip> {
    match now.weekday() {
        Weekday::Sat | Weekday::Sun => return Err(LastPriceAutoSkip::Weekend),
        _ => {}
    }
    let minutes = (now.hour() as i32) * 60 + (now.minute() as i32);
    if minutes < 9 * 60 || minutes >= 16 * 60 {
        return Err(LastPriceAutoSkip::OutsideHours);
    }
    if let Some(then) = last_ok {
        if now.signed_duration_since(then) < Duration::hours(LAST_PRICE_FRESH_HOURS) {
            return Err(LastPriceAutoSkip::FreshUnderFourHours);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn et(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Tz> {
        New_York
            .with_ymd_and_hms(y, m, d, h, min, 0)
            .single()
            .expect("eastern")
    }

    #[test]
    fn saturday_skips() {
        assert_eq!(
            auto_last_price_allowed(et(2026, 9, 12, 10, 0), None),
            Err(LastPriceAutoSkip::Weekend)
        );
    }

    #[test]
    fn sunday_skips() {
        assert_eq!(
            last_price_auto_window(et(2026, 9, 13, 10, 0)),
            Err(LastPriceAutoSkip::Weekend)
        );
    }

    #[test]
    fn monday_before_nine_skips() {
        assert_eq!(
            auto_last_price_allowed(et(2026, 9, 14, 8, 59), None),
            Err(LastPriceAutoSkip::OutsideHours)
        );
    }

    #[test]
    fn monday_ten_and_five_hours_ago_runs() {
        assert_eq!(
            auto_last_price_allowed(et(2026, 9, 14, 10, 0), Some(et(2026, 9, 14, 5, 0))),
            Ok(())
        );
    }

    #[test]
    fn monday_ten_and_three_hours_ago_skips() {
        assert_eq!(
            auto_last_price_allowed(et(2026, 9, 14, 10, 0), Some(et(2026, 9, 14, 7, 0))),
            Err(LastPriceAutoSkip::FreshUnderFourHours)
        );
    }

    #[test]
    fn four_hour_cooldown_uses_stamp_not_ok_flag() {
        let stamp = "2026-09-14T10:00:00";
        let then = parse_run_stamp_et(stamp);
        assert!(then.is_some());
        assert_eq!(
            auto_last_price_allowed(et(2026, 9, 14, 11, 0), then),
            Err(LastPriceAutoSkip::FreshUnderFourHours)
        );
    }

    #[test]
    fn monday_four_pm_skips() {
        assert_eq!(
            auto_last_price_allowed(et(2026, 9, 14, 16, 0), None),
            Err(LastPriceAutoSkip::OutsideHours)
        );
    }

    #[test]
    fn monday_nine_with_no_prior_run_allows() {
        assert_eq!(auto_last_price_allowed(et(2026, 9, 14, 9, 0), None), Ok(()));
    }

    #[test]
    fn skip_reasons_are_owner_facing() {
        assert_eq!(
            LastPriceAutoSkip::Weekend.reason(),
            "outside weekday 9–4 Eastern"
        );
        assert_eq!(
            LastPriceAutoSkip::FreshUnderFourHours.reason(),
            "last refresh under 4 hours"
        );
        assert!(LAST_PRICE_AUTO_WINDOW.contains("weekday 9-4 Eastern"));
    }
}
