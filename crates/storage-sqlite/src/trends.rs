//! Weekly Trends account balances and entered week metrics.

use application_core::contracts::{AccountBalanceSnapshotRecord, TrendsWeekSourceRecord};
use application_core::ports::platform::PlatformError;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::store::StorageError;

fn map_err(err: StorageError) -> PlatformError {
    PlatformError::new("storage_error", err.to_string())
}

pub async fn trends_week_upsert(
    pool: &SqlitePool,
    record: TrendsWeekSourceRecord,
) -> Result<TrendsWeekSourceRecord, PlatformError> {
    sqlx::query(
        "INSERT INTO trends_week_source (
            period_end, period_start, profit_minor, monthly_divs_minor, fidelity_total_minor, schwab_total_minor,
            income_cash_minor, acct9_cash_minor, acct9_etf_value_minor, scale, captured_at, closed
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(period_end) DO UPDATE SET
            period_start = excluded.period_start,
            profit_minor = excluded.profit_minor,
            monthly_divs_minor = excluded.monthly_divs_minor,
            fidelity_total_minor = excluded.fidelity_total_minor,
            schwab_total_minor = excluded.schwab_total_minor,
            income_cash_minor = excluded.income_cash_minor,
            acct9_cash_minor = excluded.acct9_cash_minor,
            acct9_etf_value_minor = excluded.acct9_etf_value_minor,
            scale = excluded.scale,
            captured_at = excluded.captured_at,
            closed = excluded.closed",
    )
    .bind(&record.period_end)
    .bind(&record.period_start)
    .bind(record.profit_minor)
    .bind(record.monthly_divs_minor)
    .bind(record.fidelity_total_minor)
    .bind(record.schwab_total_minor)
    .bind(record.income_cash_minor)
    .bind(record.acct9_cash_minor)
    .bind(record.acct9_etf_value_minor)
    .bind(record.scale as i64)
    .bind(&record.captured_at)
    .bind(if record.closed { 1 } else { 0 })
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(record)
}

fn map_week_row(row: &sqlx::sqlite::SqliteRow) -> Result<TrendsWeekSourceRecord, PlatformError> {
    let closed: i64 = row.try_get("closed").unwrap_or(0);
    let period_start: String = row.try_get("period_start").unwrap_or_default();
    Ok(TrendsWeekSourceRecord {
        period_end: row.try_get("period_end").map_err(|e| map_err(e.into()))?,
        period_start,
        profit_minor: row.try_get("profit_minor").map_err(|e| map_err(e.into()))?,
        monthly_divs_minor: row
            .try_get("monthly_divs_minor")
            .map_err(|e| map_err(e.into()))?,
        fidelity_total_minor: row
            .try_get("fidelity_total_minor")
            .map_err(|e| map_err(e.into()))?,
        schwab_total_minor: row
            .try_get("schwab_total_minor")
            .map_err(|e| map_err(e.into()))?,
        income_cash_minor: row
            .try_get("income_cash_minor")
            .map_err(|e| map_err(e.into()))?,
        acct9_cash_minor: row
            .try_get("acct9_cash_minor")
            .map_err(|e| map_err(e.into()))?,
        acct9_etf_value_minor: row
            .try_get("acct9_etf_value_minor")
            .map_err(|e| map_err(e.into()))?,
        scale: row.try_get::<i64, _>("scale").map_err(|e| map_err(e.into()))? as u8,
        captured_at: row.try_get("captured_at").map_err(|e| map_err(e.into()))?,
        closed: closed != 0,
    })
}

