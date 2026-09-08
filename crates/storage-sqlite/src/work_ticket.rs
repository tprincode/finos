//! Standing work tickets (one open row per security_id + code).

use application_core::contracts::WorkTicketRecord;
use application_core::ports::platform::PlatformError;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

fn map_storage(err: sqlx::Error) -> PlatformError {
    PlatformError::new("storage_error", err.to_string())
}

fn row_to_ticket(row: &sqlx::sqlite::SqliteRow) -> Result<WorkTicketRecord, PlatformError> {
    Ok(WorkTicketRecord {
        ticket_id: Uuid::parse_str(
            &row.try_get::<String, _>("ticket_id")
                .map_err(|e| map_storage(e))?,
        )
        .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
        security_id: Uuid::parse_str(
            &row.try_get::<String, _>("security_id")
                .map_err(|e| map_storage(e))?,
        )
        .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
        symbol: row.try_get("symbol").map_err(|e| map_storage(e))?,
        field: row.try_get("field").map_err(|e| map_storage(e))?,
        code: row.try_get("code").map_err(|e| map_storage(e))?,
        tool: row.try_get("tool").map_err(|e| map_storage(e))?,
        reason: row.try_get("reason").map_err(|e| map_storage(e))?,
        urls_tried: row.try_get("urls_tried").map_err(|e| map_storage(e))?,
        opened_on: row.try_get("opened_on").map_err(|e| map_storage(e))?,
        last_seen_on: row.try_get("last_seen_on").map_err(|e| map_storage(e))?,
        status: row.try_get("status").map_err(|e| map_storage(e))?,
        filed_on: row.try_get("filed_on").map_err(|e| map_storage(e))?,
        completed_how: row.try_get("completed_how").map_err(|e| map_storage(e))?,
        owner_note: row.try_get("owner_note").map_err(|e| map_storage(e))?,
        retrieve_run_id: row
            .try_get("retrieve_run_id")
            .map_err(|e| map_storage(e))?,
    })
}

pub async fn work_ticket_raise(
    pool: &SqlitePool,
    record: WorkTicketRecord,
) -> Result<WorkTicketRecord, PlatformError> {
    sqlx::query(
        "INSERT INTO work_ticket (
            ticket_id, security_id, symbol, field, code, tool, reason, urls_tried,
            opened_on, last_seen_on, status, filed_on, completed_how, owner_note, retrieve_run_id
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(record.ticket_id.to_string())
    .bind(record.security_id.to_string())
    .bind(&record.symbol)
    .bind(&record.field)
    .bind(&record.code)
    .bind(&record.tool)
    .bind(&record.reason)
    .bind(&record.urls_tried)
    .bind(&record.opened_on)
    .bind(&record.last_seen_on)
    .bind(&record.status)
    .bind(&record.filed_on)
    .bind(&record.completed_how)
    .bind(&record.owner_note)
    .bind(&record.retrieve_run_id)
    .execute(pool)
    .await
    .map_err(map_storage)?;
    Ok(record)
}

pub async fn work_ticket_list(
    pool: &SqlitePool,
    security_id: Option<Uuid>,
    status: Option<String>,
) -> Result<Vec<WorkTicketRecord>, PlatformError> {
    let sid = security_id.map(|id| id.to_string()).unwrap_or_default();
    let st = status.unwrap_or_default();
    let rows = sqlx::query(
        "SELECT ticket_id, security_id, symbol, field, code, tool, reason, urls_tried,
                opened_on, last_seen_on, status, filed_on, completed_how, owner_note, retrieve_run_id
         FROM work_ticket
         WHERE (? = '' OR security_id = ?)
           AND (? = '' OR status = ?)
         ORDER BY last_seen_on DESC",
    )
    .bind(&sid)
    .bind(&sid)
    .bind(&st)
    .bind(&st)
    .fetch_all(pool)
    .await
    .map_err(map_storage)?;
    rows.iter().map(row_to_ticket).collect()
}

pub async fn work_ticket_get(
    pool: &SqlitePool,
    ticket_id: Uuid,
) -> Result<WorkTicketRecord, PlatformError> {
    let row = sqlx::query(
        "SELECT ticket_id, security_id, symbol, field, code, tool, reason, urls_tried,
                opened_on, last_seen_on, status, filed_on, completed_how, owner_note, retrieve_run_id
         FROM work_ticket WHERE ticket_id = ?",
    )
    .bind(ticket_id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(map_storage)?
    .ok_or_else(|| PlatformError::new("not_found", "work ticket not found"))?;
    row_to_ticket(&row)
}

pub async fn work_ticket_update(
    pool: &SqlitePool,
    record: WorkTicketRecord,
) -> Result<WorkTicketRecord, PlatformError> {
    sqlx::query(
        "UPDATE work_ticket SET
            symbol = ?, field = ?, code = ?, tool = ?, reason = ?, urls_tried = ?,
            last_seen_on = ?, status = ?, filed_on = ?, completed_how = ?,
            owner_note = ?, retrieve_run_id = ?
         WHERE ticket_id = ?",
    )
    .bind(&record.symbol)
    .bind(&record.field)
    .bind(&record.code)
    .bind(&record.tool)
    .bind(&record.reason)
    .bind(&record.urls_tried)
    .bind(&record.last_seen_on)
    .bind(&record.status)
    .bind(&record.filed_on)
    .bind(&record.completed_how)
    .bind(&record.owner_note)
    .bind(&record.retrieve_run_id)
    .bind(record.ticket_id.to_string())
    .execute(pool)
    .await
    .map_err(map_storage)?;
    Ok(record)
}
