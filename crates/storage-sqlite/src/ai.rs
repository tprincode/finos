//! Analysis runs (table-isolated from ledger and MAGI). API keys are never stored.

use application_core::contracts::{AiRunListBody, AiRunRecord};
use application_core::ports::platform::PlatformError;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::store::StorageError;

fn map_err(err: StorageError) -> PlatformError {
    PlatformError::new("storage_error", err.to_string())
}

pub async fn run_insert(
    pool: &SqlitePool,
    prompt: String,
    recommendation: String,
    provider: String,
    model: String,
    status: String,
) -> Result<AiRunRecord, PlatformError> {
    let run_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO analysis_run (run_id, prompt, recommendation, provider, model, status)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(run_id.to_string())
    .bind(&prompt)
    .bind(&recommendation)
    .bind(&provider)
    .bind(&model)
    .bind(&status)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(AiRunRecord {
        run_id,
        prompt,
        recommendation,
        provider,
        model,
        status,
    })
}

pub async fn run_get(pool: &SqlitePool, run_id: Uuid) -> Result<AiRunRecord, PlatformError> {
    let row = sqlx::query(
        "SELECT run_id, prompt, recommendation, provider, model, status
         FROM analysis_run WHERE run_id = ?",
    )
    .bind(run_id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(|e| map_err(e.into()))?
    .ok_or_else(|| PlatformError::new("not_found", "analysis run not found"))?;
    row_to_record(row)
}

pub async fn run_list(pool: &SqlitePool) -> Result<AiRunListBody, PlatformError> {
    let rows = sqlx::query(
        "SELECT run_id, prompt, recommendation, provider, model, status
         FROM analysis_run ORDER BY run_id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    let mut runs = Vec::new();
    for row in rows {
        runs.push(row_to_record(row)?);
    }
    Ok(AiRunListBody { runs })
}

fn row_to_record(row: sqlx::sqlite::SqliteRow) -> Result<AiRunRecord, PlatformError> {
    let id: String = row.try_get("run_id").map_err(|e| map_err(e.into()))?;
    Ok(AiRunRecord {
        run_id: Uuid::parse_str(&id).map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
        prompt: row.try_get("prompt").map_err(|e| map_err(e.into()))?,
        recommendation: row
            .try_get("recommendation")
            .map_err(|e| map_err(e.into()))?,
        provider: row.try_get("provider").map_err(|e| map_err(e.into()))?,
        model: row.try_get("model").map_err(|e| map_err(e.into()))?,
        status: row.try_get("status").map_err(|e| map_err(e.into()))?,
    })
}
