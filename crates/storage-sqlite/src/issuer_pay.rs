//! Issuer remaining-year pay dates. Future unpublished dates are replaced on retrieve.

use application_core::contracts::IssuerPayDateRecord;
use application_core::ports::platform::PlatformError;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::store::StorageError;

fn map_err(err: StorageError) -> PlatformError {
    PlatformError::new("storage_error", err.to_string())
}

pub async fn pay_date_replace(
    pool: &SqlitePool,
    security_id: Uuid,
    as_of: String,
    dates: Vec<IssuerPayDateRecord>,
) -> Result<Vec<IssuerPayDateRecord>, PlatformError> {
    let batch = Uuid::new_v4().to_string();
    sqlx::query(
        "UPDATE issuer_pay_date
         SET superseded_by = ?
         WHERE security_id = ? AND pay_on >= ? AND superseded_by IS NULL",
    )
    .bind(&batch)
    .bind(security_id.to_string())
    .bind(&as_of)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    let mut stored = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for mut row in dates {
        if row.pay_on.trim().is_empty() {
            continue;
        }
        if !seen.insert(row.pay_on.clone()) {
            continue;
        }
        row.pay_date_id = Uuid::new_v4();
        row.security_id = security_id;
        sqlx::query(
            "INSERT INTO issuer_pay_date (
                pay_date_id, security_id, pay_on, source, recorded_at, superseded_by
             ) VALUES (?, ?, ?, ?, ?, NULL)",
        )
        .bind(row.pay_date_id.to_string())
        .bind(security_id.to_string())
        .bind(&row.pay_on)
        .bind(&row.source)
        .bind(&row.recorded_at)
        .execute(pool)
        .await
        .map_err(|e| map_err(e.into()))?;
        stored.push(row);
    }
    Ok(stored)
}

pub async fn pay_date_list(
    pool: &SqlitePool,
    security_id: Uuid,
) -> Result<Vec<IssuerPayDateRecord>, PlatformError> {
    let rows = sqlx::query(
        "SELECT pay_date_id, security_id, pay_on, source, recorded_at
         FROM issuer_pay_date
         WHERE security_id = ? AND superseded_by IS NULL
         ORDER BY pay_on, pay_date_id",
    )
    .bind(security_id.to_string())
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    rows.iter()
        .map(|row| {
            Ok(IssuerPayDateRecord {
                pay_date_id: parse_uuid(row, "pay_date_id")?,
                security_id: parse_uuid(row, "security_id")?,
                pay_on: row.try_get("pay_on").map_err(|e| map_err(e.into()))?,
                source: row.try_get("source").map_err(|e| map_err(e.into()))?,
                recorded_at: row.try_get("recorded_at").map_err(|e| map_err(e.into()))?,
            })
        })
        .collect()
}

fn parse_uuid(row: &sqlx::sqlite::SqliteRow, col: &str) -> Result<Uuid, PlatformError> {
    let raw: String = row.try_get(col).map_err(|e| map_err(e.into()))?;
    Uuid::parse_str(&raw).map_err(|e| PlatformError::new("parse_error", e.to_string()))
}
