//! Slice 3 Cash Register: one book, Actual/Planned rows, running cash.

use crate::contracts::{
    CashElementHistoryBody, CashElementHistoryRow, CashElementHistoryTotals, CashElementListBody,
    CashElementListItem, CashElementOccurrenceInput, CashElementRecord, CashElementSaveBody,
    CashRegisterBody, CashRegisterRow, CashRegisterSeriesPoint, PlannedOccurrenceRecord,
    PlannedOccurrenceSaveBody,
};
use crate::ports::canonical::Canonical;
use crate::ports::platform::PlatformError;
use crate::week_ahead::{ensure_horizon, ensure_seed};
use financial_domain::cash_management::{
    book_matches_income_plan_account, element_history_period_bounds, element_history_posted_slice,
    element_horizon_dates, element_is_retired,
    fold_running_cash, is_cash_adjust_type,
    is_cash_distribution_type, register_book_label, register_ledger_account, register_period_bounds,
};
use financial_domain::trends::parse_iso_date;
use uuid::Uuid;

fn ui_transaction(kind: &str, amount_minor: i64) -> &'static str {
    if kind.eq_ignore_ascii_case("Deposit") || (is_cash_adjust_type(kind) && amount_minor > 0) {
        "Deposit"
    } else {
        "Withdrawal"
    }
}

fn is_dividend_type(activity_type: &str) -> bool {
    activity_type.eq_ignore_ascii_case("dividend")
}

fn sat_of(as_of: &str) -> Result<String, PlatformError> {
    let day = parse_iso_date(as_of)
        .ok_or_else(|| PlatformError::new("bad_date", format!("invalid asOfDate {as_of}")))?;
    Ok(financial_domain::trends::trends_period_for_capture(day)
        .start
        .format("%Y-%m-%d")
        .to_string())
}

async fn start_cash_minor(
    canonical: &dyn Canonical,
    book: &str,
    period_start: &str,
) -> Result<Option<i64>, PlatformError> {
    let ledger = register_ledger_account(book);
    let accounts = canonical.account_list().await?;
    let Some(account) = accounts
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case(ledger))
    else {
        return Ok(None);
    };
    let snaps = canonical.account_balance_snapshot_list().await.unwrap_or_default();
    let latest = snaps
        .iter()
        .filter(|s| {
            s.account_id == account.account_id
                && s.cash_minor.is_some()
                && s.period_end.as_str() < period_start
        })
        .max_by(|a, b| a.period_end.cmp(&b.period_end).then(a.captured_at.cmp(&b.captured_at)));
    if let Some(snap) = latest {
        return Ok(snap.cash_minor);
    }
    Ok(None)
}

struct DraftRow {
    occurred_on: String,
    transaction: String,
    amount_minor: i64,
    label: String,
    posted: bool,
    source: String,
    occurrence_id: Option<Uuid>,
    element_id: Option<Uuid>,
}

fn status_of(date: &str, as_of: &str, posted: bool) -> &'static str {
    if date <= as_of && posted {
        "Actual"
    } else {
        "Planned"
    }
}

async fn income_plan_day_deposits(
    canonical: &dyn Canonical,
    book: &str,
    start: &str,
    end: &str,
    as_of: &str,
) -> Result<Vec<DraftRow>, PlatformError> {
    if book.eq_ignore_ascii_case("SSA_2026") {
        return Ok(Vec::new());
    }
    let views =
        crate::queries::income_plan_week_views_in_range(canonical, start, end, as_of).await?;
    let mut by_name: std::collections::BTreeMap<(String, String), i64> =
        std::collections::BTreeMap::new();
    for view in &views {
        for pos in &view.positions {
            if !pos.plan_known || pos.planned_minor == 0 {
                continue;
            }
            if pos.actual_known {
                continue;
            }
            if pos.pay_on.is_empty() || pos.pay_on.as_str() < start || pos.pay_on.as_str() > end {
                continue;
            }
            if pos.pay_on.as_str() < as_of {
                continue;
            }
            let day_sum: i64 = pos
                .accounts
                .iter()
                .filter(|a| a.plan_known && book_matches_income_plan_account(book, &a.account_name))
                .map(|a| a.planned_minor)
                .sum();
            if day_sum > 0 {
                let name = if pos.symbol.trim().is_empty() {
                    "Income Plan".to_string()
                } else {
                    pos.symbol.clone()
                };
                *by_name.entry((pos.pay_on.clone(), name)).or_insert(0) += day_sum;
            }
        }
    }
    Ok(by_name
        .into_iter()
        .map(|((on, name), amt)| DraftRow {
            occurred_on: on,
            transaction: "Deposit".into(),
            amount_minor: amt,
            label: name,
            posted: false,
            source: "income-plan".into(),
            occurrence_id: None,
            element_id: None,
        })
        .collect())
}

