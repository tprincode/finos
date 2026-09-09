//! Remaining-year payment dates from issuer calendar or declaration cadence. Unknown is not $0.

use chrono::{Datelike, Duration, Months, NaiveDate};

use crate::calculator::{plan_payment_cents, PaymentCadence};
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

    /// Blank policy: prefer issuer calendar when the name pays; derived walk is fallback at schedule time.
    pub fn parse_or_infer(raw: &str, periods_per_year: u8) -> Self {
        if let Some(policy) = Self::parse(raw) {
            return policy;
        }
        if periods_per_year == 0 {
            Self::None
        } else {
            Self::IssuerCalendar
        }
    }

    /// Stored policy plus live issuer pay rows — vendor dates win when present.
    pub fn resolve(stored: &str, periods_per_year: u8, issuer_pay_count: usize) -> Self {
        if issuer_pay_count > 0 {
            return Self::IssuerCalendar;
        }
        Self::parse_or_infer(stored, periods_per_year)
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

/// A pay/declaration date has occurred only when it is strictly before `as_of`.
/// Future placeholders with a copied amount are not paid history.
pub fn period_has_occurred(period: &str, as_of: &str) -> bool {
    match (parse_iso_day(period), parse_iso_day(as_of)) {
        (Some(d), Some(as_of_d)) => d < as_of_d,
        _ => true,
    }
}

/// Unoccurred copied last-pay / orphan calendar rows. Not an issuer notice.
/// Future planning $ is Plan $/share, not this row.
/// A row entered on or after `as_of` is a live issuer notice — keep it even when
/// the dollar matches a prior paid amount (XPAY and other same-$ monthlies).
pub fn unoccurred_declaration_is_placeholder(
    period: &str,
    amount: i64,
    scale: u8,
    as_of: &str,
    occurred_amounts: &[(i64, u8)],
    pay_ons: &[&str],
    entered_at: &str,
) -> bool {
    if period_has_occurred(period, as_of) {
        return false;
    }
    if let (Some(entered), Some(as_of_d)) = (parse_iso_day(entered_at), parse_iso_day(as_of)) {
        if entered >= as_of_d {
            return false;
        }
    }
    let key = period.trim();
    let key10 = if key.len() >= 10 { &key[..10] } else { key };
    let on_calendar = pay_ons.iter().any(|p| {
        let p = p.trim();
        p == key || (p.len() >= 10 && &p[..10] == key10)
    });
    if !on_calendar {
        return true;
    }
    occurred_amounts
        .iter()
        .any(|(a, s)| crate::money::amounts_equal(*a, *s, amount, scale))
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

/// Pay date in the Sat–Fri week, if this cadence pays that week.
/// Remaining / issuer dates win. Weekly is every week (same weekday as last actual or a remaining date).
/// Monthly / quarterly only when a remaining pay-on falls in the week.
pub fn pay_on_for_week(
    periods_per_year: u8,
    week_start: &str,
    week_end: &str,
    remaining_pay_ons: &[&str],
    last_actual_on: Option<&str>,
) -> Option<String> {
    let mut in_week: Vec<String> = remaining_pay_ons
        .iter()
        .copied()
        .filter(|d| pay_on_in_week(d, week_start, week_end))
        .map(|d| {
            let t = d.trim();
            if t.len() >= 10 {
                t[..10].to_string()
            } else {
                t.to_string()
            }
        })
        .collect();
    in_week.sort();
    if let Some(d) = in_week.first() {
        return Some(d.clone());
    }
    if periods_per_year == 52 {
        return weekly_pay_on_in_week(week_start, week_end, last_actual_on, remaining_pay_ons);
    }
    None
}

fn weekly_pay_on_in_week(
    week_start: &str,
    week_end: &str,
    last_actual_on: Option<&str>,
    remaining_pay_ons: &[&str],
) -> Option<String> {
    let start = parse_iso_day(week_start)?;
    let end = parse_iso_day(week_end)?;
    let anchor = last_actual_on
        .and_then(parse_iso_day)
        .or_else(|| remaining_pay_ons.iter().copied().find_map(parse_iso_day));
    let weekday = anchor.map(|d| d.weekday()).unwrap_or(chrono::Weekday::Fri);
    let mut d = start;
    while d <= end {
        if d.weekday() == weekday {
            return Some(d.format("%Y-%m-%d").to_string());
        }
        d += Duration::days(1);
    }
    Some(end.format("%Y-%m-%d").to_string())
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorPayableConflict {
    pub vendor_pay_on: String,
    pub existing_pay_on: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimePayablePlan {
    pub keep: Vec<String>,
    pub add: Vec<String>,
    pub moves: Vec<(String, String)>,
    pub conflicts: Vec<VendorPayableConflict>,
}

fn year_month(raw: &str) -> Option<String> {
    let d = parse_iso_day(raw)?;
    Some(format!("{:04}-{:02}", d.year(), d.month()))
}

/// Runtime collect: add new unpaid payables and move unoccurred same-month dates.
/// Already-paid months are conflicts — do not silent-supersede. Keep other dates.
pub fn merge_runtime_vendor_payables(
    as_of: &str,
    existing: &[String],
    paid_periods: &[String],
    vendor: &[String],
) -> RuntimePayablePlan {
    let as_of_d = parse_iso_day(as_of);
    let paid_months: Vec<String> = paid_periods
        .iter()
        .filter_map(|p| {
            let ym = year_month(p)?;
            period_has_occurred(p, as_of).then_some(ym)
        })
        .collect();
    let mut keep: Vec<String> = existing
        .iter()
        .filter(|d| !d.trim().is_empty())
        .cloned()
        .collect();
    keep.sort();
    keep.dedup();
    let mut plan = RuntimePayablePlan {
        keep: keep.clone(),
        ..RuntimePayablePlan::default()
    };
    for vendor_raw in vendor {
        let vendor_on = vendor_raw.trim();
        if vendor_on.is_empty() || parse_iso_day(vendor_on).is_none() {
            continue;
        }
        let Some(month) = year_month(vendor_on) else {
            continue;
        };
        if paid_months.iter().any(|m| m == &month) {
            if let Some(existing_on) = keep.iter().find(|d| year_month(d).as_deref() == Some(month.as_str()))
            {
                if existing_on != vendor_on {
                    plan.conflicts.push(VendorPayableConflict {
                        vendor_pay_on: vendor_on.to_string(),
                        existing_pay_on: existing_on.clone(),
                    });
                }
            } else {
                plan.conflicts.push(VendorPayableConflict {
                    vendor_pay_on: vendor_on.to_string(),
                    existing_pay_on: String::new(),
                });
            }
            continue;
        }
        if keep.iter().any(|d| d == vendor_on) {
            continue;
        }
        if let Some(existing_on) = keep.iter().find(|d| year_month(d).as_deref() == Some(month.as_str())).cloned()
        {
            let unoccurred = as_of_d
                .and_then(|as_of| parse_iso_day(&existing_on).map(|d| d >= as_of))
                .unwrap_or(false);
            if unoccurred {
                keep.retain(|d| d != &existing_on);
                keep.push(vendor_on.to_string());
                plan.moves.push((existing_on, vendor_on.to_string()));
            } else {
                plan.conflicts.push(VendorPayableConflict {
                    vendor_pay_on: vendor_on.to_string(),
                    existing_pay_on: existing_on,
                });
            }
            continue;
        }
        keep.push(vendor_on.to_string());
        plan.add.push(vendor_on.to_string());
    }
    keep.sort();
    keep.dedup();
    plan.keep = keep;
    plan
}

/// Stored date is the vendor ex/record leftover; keep the later payable.
/// Same dollar only. Cross-month leftovers (Nov ex → Dec pay) are included.
pub fn leftover_ex_to_payable_moves(
    stored: &[(String, i64, u8)],
    vendor: &[(String, Option<String>, Option<String>, i64, u8)],
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (pay, ex, record, amt, _scale) in vendor {
        if *amt <= 0 {
            continue;
        }
        let Some(pay_d) = parse_iso_day(pay) else {
            continue;
        };
        for leftover in [ex.as_deref(), record.as_deref()].into_iter().flatten() {
            if leftover == pay.as_str() {
                continue;
            }
            let Some(left_d) = parse_iso_day(leftover) else {
                continue;
            };
            if left_d >= pay_d {
                continue;
            }
            let hit = stored.iter().any(|(on, _, _)| on == leftover);
            if hit && !out.iter().any(|(from, to)| from == leftover && to == pay) {
                out.push((leftover.to_string(), pay.clone()));
            }
        }
    }
    out
}

/// Remaining unpaid dates through 31 Dec from paid history + cadence.
/// Monthly: walk one month from the latest paid day-of-month (not last calendar day).
pub fn derive_remaining_pay_ons(
    as_of: &str,
    payment_frequency: &str,
    paid_periods: &[&str],
) -> Vec<String> {
    let Some(as_of_d) = parse_iso_day(as_of) else {
        return Vec::new();
    };
    let year_end = remaining_year_end(as_of_d);
    let mut paid: Vec<NaiveDate> = paid_periods
        .iter()
        .filter_map(|p| parse_iso_day(p))
        .filter(|d| *d < as_of_d)
        .collect();
    paid.sort();
    paid.dedup();
    let Some(latest) = paid.last().copied() else {
        return Vec::new();
    };
    match PaymentCadence::parse(payment_frequency).and_then(PaymentCadence::periods) {
        Some(12) => {
            let mut out = Vec::new();
            let mut cursor = latest;
            for _ in 0..12 {
                let Some(next) = cursor.checked_add_months(Months::new(1)) else {
                    break;
                };
                cursor = next;
                if cursor > year_end {
                    break;
                }
                if cursor >= as_of_d {
                    out.push(cursor.format("%Y-%m-%d").to_string());
                }
            }
            out
        }
        Some(4) => {
            let mut out = Vec::new();
            let mut cursor = latest;
            for _ in 0..4 {
                let Some(next) = cursor.checked_add_signed(Duration::days(91)) else {
                    break;
                };
                cursor = next;
                if cursor > year_end {
                    break;
                }
                if cursor >= as_of_d {
                    out.push(cursor.format("%Y-%m-%d").to_string());
                }
            }
            out
        }
        _ => Vec::new(),
    }
}

/// mlp_sec_8k remaining-year: +1 calendar quarter from last accepted payable, stop at 31 Dec.
pub fn derive_quarterly_template_pay_ons(as_of: &str, paid_periods: &[&str]) -> Vec<String> {
    let Some(as_of_d) = parse_iso_day(as_of) else {
        return Vec::new();
    };
    let year_end = remaining_year_end(as_of_d);
    let mut paid: Vec<NaiveDate> = paid_periods
        .iter()
        .filter_map(|p| parse_iso_day(p))
        .filter(|d| *d < as_of_d)
        .collect();
    paid.sort();
    paid.dedup();
    let Some(latest) = paid.last().copied() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut cursor = latest;
    for _ in 0..4 {
        let Some(next) = cursor.checked_add_months(Months::new(3)) else {
            break;
        };
        cursor = next;
        if cursor > year_end {
            break;
        }
        if cursor >= as_of_d {
            out.push(cursor.format("%Y-%m-%d").to_string());
        }
    }
    out
}

/// Remaining pay periods from `as_of` through 31 Dec, capped at 4 / 12 / 52.
/// A monthly name in August has 5 remaining months, not 12.
pub fn remaining_periods_to_year_end(as_of: &str, periods_per_year: u8) -> Option<u8> {
    let as_of_d = NaiveDate::parse_from_str(as_of.trim(), "%Y-%m-%d").ok()?;
    let cap = match periods_per_year {
        4 | 12 | 52 => periods_per_year,
        _ => return None,
    };
    let year_end = remaining_year_end(as_of_d);
    if as_of_d > year_end {
        return Some(0);
    }
    let raw = match cap {
        12 => 13u32.saturating_sub(u32::from(as_of_d.month())) as u8,
        4 => {
            let quarter = ((as_of_d.month() - 1) / 3) + 1;
            5u8.saturating_sub(quarter as u8)
        }
        52 => weekly_starts(as_of_d, year_end).len().min(usize::from(cap)) as u8,
        _ => return None,
    };
    Some(raw.min(cap))
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

fn last_day_of_month(year: i32, month: u32) -> Option<NaiveDate> {
    if month == 12 {
        NaiveDate::from_ymd_opt(year, 12, 31)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).and_then(|d| d.pred_opt())
    }
}

/// Last calendar day of each remaining month from `as_of` through 31 Dec.
fn monthly_last_calendar_days(as_of: NaiveDate, year_end: NaiveDate) -> Vec<NaiveDate> {
    let mut dates = Vec::new();
    let mut month = as_of.month();
    let year = as_of.year();
    for _ in 0..12 {
        let Some(last) = last_day_of_month(year, month) else {
            break;
        };
        if last >= as_of && last <= year_end {
            dates.push(last);
        }
        if month == 12 || last >= year_end {
            break;
        }
        month += 1;
    }
    dates
}

fn apply_owner_month_anchor(
    raw: &mut Vec<NaiveDate>,
    overrides: &[DateOverride],
    as_of: NaiveDate,
    year_end: NaiveDate,
) -> bool {
    let Some(anchor) = owner_anchor(overrides) else {
        return false;
    };
    if anchor < as_of || anchor > year_end {
        return false;
    }
    raw.retain(|d| d.year() != anchor.year() || d.month() != anchor.month());
    raw.push(anchor);
    raw.sort();
    raw.dedup();
    true
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

/// Rest-of-year payment dates. Weekly = remaining Sat–Fri weeks. Monthly = last
/// calendar day of each remaining month through 31 Dec. Quarterly walks ~91 days
/// from the latest parseable declaration period (or an owner-named next date).
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
        CalendarPolicy::IssuerCalendar => {
            if spec.issuer_pay_ons.is_empty() && spec.periods_per_year > 0 {
                derived_walk_schedule(spec, as_of_d, year_end)
            } else {
                issuer_calendar_schedule(spec, as_of_d, year_end)
            }
        }
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
    if spec.periods_per_year == 12 {
        let mut raw = monthly_last_calendar_days(as_of_d, year_end);
        let from_owner = apply_owner_month_anchor(&mut raw, spec.overrides, as_of_d, year_end);
        let orphaned = orphaned_from(&raw, spec.overrides);
        let tagged = apply_overrides(&raw, spec.overrides);
        let provenance = if from_owner || tagged.iter().any(|(_, _, o)| *o) {
            "owner date override".into()
        } else {
            "last calendar day of each remaining month through 31 Dec".into()
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
    fn leftover_ex_moves_to_later_payable_including_cross_month() {
        let stored = vec![
            ("2025-11-24".into(), 1728, 4),
            ("2026-08-24".into(), 1829, 4),
        ];
        let vendor = vec![
            (
                "2025-12-02".into(),
                Some("2025-11-24".into()),
                Some("2025-11-24".into()),
                1728,
                4,
            ),
            (
                "2026-08-27".into(),
                Some("2026-08-24".into()),
                Some("2026-08-24".into()),
                1829,
                4,
            ),
        ];
        let moves = leftover_ex_to_payable_moves(&stored, &vendor);
        assert!(moves.contains(&("2025-11-24".into(), "2025-12-02".into())));
        assert!(moves.contains(&("2026-08-24".into(), "2026-08-27".into())));
        let rounded = leftover_ex_to_payable_moves(
            &[("2014-07-23".into(), 209375, 6)],
            &[(
                "2014-07-30".into(),
                Some("2014-07-23".into()),
                Some("2014-07-25".into()),
                20938,
                5,
            )],
        );
        assert_eq!(rounded, vec![("2014-07-23".into(), "2014-07-30".into())]);
    }

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
            ["2026-08-31", "2026-09-30", "2026-10-31", "2026-11-30", "2026-12-31"]
        );
        assert_eq!(schedule.remaining_periods, Some(5));
        assert_eq!(schedule.year_to_go_minor, Some(5_000));
        assert_eq!(schedule.months.len(), 5);
        assert!(schedule.months.iter().all(|m| m.cash_minor != 0));
        assert!(!schedule.months.iter().any(|m| m.month == "2026-07"));
        assert!(schedule.provenance.contains("last calendar day"));
        assert!(schedule.payments.iter().all(|p| p.date_provenance == "derived_walk"));
    }

    #[test]
    fn unparseable_period_is_unknown_not_zero() {
        let schedule = remaining_year_payments("2026-08-22", Some("w1"), 4, 10, 0, 100, 2, &[]);
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
                original_pay_on: "2026-08-31".into(),
                pay_on: "2026-08-28".into(),
            }],
        );
        assert_eq!(schedule.payments[0].pay_on, "2026-08-28");
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
    fn issuer_calendar_empty_falls_back_to_derived_walk() {
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
        assert!(schedule.known);
        assert!(!schedule.payments.is_empty());
        assert!(schedule
            .payments
            .iter()
            .all(|p| p.date_provenance == "derived_walk"));
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
        let aug = schedule.payments.iter().find(|p| p.pay_on == "2026-08-31").unwrap();
        let oct = schedule.payments.iter().find(|p| p.pay_on == "2026-10-31").unwrap();
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
    fn monthly_last_calendar_days_ignore_declaration_anchor() {
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
        assert_eq!(first.payments[0].pay_on, "2026-08-31");
        assert_eq!(second.payments[0].pay_on, "2026-08-31");
        assert_eq!(first.payments.last().map(|p| p.pay_on.as_str()), Some("2026-12-31"));
    }

    #[test]
    fn pay_on_for_week_weekly_every_week_and_monthly_uses_remaining() {
        assert_eq!(
            pay_on_for_week(52, "2026-08-29", "2026-09-04", &[], Some("2026-07-28")),
            Some("2026-09-01".into())
        );
        assert_eq!(
            pay_on_for_week(
                52,
                "2026-08-29",
                "2026-09-04",
                &["2026-09-01"],
                Some("2026-07-28")
            ),
            Some("2026-09-01".into())
        );
        assert_eq!(
            pay_on_for_week(12, "2026-08-29", "2026-09-04", &["2026-08-31"], None),
            Some("2026-08-31".into())
        );
        assert_eq!(
            pay_on_for_week(12, "2026-08-22", "2026-08-28", &["2026-08-31"], Some("2026-07-31")),
            None
        );
    }

    #[test]
    fn derive_monthly_walks_from_latest_paid_day() {
        let paid = [
            "2025-10-03",
            "2025-11-05",
            "2025-12-03",
            "2026-01-05",
            "2026-02-04",
            "2026-03-04",
            "2026-04-06",
            "2026-05-05",
            "2026-06-03",
            "2026-07-06",
            "2026-08-05",
            "2026-09-03",
        ];
        let refs: Vec<&str> = paid.to_vec();
        assert_eq!(
            derive_remaining_pay_ons("2026-09-07", "Monthly", &refs),
            ["2026-10-03", "2026-11-03", "2026-12-03"]
        );
        let with_future_placeholder = [
            "2026-09-03",
            "2026-10-01",
        ];
        let refs: Vec<&str> = with_future_placeholder.to_vec();
        assert_eq!(
            derive_remaining_pay_ons("2026-09-07", "Monthly", &refs),
            ["2026-10-03", "2026-11-03", "2026-12-03"]
        );
    }

    #[test]
    fn remaining_periods_to_year_end_is_stub_year_not_full_year() {
        assert_eq!(remaining_periods_to_year_end("2026-08-22", 12), Some(5));
        assert_eq!(remaining_periods_to_year_end("2026-08-22", 4), Some(2));
        assert_eq!(remaining_periods_to_year_end("2026-12-01", 12), Some(1));
        assert_eq!(remaining_periods_to_year_end("2026-01-15", 12), Some(12));
    }

    #[test]
    fn runtime_vendor_adds_and_moves_unoccurred_only() {
        let plan = merge_runtime_vendor_payables(
            "2026-09-05",
            &["2026-09-30".into(), "2026-10-31".into(), "2026-11-30".into()],
            &["2026-08-31".into()],
            &["2026-09-28".into(), "2026-12-31".into()],
        );
        assert_eq!(plan.add, ["2026-12-31"]);
        assert_eq!(plan.moves, [("2026-09-30".into(), "2026-09-28".into())]);
        assert!(plan.conflicts.is_empty());
        assert_eq!(
            plan.keep,
            ["2026-09-28", "2026-10-31", "2026-11-30", "2026-12-31"]
        );
    }

    #[test]
    fn unoccurred_copy_and_orphan_are_placeholders() {
        assert!(unoccurred_declaration_is_placeholder(
            "2026-10-03",
            70497,
            5,
            "2026-09-07",
            &[(68255, 5), (70497, 5)],
            &["2026-10-03"],
            "2026-08-15",
        ));
        assert!(unoccurred_declaration_is_placeholder(
            "2026-09-30",
            1215,
            4,
            "2026-09-07",
            &[(1215, 4)],
            &["2026-09-15"],
            "2026-08-15",
        ));
        assert!(unoccurred_declaration_is_placeholder(
            "2026-11-06",
            34,
            2,
            "2026-09-07",
            &[(3400, 4)],
            &["2026-11-06"],
            "2026-08-15",
        ));
        assert!(!unoccurred_declaration_is_placeholder(
            "2026-09-09",
            74947,
            5,
            "2026-09-07",
            &[(572959, 6)],
            &["2026-09-09"],
            "2026-08-15",
        ));
        assert!(!unoccurred_declaration_is_placeholder(
            "2026-09-03",
            68255,
            5,
            "2026-09-07",
            &[(68255, 5)],
            &["2026-09-03"],
            "2026-09-03",
        ));
        assert!(!unoccurred_declaration_is_placeholder(
            "2026-09-10",
            899537,
            6,
            "2026-09-09",
            &[(899537, 6)],
            &["2026-09-10", "2026-10-15"],
            "2026-09-09",
        ));
    }

    #[test]
    fn runtime_vendor_moves_future_placeholder_with_amount() {
        let plan = merge_runtime_vendor_payables(
            "2026-09-07",
            &["2026-10-01".into(), "2026-11-03".into()],
            &["2026-09-03".into(), "2026-10-01".into()],
            &["2026-10-03".into()],
        );
        assert_eq!(plan.moves, [("2026-10-01".into(), "2026-10-03".into())]);
        assert!(plan.conflicts.is_empty());
        assert!(plan.keep.contains(&"2026-10-03".to_string()));
        assert!(!plan.keep.contains(&"2026-10-01".to_string()));
    }

    #[test]
    fn runtime_vendor_does_not_supersede_paid_month() {
        let plan = merge_runtime_vendor_payables(
            "2026-09-05",
            &["2026-08-31".into(), "2026-09-30".into()],
            &["2026-08-31".into()],
            &["2026-08-28".into()],
        );
        assert!(plan.add.is_empty());
        assert!(plan.moves.is_empty());
        assert_eq!(plan.conflicts.len(), 1);
        assert_eq!(plan.conflicts[0].existing_pay_on, "2026-08-31");
        assert!(plan.keep.contains(&"2026-08-31".to_string()));
    }

    #[test]
    fn mlp_sec_8k_derive_one_remaining_2026_quarter_no_2027() {
        let dates = derive_quarterly_template_pay_ons("2026-09-08", &["2026-08-19"]);
        assert_eq!(dates, vec!["2026-11-19".to_string()]);
        assert!(!dates.iter().any(|d| d.starts_with("2027")));
    }
}
