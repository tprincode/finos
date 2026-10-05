//! Household task_rule / task. Separate from collector work_ticket.

use application_core::contracts::{TaskRecord, TaskRuleRecord};
use application_core::ports::platform::PlatformError;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

fn map_sql(err: sqlx::Error) -> PlatformError {
    PlatformError::new("storage_error", err.to_string())
}

fn row_rule(row: &sqlx::sqlite::SqliteRow) -> Result<TaskRuleRecord, PlatformError> {
    Ok(TaskRuleRecord {
        rule_id: row.try_get("rule_id").map_err(map_sql)?,
        code: row.try_get("code").map_err(map_sql)?,
        title: row.try_get("title").map_err(map_sql)?,
        enabled: row.try_get::<i64, _>("enabled").map_err(map_sql)? != 0,
        cadence: row.try_get("cadence").map_err(map_sql)?,
        domain: row.try_get("domain").map_err(map_sql)?,
        owner_note: row.try_get("owner_note").map_err(map_sql)?,
    })
}

fn row_task(row: &sqlx::sqlite::SqliteRow) -> Result<TaskRecord, PlatformError> {
    let task_id: String = row.try_get("task_id").map_err(map_sql)?;
    let rule_id: Option<String> = row.try_get("rule_id").map_err(map_sql)?;
    Ok(TaskRecord {
        task_id: Uuid::parse_str(&task_id)
            .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
        rule_id: rule_id.filter(|s| !s.is_empty()),
        code: row.try_get("code").map_err(map_sql)?,
        title: row.try_get("title").map_err(map_sql)?,
        status: row.try_get("status").map_err(map_sql)?,
        domain: row.try_get("domain").map_err(map_sql)?,
        week_start: row.try_get("week_start").map_err(map_sql)?,
        due_on: row.try_get("due_on").map_err(map_sql)?,
        ignore_until: row.try_get("ignore_until").map_err(map_sql)?,
        payload_json: row.try_get("payload_json").map_err(map_sql)?,
        created_on: row.try_get("created_on").map_err(map_sql)?,
        resolved_on: row.try_get("resolved_on").map_err(map_sql)?,
    })
}

pub async fn task_rule_list(pool: &SqlitePool) -> Result<Vec<TaskRuleRecord>, PlatformError> {
    let rows = sqlx::query(
        "SELECT rule_id, code, title, enabled, cadence, domain, owner_note
         FROM task_rule ORDER BY code",
    )
    .fetch_all(pool)
    .await
    .map_err(map_sql)?;
    rows.iter().map(row_rule).collect()
}

pub async fn task_rule_set(
    pool: &SqlitePool,
    code: &str,
    enabled: bool,
) -> Result<TaskRuleRecord, PlatformError> {
    let updated = sqlx::query("UPDATE task_rule SET enabled = ? WHERE code = ?")
        .bind(if enabled { 1_i64 } else { 0 })
        .bind(code)
        .execute(pool)
        .await
        .map_err(map_sql)?;
    if updated.rows_affected() == 0 {
        return Err(PlatformError::new("missing_rule", format!("unknown rule {code}")));
    }
    let row = sqlx::query(
        "SELECT rule_id, code, title, enabled, cadence, domain, owner_note
         FROM task_rule WHERE code = ?",
    )
    .bind(code)
    .fetch_one(pool)
    .await
    .map_err(map_sql)?;
    row_rule(&row)
}

pub async fn task_list(
    pool: &SqlitePool,
    week_start: Option<String>,
    status: Option<String>,
) -> Result<Vec<TaskRecord>, PlatformError> {
    let week = week_start.unwrap_or_default();
    let st = status.unwrap_or_default();
    let rows = sqlx::query(
        "SELECT task_id, rule_id, code, title, status, domain, week_start, due_on,
                ignore_until, payload_json, created_on, resolved_on
         FROM task
         WHERE (? = '' OR week_start = ?)
           AND (? = '' OR status = ?)
         ORDER BY due_on, title",
    )
    .bind(&week)
    .bind(&week)
    .bind(&st)
    .bind(&st)
    .fetch_all(pool)
    .await
    .map_err(map_sql)?;
    rows.iter().map(row_task).collect()
}

pub async fn task_get(pool: &SqlitePool, task_id: Uuid) -> Result<TaskRecord, PlatformError> {
    let row = sqlx::query(
        "SELECT task_id, rule_id, code, title, status, domain, week_start, due_on,
                ignore_until, payload_json, created_on, resolved_on
         FROM task WHERE task_id = ?",
    )
    .bind(task_id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(map_sql)?
    .ok_or_else(|| PlatformError::new("missing_task", "task not found"))?;
    row_task(&row)
}

pub async fn task_by_code_week(
    pool: &SqlitePool,
    code: &str,
    week_start: &str,
) -> Result<Option<TaskRecord>, PlatformError> {
    let row = sqlx::query(
        "SELECT task_id, rule_id, code, title, status, domain, week_start, due_on,
                ignore_until, payload_json, created_on, resolved_on
         FROM task WHERE code = ? AND week_start = ?
         ORDER BY created_on DESC",
    )
    .bind(code)
    .bind(week_start)
    .fetch_optional(pool)
    .await
    .map_err(map_sql)?;
    row.as_ref().map(row_task).transpose()
}

pub async fn task_insert(pool: &SqlitePool, record: TaskRecord) -> Result<TaskRecord, PlatformError> {
    sqlx::query(
        "INSERT INTO task (
            task_id, rule_id, code, title, status, domain, week_start, due_on,
            ignore_until, payload_json, created_on, resolved_on
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(record.task_id.to_string())
    .bind(record.rule_id.as_deref())
    .bind(&record.code)
    .bind(&record.title)
    .bind(&record.status)
    .bind(&record.domain)
    .bind(&record.week_start)
    .bind(&record.due_on)
    .bind(&record.ignore_until)
    .bind(&record.payload_json)
    .bind(&record.created_on)
    .bind(&record.resolved_on)
    .execute(pool)
    .await
    .map_err(map_sql)?;
    Ok(record)
}

pub async fn task_update(pool: &SqlitePool, record: TaskRecord) -> Result<TaskRecord, PlatformError> {
    let updated = sqlx::query(
        "UPDATE task SET
            rule_id = ?, code = ?, title = ?, status = ?, domain = ?,
            week_start = ?, due_on = ?, ignore_until = ?, payload_json = ?,
            created_on = ?, resolved_on = ?
         WHERE task_id = ?",
    )
    .bind(record.rule_id.as_deref())
    .bind(&record.code)
    .bind(&record.title)
    .bind(&record.status)
    .bind(&record.domain)
    .bind(&record.week_start)
    .bind(&record.due_on)
    .bind(&record.ignore_until)
    .bind(&record.payload_json)
    .bind(&record.created_on)
    .bind(&record.resolved_on)
    .bind(record.task_id.to_string())
    .execute(pool)
    .await
    .map_err(map_sql)?;
    if updated.rows_affected() == 0 {
        return Err(PlatformError::new("missing_task", "task not found"));
    }
    Ok(record)
}
