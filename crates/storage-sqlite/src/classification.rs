//! Classification reviews (table-isolated from ledger and MAGI oracles).

use application_core::contracts::{ClassificationReviewGetBody, ClassificationReviewRecord};
use application_core::ports::platform::PlatformError;
use financial_domain::classification::prepare_review;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::store::StorageError;

fn map_err(err: StorageError) -> PlatformError {
    PlatformError::new("storage_error", err.to_string())
}

pub async fn review_record(
    pool: &SqlitePool,
    fact_key: String,
    classification: String,
    status: String,
) -> Result<ClassificationReviewGetBody, PlatformError> {
    let prepared = prepare_review(fact_key, classification, status);
    sqlx::query(
        "INSERT INTO classification_review (review_id, fact_key, classification, status)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(fact_key) DO UPDATE SET
            classification = excluded.classification,
            status = excluded.status",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&prepared.fact_key)
    .bind(&prepared.classification)
    .bind(&prepared.status)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    review_get(pool).await
}

pub async fn review_get(pool: &SqlitePool) -> Result<ClassificationReviewGetBody, PlatformError> {
    let rows = sqlx::query(
        "SELECT review_id, fact_key, classification, status
         FROM classification_review ORDER BY fact_key",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    let mut reviews = Vec::new();
    for row in &rows {
        let id: String = row.try_get("review_id").map_err(|e| map_err(e.into()))?;
        reviews.push(ClassificationReviewRecord {
            review_id: Uuid::parse_str(&id)
                .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
            fact_key: row.try_get("fact_key").map_err(|e| map_err(e.into()))?,
            classification: row
                .try_get("classification")
                .map_err(|e| map_err(e.into()))?,
            status: row.try_get("status").map_err(|e| map_err(e.into()))?,
        });
    }
    Ok(ClassificationReviewGetBody { reviews })
}
