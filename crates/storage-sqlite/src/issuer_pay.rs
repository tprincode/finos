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

pub async fn pay_date_insert(
    pool: &SqlitePool,
    record: IssuerPayDateRecord,
) -> Result<IssuerPayDateRecord, PlatformError> {
    pay_date_dedupe(pool, record.security_id).await?;
    let existing = sqlx::query_scalar::<_, String>(
        "SELECT pay_date_id FROM issuer_pay_date
         WHERE security_id = ? AND pay_on = ? AND superseded_by IS NULL
         ORDER BY pay_date_id LIMIT 1",
    )
    .bind(record.security_id.to_string())
    .bind(&record.pay_on)
    .fetch_optional(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    if existing.is_some() {
        return Ok(record);
    }
    sqlx::query(
        "INSERT INTO issuer_pay_date (
            pay_date_id, security_id, pay_on, source, recorded_at, superseded_by
         ) VALUES (?, ?, ?, ?, ?, NULL)",
    )
    .bind(record.pay_date_id.to_string())
    .bind(record.security_id.to_string())
    .bind(&record.pay_on)
    .bind(&record.source)
    .bind(&record.recorded_at)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(record)
}

/// Keep one current row per (security, pay_on). Sloppy inserts stacked HAKY to 234.
pub async fn pay_date_dedupe(
    pool: &SqlitePool,
    security_id: Uuid,
) -> Result<u64, PlatformError> {
    let rows = sqlx::query(
        "SELECT pay_date_id, pay_on FROM issuer_pay_date
         WHERE security_id = ? AND superseded_by IS NULL
         ORDER BY pay_on, pay_date_id",
    )
    .bind(security_id.to_string())
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    let mut keep: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut drop_ids = Vec::new();
    for row in rows {
        let pay_on: String = row.try_get("pay_on").map_err(|e| map_err(e.into()))?;
        let id: String = row.try_get("pay_date_id").map_err(|e| map_err(e.into()))?;
        if !keep.insert(pay_on) {
            drop_ids.push(id);
        }
    }
    if drop_ids.is_empty() {
        return Ok(0);
    }
    let batch = Uuid::new_v4().to_string();
    for id in &drop_ids {
        sqlx::query("UPDATE issuer_pay_date SET superseded_by = ? WHERE pay_date_id = ?")
            .bind(&batch)
            .bind(id)
            .execute(pool)
            .await
            .map_err(|e| map_err(e.into()))?;
    }
    Ok(drop_ids.len() as u64)
}

pub async fn pay_date_supersede_one(
    pool: &SqlitePool,
    security_id: Uuid,
    pay_on: &str,
) -> Result<(), PlatformError> {
    let batch = Uuid::new_v4().to_string();
    sqlx::query(
        "UPDATE issuer_pay_date
         SET superseded_by = ?
         WHERE security_id = ? AND pay_on = ? AND superseded_by IS NULL",
    )
    .bind(batch)
    .bind(security_id.to_string())
    .bind(pay_on)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(())
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