pub async fn cash_register_get(
    canonical: &dyn Canonical,
    account: &str,
    period: &str,
    as_of: &str,
    _include_unconfirmed: bool,
    window_start: Option<&str>,
    window_end: Option<&str>,
    hits_only: bool,
) -> Result<CashRegisterBody, PlatformError> {
    let book = register_book_label(account).ok_or_else(|| {
        PlatformError::new("cash_account_kind", format!("{account} is not a Register book"))
    })?;
    let as_of_d = parse_iso_date(as_of)
        .ok_or_else(|| PlatformError::new("bad_date", format!("invalid asOfDate {as_of}")))?;
    let (start, end) = match (window_start, window_end) {
        (Some(start), Some(end)) if start.len() >= 10 && end.len() >= 10 => {
            (start[..10].to_string(), end[..10].to_string())
        }
        _ => {
            let (start_d, end_d) = register_period_bounds(period, as_of_d)
                .map_err(|e| PlatformError::new("bad_date", format!("{e:?}")))?;
            (
                start_d.format("%Y-%m-%d").to_string(),
                end_d.format("%Y-%m-%d").to_string(),
            )
        }
    };
    if !hits_only {
        ensure_seed(canonical, &sat_of(as_of)?).await?;
        ensure_horizon(canonical, &start, &end).await?;
        crate::week_ahead::prune_off_schedule_occurrences(canonical, as_of).await?;
    }

    let ledger = register_ledger_account(book);
    let accounts = canonical.account_list().await?;
    let ledger_id = accounts
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case(ledger))
        .map(|a| a.account_id);

    let mut drafts: Vec<DraftRow> = Vec::new();
    let occurrences = canonical.planned_occurrence_list().await?;
    for o in occurrences {
        if register_book_label(&o.account) != Some(book) {
            continue;
        }
        if o.occurred_on.as_str() < start.as_str() || o.occurred_on.as_str() > end.as_str() {
            continue;
        }
        if o.note.eq_ignore_ascii_case("dividend") {
            continue;
        }
        if o.is_cancelled {
            continue;
        }
        let posted = o.confirmed_at.is_some();
        if !posted && o.occurred_on.as_str() < as_of {
            continue;
        }
        let amount_minor = o.amount_minor.abs();
        drafts.push(DraftRow {
            occurred_on: o.occurred_on,
            transaction: ui_transaction(&o.kind, amount_minor).into(),
            amount_minor,
            label: o.note,
            posted,
            source: "occurrence".into(),
            occurrence_id: Some(o.occurrence_id),
            element_id: Some(o.element_id),
        });
    }

    if let Some(account_id) = ledger_id {
        for act in canonical
            .activity_list_in_range(Some(account_id), &start, &end)
            .await?
        {
            if act.idempotency_key.starts_with("week-ahead-") {
                continue;
            }
            if is_dividend_type(&act.activity_type) {
                continue;
            }
            if is_cash_adjust_type(&act.activity_type) {
                let txn = ui_transaction("Cash_Adjust", act.amount_minor);
                drafts.push(DraftRow {
                    occurred_on: act.occurred_on,
                    transaction: txn.into(),
                    amount_minor: act.amount_minor.abs(),
                    label: if act.note.trim().is_empty() {
                        "Adjust".into()
                    } else {
                        format!("Adjust {}", act.note)
                    },
                    posted: true,
                    source: "adjust".into(),
                    occurrence_id: None,
                    element_id: None,
                });
                continue;
            }
            if is_cash_distribution_type(&act.activity_type)
                || act.activity_type.eq_ignore_ascii_case("SSA")
            {
                let txn = if book.eq_ignore_ascii_case("SSA_2026") {
                    "Deposit"
                } else {
                    "Withdrawal"
                };
                drafts.push(DraftRow {
                    occurred_on: act.occurred_on,
                    transaction: txn.into(),
                    amount_minor: act.amount_minor.abs(),
                    label: act.activity_type,
                    posted: true,
                    source: "activity".into(),
                    occurrence_id: None,
                    element_id: None,
                });
            }
        }
        let dividend = canonical.dividend_get().await?;
        let securities = canonical.security_list().await.unwrap_or_default();
        let mut day: std::collections::BTreeMap<(String, String), i64> =
            std::collections::BTreeMap::new();
        for a in dividend.actuals {
            if a.account_id != account_id {
                continue;
            }
            if a.occurred_on.as_str() < start.as_str() || a.occurred_on.as_str() > end.as_str() {
                continue;
            }
            let name = a
                .security_id
                .and_then(|id| {
                    securities
                        .iter()
                        .find(|s| s.security_id == id)
                        .map(|s| s.symbol.clone())
                })
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "Dividend".into());
            *day.entry((a.occurred_on, name)).or_insert(0) += a.amount_minor;
        }
        for ((on, name), amt) in day {
            if amt == 0 {
                continue;
            }
            drafts.push(DraftRow {
                occurred_on: on,
                transaction: "Deposit".into(),
                amount_minor: amt.abs(),
                label: name,
                posted: true,
                source: "dividend-actual".into(),
                occurrence_id: None,
                element_id: None,
            });
        }
    }

    drafts.extend(income_plan_day_deposits(canonical, book, &start, &end, as_of).await?);
    drafts.sort_by(|a, b| {
        a.occurred_on
            .cmp(&b.occurred_on)
            .then(a.label.cmp(&b.label))
            .then(a.transaction.cmp(&b.transaction))
    });

    let start_minor = if hits_only {
        None
    } else {
        start_cash_minor(canonical, book, &start).await?
    };
    let deltas: Vec<i64> = drafts
        .iter()
        .map(|d| {
            if d.transaction == "Deposit" {
                d.amount_minor
            } else {
                -d.amount_minor
            }
        })
        .collect();
    let running = if hits_only {
        vec![None; drafts.len()]
    } else {
        fold_running_cash(start_minor, &deltas)
    };
    let rows: Vec<CashRegisterRow> = drafts
        .iter()
        .zip(running.iter())
        .map(|(d, run)| CashRegisterRow {
            occurred_on: d.occurred_on.clone(),
            status: status_of(&d.occurred_on, as_of, d.posted).into(),
            transaction: d.transaction.clone(),
            deposit_minor: if d.transaction == "Deposit" {
                d.amount_minor
            } else {
                0
            },
            withdrawal_minor: if d.transaction == "Withdrawal" {
                d.amount_minor
            } else {
                0
            },
            running_minor: *run,
            label: d.label.clone(),
            posted: d.posted,
            source: d.source.clone(),
            occurrence_id: d.occurrence_id,
            element_id: d.element_id,
            scale: 2,
        })
        .collect();

    let series = if hits_only {
        Vec::new()
    } else {
        let mut by_day: std::collections::BTreeMap<String, (i64, Option<i64>)> =
            std::collections::BTreeMap::new();
        if let Some(open) = start_minor {
            by_day.insert(start.clone(), (0, Some(open)));
        }
        for row in &rows {
            let net = row.deposit_minor - row.withdrawal_minor;
            let on_open = row.occurred_on == start;
            let entry = by_day.entry(row.occurred_on.clone()).or_insert((0, None));
            entry.0 += net;
            if !on_open {
                entry.1 = row.running_minor;
            }
        }
        by_day
            .into_iter()
            .map(|(occurred_on, (net_minor, running_minor))| CashRegisterSeriesPoint {
                occurred_on,
                net_minor,
                running_minor,
            })
            .collect()
    };

    Ok(CashRegisterBody {
        account: book.into(),
        period: period.to_string(),
        period_start: start,
        period_end: end,
        as_of_date: as_of.to_string(),
        start_known: start_minor.is_some(),
        start_minor,
        rows,
        series,
        scale: 2,
    })
}

