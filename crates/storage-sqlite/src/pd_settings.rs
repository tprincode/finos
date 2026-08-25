//! Expected payment weekday patterns and expected tax handling (TR-PD-09, TR-PD-10).
//! Owner settings. Never inferred from declaration dates or ledger cash.

use application_core::contracts::{ExpectedPaymentPattern, PositionTaxProfile};
use application_core::ports::platform::PlatformError;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::store::StorageError;

fn map_err(err: StorageError) -> PlatformError {
    PlatformError::new("storage_error", err.to_string())
}

pub async fn expected_payment_pattern_upsert(
    pool: &SqlitePool,
    record: ExpectedPaymentPattern,
) -> Result<ExpectedPaymentPattern, PlatformError> {
    sqlx::query(
        "INSERT INTO expected_payment_pattern (
            security_id, declaration_weekday, exdate_weekday, payday_weekday
         ) VALUES (?, ?, ?, ?)
         ON CONFLICT(security_id) DO UPDATE SET
            declaration_weekday = excluded.declaration_weekday,
            exdate_weekday = excluded.exdate_weekday,
            payday_weekday = excluded.payday_weekday",
    )
    .bind(record.security_id.to_string())
    .bind(&record.declaration_weekday)
    .bind(&record.exdate_weekday)
    .bind(&record.payday_weekday)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(record)
}

pub async fn expected_payment_pattern_list(
    pool: &SqlitePool,
) -> Result<Vec<ExpectedPaymentPattern>, PlatformError> {
    let rows = sqlx::query(
        "SELECT security_id, declaration_weekday, exdate_weekday, payday_weekday
         FROM expected_payment_pattern ORDER BY security_id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    rows.iter()
        .map(|row| {
            Ok(ExpectedPaymentPattern {
                security_id: parse_uuid(row, "security_id")?,
                declaration_weekday: row
                    .try_get("declaration_weekday")
                    .map_err(|e| map_err(e.into()))?,
                exdate_weekday: row
                    .try_get("exdate_weekday")
                    .map_err(|e| map_err(e.into()))?,
                payday_weekday: row
                    .try_get("payday_weekday")
                    .map_err(|e| map_err(e.into()))?,
            })
        })
        .collect()
}

pub async fn position_tax_profile_upsert(
    pool: &SqlitePool,
    record: PositionTaxProfile,
) -> Result<PositionTaxProfile, PlatformError> {
    sqlx::query(
        "INSERT INTO position_tax_profile (security_id, expected_handling) VALUES (?, ?)
         ON CONFLICT(security_id) DO UPDATE SET expected_handling = excluded.expected_handling",
    )
    .bind(record.security_id.to_string())
    .bind(&record.expected_handling)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(record)
}

pub async fn position_tax_profile_list(
    pool: &SqlitePool,
) -> Result<Vec<PositionTaxProfile>, PlatformError> {
    let rows = sqlx::query(
        "SELECT security_id, expected_handling FROM position_tax_profile ORDER BY security_id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    rows.iter()
        .map(|row| {
            Ok(PositionTaxProfile {
                security_id: parse_uuid(row, "security_id")?,
                expected_handling: row
                    .try_get("expected_handling")
                    .map_err(|e| map_err(e.into()))?,
            })
        })
        .collect()
}

fn parse_uuid(row: &sqlx::sqlite::SqliteRow, col: &str) -> Result<Uuid, PlatformError> {
    let raw: String = row.try_get(col).map_err(|e| map_err(e.into()))?;
    Uuid::parse_str(&raw).map_err(|e| PlatformError::new("parse_error", e.to_string()))
}
