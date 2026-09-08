//! Canonical Saturday–Friday week (V1.1 L0). Pure function; no calendar table.

use chrono::{Datelike, Duration, NaiveDate, Weekday};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanonicalWeek {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

/// Saturday-start week identity. Week 1 is the first week whose Saturday falls in `year`.
/// This is not ISO-8601 (Monday) week numbering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeekId {
    pub year: i32,
    pub number: u8,
    pub week: CanonicalWeek,
}

/// First Saturday on or after 1 January of `year`.
pub fn first_saturday_of_year(year: i32) -> NaiveDate {
    let mut d = NaiveDate::from_ymd_opt(year, 1, 1).expect("valid year");
    while d.weekday() != Weekday::Sat {
        d += Duration::days(1);
    }
    d
}

/// Week containing `date`, numbered in the calendar year of that week's Saturday start.
pub fn week_id_containing(date: NaiveDate) -> WeekId {
    let week = week_containing(date);
    let year = week.start.year();
    let first = first_saturday_of_year(year);
    let number = (((week.start - first).num_days() / 7) + 1) as u8;
    WeekId { year, number, week }
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

/// ISO calendar day from a payment period (`YYYY-MM-DD` prefix).
pub fn parse_iso_day(raw: &str) -> Option<NaiveDate> {
    let day = raw.trim();
    let day = if day.len() >= 10 { &day[..10] } else { day };
    NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()
}

/// Friday week-ends newest-first: the week containing `as_of`, then prior Fridays.
pub fn friday_week_ends_back(as_of: NaiveDate, count: u32) -> Vec<NaiveDate> {
    let n = count.max(1);
    let newest = week_containing(as_of).end;
    (0..n)
        .map(|i| newest - Duration::days(7 * i as i64))
        .collect()
}

/// Friday week-ends newest-first whose week-end falls between `from` and `to` (inclusive).
/// Uses the Friday of the week containing each bound. Caps at 52 columns.
pub fn friday_week_ends_in_range(from: NaiveDate, to: NaiveDate) -> Vec<NaiveDate> {
    let (from, to) = if from <= to { (from, to) } else { (to, from) };
    let newest = week_containing(to).end;
    let oldest = week_containing(from).end;
    let mut out = Vec::new();
    let mut d = newest;
    while d >= oldest {
        out.push(d);
        if out.len() >= 52 {
            break;
        }
        d -= Duration::days(7);
    }
    out
}

/// Start of the default Calculator history window: 60 calendar days before `as_of`.
pub fn history_window_start(as_of: NaiveDate) -> NaiveDate {
    as_of - Duration::days(60)
}

/// Friday that owns this pay date in the Saturday–Friday week.
pub fn week_end_for_pay_on(pay_on: NaiveDate) -> NaiveDate {
    week_containing(pay_on).end
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

    #[test]
    fn friday_columns_match_owner_spreadsheet_week() {
        let as_of = NaiveDate::from_ymd_opt(2026, 9, 2).unwrap();
        let ends = friday_week_ends_back(as_of, 8);
        assert_eq!(ends[0], NaiveDate::from_ymd_opt(2026, 9, 4).unwrap());
        assert_eq!(ends[1], NaiveDate::from_ymd_opt(2026, 8, 28).unwrap());
        assert_eq!(ends[7], NaiveDate::from_ymd_opt(2026, 7, 17).unwrap());
        assert_eq!(
            week_end_for_pay_on(NaiveDate::from_ymd_opt(2026, 8, 31).unwrap()),
            NaiveDate::from_ymd_opt(2026, 9, 4).unwrap()
        );
    }

    #[test]
    fn sixty_day_window_includes_owner_amdw_fridays() {
        let as_of = NaiveDate::from_ymd_opt(2026, 9, 2).unwrap();
        let from = history_window_start(as_of);
        assert_eq!(from, NaiveDate::from_ymd_opt(2026, 7, 4).unwrap());
        let ends = friday_week_ends_in_range(from, as_of);
        assert_eq!(ends[0], NaiveDate::from_ymd_opt(2026, 9, 4).unwrap());
        assert!(ends.contains(&NaiveDate::from_ymd_opt(2026, 8, 7).unwrap()));
        assert!(ends.contains(&NaiveDate::from_ymd_opt(2026, 8, 14).unwrap()));
        assert!(ends.contains(&NaiveDate::from_ymd_opt(2026, 8, 21).unwrap()));
        assert_eq!(ends.last().copied(), Some(NaiveDate::from_ymd_opt(2026, 7, 10).unwrap()));
    }

    #[test]
    fn week_numbers_use_saturday_year_not_iso_monday() {
        let w1 = week_id_containing(NaiveDate::from_ymd_opt(2026, 1, 3).unwrap());
        assert_eq!(w1.year, 2026);
        assert_eq!(w1.number, 1);
        assert_eq!(w1.week.start, NaiveDate::from_ymd_opt(2026, 1, 3).unwrap());
        assert_eq!(w1.week.end, NaiveDate::from_ymd_opt(2026, 1, 9).unwrap());

        let jan1 = week_id_containing(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        assert_eq!(jan1.week.start, NaiveDate::from_ymd_opt(2025, 12, 27).unwrap());
        assert_eq!(jan1.year, 2025);

        let amdw_week = week_id_containing(NaiveDate::from_ymd_opt(2026, 9, 2).unwrap());
        assert_eq!(amdw_week.year, 2026);
        assert_eq!(amdw_week.number, 35);
        assert_eq!(amdw_week.week.start, NaiveDate::from_ymd_opt(2026, 8, 29).unwrap());
        assert_eq!(amdw_week.week.end, NaiveDate::from_ymd_opt(2026, 9, 4).unwrap());

        let tue = week_id_containing(NaiveDate::from_ymd_opt(2026, 8, 18).unwrap());
        let sat = week_id_containing(NaiveDate::from_ymd_opt(2026, 8, 15).unwrap());
        assert_eq!(tue, sat);
        assert_eq!(sat.number, 33);
    }
}