fn series_want(
    cadence: &str,
    weekday_or_month_day: &str,
    start_on: &str,
    stop_on: &str,
    as_of: &str,
) -> Vec<String> {
    let Some(as_of_d) = parse_iso_date(as_of) else {
        return Vec::new();
    };
    let mut start = as_of_d;
    if let Some(bound) = parse_iso_date(start_on) {
        if bound > start {
            start = bound;
        }
    }
    let mut end = as_of_d + chrono::Duration::days(366);
    if let Some(bound) = parse_iso_date(stop_on) {
        if bound < end {
            end = bound;
        }
    }
    if start > end {
        return Vec::new();
    }
    element_horizon_dates(cadence, weekday_or_month_day, start, end)
}

/// Catalog list is read-only. Horizon fill / prune stay on Confirm, Save, and Register.
pub async fn cash_element_list_get(
    canonical: &dyn Canonical,
    account: &str,
    as_of: &str,
) -> Result<CashElementListBody, PlatformError> {
    let all = account.eq_ignore_ascii_case("all");
    let book = if all {
        None
    } else {
        Some(register_book_label(account).ok_or_else(|| {
            PlatformError::new("cash_account_kind", format!("{account} is not a Register book"))
        })?)
    };
    let occs = canonical.planned_occurrence_list().await?;
    let items = canonical
        .cash_element_list()
        .await?
        .into_iter()
        .filter(|e| match book {
            None => true,
            Some(label) => register_book_label(&e.account) == Some(label),
        })
        .map(|e| {
            let want = series_want(
                &e.cadence,
                &e.weekday_or_month_day,
                &e.start_on,
                &e.stop_on,
                as_of,
            );
            let upcoming: Vec<CashElementOccurrenceInput> = {
                let mut rows: Vec<CashElementOccurrenceInput> = occs
                    .iter()
                    .filter(|o| {
                        o.element_id == e.element_id
                            && o.confirmed_at.is_none()
                            && o.occurred_on.as_str() >= as_of
                    })
                    .map(|o| CashElementOccurrenceInput {
                        occurred_on: o.occurred_on.clone(),
                        amount_minor: o.amount_minor,
                        occurrence_id: Some(o.occurrence_id),
                        is_exception: o.is_exception,
                        is_cancelled: o.is_cancelled,
                    })
                    .collect();
                rows.sort_by(|a, b| a.occurred_on.cmp(&b.occurred_on));
                rows
            };
            let exceptions: Vec<CashElementOccurrenceInput> = upcoming
                .iter()
                .filter(|o| o.is_exception || o.is_cancelled)
                .cloned()
                .collect();
            let next_hit = upcoming.iter().find(|o| {
                !o.is_cancelled
                    && (o.is_exception
                        || want.is_empty()
                        || want.iter().any(|d| d == &o.occurred_on))
            });
            let projected = want.iter().find(|d| d.as_str() >= as_of);
            let (next_on, next_amt) = if let Some(row) = next_hit {
                (Some(row.occurred_on.clone()), Some(row.amount_minor))
            } else if let Some(on) = projected {
                (Some(on.clone()), Some(e.amount_minor))
            } else {
                (None, None)
            };
            CashElementListItem {
                element_id: e.element_id,
                account: e.account,
                kind: e.kind,
                cadence: e.cadence,
                amount_minor: e.amount_minor,
                note: e.note,
                weekday_or_month_day: e.weekday_or_month_day,
                start_on: e.start_on,
                stop_on: e.stop_on,
                next_occurred_on: next_on,
                next_amount_minor: next_amt,
                exception_count: exceptions.len() as u32,
                exceptions,
                upcoming,
            }
        })
        .collect();
    Ok(CashElementListBody {
        account: if all {
            "all".into()
        } else {
            book.unwrap().into()
        },
        items,
    })
}

