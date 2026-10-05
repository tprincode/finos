//! Household Task Manager. MAGI / budget / options / manual — not collector work_ticket.

use crate::contracts::{
    IssuerDeclarationRecord, MagiCliffTaskSyncBody, PlanHistoryRecord, PlanSaturdayTaskSyncBody,
    TaskListBody, TaskRecord, TaskRuleListBody, TaskRuleRecord,
};
use crate::ports::canonical::Canonical;
use crate::ports::platform::PlatformError;
use chrono::Duration;
use financial_domain::trends::parse_iso_date;
use serde_json::{json, Value};
use std::collections::HashSet;
use uuid::Uuid;

pub const CODE_MAGI_CLIFF_OVER: &str = "magi_cliff_over";
pub const STATUS_OPEN: &str = "open";
pub const STATUS_DONE: &str = "done";
pub const STATUS_IGNORED: &str = "ignored_until";
pub const DOMAIN_MANUAL: &str = "manual";
pub const DOMAIN_MAGI: &str = "magi";
pub const DOMAIN_PLAN: &str = "plan";
pub const PLAN_UNDER_MIN: usize = 3;
pub const PLAN_OVER_MIN: usize = 5;
pub const PLAN_UNDER_TITLE: &str = "Plan amount is wrong and must be adjusted.";
pub const PLAN_OVER_TITLE: &str = "Validate Plan amount — paid above Plan for 5 periods.";

pub fn week_start_iso(as_of: &str) -> Result<String, PlatformError> {
    let day = parse_iso_date(as_of)
        .ok_or_else(|| PlatformError::new("bad_date", format!("invalid date {as_of}")))?;
    let week = financial_domain::trends::trends_period_for_capture(day);
    Ok(week.start.format("%Y-%m-%d").to_string())
}

pub fn week_end_iso(as_of: &str) -> Result<String, PlatformError> {
    let day = parse_iso_date(as_of)
        .ok_or_else(|| PlatformError::new("bad_date", format!("invalid date {as_of}")))?;
    let week = financial_domain::trends::trends_period_for_capture(day);
    Ok(week.end.format("%Y-%m-%d").to_string())
}

pub fn next_saturday_iso(as_of: &str) -> Result<String, PlatformError> {
    let day = parse_iso_date(as_of)
        .ok_or_else(|| PlatformError::new("bad_date", format!("invalid date {as_of}")))?;
    let week = financial_domain::trends::trends_period_for_capture(day);
    Ok((week.start + Duration::days(7))
        .format("%Y-%m-%d")
        .to_string())
}

pub fn magi_week_title(overage_minor: i64) -> String {
    format!(
        "MAGI over cliff by {} — resolve or snooze till next plan week.",
        usd(overage_minor)
    )
}

fn usd(minor: i64) -> String {
    let sign = if minor < 0 { "-" } else { "" };
    let abs = minor.unsigned_abs();
    let dollars = abs / 100;
    let cents = abs % 100;
    let mut grouped = String::new();
    let raw = dollars.to_string();
    for (i, ch) in raw.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let dollars: String = grouped.chars().rev().collect();
    format!("{sign}${dollars}.{cents:02}")
}

pub async fn task_rule_list(canonical: &dyn Canonical) -> Result<TaskRuleListBody, PlatformError> {
    Ok(TaskRuleListBody {
        items: canonical.task_rule_list().await?,
    })
}

pub async fn task_rule_set(
    canonical: &dyn Canonical,
    code: &str,
    enabled: bool,
) -> Result<TaskRuleRecord, PlatformError> {
    if code.trim().is_empty() {
        return Err(PlatformError::new("missing_code", "rule code is required"));
    }
    canonical
        .task_rule_set(code.trim().to_string(), enabled)
        .await
}

pub async fn task_list(
    canonical: &dyn Canonical,
    week_start: Option<String>,
    status: Option<String>,
) -> Result<TaskListBody, PlatformError> {
    Ok(TaskListBody {
        items: canonical.task_list(week_start, status).await?,
    })
}

