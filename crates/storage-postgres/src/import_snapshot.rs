//! Copy a local SQLite snapshot into PostgreSQL (AC-ARCH-08). Not desktop cutover.

use std::path::Path;

use application_core::contracts::ReconcileCounts;
use application_core::ports::platform::PlatformError;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{PgPool, Row, SqlitePool};

use crate::{map_err, StorageError};

pub async fn reconcile_counts(pool: &PgPool) -> Result<ReconcileCounts, PlatformError> {
    let amount: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_minor), 0)::bigint FROM activity_event WHERE corrects_activity_id IS NULL",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(ReconcileCounts {
        accounts: count(pool, "SELECT COUNT(*) FROM account").await?,
        securities: count(pool, "SELECT COUNT(*) FROM security").await?,
        evidence: 0,
        import_batches: 0,
        posted_activities: count(
            pool,
            "SELECT COUNT(*) FROM activity_event WHERE corrects_activity_id IS NULL",
        )
        .await?,
        amount_minor_sum: amount,
        scale: 2,
        audit_records: count(pool, "SELECT COUNT(*) FROM audit_record").await?,
        exceptions_open: 0,
    })
}

async fn count(pool: &PgPool, sql: &str) -> Result<u64, PlatformError> {
    let n: i64 = sqlx::query_scalar(sql)
        .fetch_one(pool)
        .await
        .map_err(|e| map_err(e.into()))?;
    Ok(n as u64)
}

pub async fn import_sqlite_snapshot(
    pg: &PgPool,
    sqlite_path: &Path,
) -> Result<ReconcileCounts, PlatformError> {
    if !sqlite_path.exists() {
        return Err(PlatformError::new(
            "not_found",
            format!("sqlite snapshot missing: {}", sqlite_path.display()),
        ));
    }
    sqlx::query(
        "TRUNCATE TABLE magi_adjustment, magi_fact, magi_coverage, magi_rule,
                        lot, dividend_actual, dividend_declaration, activity_event,
                        audit_record, command_audit, security, account",
    )
    .execute(pg)
    .await
    .map_err(|e| map_err(e.into()))?;

    let sqlite = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(sqlite_path)
                .read_only(true),
        )
        .await
        .map_err(|e| map_err(StorageError::from(e)))?;

    copy_accounts(pg, &sqlite).await?;
    copy_securities(pg, &sqlite).await?;
    copy_activities(pg, &sqlite).await?;
    copy_dividends(pg, &sqlite).await?;
    copy_lots(pg, &sqlite).await?;
    copy_audit(pg, &sqlite).await?;
    copy_magi(pg, &sqlite).await?;

    reconcile_counts(pg).await
}

async fn copy_accounts(pg: &PgPool, sqlite: &SqlitePool) -> Result<(), PlatformError> {
    let rows = sqlx::query("SELECT account_id, name, kind FROM account")
        .fetch_all(sqlite)
        .await
        .map_err(|e| map_err(e.into()))?;
    for row in rows {
        let id: String = row.try_get("account_id").map_err(|e| map_err(e.into()))?;
        let name: String = row.try_get("name").map_err(|e| map_err(e.into()))?;
        let kind: String = row.try_get("kind").map_err(|e| map_err(e.into()))?;
        sqlx::query("INSERT INTO account (account_id, name, kind, row_version) VALUES ($1, $2, $3, 1)")
            .bind(id)
            .bind(name)
            .bind(kind)
            .execute(pg)
            .await
            .map_err(|e| map_err(e.into()))?;
    }
    Ok(())
}

async fn copy_securities(pg: &PgPool, sqlite: &SqlitePool) -> Result<(), PlatformError> {
    let rows = sqlx::query("SELECT security_id, symbol, name FROM security")
        .fetch_all(sqlite)
        .await
        .map_err(|e| map_err(e.into()))?;
    for row in rows {
        sqlx::query("INSERT INTO security (security_id, symbol, name) VALUES ($1, $2, $3)")
            .bind(row.try_get::<String, _>("security_id").map_err(|e| map_err(e.into()))?)
            .bind(row.try_get::<String, _>("symbol").map_err(|e| map_err(e.into()))?)
            .bind(row.try_get::<String, _>("name").map_err(|e| map_err(e.into()))?)
            .execute(pg)
            .await
            .map_err(|e| map_err(e.into()))?;
    }
    Ok(())
}

