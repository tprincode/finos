//! Remaining-year payment dates from issuer calendar or declaration cadence. Unknown is not $0.

use chrono::{Datelike, Duration, NaiveDate};

use crate::calculator::plan_payment_cents;
use crate::week::week_containing;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarPolicy {
    IssuerCalendar,
    DerivedWalk,
    None,
}

impl CalendarPolicy {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "issuer_calendar" | "issuer" => Some(Self::IssuerCalendar),
            "derived_walk" | "derived" => Some(Self::DerivedWalk),
            "none" => Some(Self::None),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::IssuerCalendar => "issuer_calendar",
            Self::DerivedWalk => "derived_walk",
            Self::None => "none",
        }
    }

    /// Old template rows with a blank policy: walk if the cadence pays, else none.
    pub fn parse_or_infer(raw: &str, periods_per_year: u8) -> Self {
        if let Some(policy) = Self::parse(raw) {
            return policy;
        }
        if periods_per_year == 0 {
            Self::None
        } else {
            Self::DerivedWalk
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateOverride {
    pub original_pay_on: String,
    pub pay_on: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenLotQty {
    pub opened_on: String,
    pub quantity_minor: i64,
    pub quantity_scale: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemainingPayment {
    pub pay_on: String,
    pub original_pay_on: String,
    pub month: String,
    pub cash_minor: i64,
    pub owner_override: bool,
    pub date_provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemainingMonthTotal {
    pub month: String,
    pub cash_minor: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemainingYearSchedule {
    pub known: bool,
    pub provenance: String,
    pub remaining_periods: Option<i64>,
    pub year_to_go_minor: Option<i64>,
    pub payments: Vec<RemainingPayment>,
    pub months: Vec<RemainingMonthTotal>,
    pub orphaned_overrides: Vec<DateOverride>,
}

#[derive(Debug, Clone)]
pub struct RemainingYearSpec<'a> {
    pub as_of: &'a str,
    pub latest_payment_period: Option<&'a str>,
    pub periods_per_year: u8,
    pub lots: &'a [OpenLotQty],
    pub plan_minor: i64,
    pub plan_scale: u8,
    pub overrides: &'a [DateOverride],
    pub issuer_pay_ons: &'a [String],
    pub calendar_policy: CalendarPolicy,
}

fn parse_iso_day(raw: &str) -> Option<NaiveDate> {
    let day = if raw.len() >= 10 { &raw[..10] } else { raw };
    NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()
}

/// Latest `YYYY-MM-DD` payment period. Labels such as `w1` are not dates.
pub fn latest_parseable_period<'a>(periods: impl IntoIterator<Item = &'a str>) -> Option<String> {
    periods
        .into_iter()
        .filter_map(|p| parse_iso_day(p).map(|d| (d, p[..10.min(p.len())].to_string())))
        .max_by_key(|(d, _)| *d)
        .map(|(_, p)| p)
}

pub fn frequency_label(periods_per_year: u8) -> &'static str {
    crate::calculator::PaymentCadence::parse_periods(periods_per_year)
        .map(crate::calculator::PaymentCadence::label)
        .unwrap_or("unknown")
}

pub fn pay_on_in_week(pay_on: &str, week_start: &str, week_end: &str) -> bool {
    let Some(d) = parse_iso_day(pay_on) else {
        return false;
    };
    let Some(start) = parse_iso_day(week_start) else {
        return false;
    };
    let Some(end) = parse_iso_day(week_end) else {
        return false;
    };
    d >= start && d <= end
}

/// Quantity still owned on `pay_on`: lots opened on or before that date.
pub fn qty_open_on(lots: &[OpenLotQty], pay_on: &str) -> (i64, u8) {
    let Some(pay) = parse_iso_day(pay_on) else {
        return (0, 0);
    };
    let mut qty = 0i64;
    let mut scale = 0u8;
    for lot in lots {
        if lot.quantity_minor <= 0 {
            continue;
        }
        let Some(opened) = parse_iso_day(&lot.opened_on) else {
            continue;
        };
        if opened <= pay {
            qty = qty.saturating_add(lot.quantity_minor);
            scale = lot.quantity_scale;
        }
    }
    (qty, scale)
}

fn unknown(reason: &str) -> RemainingYearSchedule {
    RemainingYearSchedule {
        known: false,
        provenance: reason.to_string(),
        remaining_periods: None,
        year_to_go_minor: None,
        payments: Vec::new(),
        months: Vec::new(),
        orphaned_overrides: Vec::new(),
    }
}

fn remaining_year_end(as_of: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(as_of.year(), 12, 31).unwrap_or(as_of)
}

fn weekly_starts(as_of: NaiveDate, year_end: NaiveDate) -> Vec<NaiveDate> {
    let mut week = week_containing(as_of);
    let mut dates = Vec::new();
    for _ in 0..60 {
        if week.start > year_end {
            break;
        }
        if week.end >= as_of {
            dates.push(week.start);
        }
        week.start += Duration::days(7);
        week.end += Duration::days(7);
    }
    dates
}

fn stepped_dates(anchor: NaiveDate, step_days: i64, as_of: NaiveDate, year_end: NaiveDate) -> Vec<NaiveDate> {
    let step = Duration::days(step_days);
    let mut d = anchor;
    let mut dates = Vec::new();
    for _ in 0..60 {
        if d > year_end {
            break;
        }
        if d >= as_of {
            dates.push(d);
        }
        d += step;
    }
    dates
}

fn apply_overrides(dates: &[NaiveDate], overrides: &[DateOverride]) -> Vec<(NaiveDate, NaiveDate, bool)> {
    dates
        .iter()
        .copied()
        .map(|original| {
            if let Some(ov) = overrides
                .iter()
                .find(|o| parse_iso_day(&o.original_pay_on) == Some(original))
            {
                if let Some(new_on) = parse_iso_day(&ov.pay_on) {
                    return (new_on, original, true);
                }
            }
            (original, original, false)
        })
        .collect()
}

fn orphaned_from(system: &[NaiveDate], overrides: &[DateOverride]) -> Vec<DateOverride> {
    overrides
        .iter()
        .filter(|ov| {
            let original = ov.original_pay_on.trim();
            !original.is_empty()
                && parse_iso_day(original).is_some_and(|d| !system.iter().any(|s| *s == d))
        })
        .cloned()
        .collect()
}

fn assemble(
    dates: Vec<(NaiveDate, NaiveDate, bool)>,
    year_end: NaiveDate,
    provenance: String,
    base_provenance: &str,
    lots: &[OpenLotQty],
    plan_minor: i64,
    plan_scale: u8,
    orphaned: Vec<DateOverride>,
) -> RemainingYearSchedule {
    let mut payments: Vec<RemainingPayment> = dates
        .into_iter()
        .filter(|(d, _, _)| *d <= year_end)
        .map(|(d, original, owner)| {
            let pay_on = d.format("%Y-%m-%d").to_string();
            let (qty, scale) = qty_open_on(lots, &pay_on);
            RemainingPayment {
                pay_on,
                original_pay_on: original.format("%Y-%m-%d").to_string(),
                month: d.format("%Y-%m").to_string(),
                cash_minor: plan_payment_cents(qty, scale, plan_minor, plan_scale),
                owner_override: owner,
                date_provenance: if owner {
                    "owner_override".into()
                } else {
                    base_provenance.to_string()
                },
            }
        })
        .collect();
    payments.sort_by(|a, b| a.pay_on.cmp(&b.pay_on));
    let mut months: Vec<RemainingMonthTotal> = Vec::new();
    for pay in &payments {
        match months.last_mut() {
            Some(last) if last.month == pay.month => last.cash_minor += pay.cash_minor,
            _ => months.push(RemainingMonthTotal {
                month: pay.month.clone(),
                cash_minor: pay.cash_minor,
            }),
        }
    }
    let year_to_go: i64 = payments.iter().map(|p| p.cash_minor).sum();
    RemainingYearSchedule {
        known: true,
        provenance,
        remaining_periods: Some(payments.len() as i64),
        year_to_go_minor: Some(year_to_go),
        payments,
        months,
        orphaned_overrides: orphaned,
    }
}

fn owner_anchor(overrides: &[DateOverride]) -> Option<NaiveDate> {
    overrides
        .iter()
        .filter(|o| o.original_pay_on.trim().is_empty())
        .filter_map(|o| parse_iso_day(&o.pay_on))
        .max()
}

fn single_lot(quantity_minor: i64, quantity_scale: u8) -> OpenLotQty {
    OpenLotQty {
        opened_on: "1900-01-01".into(),
        quantity_minor,
        quantity_scale,
    }
}

/// Rest-of-year payment dates. Weekly = remaining Sat–Fri weeks. Monthly/quarterly walk
/// 30/91 days from the latest parseable declaration period (or an owner-named next date).
pub fn remaining_year_payments(
    as_of: &str,
    latest_payment_period: Option<&str>,
    periods_per_year: u8,
    quantity_minor: i64,
    quantity_scale: u8,
    plan_minor: i64,
    plan_scale: u8,
    overrides: &[DateOverride],
) -> RemainingYearSchedule {
    let lot = single_lot(quantity_minor, quantity_scale);
    remaining_year_from_spec(RemainingYearSpec {
        as_of,
        latest_payment_period,
        periods_per_year,
        lots: std::slice::from_ref(&lot),
        plan_minor,
        plan_scale,
        overrides,
        issuer_pay_ons: &[],
        calendar_policy: CalendarPolicy::parse_or_infer("", periods_per_year),
    })
}

pub fn remaining_year_from_spec(spec: RemainingYearSpec<'_>) -> RemainingYearSchedule {
    let Some(as_of_d) = parse_iso_day(spec.as_of) else {
        return unknown("unknown — as-of is not a date");
    };
    let year_end = remaining_year_end(as_of_d);
    match spec.calendar_policy {
        CalendarPolicy::None => unknown("unknown — cadence is None"),
        CalendarPolicy::IssuerCalendar => issuer_calendar_schedule(spec, as_of_d, year_end),
        CalendarPolicy::DerivedWalk => derived_walk_schedule(spec, as_of_d, year_end),
    }
}

fn issuer_calendar_schedule(
    spec: RemainingYearSpec<'_>,
    as_of_d: NaiveDate,
    year_end: NaiveDate,
) -> RemainingYearSchedule {
    let mut raw: Vec<NaiveDate> = spec
        .issuer_pay_ons
        .iter()
        .filter_map(|p| parse_iso_day(p))
        .filter(|d| *d >= as_of_d && *d <= year_end)
        .collect();
    raw.sort();
    raw.dedup();
    if raw.is_empty() {
        return unknown("unknown — issuer calendar has no remaining pay dates");
    }
    let orphaned = orphaned_from(&raw, spec.overrides);
    let tagged = apply_overrides(&raw, spec.overrides);
    let provenance = if tagged.iter().any(|(_, _, o)| *o) {
        "issuer_calendar + owner date override".into()
    } else {
        "issuer_calendar".into()
    };
    assemble(
        tagged,
        year_end,
        provenance,
        "issuer_calendar",
        spec.lots,
        spec.plan_minor,
        spec.plan_scale,
        orphaned,
    )
}

fn derived_walk_schedule(
    spec: RemainingYearSpec<'_>,
    as_of_d: NaiveDate,
    year_end: NaiveDate,
) -> RemainingYearSchedule {
    if spec.periods_per_year == 52 {
        let raw = weekly_starts(as_of_d, year_end);
        let orphaned = orphaned_from(&raw, spec.overrides);
        let tagged = apply_overrides(&raw, spec.overrides);
        let provenance = if tagged.iter().any(|(_, _, o)| *o) {
            "owner date override".into()
        } else {
            "every remaining Sat–Fri week".into()
        };
        return assemble(
            tagged,
            year_end,
            provenance,
            "derived_walk",
            spec.lots,
            spec.plan_minor,
            spec.plan_scale,
            orphaned,
        );
    }
    let step_days = match spec.periods_per_year {
        12 => 30,
        4 => 91,
        _ => return unknown("unknown — frequency is not Weekly, Monthly, or Quarterly"),
    };
    let decl_anchor = spec.latest_payment_period.and_then(parse_iso_day);
    let named_anchor = owner_anchor(spec.overrides);
    let (anchor, from_owner) = match (named_anchor, decl_anchor) {
        (Some(a), _) => (a, true),
        (None, Some(a)) => (a, false),
        (None, None) => {
            return unknown("unknown — no parseable declaration period");
        }
    };
    let raw = stepped_dates(anchor, step_days, as_of_d, year_end);
    let orphaned = orphaned_from(&raw, spec.overrides);
    let tagged = apply_overrides(&raw, spec.overrides);
    let freq = frequency_label(spec.periods_per_year);
    let provenance = if from_owner || tagged.iter().any(|(_, _, o)| *o) {
        "owner date override".into()
    } else {
        format!(
            "from latest declaration {} + {}",
            anchor.format("%Y-%m-%d"),
            freq
        )
    };
    assemble(
        tagged,
        year_end,
        provenance,
        "derived_walk",
        spec.lots,
        spec.plan_minor,
        spec.plan_scale,
        orphaned,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monthly_walk_from_july_skips_past_and_groups_months() {
        let schedule = remaining_year_payments(
            "2026-08-22",
            Some("2026-07-01"),
            12,
            100,
            0,
            1_000,
            4,
            &[],
        );
        assert!(schedule.known);
        let dates: Vec<_> = schedule.payments.iter().map(|p| p.pay_on.as_str()).collect();
        assert_eq!(
            dates,
            ["2026-08-30", "2026-09-29", "2026-10-29", "2026-11-28", "2026-12-28"]
        );
        assert_eq!(schedule.remaining_periods, Some(5));
        assert_eq!(schedule.year_to_go_minor, Some(5_000));
        assert_eq!(schedule.months.len(), 5);
        assert!(schedule.months.iter().all(|m| m.cash_minor != 0));
        assert!(!schedule.months.iter().any(|m| m.month == "2026-07"));
        assert!(schedule.provenance.contains("2026-07-01"));
        assert!(schedule.provenance.contains("Monthly"));
        assert!(schedule.payments.iter().all(|p| p.date_provenance == "derived_walk"));
    }

    #[test]
    fn unparseable_period_is_unknown_not_zero() {
        let schedule = remaining_year_payments("2026-08-22", Some("w1"), 12, 10, 0, 100, 2, &[]);
        assert!(!schedule.known);
        assert!(schedule.payments.is_empty());
        assert_eq!(schedule.remaining_periods, None);
        assert_eq!(schedule.year_to_go_minor, None);
        assert!(schedule.provenance.contains("no parseable"));
    }

    #[test]
    fn weekly_remaining_includes_as_of_week() {
        let schedule = remaining_year_payments("2026-08-22", None, 52, 1, 0, 100, 2, &[]);
        assert!(schedule.known);
        assert_eq!(schedule.payments[0].pay_on, "2026-08-22");
        assert_eq!(schedule.payments.last().map(|p| p.pay_on.as_str()), Some("2026-12-26"));
        assert_eq!(schedule.remaining_periods, Some(schedule.payments.len() as i64));
        assert!(!schedule.months.iter().any(|m| m.cash_minor == 0 && m.month.is_empty()));
    }

    #[test]
    fn owner_override_replaces_one_date() {
        let schedule = remaining_year_payments(
            "2026-08-22",
            Some("2026-07-01"),
            12,
            1,
            0,
            100,
            2,
            &[DateOverride {
                original_pay_on: "2026-08-30".into(),
                pay_on: "2026-08-31".into(),
            }],
        );
        assert_eq!(schedule.payments[0].pay_on, "2026-08-31");
        assert!(schedule.payments[0].owner_override);
        assert_eq!(schedule.payments[0].date_provenance, "owner_override");
        assert_eq!(schedule.payments[0].month, "2026-08");
        assert_eq!(schedule.provenance, "owner date override");
    }

    #[test]
    fn owner_named_next_date_walks_when_declaration_is_not_a_date() {
        let schedule = remaining_year_payments(
            "2026-08-22",
            Some("obs-1"),
            12,
            1,
            0,
            50,
            2,
            &[DateOverride {
                original_pay_on: String::new(),
                pay_on: "2026-08-31".into(),
            }],
        );
        assert!(schedule.known);
        assert_eq!(schedule.payments[0].pay_on, "2026-08-31");
        assert_eq!(schedule.provenance, "owner date override");
    }

    #[test]
    fn pay_on_in_week_matches_containing_week() {
        assert!(pay_on_in_week("2026-08-30", "2026-08-29", "2026-09-04"));
        assert!(!pay_on_in_week("2026-08-30", "2026-08-22", "2026-08-28"));
    }

    #[test]
    fn latest_parseable_skips_labels() {
        assert_eq!(
            latest_parseable_period(["w1", "2026-07-01", "2026-06-01"]),
            Some("2026-07-01".into())
        );
        assert_eq!(latest_parseable_period(["w1", "obs-2"]), None);
    }

    #[test]
    fn issuer_calendar_uses_published_dates_not_thirty_day_walk() {
        let lots = [OpenLotQty {
            opened_on: "2026-01-01".into(),
            quantity_minor: 10,
            quantity_scale: 0,
        }];
        let dates = [
            "2026-08-31".into(),
            "2026-09-30".into(),
            "2026-10-30".into(),
            "2026-11-30".into(),
            "2026-12-31".into(),
        ];
        let schedule = remaining_year_from_spec(RemainingYearSpec {
            as_of: "2026-08-22",
            latest_payment_period: Some("2026-07-01"),
            periods_per_year: 12,
            lots: &lots,
            plan_minor: 100,
            plan_scale: 2,
            overrides: &[],
            issuer_pay_ons: &dates,
            calendar_policy: CalendarPolicy::IssuerCalendar,
        });
        assert!(schedule.known);
        let pay: Vec<_> = schedule.payments.iter().map(|p| p.pay_on.as_str()).collect();
        assert_eq!(pay, ["2026-08-31", "2026-09-30", "2026-10-30", "2026-11-30", "2026-12-31"]);
        assert_eq!(schedule.year_to_go_minor, Some(5_000));
        assert!(schedule.payments.iter().all(|p| p.date_provenance == "issuer_calendar"));
    }

    #[test]
    fn issuer_calendar_empty_is_unknown_not_walk() {
        let lots = [single_lot(10, 0)];
        let schedule = remaining_year_from_spec(RemainingYearSpec {
            as_of: "2026-08-22",
            latest_payment_period: Some("2026-07-01"),
            periods_per_year: 12,
            lots: &lots,
            plan_minor: 100,
            plan_scale: 2,
            overrides: &[],
            issuer_pay_ons: &[],
            calendar_policy: CalendarPolicy::IssuerCalendar,
        });
        assert!(!schedule.known);
        assert!(schedule.payments.is_empty());
        assert!(schedule.provenance.contains("issuer calendar"));
    }

    #[test]
    fn qty_on_pay_date_excludes_lot_opened_after_that_date() {
        let lots = [
            OpenLotQty {
                opened_on: "2026-01-01".into(),
                quantity_minor: 10,
                quantity_scale: 0,
            },
            OpenLotQty {
                opened_on: "2026-10-01".into(),
                quantity_minor: 10,
                quantity_scale: 0,
            },
        ];
        let schedule = remaining_year_from_spec(RemainingYearSpec {
            as_of: "2026-08-22",
            latest_payment_period: Some("2026-07-01"),
            periods_per_year: 12,
            lots: &lots,
            plan_minor: 100,
            plan_scale: 2,
            overrides: &[],
            issuer_pay_ons: &[],
            calendar_policy: CalendarPolicy::DerivedWalk,
        });
        let aug = schedule.payments.iter().find(|p| p.pay_on == "2026-08-30").unwrap();
        let oct = schedule.payments.iter().find(|p| p.pay_on == "2026-10-29").unwrap();
        assert_eq!(aug.cash_minor, 1_000);
        assert_eq!(oct.cash_minor, 2_000);
    }

    #[test]
    fn issuer_replace_drops_old_date_and_orphans_override() {
        let lots = [single_lot(1, 0)];
        let first = remaining_year_from_spec(RemainingYearSpec {
            as_of: "2026-08-22",
            latest_payment_period: None,
            periods_per_year: 12,
            lots: &lots,
            plan_minor: 100,
            plan_scale: 2,
            overrides: &[DateOverride {
                original_pay_on: "2026-09-10".into(),
                pay_on: "2026-09-11".into(),
            }],
            issuer_pay_ons: &["2026-09-10".into(), "2026-10-10".into()],
            calendar_policy: CalendarPolicy::IssuerCalendar,
        });
        assert_eq!(first.payments[0].pay_on, "2026-09-11");
        assert!(first.orphaned_overrides.is_empty());

        let replaced = remaining_year_from_spec(RemainingYearSpec {
            as_of: "2026-08-22",
            latest_payment_period: None,
            periods_per_year: 12,
            lots: &lots,
            plan_minor: 100,
            plan_scale: 2,
            overrides: &[DateOverride {
                original_pay_on: "2026-09-10".into(),
                pay_on: "2026-09-11".into(),
            }],
            issuer_pay_ons: &["2026-09-15".into(), "2026-10-15".into()],
            calendar_policy: CalendarPolicy::IssuerCalendar,
        });
        assert_eq!(replaced.payments[0].pay_on, "2026-09-15");
        assert!(!replaced.payments[0].owner_override);
        assert_eq!(replaced.orphaned_overrides.len(), 1);
        assert_eq!(replaced.orphaned_overrides[0].original_pay_on, "2026-09-10");
    }

    #[test]
    fn derived_walk_moves_when_anchor_declaration_changes() {
        let first = remaining_year_payments(
            "2026-08-22",
            Some("2026-07-01"),
            12,
            1,
            0,
            100,
            2,
            &[],
        );
        let second = remaining_year_payments(
            "2026-08-22",
            Some("2026-08-15"),
            12,
            1,
            0,
            100,
            2,
            &[],
        );
        assert_eq!(first.payments[0].pay_on, "2026-08-30");
        assert_eq!(second.payments[0].pay_on, "2026-09-14");
        assert_ne!(first.payments[0].pay_on, second.payments[0].pay_on);
    }
}
