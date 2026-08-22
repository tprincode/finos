//! Distribution characterization. Mirrors SQLite; not a centralization slice.

use application_core::contracts::{DistributionGetBody, DistributionRecord};
use application_core::ports::platform::PlatformError;
use financial_domain::distribution::prepare_characterization;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{map_err, StorageError};

pub async fn characterize(
    pool: &PgPool,
    activity_id: Uuid,
    category: String,
    amount_minor: i64,
    scale: u8,
) -> Result<DistributionGetBody, PlatformError> {
    let prepared = prepare_characterization(category, amount_minor, scale);
    sqlx::query(
        "INSERT INTO distribution_characterization (
            characterization_id, activity_id, category, amount_minor, scale
         ) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(activity_id.to_string())
    .bind(&prepared.category)
    .bind(prepared.amount_minor)
    .bind(prepared.scale as i32)
    .execute(pool)
    .await
    .map_err(|e| map_err(StorageError::from(e)))?;
    distribution_get(pool).await
}

pub async fn distribution_get(pool: &PgPool) -> Result<DistributionGetBody, PlatformError> {
    let rows = sqlx::query(
        "SELECT characterization_id, activity_id, category, amount_minor, scale
         FROM distribution_characterization ORDER BY characterization_id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(StorageError::from(e)))?;
    let mut characterizations = Vec::new();
    for row in &rows {
        let id: String = row
            .try_get("characterization_id")
            .map_err(|e| map_err(StorageError::from(e)))?;
        let activity: String = row
            .try_get("activity_id")
            .map_err(|e| map_err(StorageError::from(e)))?;
        characterizations.push(DistributionRecord {
            characterization_id: Uuid::parse_str(&id)
                .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
            activity_id: Uuid::parse_str(&activity)
                .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
            category: row
                .try_get("category")
                .map_err(|e| map_err(StorageError::from(e)))?,
            amount_minor: row
                .try_get("amount_minor")
                .map_err(|e| map_err(StorageError::from(e)))?,
            scale: row
                .try_get::<i32, _>("scale")
                .map_err(|e| map_err(StorageError::from(e)))? as u8,
        });
    }
    Ok(DistributionGetBody { characterizations })
}