async fn copy_activities(pg: &PgPool, sqlite: &SqlitePool) -> Result<(), PlatformError> {
    let rows = sqlx::query(
        "SELECT activity_id, account_id, security_id, activity_type, amount_minor, scale,
                occurred_on, corrects_activity_id, import_batch_id, idempotency_key
         FROM activity_event",
    )
    .fetch_all(sqlite)
    .await
    .map_err(|e| map_err(e.into()))?;
    for row in rows {
        sqlx::query(
            "INSERT INTO activity_event (
                activity_id, account_id, security_id, activity_type, amount_minor, scale,
                occurred_on, corrects_activity_id, import_batch_id, idempotency_key
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(row.try_get::<String, _>("activity_id").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<String, _>("account_id").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<Option<String>, _>("security_id").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<String, _>("activity_type").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<i64, _>("amount_minor").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<i64, _>("scale").map_err(|e| map_err(e.into()))? as i32)
        .bind(row.try_get::<String, _>("occurred_on").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<Option<String>, _>("corrects_activity_id").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<Option<String>, _>("import_batch_id").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<String, _>("idempotency_key").map_err(|e| map_err(e.into()))?)
        .execute(pg)
        .await
        .map_err(|e| map_err(e.into()))?;
    }
    Ok(())
}

async fn copy_dividends(pg: &PgPool, sqlite: &SqlitePool) -> Result<(), PlatformError> {
    let decls = sqlx::query(
        "SELECT declaration_id, security_symbol, declared_on, amount_minor, scale FROM dividend_declaration",
    )
    .fetch_all(sqlite)
    .await
    .map_err(|e| map_err(e.into()))?;
    for row in decls {
        sqlx::query(
            "INSERT INTO dividend_declaration (declaration_id, security_symbol, declared_on, amount_minor, scale)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(row.try_get::<String, _>("declaration_id").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<String, _>("security_symbol").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<String, _>("declared_on").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<i64, _>("amount_minor").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<i64, _>("scale").map_err(|e| map_err(e.into()))? as i32)
        .execute(pg)
        .await
        .map_err(|e| map_err(e.into()))?;
    }
    let actuals = sqlx::query(
        "SELECT actual_id, account_id, security_id, occurred_on, amount_minor, scale, activity_id, idempotency_key
         FROM dividend_actual",
    )
    .fetch_all(sqlite)
    .await
    .map_err(|e| map_err(e.into()))?;
    for row in actuals {
        sqlx::query(
            "INSERT INTO dividend_actual (
                actual_id, account_id, security_id, occurred_on, amount_minor, scale, activity_id, idempotency_key
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(row.try_get::<String, _>("actual_id").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<String, _>("account_id").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<Option<String>, _>("security_id").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<String, _>("occurred_on").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<i64, _>("amount_minor").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<i64, _>("scale").map_err(|e| map_err(e.into()))? as i32)
        .bind(row.try_get::<Option<String>, _>("activity_id").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<String, _>("idempotency_key").map_err(|e| map_err(e.into()))?)
        .execute(pg)
        .await
        .map_err(|e| map_err(e.into()))?;
    }
    Ok(())
}

async fn copy_lots(pg: &PgPool, sqlite: &SqlitePool) -> Result<(), PlatformError> {
    let rows = sqlx::query(
        "SELECT lot_id, account_id, security_id, opened_on, origin, quantity_minor,
                remaining_quantity_minor, quantity_scale, performance_basis_minor, tax_basis_minor,
                remaining_performance_minor, remaining_tax_minor, scale, crf_zero_cost,
                opening_activity_id
         FROM lot",
    )
    .fetch_all(sqlite)
    .await
    .map_err(|e| map_err(e.into()))?;
    for row in rows {
        sqlx::query(
            "INSERT INTO lot (
                lot_id, account_id, security_id, opened_on, origin, quantity_minor,
                remaining_quantity_minor, quantity_scale, performance_basis_minor, tax_basis_minor,
                remaining_performance_minor, remaining_tax_minor, scale, crf_zero_cost,
                opening_activity_id
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
        )
        .bind(row.try_get::<String, _>("lot_id").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<String, _>("account_id").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<String, _>("security_id").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<String, _>("opened_on").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<String, _>("origin").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<i64, _>("quantity_minor").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<i64, _>("remaining_quantity_minor").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<i64, _>("quantity_scale").map_err(|e| map_err(e.into()))? as i32)
        .bind(row.try_get::<i64, _>("performance_basis_minor").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<i64, _>("tax_basis_minor").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<i64, _>("remaining_performance_minor").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<i64, _>("remaining_tax_minor").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<i64, _>("scale").map_err(|e| map_err(e.into()))? as i32)
        .bind(row.try_get::<i64, _>("crf_zero_cost").map_err(|e| map_err(e.into()))? as i32)
        .bind(row.try_get::<Option<String>, _>("opening_activity_id").map_err(|e| map_err(e.into()))?)
        .execute(pg)
        .await
        .map_err(|e| map_err(e.into()))?;
    }
    Ok(())
}

async fn copy_audit(pg: &PgPool, sqlite: &SqlitePool) -> Result<(), PlatformError> {
    let rows = sqlx::query(
        "SELECT audit_id, action, entity_type, entity_id, recorded_at FROM audit_record",
    )
    .fetch_all(sqlite)
    .await
    .map_err(|e| map_err(e.into()))?;
    for row in rows {
        sqlx::query(
            "INSERT INTO audit_record (audit_id, action, entity_type, entity_id, recorded_at)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(row.try_get::<String, _>("audit_id").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<String, _>("action").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<String, _>("entity_type").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<String, _>("entity_id").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<String, _>("recorded_at").map_err(|e| map_err(e.into()))?)
        .execute(pg)
        .await
        .map_err(|e| map_err(e.into()))?;
    }
    Ok(())
}

async fn copy_magi(pg: &PgPool, sqlite: &SqlitePool) -> Result<(), PlatformError> {
    if let Ok(Some(row)) = sqlx::query(
        "SELECT threshold_minor, safety_reserve_minor, scale FROM magi_rule WHERE singleton = 1",
    )
    .fetch_optional(sqlite)
    .await
    {
        sqlx::query(
            "INSERT INTO magi_rule (singleton, threshold_minor, safety_reserve_minor, scale)
             VALUES (1, $1, $2, $3)",
        )
        .bind(row.try_get::<i64, _>("threshold_minor").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<i64, _>("safety_reserve_minor").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<i64, _>("scale").map_err(|e| map_err(e.into()))? as i32)
        .execute(pg)
        .await
        .map_err(|e| map_err(e.into()))?;
    }
    let facts = sqlx::query(
        "SELECT fact_id, source_id, treatment, amount_minor, scale, category FROM magi_fact",
    )
    .fetch_all(sqlite)
    .await
    .map_err(|e| map_err(e.into()))?;
    for row in facts {
        sqlx::query(
            "INSERT INTO magi_fact (fact_id, source_id, treatment, amount_minor, scale, category)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(row.try_get::<String, _>("fact_id").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<String, _>("source_id").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<String, _>("treatment").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<i64, _>("amount_minor").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<i64, _>("scale").map_err(|e| map_err(e.into()))? as i32)
        .bind(row.try_get::<String, _>("category").map_err(|e| map_err(e.into()))?)
        .execute(pg)
        .await
        .map_err(|e| map_err(e.into()))?;
    }
    if let Ok(Some(row)) = sqlx::query(
        "SELECT completeness, remaining_minor, withholding_minor, form_total_minor, warnings_json
         FROM magi_coverage WHERE singleton = 1",
    )
    .fetch_optional(sqlite)
    .await
    {
        sqlx::query(
            "INSERT INTO magi_coverage (
                singleton, completeness, remaining_minor, withholding_minor, form_total_minor, warnings_json
             ) VALUES (1, $1, $2, $3, $4, $5)",
        )
        .bind(row.try_get::<String, _>("completeness").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<i64, _>("remaining_minor").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<i64, _>("withholding_minor").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<i64, _>("form_total_minor").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<String, _>("warnings_json").map_err(|e| map_err(e.into()))?)
        .execute(pg)
        .await
        .map_err(|e| map_err(e.into()))?;
    }
    let adjs = sqlx::query(
        "SELECT adjustment_id, amount_minor, scale, status, reason FROM magi_adjustment",
    )
    .fetch_all(sqlite)
    .await
    .map_err(|e| map_err(e.into()))?;
    for row in adjs {
        sqlx::query(
            "INSERT INTO magi_adjustment (adjustment_id, amount_minor, scale, status, reason)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(row.try_get::<String, _>("adjustment_id").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<i64, _>("amount_minor").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<i64, _>("scale").map_err(|e| map_err(e.into()))? as i32)
        .bind(row.try_get::<String, _>("status").map_err(|e| map_err(e.into()))?)
        .bind(row.try_get::<String, _>("reason").map_err(|e| map_err(e.into()))?)
        .execute(pg)
        .await
        .map_err(|e| map_err(e.into()))?;
    }
    Ok(())
}