fn element_history_side(kind: &str) -> &'static str {
    if kind.eq_ignore_ascii_case("Deposit") {
        "Credit"
    } else {
        "Debit"
    }
}

fn element_history_activity_type(account: &str, kind: &str, posted: bool, posted_type: &str) -> String {
    if !posted_type.is_empty() {
        return posted_type.to_string();
    }
    if !posted {
        return String::new();
    }
    if kind.eq_ignore_ascii_case("Deposit") && account.eq_ignore_ascii_case("SSA_2026") {
        return "SSA".into();
    }
    if account.eq_ignore_ascii_case("Income") {
        return "IRA_Distribution".into();
    }
    if account.eq_ignore_ascii_case("Health") {
        return "HSA_Withdrawal".into();
    }
    if account.eq_ignore_ascii_case("Car") {
        return "Withdrawal".into();
    }
    String::new()
}

pub async fn cash_element_history_get(
    canonical: &dyn Canonical,
    element_id: Uuid,
    duration: &str,
    as_of: &str,
) -> Result<CashElementHistoryBody, PlatformError> {
    let as_of_d = parse_iso_date(as_of)
        .ok_or_else(|| PlatformError::new("bad_date", format!("invalid asOfDate {as_of}")))?;
    let duration_key = match duration.trim().to_ascii_lowercase().as_str() {
        "" | "ytd" => "ytd",
        "all" => "all",
        "1m" | "1 month" | "1 months" => "1m",
        "3m" | "3 month" | "3 months" => "3m",
        "6m" | "6 month" | "6 months" => "6m",
        other => {
            return Err(PlatformError::new(
                "bad_date",
                format!("duration must be all, ytd, 6 months, 3 months, or 1 month, not {other}"),
            ))
        }
    };
    let (start, end) = element_history_period_bounds(duration_key, as_of_d).map_err(|_| {
        PlatformError::new("bad_date", format!("invalid duration {duration}"))
    })?;
    let period_start = start.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default();
    let period_end = end.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default();
    let element = canonical
        .cash_element_list()
        .await?
        .into_iter()
        .find(|e| e.element_id == element_id)
        .ok_or_else(|| PlatformError::new("not_found", "element not found"))?;
    let occs = canonical.planned_occurrence_list().await?;
    let act_start = if period_start.is_empty() {
        "0001-01-01"
    } else {
        period_start.as_str()
    };
    let act_end = if period_end.is_empty() {
        "9999-12-31"
    } else {
        period_end.as_str()
    };
    let posted = canonical
        .activity_list_in_range(None, act_start, act_end)
        .await
        .unwrap_or_default();
    let mut rows: Vec<CashElementHistoryRow> = occs
        .into_iter()
        .filter(|o| o.element_id == element_id && !o.is_cancelled)
        .filter(|o| {
            (period_start.is_empty() || o.occurred_on.as_str() >= period_start.as_str())
                && (period_end.is_empty() || o.occurred_on.as_str() <= period_end.as_str())
        })
        .filter(|o| o.confirmed_at.is_some() || o.occurred_on.as_str() >= as_of)
        .map(|o| {
            let key = format!("week-ahead-{}", o.occurrence_id);
            let activity = posted.iter().find(|a| a.idempotency_key == key);
            let confirmed = o.confirmed_at.is_some() || activity.is_some();
            let posted_type = activity.map(|a| a.activity_type.as_str()).unwrap_or("");
            let posted_key = activity
                .map(|a| a.idempotency_key.clone())
                .unwrap_or_default();
            let note = if o.note.trim().is_empty() {
                element.note.clone()
            } else {
                o.note.clone()
            };
            CashElementHistoryRow {
                occurred_on: o.occurred_on,
                element_id: element.element_id,
                occurrence_id: o.occurrence_id,
                element_name: element.note.clone(),
                account: element.account.clone(),
                kind: element.kind.clone(),
                side: element_history_side(&element.kind).into(),
                amount_minor: o.amount_minor.abs(),
                status: if confirmed { "Actual".into() } else { "Planned".into() },
                cadence: element.cadence.clone(),
                activity_type: element_history_activity_type(
                    &element.account,
                    &element.kind,
                    confirmed,
                    posted_type,
                ),
                posted_key,
                is_exception: o.is_exception,
                note,
                scale: 2,
            }
        })
        .collect();
    if let Some(book) = register_book_label(&element.account) {
        let ledger = register_ledger_account(book);
        let ledger_id = canonical
            .account_list()
            .await
            .unwrap_or_default()
            .into_iter()
            .find(|a| a.name.eq_ignore_ascii_case(ledger))
            .map(|a| a.account_id);
        if let Some(ledger_id) = ledger_id {
            let seen: std::collections::HashSet<String> = rows
                .iter()
                .map(|r| r.posted_key.clone())
                .filter(|k| !k.is_empty())
                .collect();
            for act in &posted {
                if act.account_id != ledger_id {
                    continue;
                }
                if seen.contains(&act.idempotency_key) {
                    continue;
                }
                let Some(amount) = element_history_posted_slice(
                    &element.account,
                    &element.note,
                    &act.activity_type,
                    act.amount_minor,
                    act.federal_withholding_minor,
                    act.state_withholding_minor,
                    &act.idempotency_key,
                ) else {
                    continue;
                };
                if !period_start.is_empty() && act.occurred_on.as_str() < period_start.as_str() {
                    continue;
                }
                if !period_end.is_empty() && act.occurred_on.as_str() > period_end.as_str() {
                    continue;
                }
                rows.push(CashElementHistoryRow {
                    occurred_on: act.occurred_on.clone(),
                    element_id: element.element_id,
                    occurrence_id: act.activity_id,
                    element_name: element.note.clone(),
                    account: element.account.clone(),
                    kind: element.kind.clone(),
                    side: element_history_side(&element.kind).into(),
                    amount_minor: amount,
                    status: "Actual".into(),
                    cadence: element.cadence.clone(),
                    activity_type: act.activity_type.clone(),
                    posted_key: act.idempotency_key.clone(),
                    is_exception: false,
                    note: element.note.clone(),
                    scale: act.scale,
                });
            }
        }
    }
    rows.sort_by(|a, b| {
        b.occurred_on
            .cmp(&a.occurred_on)
            .then(a.occurrence_id.cmp(&b.occurrence_id))
    });
    let mut totals = CashElementHistoryTotals {
        row_count: rows.len() as u32,
        ..CashElementHistoryTotals::default()
    };
    for row in &rows {
        let is_debit = row.side.eq_ignore_ascii_case("Debit");
        if row.status.eq_ignore_ascii_case("Actual") {
            totals.actual_count += 1;
            if is_debit {
                totals.actual_debit_minor += row.amount_minor;
            } else {
                totals.actual_credit_minor += row.amount_minor;
            }
        } else {
            totals.planned_count += 1;
            if is_debit {
                totals.planned_debit_minor += row.amount_minor;
            } else {
                totals.planned_credit_minor += row.amount_minor;
            }
        }
    }
    totals.actual_net_minor = totals.actual_credit_minor - totals.actual_debit_minor;
    Ok(CashElementHistoryBody {
        element_id: element.element_id,
        element_name: element.note,
        account: element.account,
        kind: element.kind,
        cadence: element.cadence,
        amount_minor: element.amount_minor,
        start_on: element.start_on.clone(),
        stop_on: element.stop_on.clone(),
        retired: element_is_retired(&element.stop_on, as_of_d),
        duration: duration_key.into(),
        period_start,
        period_end,
        as_of_date: as_of.to_string(),
        rows,
        totals,
        scale: 2,
    })
}