pub async fn task_add(
    canonical: &dyn Canonical,
    title: &str,
    due_on: &str,
    note: &str,
) -> Result<TaskRecord, PlatformError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(PlatformError::new("missing_title", "title is required"));
    }
    if parse_iso_date(due_on).is_none() {
        return Err(PlatformError::new("bad_date", "dueOn must be YYYY-MM-DD"));
    }
    let week_start = week_start_iso(due_on)?;
    let task_id = Uuid::new_v4();
    let record = TaskRecord {
        task_id,
        rule_id: None,
        code: format!("manual:{task_id}"),
        title: title.to_string(),
        status: STATUS_OPEN.into(),
        domain: DOMAIN_MANUAL.into(),
        week_start,
        due_on: due_on[..10].to_string(),
        ignore_until: String::new(),
        payload_json: json!({ "note": note }).to_string(),
        created_on: due_on[..10].to_string(),
        resolved_on: String::new(),
    };
    canonical.task_insert(record).await
}

pub async fn task_resolve(
    canonical: &dyn Canonical,
    task_id: Uuid,
    how: &str,
    ignore_until: Option<String>,
) -> Result<TaskRecord, PlatformError> {
    let mut row = canonical.task_get(task_id).await?;
    match how {
        "done" => {
            row.status = STATUS_DONE.into();
            row.resolved_on = if row.resolved_on.is_empty() {
                row.due_on.clone()
            } else {
                row.resolved_on.clone()
            };
            if row.resolved_on.is_empty() {
                row.resolved_on = row.week_start.clone();
            }
        }
        "ignored_until" => {
            let until = ignore_until.unwrap_or_default();
            if parse_iso_date(&until).is_none() {
                return Err(PlatformError::new(
                    "missing_ignore_until",
                    "ignoreUntil must be YYYY-MM-DD",
                ));
            }
            row.status = STATUS_IGNORED.into();
            row.ignore_until = until[..10].to_string();
        }
        _ => {
            return Err(PlatformError::new(
                "bad_how",
                "how must be done or ignored_until",
            ));
        }
    }
    canonical.task_update(row).await
}

pub async fn magi_cliff_task_sync(
    canonical: &dyn Canonical,
    as_of: &str,
    overage_minor: i64,
    credit_at_risk_minor: i64,
    suggestions: &[String],
) -> Result<MagiCliffTaskSyncBody, PlatformError> {
    let week_start = week_start_iso(as_of)?;
    let week_end = week_end_iso(as_of)?;
    let as_of = &as_of[..as_of.len().min(10)];
    let rules = canonical.task_rule_list().await?;
    let rule = rules.iter().find(|r| r.code == CODE_MAGI_CLIFF_OVER);
    let enabled = rule.map(|r| r.enabled).unwrap_or(false);
    let existing = canonical
        .task_by_code_week(CODE_MAGI_CLIFF_OVER.into(), week_start.clone())
        .await?;

    if overage_minor <= 0 || !enabled {
        if let Some(row) = existing.as_ref().filter(|r| r.status == STATUS_OPEN) {
            let mut done = row.clone();
            done.status = STATUS_DONE.into();
            done.resolved_on = as_of.to_string();
            let task = canonical.task_update(done).await?;
            return Ok(MagiCliffTaskSyncBody {
                action: "completed".into(),
                task: Some(task),
            });
        }
        return Ok(MagiCliffTaskSyncBody {
            action: "skipped".into(),
            task: existing,
        });
    }

    if let Some(row) = existing.clone() {
        if row.status == STATUS_IGNORED && !row.ignore_until.is_empty() && as_of < row.ignore_until.as_str()
        {
            return Ok(MagiCliffTaskSyncBody {
                action: "skipped".into(),
                task: Some(row),
            });
        }
        if row.status == STATUS_DONE {
            return Ok(MagiCliffTaskSyncBody {
                action: "skipped".into(),
                task: Some(row),
            });
        }
        if row.status == STATUS_OPEN {
            let mut open = row;
            open.payload_json = magi_payload(overage_minor, credit_at_risk_minor, suggestions);
            let task = canonical.task_update(open).await?;
            return Ok(MagiCliffTaskSyncBody {
                action: "refreshed".into(),
                task: Some(task),
            });
        }
    }

    let rule_id = rule.map(|r| r.rule_id.clone());
    let record = TaskRecord {
        task_id: Uuid::new_v4(),
        rule_id,
        code: CODE_MAGI_CLIFF_OVER.into(),
        title: "Resolve MAGI cliff gap".into(),
        status: STATUS_OPEN.into(),
        domain: DOMAIN_MAGI.into(),
        week_start,
        due_on: week_end,
        ignore_until: String::new(),
        payload_json: magi_payload(overage_minor, credit_at_risk_minor, suggestions),
        created_on: as_of.to_string(),
        resolved_on: String::new(),
    };
    let task = canonical.task_insert(record).await?;
    Ok(MagiCliffTaskSyncBody {
        action: "opened".into(),
        task: Some(task),
    })
}

