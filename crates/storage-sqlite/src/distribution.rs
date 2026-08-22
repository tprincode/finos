//! Distribution characterization (table-isolated from MAGI oracles).

use application_core::contracts::{DistributionGetBody, DistributionRecord};
use application_core::ports::platform::PlatformError;
use financial_domain::distribution::prepare_characterization;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::store::StorageError;

fn map_err(err: StorageError) -> PlatformError {
    PlatformError::new("storage_error", err.to_string())
}

pub async fn characterize(
    pool: &SqlitePool,
    activity_id: Uuid,
    category: String,
    amount_minor: i64,
    scale: u8,
) -> Result<DistributionGetBody, PlatformError> {
    let prepared = prepare_characterization(category, amount_minor, scale);
    sqlx::query(
        "INSERT INTO distribution_characterization (
            characterization_id, activity_id, category, amount_minor, scale
         ) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(activity_id.to_string())
    .bind(&prepared.category)
    .bind(prepared.amount_minor)
    .bind(prepared.scale as i64)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    distribution_get(pool).await
}

pub async fn distribution_get(pool: &SqlitePool) -> Result<DistributionGetBody, PlatformError> {
    let rows = sqlx::query(
        "SELECT characterization_id, activity_id, category, amount_minor, scale
         FROM distribution_characterization ORDER BY characterization_id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    let mut characterizations = Vec::new();
    for row in &rows {
        let id: String = row
            .try_get("characterization_id")
            .map_err(|e| map_err(e.into()))?;
        let activity: String = row.try_get("activity_id").map_err(|e| map_err(e.into()))?;
        characterizations.push(DistributionRecord {
            characterization_id: Uuid::parse_str(&id)
                .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
            activity_id: Uuid::parse_str(&activity)
                .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
            category: row.try_get("category").map_err(|e| map_err(e.into()))?,
            amount_minor: row.try_get("amount_minor").map_err(|e| map_err(e.into()))?,
            scale: row.try_get::<i64, _>("scale").map_err(|e| map_err(e.into()))? as u8,
        });
    }
    Ok(DistributionGetBody { characterizations })
}