fn normalize_kind(raw: &str) -> Result<String, PlatformError> {
    match raw.trim() {
        "Deposit" => Ok("Deposit".into()),
        "Withdrawal" => Ok("Withdrawal".into()),
        other => Err(PlatformError::new(
            "cash_account_kind",
            format!("type must be Deposit or Withdrawal, not {other}"),
        )),
    }
}

fn normalize_cadence(raw: &str) -> Result<String, PlatformError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "weekly" => Ok("weekly".into()),
        "monthly" => Ok("monthly".into()),
        "annual" => Ok("annual".into()),
        "one-time" | "onetime" | "one_time" => Ok("one-time".into()),
        other => Err(PlatformError::new(
            "bad_date",
            format!("frequency must be Weekly, Monthly, Annual, or One-time, not {other}"),
        )),
    }
}

fn series_window(
    as_of: &str,
    start_on: &str,
    stop_on: &str,
) -> Result<(chrono::NaiveDate, chrono::NaiveDate), PlatformError> {
    let as_of_d = parse_iso_date(as_of)
        .ok_or_else(|| PlatformError::new("bad_date", format!("invalid asOfDate {as_of}")))?;
    let mut start = as_of_d;
    if let Some(bound) = parse_iso_date(start_on) {
        if bound > start {
            start = bound;
        }
    }
    let mut end = as_of_d + chrono::Duration::days(366);
    if let Some(bound) = parse_iso_date(stop_on) {
        end = bound;
    }
    Ok((start, end))
}

