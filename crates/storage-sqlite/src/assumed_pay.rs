//! Assumed next-year pay dates. Vendor issuer_pay_date prunes the same slot.

use application_core::contracts::AssumedPayDateRecord;
use application_core::ports::platform::PlatformError;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::store::StorageError;

fn map_err(err: StorageError) -> PlatformError {
    PlatformError::new("storage_error", err.to_string())
}

pub async fn list(
    pool: &SqlitePool,
    security_id: Uuid,
) -> Result<Vec<AssumedPayDateRecord>, PlatformError> {
    let rows = sqlx::query(
        "SELECT assumed_pay_date_id, security_id, pay_on, cadence, provenance, assumed_on
         FROM assumed_pay_date
         WHERE security_id = ?
         ORDER BY pay_on, assumed_pay_date_id",
    )
    .bind(security_id.to_string())
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    rows.iter()
        .map(|row| {
            Ok(AssumedPayDateRecord {
                assumed_pay_date_id: parse_uuid(row, "assumed_pay_date_id")?,
                security_id: parse_uuid(row, "security_id")?,
                pay_on: row.try_get("pay_on").map_err(|e| map_err(e.into()))?,
                cadence: row.try_get("cadence").map_err(|e| map_err(e.into()))?,
                provenance: row.try_get("provenance").map_err(|e| map_err(e.into()))?,
                assumed_on: row.try_get("assumed_on").map_err(|e| map_err(e.into()))?,
            })
        })
        .collect()
}

pub async fn insert(
    pool: &SqlitePool,
    mut record: AssumedPayDateRecord,
) -> Result<AssumedPayDateRecord, PlatformError> {
    if record.assumed_pay_date_id == Uuid::nil() {
        record.assumed_pay_date_id = Uuid::new_v4();
    }
    sqlx::query(
        "INSERT OR IGNORE INTO assumed_pay_date (
            assumed_pay_date_id, security_id, pay_on, cadence, provenance, assumed_on
         ) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(record.assumed_pay_date_id.to_string())
    .bind(record.security_id.to_string())
    .bind(&record.pay_on)
    .bind(&record.cadence)
    .bind(&record.provenance)
    .bind(&record.assumed_on)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(record)
}

pub async fn prune_for_vendor(
    pool: &SqlitePool,
    security_id: Uuid,
    vendor_pay_on: &str,
) -> Result<u64, PlatformError> {
    let rows = list(pool, security_id).await?;
    let mut dropped = 0u64;
    for row in rows {
        let same = row.pay_on == vendor_pay_on
            || financial_domain::schedule::vendor_payables_same_period(
                &row.cadence,
                &row.pay_on,
                vendor_pay_on,
            );
        if !same {
            continue;
        }
        sqlx::query("DELETE FROM assumed_pay_date WHERE assumed_pay_date_id = ?")
            .bind(row.assumed_pay_date_id.to_string())
            .execute(pool)
            .await
            .map_err(|e| map_err(e.into()))?;
        dropped = dropped.saturating_add(1);
    }
    Ok(dropped)
}

fn parse_uuid(row: &sqlx::sqlite::SqliteRow, col: &str) -> Result<Uuid, PlatformError> {
    let raw: String = row.try_get(col).map_err(|e| map_err(e.into()))?;
    Uuid::parse_str(&raw).map_err(|e| PlatformError::new("parse_error", e.to_string()))
}
