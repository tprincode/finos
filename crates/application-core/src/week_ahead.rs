//! Slice 2 Week Ahead: unconfirmed planned Element occurrences for this Sat–Fri week.

use crate::cash_management::{cash_distribution_post, ssa_confirm};
use crate::contracts::{
    ActivityRecord, CashElementRecord, PlannedOccurrenceRecord, WeekAheadBody, WeekAheadRow,
};
use crate::ports::canonical::Canonical;
use crate::ports::platform::PlatformError;
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use financial_domain::cash_management::{
    week_ahead_in_window, BARBARA_SSA_EXPECTED_MINOR, TOM_SSA_EXPECTED_MINOR,
};
use financial_domain::trends::parse_iso_date;
use std::collections::HashSet;
use uuid::Uuid;

pub fn invalidate_horizon_cache() {}

fn sat_of(as_of: &str) -> Result<(NaiveDate, NaiveDate, String, String), PlatformError> {
    let day = parse_iso_date(as_of)
        .ok_or_else(|| PlatformError::new("bad_date", format!("invalid asOfDate {as_of}")))?;
    let week = financial_domain::trends::trends_period_for_capture(day);
    Ok((
        week.start,
        week.end,
        week.start.format("%Y-%m-%d").to_string(),
        week.end.format("%Y-%m-%d").to_string(),
    ))
}

fn ui_transaction(kind: &str) -> String {
    if kind.eq_ignore_ascii_case("Deposit") {
        "Deposit".into()
    } else {
        "Withdrawal".into()
    }
}

pub(crate) fn ledger_account_name(occurrence_account: &str) -> &'static str {
    if occurrence_account.eq_ignore_ascii_case("SSA_2026") {
        "External"
    } else if occurrence_account.eq_ignore_ascii_case("9") {
        "9"
    } else if occurrence_account.eq_ignore_ascii_case("FI Roth") {
        "FI Roth"
    } else if occurrence_account.eq_ignore_ascii_case("Health") {
        "Health"
    } else if occurrence_account.eq_ignore_ascii_case("Car") {
        "Car"
    } else {
        "Income"
    }
}

fn series_window(
    as_of: &str,
    start_on: &str,
    stop_on: &str,
) -> Result<(NaiveDate, NaiveDate), PlatformError> {
    let as_of_d = parse_iso_date(as_of)
        .ok_or_else(|| PlatformError::new("bad_date", format!("invalid asOfDate {as_of}")))?;
    let mut start = as_of_d;
    if let Some(bound) = parse_iso_date(start_on) {
        if bound > start {
            start = bound;
        }
    }
    let mut end = as_of_d + Duration::days(366);
    if let Some(bound) = parse_iso_date(stop_on) {
        end = bound;
    }
    Ok((start, end))
}

fn is_saturday(on: &str) -> bool {
    parse_iso_date(on).is_some_and(|d| d.weekday() == Weekday::Sat)
}

/// Drop leftover seed Saturdays and off-schedule future rows so Week Ahead
/// and the Elements catalog share the same series.
pub(crate) async fn prune_off_schedule_occurrences(
    canonical: &dyn Canonical,
    as_of: &str,
) -> Result<(), PlatformError> {
    let (_, _, week_start, week_end) = sat_of(as_of)?;
    let elements = canonical.cash_element_list().await?;
    let occs = canonical.planned_occurrence_list().await?;
    for element in elements {
        if element.cadence == "one-time" {
            continue;
        }
        let (win_start, win_end) = series_window(as_of, &element.start_on, &element.stop_on)?;
        let want: HashSet<String> = financial_domain::cash_management::element_horizon_dates(
            &element.cadence,
            &element.weekday_or_month_day,
            win_start,
            win_end,
        )
        .into_iter()
        .collect();
        let mut seen = HashSet::new();
        for row in occs.iter().filter(|o| o.element_id == element.element_id) {
            if row.confirmed_at.is_some() {
                continue;
            }
            if row.is_exception || row.is_cancelled {
                continue;
            }
            if want.contains(&row.occurred_on) {
                if !seen.insert(row.occurred_on.clone()) {
                    canonical
                        .planned_occurrence_delete(row.occurrence_id)
                        .await?;
                    continue;
                }
                if row.amount_minor != element.amount_minor || row.note != element.note {
                    let mut next = row.clone();
                    next.amount_minor = element.amount_minor;
                    next.note = element.note.clone();
                    canonical.planned_occurrence_upsert(next).await?;
                }
                continue;
            }
            let past = row.occurred_on.as_str() <= as_of;
            let leftover_saturday_dump =
                is_saturday(&row.occurred_on) && element.cadence != "weekly";
            let same_week_weekly_move = element.cadence == "weekly"
                && week_ahead_in_window(
                    &element.account,
                    &row.occurred_on,
                    &week_start,
                    &week_end,
                );
            if (past && !leftover_saturday_dump) || same_week_weekly_move {
                continue;
            }
            canonical
                .planned_occurrence_delete(row.occurrence_id)
                .await?;
        }
    }
    Ok(())
}