pub async fn cash_element_save(
    canonical: &dyn Canonical,
    account: &str,
    name: &str,
    kind: &str,
    cadence: &str,
    weekday_or_month_day: &str,
    amount_minor: i64,
    element_id: Option<Uuid>,
    occurrences: &[(Option<Uuid>, String, i64)],
    as_of: &str,
    start_on: &str,
    stop_on: &str,
) -> Result<CashElementSaveBody, PlatformError> {
    crate::week_ahead::invalidate_horizon_cache();
    let book = register_book_label(account).ok_or_else(|| {
        PlatformError::new("cash_account_kind", format!("{account} is not a Register book"))
    })?;
    if amount_minor <= 0 {
        return Err(PlatformError::new("unknown_amount", "amount must be known"));
    }
    let kind = normalize_kind(kind)?;
    let cadence = normalize_cadence(cadence)?;
    let id = element_id.unwrap_or_else(Uuid::new_v4);
    let existing = canonical.cash_element_list().await?;
    if let Some(found) = existing.iter().find(|e| e.element_id == id) {
        if register_book_label(&found.account) != Some(book) {
            return Err(PlatformError::new(
                "cash_account_kind",
                "element stays on its book",
            ));
        }
    }
    if let (Some(start), Some(stop)) = (parse_iso_date(start_on), parse_iso_date(stop_on)) {
        if start > stop {
            return Err(PlatformError::new(
                "bad_date",
                "start date is after stop date",
            ));
        }
    }
    let record = CashElementRecord {
        element_id: id,
        account: book.into(),
        kind: kind.clone(),
        cadence: cadence.clone(),
        amount_minor,
        note: name.trim().to_string(),
        weekday_or_month_day: weekday_or_month_day.to_string(),
        start_on: start_on.trim().to_string(),
        stop_on: stop_on.trim().to_string(),
    };
    canonical.cash_element_upsert(record.clone()).await?;

    let (win_start, win_end) = series_window(as_of, start_on, stop_on)?;
    let want: std::collections::HashSet<String> =
        if cadence == "one-time" {
            occurrences
                .iter()
                .filter(|(_, on, _)| on.as_str() > as_of)
                .map(|(_, on, _)| on.clone())
                .collect()
        } else if win_start > win_end {
            std::collections::HashSet::new()
        } else {
            financial_domain::cash_management::element_horizon_dates(
                &cadence,
                weekday_or_month_day,
                win_start,
                win_end,
            )
            .into_iter()
            .collect()
        };

    let stored = canonical.planned_occurrence_list().await?;
    let mut saved = Vec::new();
    let extras: std::collections::HashSet<String> = occurrences
        .iter()
        .filter(|(occ_id, on, _)| occ_id.is_none() && on.as_str() > as_of)
        .map(|(_, on, _)| on.clone())
        .collect();

    for (occ_id, on, amt) in occurrences {
        if parse_iso_date(on).is_none() {
            return Err(PlatformError::new("bad_date", format!("invalid occurredOn {on}")));
        }
        if *amt <= 0 {
            return Err(PlatformError::new("unknown_amount", "amount must be known"));
        }
        if let Some(oid) = occ_id {
            let mut row = canonical.planned_occurrence_get(*oid).await?;
            if row.confirmed_at.is_some() || row.occurred_on.as_str() < as_of {
                saved.push(row);
                continue;
            }
            if !want.contains(on) {
                continue;
            }
            row.occurred_on = on.clone();
            row.amount_minor = amount_minor;
            row.kind = kind.clone();
            row.note = record.note.clone();
            row.account = book.into();
            saved.push(canonical.planned_occurrence_upsert(row).await?);
        } else if on.as_str() > as_of {
            saved.push(
                canonical
                    .planned_occurrence_upsert(PlannedOccurrenceRecord {
                        occurrence_id: Uuid::new_v4(),
                        element_id: id,
                        account: book.into(),
                        kind: kind.clone(),
                        occurred_on: on.clone(),
                        amount_minor: *amt,
                        confirmed_at: None,
                        note: record.note.clone(),
                        is_exception: false,
                        is_cancelled: false,
                    })
                    .await?,
            );
        }
    }

    for row in stored {
        if row.element_id != id {
            continue;
        }
        if row.confirmed_at.is_some() || row.occurred_on.as_str() < as_of {
            if !saved.iter().any(|s| s.occurrence_id == row.occurrence_id) {
                saved.push(row);
            }
            continue;
        }
        if saved.iter().any(|s| s.occurrence_id == row.occurrence_id) {
            continue;
        }
        if extras.contains(&row.occurred_on) {
            continue;
        }
        if row.is_exception {
            saved.push(row);
            continue;
        }
        if want.contains(&row.occurred_on) {
            let mut next = row;
            next.amount_minor = amount_minor;
            next.kind = kind.clone();
            next.note = record.note.clone();
            saved.push(canonical.planned_occurrence_upsert(next).await?);
        } else {
            canonical.planned_occurrence_delete(row.occurrence_id).await?;
        }
    }

    let have: std::collections::HashSet<String> = {
        let now = canonical.planned_occurrence_list().await?;
        now.into_iter()
            .filter(|o| o.element_id == id)
            .map(|o| o.occurred_on)
            .collect()
    };
    for on in &want {
        if have.contains(on) {
            continue;
        }
        saved.push(
            canonical
                .planned_occurrence_upsert(PlannedOccurrenceRecord {
                    occurrence_id: Uuid::new_v4(),
                    element_id: id,
                    account: book.into(),
                    kind: kind.clone(),
                    occurred_on: on.clone(),
                    amount_minor,
                    confirmed_at: None,
                    note: record.note.clone(),
                    is_exception: false,
                    is_cancelled: false,
                })
                .await?,
        );
    }

    Ok(CashElementSaveBody {
        element: record,
        occurrences: saved,
    })
}

