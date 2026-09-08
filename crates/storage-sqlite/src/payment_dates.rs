//! Owner overrides of remaining-year payment dates. System dates are not stored.

use application_core::contracts::RemainingPaymentDateOverride;
use application_core::ports::platform::PlatformError;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::store::StorageError;

fn map_err(err: StorageError) -> PlatformError {
    PlatformError::new("storage_error", err.to_string())
}

pub async fn override_record(
    pool: &SqlitePool,
    record: RemainingPaymentDateOverride,
) -> Result<RemainingPaymentDateOverride, PlatformError> {
    sqlx::query(
        "UPDATE remaining_payment_date_override
         SET superseded_by = ?
         WHERE security_id = ? AND original_pay_on = ? AND superseded_by IS NULL",
    )
    .bind(record.override_id.to_string())
    .bind(record.security_id.to_string())
    .bind(&record.original_pay_on)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    sqlx::query(
        "INSERT INTO remaining_payment_date_override (
            override_id, security_id, original_pay_on, pay_on, recorded_at, superseded_by
         ) VALUES (?, ?, ?, ?, ?, NULL)",
    )
    .bind(record.override_id.to_string())
    .bind(record.security_id.to_string())
    .bind(&record.original_pay_on)
    .bind(&record.pay_on)
    .bind(&record.recorded_at)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(record)
}

pub async fn override_list(
    pool: &SqlitePool,
    security_id: Uuid,
) -> Result<Vec<RemainingPaymentDateOverride>, PlatformError> {
    let rows = sqlx::query(
        "SELECT override_id, security_id, original_pay_on, pay_on, recorded_at
         FROM remaining_payment_date_override
         WHERE security_id = ? AND superseded_by IS NULL
         ORDER BY recorded_at, override_id",
    )
    .bind(security_id.to_string())
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    rows.iter().map(override_from_row).collect()
}

fn override_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<RemainingPaymentDateOverride, PlatformError> {
    Ok(RemainingPaymentDateOverride {
        override_id: Uuid::parse_str(
            &row.try_get::<String, _>("override_id")
                .map_err(|e| map_err(e.into()))?,
        )
        .map_err(|e| PlatformError::new("storage_error", e.to_string()))?,
        security_id: Uuid::parse_str(
            &row.try_get::<String, _>("security_id")
                .map_err(|e| map_err(e.into()))?,
        )
        .map_err(|e| PlatformError::new("storage_error", e.to_string()))?,
        original_pay_on: row.try_get("original_pay_on").map_err(|e| map_err(e.into()))?,
        pay_on: row.try_get("pay_on").map_err(|e| map_err(e.into()))?,
        recorded_at: row.try_get("recorded_at").map_err(|e| map_err(e.into()))?,
    })
}