pub(crate) async fn ensure_seed(canonical: &dyn Canonical, _saturday: &str) -> Result<(), PlatformError> {
    let existing = canonical.planned_occurrence_list().await?;
    if !existing.is_empty() {
        return Ok(());
    }
    let elements = canonical.cash_element_list().await?;
    if elements.is_empty() {
        let templates = [
            (
                "Income",
                "Withdrawal",
                "weekly",
                77_500_i64,
                "net",
                "Sat",
            ),
            (
                "Income",
                "Withdrawal",
                "weekly",
                18_000,
                "fed",
                "Sat",
            ),
            (
                "Income",
                "Withdrawal",
                "weekly",
                4_500,
                "state",
                "Sat",
            ),
            (
                "SSA_2026",
                "Deposit",
                "monthly",
                TOM_SSA_EXPECTED_MINOR,
                "tom",
                "1",
            ),
            (
                "SSA_2026",
                "Deposit",
                "monthly",
                BARBARA_SSA_EXPECTED_MINOR,
                "barbara",
                "1",
            ),
            ("Car", "Withdrawal", "monthly", 85_000, "car", "1"),
            ("Health", "Withdrawal", "monthly", 20_000, "hsa1", "1"),
            ("Health", "Withdrawal", "monthly", 15_000, "hsa2", "1"),
        ];
        for (account, kind, cadence, amount, note, day) in templates {
            let record = CashElementRecord {
                element_id: Uuid::new_v4(),
                account: account.into(),
                kind: kind.into(),
                cadence: cadence.into(),
                amount_minor: amount,
                note: note.into(),
                weekday_or_month_day: day.into(),
                start_on: String::new(),
                stop_on: String::new(),
            };
            canonical.cash_element_upsert(record).await?;
        }
    }
    Ok(())
}

pub(crate) async fn ensure_horizon(
    canonical: &dyn Canonical,
    start: &str,
    end: &str,
) -> Result<(), PlatformError> {
    let start_d = parse_iso_date(start)
        .ok_or_else(|| PlatformError::new("bad_date", format!("invalid start {start}")))?;
    let end_d = parse_iso_date(end)
        .ok_or_else(|| PlatformError::new("bad_date", format!("invalid end {end}")))?;
    let mut existing = canonical.planned_occurrence_list().await?;
    let elements = canonical.cash_element_list().await?;
    let week_start_of = |on: &str| {
        parse_iso_date(on).map(|day| financial_domain::week::week_containing(day).start)
    };
    for element in elements {
        if element.amount_minor <= 0 {
            continue;
        }
        let mut range_start = start_d;
        let mut range_end = end_d;
        if let Some(s) = parse_iso_date(&element.start_on) {
            if s > range_start {
                range_start = s;
            }
        }
        if let Some(t) = parse_iso_date(&element.stop_on) {
            if t < range_end {
                range_end = t;
            }
        }
        if range_start > range_end {
            continue;
        }
        for on in financial_domain::cash_management::element_horizon_dates(
            &element.cadence,
            &element.weekday_or_month_day,
            range_start,
            range_end,
        ) {
            if existing.iter().any(|o| {
                o.element_id == element.element_id
                    && (o.occurred_on == on
                        || (o.confirmed_at.is_none()
                            && week_start_of(&o.occurred_on) == week_start_of(&on)))
            }) {
                continue;
            }
            let row = canonical
                .planned_occurrence_upsert(PlannedOccurrenceRecord {
                    occurrence_id: Uuid::new_v4(),
                    element_id: element.element_id,
                    account: element.account.clone(),
                    kind: element.kind.clone(),
                    occurred_on: on,
                    amount_minor: element.amount_minor,
                    confirmed_at: None,
                    note: element.note.clone(),
                    is_exception: false,
                    is_cancelled: false,
                })
                .await?;
            existing.push(row);
        }
    }
    Ok(())
}

pub async fn week_ahead_get(
    canonical: &dyn Canonical,
    as_of: &str,
) -> Result<WeekAheadBody, PlatformError> {
    let (sat, _, start, end) = sat_of(as_of)?;
    let lookback = (sat - Duration::days(4)).format("%Y-%m-%d").to_string();
    ensure_seed(canonical, &start).await?;
    ensure_horizon(canonical, &lookback, &end).await?;
    prune_off_schedule_occurrences(canonical, as_of).await?;
    let elements = canonical.cash_element_list().await?;
    let mut rows: Vec<WeekAheadRow> = canonical
        .planned_occurrence_list()
        .await?
        .into_iter()
        .filter(|o| o.confirmed_at.is_none() && !o.is_cancelled)
        .filter(|o| week_ahead_in_window(&o.account, &o.occurred_on, &start, &end))
        .filter_map(|o| {
            let element = elements.iter().find(|e| e.element_id == o.element_id)?;
            if element.note.eq_ignore_ascii_case("dividend")
                || element.account.eq_ignore_ascii_case("CLM")
                || element.account.eq_ignore_ascii_case("CRF")
            {
                return None;
            }
            Some(WeekAheadRow {
                occurrence_id: o.occurrence_id,
                element_id: o.element_id,
                occurred_on: o.occurred_on,
                account: element.account.clone(),
                transaction: ui_transaction(&element.kind),
                amount_minor: if o.is_exception {
                    o.amount_minor
                } else {
                    element.amount_minor
                },
                note: element.note.clone(),
                scale: 2,
            })
        })
        .collect();
    rows.sort_by(|a, b| {
        a.occurred_on
            .cmp(&b.occurred_on)
            .then(a.account.cmp(&b.account))
            .then(a.note.cmp(&b.note))
    });
    Ok(WeekAheadBody {
        period_start: start,
        period_end: end,
        rows,
        scale: 2,
    })
}