pub async fn cash_element_delete(
    canonical: &dyn Canonical,
    element_id: Uuid,
) -> Result<(), PlatformError> {
    crate::week_ahead::invalidate_horizon_cache();
    let kids = canonical.planned_occurrence_list().await?;
    if kids
        .iter()
        .any(|o| o.element_id == element_id && o.confirmed_at.is_some())
    {
        return Err(PlatformError::new(
            "occurrence_confirmed",
            "posted history stays frozen",
        ));
    }
    canonical.cash_element_delete(element_id).await
}

pub async fn planned_occurrence_delete(
    canonical: &dyn Canonical,
    occurrence_id: Uuid,
) -> Result<(), PlatformError> {
    crate::week_ahead::invalidate_horizon_cache();
    let row = canonical.planned_occurrence_get(occurrence_id).await?;
    if row.confirmed_at.is_some() {
        return Err(PlatformError::new(
            "occurrence_confirmed",
            "posted history stays frozen",
        ));
    }
    canonical.planned_occurrence_delete(occurrence_id).await
}

pub async fn planned_occurrence_save(
    canonical: &dyn Canonical,
    element_id: Uuid,
    occurrences: &[(Option<Uuid>, String, i64)],
    as_of: &str,
) -> Result<PlannedOccurrenceSaveBody, PlatformError> {
    crate::week_ahead::invalidate_horizon_cache();
    let element = canonical
        .cash_element_list()
        .await?
        .into_iter()
        .find(|e| e.element_id == element_id)
        .ok_or_else(|| PlatformError::new("not_found", "element missing"))?;
    let stored = canonical.planned_occurrence_list().await?;
    let mut saved = Vec::new();
    let mut keep = std::collections::HashSet::new();

    for (occ_id, on, amt) in occurrences {
        if parse_iso_date(on).is_none() {
            return Err(PlatformError::new(
                "bad_date",
                format!("invalid occurredOn {on}"),
            ));
        }
        if *amt <= 0 {
            return Err(PlatformError::new("unknown_amount", "amount must be known"));
        }
        let by_date = stored.iter().find(|o| {
            o.element_id == element_id && o.occurred_on == *on
        });
        let mut row = if let Some(oid) = occ_id {
            match stored.iter().find(|o| o.occurrence_id == *oid) {
                Some(found) if found.element_id != element_id => {
                    return Err(PlatformError::new(
                        "cash_account_kind",
                        "occurrence stays on its element",
                    ));
                }
                Some(found) if found.confirmed_at.is_some() => {
                    saved.push(found.clone());
                    keep.insert(found.occurrence_id);
                    continue;
                }
                Some(found) => found.clone(),
                None => PlannedOccurrenceRecord {
                    occurrence_id: *oid,
                    element_id,
                    account: element.account.clone(),
                    kind: element.kind.clone(),
                    occurred_on: on.clone(),
                    amount_minor: *amt,
                    confirmed_at: None,
                    note: element.note.clone(),
                    is_exception: true,
                    is_cancelled: false,
                },
            }
        } else if let Some(found) = by_date {
            if found.confirmed_at.is_some() {
                saved.push(found.clone());
                keep.insert(found.occurrence_id);
                continue;
            }
            found.clone()
        } else {
            PlannedOccurrenceRecord {
                occurrence_id: Uuid::new_v4(),
                element_id,
                account: element.account.clone(),
                kind: element.kind.clone(),
                occurred_on: on.clone(),
                amount_minor: *amt,
                confirmed_at: None,
                note: element.note.clone(),
                is_exception: true,
                is_cancelled: false,
            }
        };
        if let Some(other) = by_date {
            if other.occurrence_id != row.occurrence_id {
                if other.confirmed_at.is_some() {
                    return Err(PlatformError::new(
                        "occurrence_confirmed",
                        "posted history stays frozen",
                    ));
                }
                if occ_id.is_some() {
                    canonical
                        .planned_occurrence_delete(row.occurrence_id)
                        .await?;
                }
                row = other.clone();
            }
        }
        row.occurred_on = on.clone();
        row.amount_minor = *amt;
        row.is_exception = true;
        row.kind = element.kind.clone();
        row.note = element.note.clone();
        row.account = element.account.clone();
        row.element_id = element_id;
        let out = canonical.planned_occurrence_upsert(row).await?;
        keep.insert(out.occurrence_id);
        saved.push(out);
    }

    for row in stored {
        if row.element_id != element_id {
            continue;
        }
        if !row.is_exception {
            continue;
        }
        if row.confirmed_at.is_some() || row.occurred_on.as_str() <= as_of {
            continue;
        }
        if keep.contains(&row.occurrence_id) {
            continue;
        }
        canonical
            .planned_occurrence_delete(row.occurrence_id)
            .await?;
    }

    Ok(PlannedOccurrenceSaveBody {
        element_id,
        occurrences: saved,
    })
}

