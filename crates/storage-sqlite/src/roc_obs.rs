//! Stored 19a-1 / owner-override ROC research provenance. Not MAGI facts.

use application_core::contracts::RocResearchObservation;
use application_core::ports::platform::PlatformError;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::store::StorageError;

fn map_err(err: StorageError) -> PlatformError {
    PlatformError::new("storage_error", err.to_string())
}

pub async fn observation_record(
    pool: &SqlitePool,
    record: RocResearchObservation,
) -> Result<RocResearchObservation, PlatformError> {
    sqlx::query(
        "INSERT INTO roc_research_observation (
            observation_id, security_id, roc_pct_minor, scale, tax_year, source,
            source_url, method, as_of, kind, established_how, owner_override, recorded_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(record.observation_id.to_string())
    .bind(record.security_id.to_string())
    .bind(record.roc_pct_minor)
    .bind(record.scale as i64)
    .bind(&record.tax_year)
    .bind(&record.source)
    .bind(&record.source_url)
    .bind(&record.method)
    .bind(&record.as_of)
    .bind(&record.kind)
    .bind(&record.established_how)
    .bind(if record.owner_override { 1 } else { 0 })
    .bind(&record.recorded_at)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(record)
}

pub async fn observation_list(
    pool: &SqlitePool,
    security_id: Uuid,
) -> Result<Vec<RocResearchObservation>, PlatformError> {
    let rows = sqlx::query(
        "SELECT observation_id, security_id, roc_pct_minor, scale, tax_year, source,
                source_url, method, as_of, kind, established_how, owner_override, recorded_at
         FROM roc_research_observation
         WHERE security_id = ?
         ORDER BY recorded_at, observation_id",
    )
    .bind(security_id.to_string())
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    rows.iter().map(observation_from_row).collect()
}

fn observation_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<RocResearchObservation, PlatformError> {
    Ok(RocResearchObservation {
        observation_id: Uuid::parse_str(&row.try_get::<String, _>("observation_id").map_err(|e| map_err(e.into()))?)
            .map_err(|e| PlatformError::new("storage_error", e.to_string()))?,
        security_id: Uuid::parse_str(&row.try_get::<String, _>("security_id").map_err(|e| map_err(e.into()))?)
            .map_err(|e| PlatformError::new("storage_error", e.to_string()))?,
        roc_pct_minor: row.try_get("roc_pct_minor").map_err(|e| map_err(e.into()))?,
        scale: row.try_get::<i64, _>("scale").map_err(|e| map_err(e.into()))? as u8,
        tax_year: row.try_get("tax_year").map_err(|e| map_err(e.into()))?,
        source: row.try_get("source").map_err(|e| map_err(e.into()))?,
        source_url: row.try_get("source_url").map_err(|e| map_err(e.into()))?,
        method: row.try_get("method").map_err(|e| map_err(e.into()))?,
        as_of: row.try_get("as_of").map_err(|e| map_err(e.into()))?,
        kind: row.try_get("kind").map_err(|e| map_err(e.into()))?,
        established_how: row.try_get("established_how").map_err(|e| map_err(e.into()))?,
        owner_override: row.try_get::<i64, _>("owner_override").map_err(|e| map_err(e.into()))? != 0,
        recorded_at: row.try_get("recorded_at").map_err(|e| map_err(e.into()))?,
    })
}
