//! Week cash disposition — deposit_received is stamped when broker cash posts.

use application_core::contracts::IncomeWeekCashDispositionRecord;
use application_core::ports::platform::PlatformError;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::store::StorageError;

fn map_err(err: StorageError) -> PlatformError {
    PlatformError::new("storage_error", err.to_string())
}

pub async fn list_for_week(
    pool: &SqlitePool,
    week_end: &str,
) -> Result<Vec<IncomeWeekCashDispositionRecord>, PlatformError> {
    let rows = sqlx::query(
        "SELECT week_end, security_id, disposition, confirmed_at, note
         FROM income_week_cash_disposition
         WHERE week_end = ?
         ORDER BY security_id",
    )
    .bind(week_end)
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    rows.iter()
        .map(|row| {
            let sid: String = row.try_get("security_id").map_err(|e| map_err(e.into()))?;
            Ok(IncomeWeekCashDispositionRecord {
                week_end: row.try_get("week_end").map_err(|e| map_err(e.into()))?,
                security_id: Uuid::parse_str(&sid)
                    .map_err(|e| PlatformError::new("storage_error", e.to_string()))?,
                disposition: row.try_get("disposition").map_err(|e| map_err(e.into()))?,
                confirmed_at: row.try_get("confirmed_at").map_err(|e| map_err(e.into()))?,
                note: row.try_get("note").map_err(|e| map_err(e.into()))?,
            })
        })
        .collect()
}

pub async fn upsert(
    pool: &SqlitePool,
    record: IncomeWeekCashDispositionRecord,
) -> Result<IncomeWeekCashDispositionRecord, PlatformError> {
    if record.disposition != "deposit_received" && record.disposition != "skipped" {
        return Err(PlatformError::new(
            "invalid_disposition",
            "disposition must be deposit_received or skipped",
        ));
    }
    sqlx::query(
        "INSERT INTO income_week_cash_disposition (
            week_end, security_id, disposition, confirmed_at, note
         ) VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(week_end, security_id) DO UPDATE SET
            disposition = excluded.disposition,
            confirmed_at = excluded.confirmed_at,
            note = excluded.note",
    )
    .bind(&record.week_end)
    .bind(record.security_id.to_string())
    .bind(&record.disposition)
    .bind(&record.confirmed_at)
    .bind(&record.note)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(record)
}

pub async fn clear(
    pool: &SqlitePool,
    week_end: &str,
    security_id: Uuid,
) -> Result<u64, PlatformError> {
    let result = sqlx::query(
        "DELETE FROM income_week_cash_disposition
         WHERE week_end = ? AND security_id = ?",
    )
    .bind(week_end)
    .bind(security_id.to_string())
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(result.rows_affected())
}