pub async fn week_ahead_edit(
    canonical: &dyn Canonical,
    occurrence_id: Uuid,
    occurred_on: Option<String>,
    amount_minor: Option<i64>,
) -> Result<PlannedOccurrenceRecord, PlatformError> {
    let mut row = canonical.planned_occurrence_get(occurrence_id).await?;
    if row.confirmed_at.is_some() {
        return Err(PlatformError::new(
            "occurrence_confirmed",
            "posted history stays frozen",
        ));
    }
    if let Some(on) = occurred_on {
        if parse_iso_date(&on).is_none() {
            return Err(PlatformError::new("bad_date", format!("invalid occurredOn {on}")));
        }
        row.occurred_on = on;
    }
    if let Some(amt) = amount_minor {
        if amt <= 0 {
            return Err(PlatformError::new("unknown_amount", "amount must be known"));
        }
        row.amount_minor = amt;
    }
    canonical.planned_occurrence_upsert(row).await
}

pub async fn week_ahead_defer(
    canonical: &dyn Canonical,
    occurrence_id: Uuid,
) -> Result<PlannedOccurrenceRecord, PlatformError> {
    let row = canonical.planned_occurrence_get(occurrence_id).await?;
    if row.confirmed_at.is_some() {
        return Err(PlatformError::new(
            "occurrence_confirmed",
            "posted history stays frozen",
        ));
    }
    let day = parse_iso_date(&row.occurred_on)
        .ok_or_else(|| PlatformError::new("bad_date", "invalid occurredOn"))?;
    let next = (day + Duration::days(1)).format("%Y-%m-%d").to_string();
    week_ahead_edit(canonical, occurrence_id, Some(next), None).await
}

async fn account_id_named(
    canonical: &dyn Canonical,
    name: &str,
) -> Result<Uuid, PlatformError> {
    let accounts = canonical.account_list().await?;
    accounts
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case(name))
        .map(|a| a.account_id)
        .ok_or_else(|| PlatformError::new("missing_account", format!("account {name} is required")))
}

pub async fn week_ahead_confirm(
    canonical: &dyn Canonical,
    occurrence_id: Uuid,
) -> Result<ActivityRecord, PlatformError> {
    let mut row = canonical.planned_occurrence_get(occurrence_id).await?;
    if row.confirmed_at.is_some() {
        return Err(PlatformError::new(
            "occurrence_confirmed",
            "already confirmed",
        ));
    }
    let ledger_name = ledger_account_name(&row.account);
    let account_id = account_id_named(canonical, ledger_name).await?;
    let posted = if row.account.eq_ignore_ascii_case("SSA_2026") {
        let payee = if row.note.to_ascii_lowercase().contains("barbara") {
            "barbara"
        } else {
            "tom"
        };
        ssa_confirm(
            canonical,
            account_id,
            row.occurred_on.clone(),
            Some(row.amount_minor),
            2,
            Some(payee.into()),
        )
        .await?
    } else if row.account.eq_ignore_ascii_case("Income") {
        cash_distribution_post(
            canonical,
            account_id,
            "IRA_Distribution".into(),
            row.occurred_on.clone(),
            Some(row.amount_minor),
            0,
            0,
            2,
            Some(format!("week-ahead-{}", row.occurrence_id)),
        )
        .await?
    } else if row.account.eq_ignore_ascii_case("Car") {
        cash_distribution_post(
            canonical,
            account_id,
            "Withdrawal".into(),
            row.occurred_on.clone(),
            Some(row.amount_minor),
            0,
            0,
            2,
            Some(format!("week-ahead-{}", row.occurrence_id)),
        )
        .await?
    } else if row.account.eq_ignore_ascii_case("Health") {
        cash_distribution_post(
            canonical,
            account_id,
            "HSA_Withdrawal".into(),
            row.occurred_on.clone(),
            Some(row.amount_minor),
            0,
            0,
            2,
            Some(format!("week-ahead-{}", row.occurrence_id)),
        )
        .await?
    } else {
        return Err(PlatformError::new(
            "cash_account_kind",
            format!("Week Ahead Confirm does not post {}", row.account),
        ));
    };
    row.confirmed_at = Some(chrono::Utc::now().to_rfc3339());
    canonical.planned_occurrence_upsert(row).await?;
    Ok(posted)
}
