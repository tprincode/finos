//! Allocation targets (table-isolated from ledger and MAGI).

use application_core::contracts::{AllocationGetBody, AllocationTargetRecord};
use application_core::ports::platform::PlatformError;
use financial_domain::allocation::prepare_target;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::store::StorageError;

fn map_err(err: StorageError) -> PlatformError {
    PlatformError::new("storage_error", err.to_string())
}

pub async fn target_set(
    pool: &SqlitePool,
    name: String,
    target_minor: i64,
    scale: u8,
) -> Result<AllocationGetBody, PlatformError> {
    let prepared = prepare_target(name, target_minor, scale);
    sqlx::query(
        "INSERT INTO allocation_target (target_id, name, target_minor, scale)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(name) DO UPDATE SET
            target_minor = excluded.target_minor,
            scale = excluded.scale",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&prepared.name)
    .bind(prepared.target_minor)
    .bind(prepared.scale as i64)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    target_get(pool).await
}

pub async fn target_get(pool: &SqlitePool) -> Result<AllocationGetBody, PlatformError> {
    let rows = sqlx::query(
        "SELECT target_id, name, target_minor, scale FROM allocation_target ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    let mut targets = Vec::new();
    for row in &rows {
        let id: String = row.try_get("target_id").map_err(|e| map_err(e.into()))?;
        targets.push(AllocationTargetRecord {
            target_id: Uuid::parse_str(&id)
                .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
            name: row.try_get("name").map_err(|e| map_err(e.into()))?,
            target_minor: row.try_get("target_minor").map_err(|e| map_err(e.into()))?,
            scale: row.try_get::<i64, _>("scale").map_err(|e| map_err(e.into()))? as u8,
        });
    }
    Ok(AllocationGetBody {
        targets,
        open_performance_minor: 0,
        open_tax_minor: 0,
        scale: 2,
    })
}