pub async fn trends_week_list(pool: &SqlitePool) -> Result<Vec<TrendsWeekSourceRecord>, PlatformError> {
    let rows = sqlx::query(
        "SELECT period_end, period_start, profit_minor, monthly_divs_minor, fidelity_total_minor, schwab_total_minor,
                income_cash_minor, acct9_cash_minor, acct9_etf_value_minor, scale, captured_at, closed
         FROM trends_week_source
         ORDER BY period_end ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    let mut out = Vec::with_capacity(rows.len());
    for row in &rows {
        out.push(map_week_row(row)?);
    }
    Ok(out)
}

pub async fn trends_week_get(
    pool: &SqlitePool,
    period_end: &str,
) -> Result<Option<TrendsWeekSourceRecord>, PlatformError> {
    let row = sqlx::query(
        "SELECT period_end, period_start, profit_minor, monthly_divs_minor, fidelity_total_minor, schwab_total_minor,
                income_cash_minor, acct9_cash_minor, acct9_etf_value_minor, scale, captured_at, closed
         FROM trends_week_source WHERE period_end = ?",
    )
    .bind(period_end)
    .fetch_optional(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    match row {
        Some(r) => Ok(Some(map_week_row(&r)?)),
        None => Ok(None),
    }
}

pub async fn trends_week_set_closed(
    pool: &SqlitePool,
    period_end: &str,
    closed: bool,
) -> Result<(), PlatformError> {
    let res = sqlx::query("UPDATE trends_week_source SET closed = ? WHERE period_end = ?")
        .bind(if closed { 1 } else { 0 })
        .bind(period_end)
        .execute(pool)
        .await
        .map_err(|e| map_err(e.into()))?;
    if res.rows_affected() == 0 {
        return Err(PlatformError::new(
            "trends_week_missing",
            format!("no trends week {period_end}"),
        ));
    }
    Ok(())
}

/// Replace-all helper for production Trends seed (drops prior synthetic/orphan weeks).
pub async fn trends_series_clear(pool: &SqlitePool) -> Result<(), PlatformError> {
    sqlx::query("DELETE FROM account_balance_snapshot")
        .execute(pool)
        .await
        .map_err(|e| map_err(e.into()))?;
    sqlx::query("DELETE FROM trends_week_source")
        .execute(pool)
        .await
        .map_err(|e| map_err(e.into()))?;
    Ok(())
}

pub async fn account_balance_snapshot_upsert(
    pool: &SqlitePool,
    account_id: Uuid,
    period_end: String,
    balance_minor: i64,
    scale: u8,
    captured_at: String,
    cash_minor: Option<i64>,
) -> Result<AccountBalanceSnapshotRecord, PlatformError> {
    let snapshot_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO account_balance_snapshot (
            snapshot_id, account_id, period_end, balance_minor, scale, captured_at, cash_minor
         ) VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(account_id, period_end) DO UPDATE SET
            balance_minor = excluded.balance_minor,
            scale = excluded.scale,
            captured_at = excluded.captured_at,
            cash_minor = COALESCE(excluded.cash_minor, account_balance_snapshot.cash_minor)",
    )
    .bind(snapshot_id.to_string())
    .bind(account_id.to_string())
    .bind(&period_end)
    .bind(balance_minor)
    .bind(scale as i64)
    .bind(&captured_at)
    .bind(cash_minor)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    let row = sqlx::query(
        "SELECT snapshot_id, account_id, period_end, balance_minor, cash_minor, scale, captured_at
         FROM account_balance_snapshot
         WHERE account_id = ? AND period_end = ?",
    )
    .bind(account_id.to_string())
    .bind(&period_end)
    .fetch_one(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    let sid: String = row.try_get("snapshot_id").map_err(|e| map_err(e.into()))?;
    let aid: String = row.try_get("account_id").map_err(|e| map_err(e.into()))?;
    Ok(AccountBalanceSnapshotRecord {
        snapshot_id: Uuid::parse_str(&sid)
            .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
        account_id: Uuid::parse_str(&aid)
            .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
        period_end: row.try_get("period_end").map_err(|e| map_err(e.into()))?,
        balance_minor: row.try_get("balance_minor").map_err(|e| map_err(e.into()))?,
        cash_minor: row.try_get("cash_minor").ok(),
        scale: row.try_get::<i64, _>("scale").map_err(|e| map_err(e.into()))? as u8,
        captured_at: row.try_get("captured_at").map_err(|e| map_err(e.into()))?,
    })
}

pub async fn account_balance_snapshot_list(
    pool: &SqlitePool,
) -> Result<Vec<AccountBalanceSnapshotRecord>, PlatformError> {
    let rows = sqlx::query(
        "SELECT snapshot_id, account_id, period_end, balance_minor, cash_minor, scale, captured_at
         FROM account_balance_snapshot
         ORDER BY period_end ASC, account_id ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    let mut out = Vec::with_capacity(rows.len());
    for row in &rows {
        let sid: String = row.try_get("snapshot_id").map_err(|e| map_err(e.into()))?;
        let aid: String = row.try_get("account_id").map_err(|e| map_err(e.into()))?;
        out.push(AccountBalanceSnapshotRecord {
            snapshot_id: Uuid::parse_str(&sid)
                .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
            account_id: Uuid::parse_str(&aid)
                .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
            period_end: row.try_get("period_end").map_err(|e| map_err(e.into()))?,
            balance_minor: row.try_get("balance_minor").map_err(|e| map_err(e.into()))?,
            cash_minor: row.try_get("cash_minor").ok(),
            scale: row.try_get::<i64, _>("scale").map_err(|e| map_err(e.into()))? as u8,
            captured_at: row.try_get("captured_at").map_err(|e| map_err(e.into()))?,
        });
    }
    Ok(out)
}

pub async fn aca_threshold_get(
    pool: &SqlitePool,
    coverage_year: i32,
    household_size: i32,
    location_code: &str,
) -> Result<Option<(i64, u8)>, PlatformError> {
    let row = sqlx::query(
        "SELECT threshold_minor, scale FROM aca_threshold_rule
         WHERE coverage_year = ? AND household_size = ? AND location_code = ?",
    )
    .bind(coverage_year)
    .bind(household_size)
    .bind(location_code)
    .fetch_optional(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    match row {
        Some(r) => Ok(Some((
            r.try_get("threshold_minor").map_err(|e| map_err(e.into()))?,
            r.try_get::<i64, _>("scale").map_err(|e| map_err(e.into()))? as u8,
        ))),
        None => Ok(None),
    }
}

/// Week row + account snapshots in one SQLite transaction.
pub async fn trends_week_save_atomic(
    pool: &SqlitePool,
    record: TrendsWeekSourceRecord,
    balances: &[(Uuid, i64, Option<i64>)],
) -> Result<(), PlatformError> {
    let mut tx = pool.begin().await.map_err(|e| map_err(e.into()))?;
    sqlx::query(
        "INSERT INTO trends_week_source (
            period_end, period_start, profit_minor, monthly_divs_minor, fidelity_total_minor, schwab_total_minor,
            income_cash_minor, acct9_cash_minor, acct9_etf_value_minor, scale, captured_at, closed
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(period_end) DO UPDATE SET
            period_start = excluded.period_start,
            profit_minor = excluded.profit_minor,
            monthly_divs_minor = excluded.monthly_divs_minor,
            fidelity_total_minor = excluded.fidelity_total_minor,
            schwab_total_minor = excluded.schwab_total_minor,
            income_cash_minor = excluded.income_cash_minor,
            acct9_cash_minor = excluded.acct9_cash_minor,
            acct9_etf_value_minor = excluded.acct9_etf_value_minor,
            scale = excluded.scale,
            captured_at = excluded.captured_at,
            closed = excluded.closed",
    )
    .bind(&record.period_end)
    .bind(&record.period_start)
    .bind(record.profit_minor)
    .bind(record.monthly_divs_minor)
    .bind(record.fidelity_total_minor)
    .bind(record.schwab_total_minor)
    .bind(record.income_cash_minor)
    .bind(record.acct9_cash_minor)
    .bind(record.acct9_etf_value_minor)
    .bind(record.scale as i64)
    .bind(&record.captured_at)
    .bind(if record.closed { 1 } else { 0 })
    .execute(&mut *tx)
    .await
    .map_err(|e| map_err(e.into()))?;
    for (account_id, balance_minor, cash_minor) in balances {
        let snapshot_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO account_balance_snapshot (
                snapshot_id, account_id, period_end, balance_minor, scale, captured_at, cash_minor
             ) VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(account_id, period_end) DO UPDATE SET
                balance_minor = excluded.balance_minor,
                scale = excluded.scale,
                captured_at = excluded.captured_at,
                cash_minor = COALESCE(excluded.cash_minor, account_balance_snapshot.cash_minor)",
        )
        .bind(snapshot_id.to_string())
        .bind(account_id.to_string())
        .bind(&record.period_end)
        .bind(balance_minor)
        .bind(record.scale as i64)
        .bind(&record.captured_at)
        .bind(cash_minor)
        .execute(&mut *tx)
        .await
        .map_err(|e| map_err(e.into()))?;
    }
    tx.commit().await.map_err(|e| map_err(e.into()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::LocalDatabase;

    fn week_record(period_end: &str) -> TrendsWeekSourceRecord {
        TrendsWeekSourceRecord {
            period_end: period_end.into(),
            period_start: "2026-01-03".into(),
            profit_minor: 1,
            monthly_divs_minor: 0,
            fidelity_total_minor: 0,
            schwab_total_minor: 0,
            income_cash_minor: 0,
            acct9_cash_minor: 0,
            acct9_etf_value_minor: 0,
            scale: 2,
            captured_at: "2026-01-09T00:00:00Z".into(),
            closed: false,
        }
    }

    #[tokio::test]
    async fn week_save_rolls_back_if_snapshot_fails() {
        let dir = tempfile::tempdir().unwrap();
        let db = LocalDatabase::open(dir.path().join("week.sqlite"))
            .await
            .unwrap();
        let record = week_record("2026-01-09");
        let mut tx = db.pool.begin().await.unwrap();
        sqlx::query(
            "INSERT INTO trends_week_source (
                period_end, period_start, profit_minor, monthly_divs_minor, fidelity_total_minor, schwab_total_minor,
                income_cash_minor, acct9_cash_minor, acct9_etf_value_minor, scale, captured_at, closed
             ) VALUES (?, ?, 1, 0, 0, 0, 0, 0, 0, 2, 't', 0)",
        )
        .bind(&record.period_end)
        .bind(&record.period_start)
        .execute(&mut *tx)
        .await
        .unwrap();
        let failed = sqlx::query(
            "INSERT INTO account_balance_snapshot (
                snapshot_id, account_id, period_end, balance_minor, scale, captured_at
             ) VALUES ('s1', 'acct', ?, NULL, 2, 't')",
        )
        .bind(&record.period_end)
        .execute(&mut *tx)
        .await;
        assert!(failed.is_err(), "NULL balance_minor must fail");
        drop(tx);
        let found = trends_week_get(&db.pool, &record.period_end)
            .await
            .unwrap();
        assert!(found.is_none(), "failed snapshot must roll back the week row");
    }
}