pub async fn planned_occurrence_edit(
    canonical: &dyn Canonical,
    element_id: Uuid,
    source_id: Uuid,
    as_of: &str,
    cancel: bool,
    new_on: Option<&str>,
    new_amt: Option<i64>,
) -> Result<PlannedOccurrenceSaveBody, PlatformError> {
    crate::week_ahead::invalidate_horizon_cache();
    let element = canonical
        .cash_element_list()
        .await?
        .into_iter()
        .find(|e| e.element_id == element_id)
        .ok_or_else(|| PlatformError::new("not_found", "element missing"))?;
    let mut source = canonical.planned_occurrence_get(source_id).await?;
    if source.element_id != element_id {
        return Err(PlatformError::new(
            "cash_account_kind",
            "occurrence stays on its element",
        ));
    }
    if source.confirmed_at.is_some() {
        return Err(PlatformError::new(
            "occurrence_confirmed",
            "posted history stays frozen",
        ));
    }
    if cancel {
        source.is_cancelled = true;
        source.is_exception = true;
        let out = canonical.planned_occurrence_upsert(source).await?;
        return Ok(PlannedOccurrenceSaveBody {
            element_id,
            occurrences: vec![out],
        });
    }
    let on = new_on
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| PlatformError::new("bad_date", "occurredOn is required"))?
        .to_string();
    if parse_iso_date(&on).is_none() {
        return Err(PlatformError::new(
            "bad_date",
            format!("invalid occurredOn {on}"),
        ));
    }
    let amt = new_amt.unwrap_or(0);
    if amt <= 0 {
        return Err(PlatformError::new("unknown_amount", "amount must be known"));
    }
    let (win_start, win_end) = series_window(as_of, &element.start_on, &element.stop_on)?;
    let want: std::collections::HashSet<String> =
        if element.cadence == "one-time" || win_start > win_end {
            std::collections::HashSet::new()
        } else {
            financial_domain::cash_management::element_horizon_dates(
                &element.cadence,
                &element.weekday_or_month_day,
                win_start,
                win_end,
            )
            .into_iter()
            .collect()
        };
    let mut saved = Vec::new();
    if on == source.occurred_on {
        source.amount_minor = amt;
        source.is_cancelled = false;
        source.is_exception = amt != element.amount_minor || source.is_exception;
        saved.push(canonical.planned_occurrence_upsert(source).await?);
    } else if want.contains(&source.occurred_on) {
        source.is_cancelled = true;
        source.is_exception = true;
        saved.push(canonical.planned_occurrence_upsert(source).await?);
        let stored = canonical.planned_occurrence_list().await?;
        if let Some(existing) = stored.iter().find(|o| {
            o.element_id == element_id && o.occurred_on == on && o.occurrence_id != source_id
        }) {
            if existing.confirmed_at.is_some() {
                return Err(PlatformError::new(
                    "occurrence_confirmed",
                    "posted history stays frozen",
                ));
            }
            let mut next = existing.clone();
            next.amount_minor = amt;
            next.is_exception = true;
            next.is_cancelled = false;
            saved.push(canonical.planned_occurrence_upsert(next).await?);
        } else {
            saved.push(
                canonical
                    .planned_occurrence_upsert(PlannedOccurrenceRecord {
                        occurrence_id: Uuid::new_v4(),
                        element_id,
                        account: element.account.clone(),
                        kind: element.kind.clone(),
                        occurred_on: on,
                        amount_minor: amt,
                        confirmed_at: None,
                        note: element.note.clone(),
                        is_exception: true,
                        is_cancelled: false,
                    })
                    .await?,
            );
        }
    } else {
        source.occurred_on = on;
        source.amount_minor = amt;
        source.is_exception = true;
        source.is_cancelled = false;
        saved.push(canonical.planned_occurrence_upsert(source).await?);
    }
    Ok(PlannedOccurrenceSaveBody {
        element_id,
        occurrences: saved,
    })
}
