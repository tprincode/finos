//! Owner-dated BacktestPeriod and PositionBacktestResult. Not MAGI, not hypothetical BacktestRun.

use application_core::contracts::{BacktestPeriodRecord, PositionBacktestResultBody};
use application_core::ports::platform::PlatformError;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::store::StorageError;

fn map_err(err: StorageError) -> PlatformError {
    PlatformError::new("storage_error", err.to_string())
}

pub async fn period_record(
    pool: &SqlitePool,
    record: BacktestPeriodRecord,
) -> Result<BacktestPeriodRecord, PlatformError> {
    sqlx::query(
        "INSERT INTO backtest_period (
            period_id, kind, name, start_on, end_on, benchmark_symbol,
            selection_reason, method, status, recorded_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(record.period_id.to_string())
    .bind(&record.kind)
    .bind(&record.name)
    .bind(&record.start_on)
    .bind(&record.end_on)
    .bind(&record.benchmark_symbol)
    .bind(&record.selection_reason)
    .bind(&record.method)
    .bind(&record.status)
    .bind(&record.recorded_at)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(record)
}

pub async fn period_list(pool: &SqlitePool) -> Result<Vec<BacktestPeriodRecord>, PlatformError> {
    let rows = sqlx::query(
        "SELECT period_id, kind, name, start_on, end_on, benchmark_symbol,
                selection_reason, method, status, recorded_at
         FROM backtest_period ORDER BY recorded_at, period_id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    rows.iter().map(period_from_row).collect()
}

pub async fn period_get(
    pool: &SqlitePool,
    period_id: Uuid,
) -> Result<BacktestPeriodRecord, PlatformError> {
    let row = sqlx::query(
        "SELECT period_id, kind, name, start_on, end_on, benchmark_symbol,
                selection_reason, method, status, recorded_at
         FROM backtest_period WHERE period_id = ?",
    )
    .bind(period_id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(|e| map_err(e.into()))?
    .ok_or_else(|| PlatformError::new("not_found", "backtest period not found"))?;
    period_from_row(&row)
}

pub async fn result_record(
    pool: &SqlitePool,
    record: PositionBacktestResultBody,
) -> Result<PositionBacktestResultBody, PlatformError> {
    sqlx::query(
        "INSERT INTO position_backtest_result (
            result_id, security_id, period_id, price_return_bps, total_return_bps,
            cushion_bps, max_drawdown_bps, recovery_ratio_bps, recovery_days,
            income_reliability_bps, bear_relative_bps, downside_capture_bps,
            upside_capture_bps, completeness, source, calculated_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(record.result_id.to_string())
    .bind(record.security_id.to_string())
    .bind(record.period_id.to_string())
    .bind(record.price_return_bps)
    .bind(record.total_return_bps)
    .bind(record.cushion_bps)
    .bind(record.max_drawdown_bps)
    .bind(record.recovery_ratio_bps)
    .bind(record.recovery_days)
    .bind(record.income_reliability_bps)
    .bind(record.bear_relative_bps)
    .bind(record.downside_capture_bps)
    .bind(record.upside_capture_bps)
    .bind(&record.completeness)
    .bind(&record.source)
    .bind(&record.calculated_at)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(record)
}

pub async fn result_list_for_security(
    pool: &SqlitePool,
    security_id: Uuid,
) -> Result<Vec<PositionBacktestResultBody>, PlatformError> {
    let rows = sqlx::query(
        "SELECT result_id, security_id, period_id, price_return_bps, total_return_bps,
                cushion_bps, max_drawdown_bps, recovery_ratio_bps, recovery_days,
                income_reliability_bps, bear_relative_bps, downside_capture_bps,
                upside_capture_bps, completeness, source, calculated_at
         FROM position_backtest_result
         WHERE security_id = ?
         ORDER BY calculated_at, result_id",
    )
    .bind(security_id.to_string())
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    rows.iter().map(result_from_row).collect()
}

fn period_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<BacktestPeriodRecord, PlatformError> {
    Ok(BacktestPeriodRecord {
        period_id: Uuid::parse_str(
            &row.try_get::<String, _>("period_id")
                .map_err(|e| map_err(e.into()))?,
        )
        .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
        kind: row.try_get("kind").map_err(|e| map_err(e.into()))?,
        name: row.try_get("name").map_err(|e| map_err(e.into()))?,
        start_on: row.try_get("start_on").map_err(|e| map_err(e.into()))?,
        end_on: row.try_get("end_on").map_err(|e| map_err(e.into()))?,
        benchmark_symbol: row
            .try_get("benchmark_symbol")
            .map_err(|e| map_err(e.into()))?,
        selection_reason: row
            .try_get("selection_reason")
            .map_err(|e| map_err(e.into()))?,
        method: row.try_get("method").map_err(|e| map_err(e.into()))?,
        status: row.try_get("status").map_err(|e| map_err(e.into()))?,
        recorded_at: row.try_get("recorded_at").map_err(|e| map_err(e.into()))?,
    })
}

fn result_from_row(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<PositionBacktestResultBody, PlatformError> {
    Ok(PositionBacktestResultBody {
        result_id: Uuid::parse_str(
            &row.try_get::<String, _>("result_id")
                .map_err(|e| map_err(e.into()))?,
        )
        .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
        security_id: Uuid::parse_str(
            &row.try_get::<String, _>("security_id")
                .map_err(|e| map_err(e.into()))?,
        )
        .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
        period_id: Uuid::parse_str(
            &row.try_get::<String, _>("period_id")
                .map_err(|e| map_err(e.into()))?,
        )
        .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
        price_return_bps: row.try_get("price_return_bps").map_err(|e| map_err(e.into()))?,
        total_return_bps: row.try_get("total_return_bps").map_err(|e| map_err(e.into()))?,
        cushion_bps: row.try_get("cushion_bps").map_err(|e| map_err(e.into()))?,
        max_drawdown_bps: row.try_get("max_drawdown_bps").map_err(|e| map_err(e.into()))?,
        recovery_ratio_bps: row
            .try_get("recovery_ratio_bps")
            .map_err(|e| map_err(e.into()))?,
        recovery_days: row.try_get("recovery_days").map_err(|e| map_err(e.into()))?,
        income_reliability_bps: row
            .try_get("income_reliability_bps")
            .map_err(|e| map_err(e.into()))?,
        bear_relative_bps: row
            .try_get("bear_relative_bps")
            .map_err(|e| map_err(e.into()))?,
        downside_capture_bps: row
            .try_get("downside_capture_bps")
            .map_err(|e| map_err(e.into()))?,
        upside_capture_bps: row
            .try_get("upside_capture_bps")
            .map_err(|e| map_err(e.into()))?,
        completeness: row.try_get("completeness").map_err(|e| map_err(e.into()))?,
        source: row.try_get("source").map_err(|e| map_err(e.into()))?,
        calculated_at: row.try_get("calculated_at").map_err(|e| map_err(e.into()))?,
    })
}
