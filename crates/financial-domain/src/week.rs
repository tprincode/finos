//! Canonical Saturday–Friday week (V1.1 L0). Pure function; no calendar table.

use chrono::{Datelike, Duration, NaiveDate, Weekday};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanonicalWeek {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

/// Week containing `date`: Saturday start through Friday end (inclusive).
pub fn week_containing(date: NaiveDate) -> CanonicalWeek {
    let days_since_saturday = match date.weekday() {
        Weekday::Sat => 0,
        Weekday::Sun => 1,
        Weekday::Mon => 2,
        Weekday::Tue => 3,
        Weekday::Wed => 4,
        Weekday::Thu => 5,
        Weekday::Fri => 6,
    };
    let start = date - Duration::days(days_since_saturday);
    let end = start + Duration::days(6);
    CanonicalWeek { start, end }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuesday_falls_in_saturday_to_friday_week() {
        let tuesday = NaiveDate::from_ymd_opt(2026, 8, 18).unwrap();
        let week = week_containing(tuesday);
        assert_eq!(week.start, NaiveDate::from_ymd_opt(2026, 8, 15).unwrap());
        assert_eq!(week.end, NaiveDate::from_ymd_opt(2026, 8, 21).unwrap());
        assert_eq!(week.start.weekday(), Weekday::Sat);
        assert_eq!(week.end.weekday(), Weekday::Fri);
    }

    #[test]
    fn saturday_starts_its_own_week() {
        let saturday = NaiveDate::from_ymd_opt(2026, 8, 15).unwrap();
        let week = week_containing(saturday);
        assert_eq!(week.start, saturday);
        assert_eq!(week.end, NaiveDate::from_ymd_opt(2026, 8, 21).unwrap());
    }
}
