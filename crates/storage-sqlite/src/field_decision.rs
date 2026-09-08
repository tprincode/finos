//! Owner Accept/Skip checklist decisions (Phase I establish).

use application_core::contracts::CollectorFieldDecisionRecord;
use application_core::ports::platform::PlatformError;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

fn map_storage(err: sqlx::Error) -> PlatformError {
    PlatformError::new("storage_error", err.to_string())
}

fn row_to_decision(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<CollectorFieldDecisionRecord, PlatformError> {
    Ok(CollectorFieldDecisionRecord {
        security_id: Uuid::parse_str(
            &row.try_get::<String, _>("security_id")
                .map_err(map_storage)?,
        )
        .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
        field: row.try_get("field").map_err(map_storage)?,
        decision: row.try_get("decision").map_err(map_storage)?,
        noted_on: row.try_get("noted_on").map_err(map_storage)?,
    })
}

pub async fn collector_field_decision_set(
    pool: &SqlitePool,
    record: CollectorFieldDecisionRecord,
) -> Result<CollectorFieldDecisionRecord, PlatformError> {
    sqlx::query(
        "INSERT INTO collector_field_decision (security_id, field, decision, noted_on)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(security_id, field) DO UPDATE SET
            decision = excluded.decision,
            noted_on = excluded.noted_on",
    )
    .bind(record.security_id.to_string())
    .bind(&record.field)
    .bind(&record.decision)
    .bind(&record.noted_on)
    .execute(pool)
    .await
    .map_err(map_storage)?;
    Ok(record)
}

pub async fn collector_field_decision_list(
    pool: &SqlitePool,
    security_id: Uuid,
) -> Result<Vec<CollectorFieldDecisionRecord>, PlatformError> {
    let rows = sqlx::query(
        "SELECT security_id, field, decision, noted_on
         FROM collector_field_decision
         WHERE security_id = ?
         ORDER BY field",
    )
    .bind(security_id.to_string())
    .fetch_all(pool)
    .await
    .map_err(map_storage)?;
    rows.iter().map(row_to_decision).collect()
}
