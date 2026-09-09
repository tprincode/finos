//! Daily holdings market-value snapshots for Home charts.

use application_core::contracts::AccountMarketValueDailyRecord;
use application_core::ports::platform::PlatformError;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::store::StorageError;

fn map_err(err: StorageError) -> PlatformError {
    PlatformError::new("storage_error", err.to_string())
}

fn map_row(row: &sqlx::sqlite::SqliteRow) -> Result<AccountMarketValueDailyRecord, PlatformError> {
    Ok(AccountMarketValueDailyRecord {
        snapshot_id: row.try_get("snapshot_id").map_err(|e| map_err(e.into()))?,
        account_id: row.try_get("account_id").map_err(|e| map_err(e.into()))?,
        account_name: row.try_get("account_name").map_err(|e| map_err(e.into()))?,
        as_of: row.try_get("as_of").map_err(|e| map_err(e.into()))?,
        market_value_minor: row
            .try_get("market_value_minor")
            .map_err(|e| map_err(e.into()))?,
        market_value_complete: row
            .try_get::<i64, _>("market_value_complete")
            .map_err(|e| map_err(e.into()))?
            != 0,
        scale: row
            .try_get::<i64, _>("scale")
            .map_err(|e| map_err(e.into()))? as u8,
        captured_at: row.try_get("captured_at").map_err(|e| map_err(e.into()))?,
    })
}

pub async fn account_market_value_daily_upsert(
    pool: &SqlitePool,
    mut record: AccountMarketValueDailyRecord,
) -> Result<AccountMarketValueDailyRecord, PlatformError> {
    if record.snapshot_id.is_empty() {
        record.snapshot_id = Uuid::new_v4().to_string();
    }
    sqlx::query(
        "INSERT INTO account_market_value_daily (
            snapshot_id, account_id, account_name, as_of, market_value_minor,
            market_value_complete, scale, captured_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(account_id, as_of) DO UPDATE SET
            snapshot_id = excluded.snapshot_id,
            account_name = excluded.account_name,
            market_value_minor = excluded.market_value_minor,
            market_value_complete = excluded.market_value_complete,
            scale = excluded.scale,
            captured_at = excluded.captured_at",
    )
    .bind(&record.snapshot_id)
    .bind(&record.account_id)
    .bind(&record.account_name)
    .bind(&record.as_of)
    .bind(record.market_value_minor)
    .bind(if record.market_value_complete { 1 } else { 0 })
    .bind(record.scale as i64)
    .bind(&record.captured_at)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(record)
}

pub async fn account_market_value_daily_list(
    pool: &SqlitePool,
) -> Result<Vec<AccountMarketValueDailyRecord>, PlatformError> {
    let rows = sqlx::query(
        "SELECT snapshot_id, account_id, account_name, as_of, market_value_minor,
                market_value_complete, scale, captured_at
         FROM account_market_value_daily
         ORDER BY as_of ASC, account_name ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    rows.iter().map(map_row).collect()
}