fn magi_payload(overage_minor: i64, credit_at_risk_minor: i64, suggestions: &[String]) -> String {
    json!({
        "overageMinor": overage_minor,
        "creditAtRiskMinor": credit_at_risk_minor,
        "suggestions": suggestions,
    })
    .to_string()
}

pub fn plan_under_code(security_id: Uuid) -> String {
    format!("plan_under:{security_id}")
}

pub fn plan_over_code(security_id: Uuid) -> String {
    format!("plan_over:{security_id}")
}

/// Saturday check. One observation per stored pay, newest period first.
/// Under 3 opens one adjust task. Over 5 opens one validate task.
/// A missing amount is not $0 and breaks the run.
pub async fn plan_saturday_task_sync(
    canonical: &dyn Canonical,
    as_of: &str,
) -> Result<PlanSaturdayTaskSyncBody, PlatformError> {
    let week_start = week_start_iso(as_of)?;
    let week_end = week_end_iso(as_of)?;
    let as_of = &as_of[..as_of.len().min(10)];
    let tasks = canonical.task_list(None, None).await?;
    let plans = canonical.plan_history_list().await?;
    let securities = canonical.security_list().await?;
    let mut opened = 0i64;
    let mut refreshed = 0i64;
    let mut completed = 0i64;
    let mut touched = HashSet::new();

    for security in securities {
        let Some(plan) = current_plan(&plans, security.security_id, as_of) else {
            continue;
        };
        let decls = canonical
            .issuer_declaration_list(security.security_id)
            .await?;
        let run = consecutive_run(&decls, plan.amount_per_share_minor, plan.amount_scale);
        for side in [
            StreakSide {
                code: plan_under_code(security.security_id),
                title: PLAN_UNDER_TITLE,
                streak: run.under,
                threshold: PLAN_UNDER_MIN,
                anchor: run.under_anchor.as_str(),
                direction: "under",
            },
            StreakSide {
                code: plan_over_code(security.security_id),
                title: PLAN_OVER_TITLE,
                streak: run.over,
                threshold: PLAN_OVER_MIN,
                anchor: run.over_anchor.as_str(),
                direction: "over",
            },
        ] {
            touched.insert(side.code.clone());
            let action = sync_streak_task(
                canonical,
                &tasks,
                as_of,
                &week_start,
                &week_end,
                &security.symbol,
                security.security_id,
                &side,
            )
            .await?;
            match action {
                "opened" => opened += 1,
                "refreshed" => refreshed += 1,
                "completed" => completed += 1,
                _ => {}
            }
        }
    }

    for row in &tasks {
        if row.status != STATUS_OPEN {
            continue;
        }
        if !row.code.starts_with("plan_under:") && !row.code.starts_with("plan_over:") {
            continue;
        }
        if touched.contains(&row.code) {
            continue;
        }
        let mut done = row.clone();
        done.status = STATUS_DONE.into();
        done.resolved_on = as_of.to_string();
        canonical.task_update(done).await?;
        completed += 1;
    }

    Ok(PlanSaturdayTaskSyncBody {
        opened,
        refreshed,
        completed,
    })
}

