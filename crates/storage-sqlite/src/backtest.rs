//! Backtest runs (table-isolated from ledger, lots, and MAGI).

use application_core::contracts::{BacktestGetBody, BacktestRunRecord};
use application_core::ports::platform::PlatformError;
use financial_domain::backtest::prepare_run;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::store::StorageError;

fn map_err(err: StorageError) -> PlatformError {
    PlatformError::new("storage_error", err.to_string())
}

pub async fn run_stub(
    pool: &SqlitePool,
    scenario: String,
    hypothetical_pnl_minor: i64,
    scale: u8,
    completed_at: String,
) -> Result<BacktestGetBody, PlatformError> {
    let prepared = prepare_run(scenario, hypothetical_pnl_minor, scale);
    sqlx::query(
        "INSERT INTO backtest_run (run_id, scenario, hypothetical_pnl_minor, scale, completed_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&prepared.scenario)
    .bind(prepared.hypothetical_pnl_minor)
    .bind(prepared.scale as i64)
    .bind(&completed_at)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    run_get(pool).await
}

pub async fn run_get(pool: &SqlitePool) -> Result<BacktestGetBody, PlatformError> {
    let rows = sqlx::query(
        "SELECT run_id, scenario, hypothetical_pnl_minor, scale, completed_at
         FROM backtest_run ORDER BY completed_at, run_id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    let mut runs = Vec::new();
    for row in &rows {
        let id: String = row.try_get("run_id").map_err(|e| map_err(e.into()))?;
        runs.push(BacktestRunRecord {
            run_id: Uuid::parse_str(&id)
                .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
            scenario: row.try_get("scenario").map_err(|e| map_err(e.into()))?,
            hypothetical_pnl_minor: row
                .try_get("hypothetical_pnl_minor")
                .map_err(|e| map_err(e.into()))?,
            scale: row.try_get::<i64, _>("scale").map_err(|e| map_err(e.into()))? as u8,
            completed_at: row.try_get("completed_at").map_err(|e| map_err(e.into()))?,
        });
    }
    Ok(BacktestGetBody { runs })
}
