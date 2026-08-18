//! Versioned Calculator Plan and cash burndown (table-isolated from MAGI and dividend actuals).

use application_core::contracts::{BurndownBody, CalculatorPlanBody};
use application_core::ports::platform::PlatformError;
use financial_domain::plan::{next_plan_version, project_burndown, sum_cash};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::store::StorageError;

fn map_err(err: StorageError) -> PlatformError {
    PlatformError::new("storage_error", err.to_string())
}

pub async fn plan_approve(
    pool: &SqlitePool,
    remaining_minor: i64,
    scale: u8,
    approved_on: String,
) -> Result<CalculatorPlanBody, PlatformError> {
    let current: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(version), 0) FROM calculator_plan")
        .fetch_one(pool)
        .await
        .map_err(|e| map_err(e.into()))?;
    let supersedes: Option<String> = sqlx::query_scalar(
        "SELECT plan_id FROM calculator_plan WHERE version = ?",
    )
    .bind(current)
    .fetch_optional(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    let version = next_plan_version(current as u32);
    let plan_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO calculator_plan (
            plan_id, version, remaining_minor, scale, approved_on, supersedes_plan_id
         ) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(plan_id.to_string())
    .bind(version as i64)
    .bind(remaining_minor)
    .bind(scale as i64)
    .bind(&approved_on)
    .bind(&supersedes)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(CalculatorPlanBody {
        plan_id,
        version,
        remaining_minor,
        scale,
        approved_on,
        supersedes_plan_id: supersedes.and_then(|s| Uuid::parse_str(&s).ok()),
    })
}

pub async fn plan_get(pool: &SqlitePool) -> Result<CalculatorPlanBody, PlatformError> {
    let row = sqlx::query(
        "SELECT plan_id, version, remaining_minor, scale, approved_on, supersedes_plan_id
         FROM calculator_plan ORDER BY version DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    let Some(row) = row else {
        return Ok(CalculatorPlanBody {
            plan_id: Uuid::nil(),
            version: 0,
            remaining_minor: 0,
            scale: 2,
            approved_on: String::new(),
            supersedes_plan_id: None,
        });
    };
    let plan_id: String = row.try_get("plan_id").map_err(|e| map_err(e.into()))?;
    let supersedes: Option<String> = row
        .try_get("supersedes_plan_id")
        .map_err(|e| map_err(e.into()))?;
    Ok(CalculatorPlanBody {
        plan_id: Uuid::parse_str(&plan_id)
            .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
        version: row.try_get::<i64, _>("version").map_err(|e| map_err(e.into()))? as u32,
        remaining_minor: row.try_get("remaining_minor").map_err(|e| map_err(e.into()))?,
        scale: row.try_get::<i64, _>("scale").map_err(|e| map_err(e.into()))? as u8,
        approved_on: row.try_get("approved_on").map_err(|e| map_err(e.into()))?,
        supersedes_plan_id: supersedes.and_then(|s| Uuid::parse_str(&s).ok()),
    })
}

pub async fn burndown_get(pool: &SqlitePool) -> Result<BurndownBody, PlatformError> {
    let plan = plan_get(pool).await?;
    let rows = sqlx::query(
        "SELECT activity_type, amount_minor FROM activity_event
         WHERE corrects_activity_id IS NULL",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    let mut items = Vec::new();
    for row in &rows {
        let activity_type: String = row.try_get("activity_type").map_err(|e| map_err(e.into()))?;
        let amount: i64 = row.try_get("amount_minor").map_err(|e| map_err(e.into()))?;
        items.push((activity_type, amount));
    }
    let cash = sum_cash(items.iter().map(|(t, a)| (t.as_str(), *a)));
    let snap = project_burndown(cash, plan.remaining_minor);
    Ok(BurndownBody {
        cash_minor: snap.cash_minor,
        obligation_minor: snap.obligation_minor,
        surplus_minor: snap.surplus_minor,
        sufficient: snap.sufficient,
        scale: plan.scale,
    })
}