struct StreakSide<'a> {
    code: String,
    title: &'a str,
    streak: usize,
    threshold: usize,
    anchor: &'a str,
    direction: &'a str,
}

struct PayRun {
    under: usize,
    over: usize,
    under_anchor: String,
    over_anchor: String,
}

fn current_plan<'a>(
    plans: &'a [PlanHistoryRecord],
    security_id: Uuid,
    as_of: &str,
) -> Option<&'a PlanHistoryRecord> {
    plans
        .iter()
        .filter(|p| p.security_id == security_id && p.effective_from.as_str() <= as_of)
        .filter(|p| p.effective_to.is_empty() || p.effective_to.as_str() >= as_of)
        .max_by(|a, b| a.effective_from.cmp(&b.effective_from))
}

fn consecutive_run(decls: &[IssuerDeclarationRecord], plan_minor: i64, plan_scale: u8) -> PayRun {
    let mut rows: Vec<&IssuerDeclarationRecord> = decls
        .iter()
        .filter(|d| !d.payment_period.is_empty())
        .collect();
    rows.sort_by(|a, b| {
        b.payment_period
            .cmp(&a.payment_period)
            .then(b.entered_at.cmp(&a.entered_at))
    });
    let mut seen = HashSet::new();
    let mut unique = Vec::new();
    for row in rows {
        if seen.insert(row.payment_period.clone()) {
            unique.push(row);
        }
    }

    let mut under = 0usize;
    let mut over = 0usize;
    let mut under_anchor = String::new();
    let mut over_anchor = String::new();
    for row in unique {
        let Some(paid) = row.amount_per_share_minor else {
            break;
        };
        let cmp = pay_vs_plan(paid, row.amount_scale, plan_minor, plan_scale);
        if cmp == 0 {
            break;
        }
        if cmp < 0 {
            if over > 0 {
                break;
            }
            if under == 0 {
                under_anchor = row.payment_period.clone();
            }
            under += 1;
        } else {
            if under > 0 {
                break;
            }
            if over == 0 {
                over_anchor = row.payment_period.clone();
            }
            over += 1;
        }
    }
    PayRun {
        under,
        over,
        under_anchor,
        over_anchor,
    }
}

fn pay_vs_plan(paid_minor: i64, paid_scale: u8, plan_minor: i64, plan_scale: u8) -> i8 {
    let to = paid_scale.max(plan_scale);
    let paid = financial_domain::money::rescale(paid_minor, paid_scale, to);
    let plan = financial_domain::money::rescale(plan_minor, plan_scale, to);
    if paid < plan {
        -1
    } else if paid > plan {
        1
    } else {
        0
    }
}

fn existing_for<'a>(tasks: &'a [TaskRecord], code: &str) -> Option<&'a TaskRecord> {
    let rows: Vec<&TaskRecord> = tasks.iter().filter(|t| t.code == code).collect();
    rows.iter()
        .copied()
        .find(|t| t.status == STATUS_OPEN)
        .or_else(|| {
            rows.into_iter()
                .max_by(|a, b| a.created_on.cmp(&b.created_on).then(a.task_id.cmp(&b.task_id)))
        })
}

fn payload_anchor(payload_json: &str) -> String {
    serde_json::from_str::<Value>(payload_json)
        .ok()
        .and_then(|v| {
            v.get("anchorPeriod")
                .and_then(|a| a.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default()
}

fn plan_payload(
    symbol: &str,
    security_id: Uuid,
    streak: usize,
    direction: &str,
    anchor: &str,
) -> String {
    json!({
        "symbol": symbol,
        "securityId": security_id,
        "streak": streak,
        "direction": direction,
        "anchorPeriod": anchor,
    })
    .to_string()
}

async fn sync_streak_task(
    canonical: &dyn Canonical,
    tasks: &[TaskRecord],
    as_of: &str,
    week_start: &str,
    week_end: &str,
    symbol: &str,
    security_id: Uuid,
    side: &StreakSide<'_>,
) -> Result<&'static str, PlatformError> {
    let existing = existing_for(tasks, &side.code);
    if side.streak < side.threshold {
        if let Some(row) = existing.filter(|r| r.status == STATUS_OPEN) {
            let mut done = row.clone();
            done.status = STATUS_DONE.into();
            done.resolved_on = as_of.to_string();
            canonical.task_update(done).await?;
            return Ok("completed");
        }
        return Ok("skipped");
    }

    if let Some(row) = existing {
        if row.status == STATUS_IGNORED
            && !row.ignore_until.is_empty()
            && as_of < row.ignore_until.as_str()
        {
            return Ok("skipped");
        }
        if row.status == STATUS_DONE {
            let old = payload_anchor(&row.payload_json);
            if side.anchor.is_empty() || (!old.is_empty() && old.as_str() >= side.anchor) {
                return Ok("skipped");
            }
        } else {
            let mut open = row.clone();
            open.status = STATUS_OPEN.into();
            open.title = side.title.into();
            open.week_start = week_start.into();
            open.due_on = week_end.into();
            open.payload_json = plan_payload(
                symbol,
                security_id,
                side.streak,
                side.direction,
                side.anchor,
            );
            open.resolved_on.clear();
            if row.status != STATUS_OPEN {
                open.ignore_until.clear();
            }
            canonical.task_update(open).await?;
            return Ok("refreshed");
        }
    }

    let record = TaskRecord {
        task_id: Uuid::new_v4(),
        rule_id: None,
        code: side.code.clone(),
        title: side.title.into(),
        status: STATUS_OPEN.into(),
        domain: DOMAIN_PLAN.into(),
        week_start: week_start.into(),
        due_on: week_end.into(),
        ignore_until: String::new(),
        payload_json: plan_payload(symbol, security_id, side.streak, side.direction, side.anchor),
        created_on: as_of.to_string(),
        resolved_on: String::new(),
    };
    canonical.task_insert(record).await?;
    Ok("opened")
}

pub const IRA_CONTRIBUTION_ACTIVITY: &str = "ira-contribution";

pub fn magi_ira_idempotency_key(task_id: Uuid) -> String {
    format!("magi-ira:{task_id}")
}

pub fn magi_contribution_account_allowed(name: &str) -> bool {
    let name = name.trim();
    name.eq_ignore_ascii_case("Income")
        || name.eq_ignore_ascii_case("Speculation")
        || name.eq_ignore_ascii_case("Account 9")
        || name == "9"
}

pub async fn magi_ira_contribution_post(
    canonical: &dyn Canonical,
    task_id: Uuid,
    account_name: &str,
    amount_minor: i64,
    occurred_on: &str,
) -> Result<TaskRecord, PlatformError> {
    if !magi_contribution_account_allowed(account_name) {
        return Err(PlatformError::new(
            "magi_account_not_offered",
            "Traditional IRA contribution is Income, Speculation, or Account 9",
        ));
    }
    if parse_iso_date(occurred_on).is_none() {
        return Err(PlatformError::new("bad_date", "occurredOn must be YYYY-MM-DD"));
    }
    let _existing = canonical.task_get(task_id).await?;
    let accounts = canonical.account_list().await?;
    let Some(account) = accounts
        .iter()
        .find(|row| row.name.eq_ignore_ascii_case(account_name.trim()))
    else {
        return Err(PlatformError::new(
            "missing_account",
            "account name is not registered",
        ));
    };
    let key = magi_ira_idempotency_key(task_id);
    crate::cash_pile::deposit_labeled_keyed(
        canonical,
        account.account_id,
        amount_minor,
        occurred_on[..10].to_string(),
        IRA_CONTRIBUTION_ACTIVITY,
        Some(&key),
    )
    .await?;
    task_resolve(canonical, task_id, "done", None).await
}
