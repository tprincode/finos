//! Versioned Calculator Plan and cash burndown (table-isolated from MAGI and dividend actuals).

use application_core::contracts::{
    BurndownBody, CalculatorPlanBody, LookthroughResearch, PlanHistoryRecord,
    PositionCharacteristicRecord,
};
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

pub async fn plan_history_record(
    pool: &SqlitePool,
    security_id: Uuid,
    amount_per_share_minor: i64,
    amount_scale: u8,
    planning_periods_per_year: u8,
    effective_from: String,
    decision_reason: String,
) -> Result<PlanHistoryRecord, PlatformError> {
    if let Some(existing) = plan_history_for_security(pool, security_id).await? {
        return Ok(existing);
    }
    let record = PlanHistoryRecord {
        plan_history_id: Uuid::new_v4(),
        security_id,
        amount_per_share_minor,
        amount_scale,
        planning_periods_per_year,
        effective_from: effective_from.clone(),
        effective_to: String::new(),
        decision_reason: decision_reason.clone(),
    };
    sqlx::query(
        "INSERT INTO plan_history (
            plan_history_id, security_id, amount_per_share_minor, amount_scale,
            planning_periods_per_year, effective_from, effective_to, decision_date, decision_reason
         ) VALUES (?, ?, ?, ?, ?, ?, NULL, ?, ?)",
    )
    .bind(record.plan_history_id.to_string())
    .bind(security_id.to_string())
    .bind(amount_per_share_minor)
    .bind(amount_scale as i64)
    .bind(planning_periods_per_year as i64)
    .bind(&effective_from)
    .bind(&effective_from)
    .bind(&decision_reason)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(record)
}

async fn plan_history_for_security(
    pool: &SqlitePool,
    security_id: Uuid,
) -> Result<Option<PlanHistoryRecord>, PlatformError> {
    let row = sqlx::query(
        "SELECT plan_history_id, security_id, amount_per_share_minor, amount_scale,
                planning_periods_per_year, effective_from, effective_to, decision_reason
         FROM plan_history WHERE security_id = ? AND effective_to IS NULL",
    )
    .bind(security_id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    row.map(|row| plan_history_from_row(&row)).transpose()
}

fn plan_history_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<PlanHistoryRecord, PlatformError> {
    Ok(PlanHistoryRecord {
        plan_history_id: Uuid::parse_str(
            &row.try_get::<String, _>("plan_history_id")
                .map_err(|e| map_err(e.into()))?,
        )
        .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
        security_id: Uuid::parse_str(
            &row.try_get::<String, _>("security_id")
                .map_err(|e| map_err(e.into()))?,
        )
        .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
        amount_per_share_minor: row
            .try_get("amount_per_share_minor")
            .map_err(|e| map_err(e.into()))?,
        amount_scale: row.try_get::<i64, _>("amount_scale").map_err(|e| map_err(e.into()))? as u8,
        planning_periods_per_year: row
            .try_get::<i64, _>("planning_periods_per_year")
            .map_err(|e| map_err(e.into()))? as u8,
        effective_from: row.try_get("effective_from").map_err(|e| map_err(e.into()))?,
        effective_to: row
            .try_get::<Option<String>, _>("effective_to")
            .ok()
            .flatten()
            .or_else(|| row.try_get::<String, _>("effective_to").ok())
            .unwrap_or_default(),
        decision_reason: row.try_get("decision_reason").map_err(|e| map_err(e.into()))?,
    })
}

pub async fn plan_history_list(pool: &SqlitePool) -> Result<Vec<PlanHistoryRecord>, PlatformError> {
    let rows = sqlx::query(
        "SELECT plan_history_id, security_id, amount_per_share_minor, amount_scale,
                planning_periods_per_year, effective_from, effective_to, decision_reason
         FROM plan_history WHERE effective_to IS NULL ORDER BY security_id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(rows.iter().map(plan_history_from_row).collect::<Result<Vec<_>, _>>()?)
}

pub async fn plan_history_version_list(
    pool: &SqlitePool,
) -> Result<Vec<PlanHistoryRecord>, PlatformError> {
    let rows = sqlx::query(
        "SELECT plan_history_id, security_id, amount_per_share_minor, amount_scale,
                planning_periods_per_year, effective_from, effective_to, decision_reason
         FROM plan_history ORDER BY security_id, effective_from",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(rows.iter().map(plan_history_from_row).collect::<Result<Vec<_>, _>>()?)
}

pub async fn plan_history_confirm(
    pool: &SqlitePool,
    security_id: Uuid,
    amount_per_share_minor: i64,
    amount_scale: u8,
    planning_periods_per_year: u8,
    effective_from: String,
    decision_reason: String,
) -> Result<PlanHistoryRecord, PlatformError> {
    sqlx::query(
        "UPDATE plan_history SET effective_to = ? WHERE security_id = ? AND effective_to IS NULL",
    )
    .bind(&effective_from)
    .bind(security_id.to_string())
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    let record = PlanHistoryRecord {
        plan_history_id: Uuid::new_v4(),
        security_id,
        amount_per_share_minor,
        amount_scale,
        planning_periods_per_year,
        effective_from: effective_from.clone(),
        effective_to: String::new(),
        decision_reason: decision_reason.clone(),
    };
    sqlx::query(
        "INSERT INTO plan_history (
            plan_history_id, security_id, amount_per_share_minor, amount_scale,
            planning_periods_per_year, effective_from, effective_to, decision_date, decision_reason
         ) VALUES (?, ?, ?, ?, ?, ?, NULL, ?, ?)",
    )
    .bind(record.plan_history_id.to_string())
    .bind(security_id.to_string())
    .bind(amount_per_share_minor)
    .bind(amount_scale as i64)
    .bind(planning_periods_per_year as i64)
    .bind(&effective_from)
    .bind(&effective_from)
    .bind(&decision_reason)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(record)
}

pub async fn position_characteristic_upsert(
    pool: &SqlitePool,
    record: PositionCharacteristicRecord,
) -> Result<PositionCharacteristicRecord, PlatformError> {
    let mut record = record;
    record.risk_tier = financial_domain::plan_review::normalize_risk_tier(&record.risk_tier);
    // Process A may persist provider / lookthrough / needs_roc before cadence is known.
    // Owner PositionCharacteristicUpsert still requires cadence via locked_cadence in queries.
    if record.payment_frequency.trim().is_empty() {
        sqlx::query(
            "INSERT INTO position_characteristic (
                security_id, payment_frequency, risk_tier, provider, underlying,
                roc_pct_2025_actual_minor, roc_pct_2026_estimate_minor, roc_pct_2026_actual_minor,
                roc_pct_2024_actual_minor, roc_scale, div_type, needs_roc_research, notes, is_active,
                lookthrough_json
             ) VALUES (?, '', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(security_id) DO UPDATE SET
                risk_tier = excluded.risk_tier,
                provider = CASE
                    WHEN excluded.provider != '' THEN excluded.provider
                    ELSE position_characteristic.provider
                END,
                underlying = CASE
                    WHEN excluded.underlying != '' THEN excluded.underlying
                    ELSE position_characteristic.underlying
                END,
                roc_pct_2025_actual_minor = COALESCE(excluded.roc_pct_2025_actual_minor, position_characteristic.roc_pct_2025_actual_minor),
                roc_pct_2026_estimate_minor = COALESCE(excluded.roc_pct_2026_estimate_minor, position_characteristic.roc_pct_2026_estimate_minor),
                roc_pct_2026_actual_minor = COALESCE(excluded.roc_pct_2026_actual_minor, position_characteristic.roc_pct_2026_actual_minor),
                roc_pct_2024_actual_minor = COALESCE(excluded.roc_pct_2024_actual_minor, position_characteristic.roc_pct_2024_actual_minor),
                roc_scale = COALESCE(excluded.roc_scale, position_characteristic.roc_scale),
                div_type = CASE
                    WHEN excluded.div_type != '' THEN excluded.div_type
                    ELSE position_characteristic.div_type
                END,
                needs_roc_research = excluded.needs_roc_research,
                notes = excluded.notes,
                is_active = excluded.is_active,
                lookthrough_json = CASE
                    WHEN excluded.lookthrough_json != '{}' AND excluded.lookthrough_json != ''
                    THEN excluded.lookthrough_json
                    ELSE position_characteristic.lookthrough_json
                END",
        )
        .bind(record.security_id.to_string())
        .bind(financial_domain::plan_review::normalize_risk_tier(&record.risk_tier))
        .bind(&record.provider)
        .bind(&record.underlying)
        .bind(record.roc_pct_2025_actual_minor)
        .bind(record.roc_pct_2026_estimate_minor)
        .bind(record.roc_pct_2026_actual_minor)
        .bind(record.roc_pct_2024_actual_minor)
        .bind(record.roc_scale.map(|s| s as i64))
        .bind(&record.div_type)
        .bind(if record.needs_roc_research { 1 } else { 0 })
        .bind(&record.notes)
        .bind(if record.is_active { 1 } else { 0 })
        .bind(record.lookthrough.to_json_string())
        .execute(pool)
        .await
        .map_err(|e| map_err(e.into()))?;
        return Ok(record);
    }
    let cadence = financial_domain::calculator::PaymentCadence::parse(&record.payment_frequency)
        .ok_or_else(|| {
            PlatformError::new(
                "payment_cadence_required",
                "Weekly (52), Monthly (12), Quarterly (4), or None (does not pay) must be identified; there is no default",
            )
        })?;
    record.payment_frequency = cadence.label().to_string();
    sqlx::query(
        "INSERT INTO position_characteristic (
            security_id, payment_frequency, risk_tier, provider, underlying,
            roc_pct_2025_actual_minor, roc_pct_2026_estimate_minor, roc_pct_2026_actual_minor,
            roc_pct_2024_actual_minor, roc_scale, div_type, needs_roc_research, notes, is_active,
            lookthrough_json
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(security_id) DO UPDATE SET
            payment_frequency = excluded.payment_frequency,
            risk_tier = excluded.risk_tier,
            provider = excluded.provider,
            underlying = excluded.underlying,
            roc_pct_2025_actual_minor = excluded.roc_pct_2025_actual_minor,
            roc_pct_2026_estimate_minor = excluded.roc_pct_2026_estimate_minor,
            roc_pct_2026_actual_minor = excluded.roc_pct_2026_actual_minor,
            roc_pct_2024_actual_minor = excluded.roc_pct_2024_actual_minor,
            roc_scale = excluded.roc_scale,
            div_type = excluded.div_type,
            needs_roc_research = excluded.needs_roc_research,
            notes = excluded.notes,
            is_active = excluded.is_active,
            lookthrough_json = excluded.lookthrough_json",
    )
    .bind(record.security_id.to_string())
    .bind(&record.payment_frequency)
    .bind(financial_domain::plan_review::normalize_risk_tier(&record.risk_tier))
    .bind(&record.provider)
    .bind(&record.underlying)
    .bind(record.roc_pct_2025_actual_minor)
    .bind(record.roc_pct_2026_estimate_minor)
    .bind(record.roc_pct_2026_actual_minor)
    .bind(record.roc_pct_2024_actual_minor)
    .bind(record.roc_scale.map(|s| s as i64))
    .bind(&record.div_type)
    .bind(if record.needs_roc_research { 1 } else { 0 })
    .bind(&record.notes)
    .bind(if record.is_active { 1 } else { 0 })
    .bind(record.lookthrough.to_json_string())
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    sqlx::query(
        "UPDATE plan_history SET planning_periods_per_year = ? WHERE security_id = ? AND effective_to IS NULL",
    )
    .bind(cadence.periods().unwrap_or(0) as i64)
    .bind(record.security_id.to_string())
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(record)
}

fn characteristic_from_row(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<PositionCharacteristicRecord, PlatformError> {
    let roc_scale: Option<i64> = row.try_get("roc_scale").map_err(|e| map_err(e.into()))?;
    Ok(PositionCharacteristicRecord {
        security_id: Uuid::parse_str(
            &row.try_get::<String, _>("security_id")
                .map_err(|e| map_err(e.into()))?,
        )
        .map_err(|e| PlatformError::new("parse_error", e.to_string()))?,
        payment_frequency: row.try_get("payment_frequency").map_err(|e| map_err(e.into()))?,
        risk_tier: row.try_get("risk_tier").map_err(|e| map_err(e.into()))?,
        provider: row.try_get("provider").map_err(|e| map_err(e.into()))?,
        underlying: row.try_get("underlying").map_err(|e| map_err(e.into()))?,
        roc_pct_2025_actual_minor: row
            .try_get("roc_pct_2025_actual_minor")
            .map_err(|e| map_err(e.into()))?,
        roc_pct_2026_estimate_minor: row
            .try_get("roc_pct_2026_estimate_minor")
            .map_err(|e| map_err(e.into()))?,
        roc_pct_2026_actual_minor: row
            .try_get("roc_pct_2026_actual_minor")
            .map_err(|e| map_err(e.into()))?,
        roc_pct_2024_actual_minor: row
            .try_get("roc_pct_2024_actual_minor")
            .map_err(|e| map_err(e.into()))?,
        roc_scale: roc_scale.map(|s| s as u8),
        div_type: row.try_get("div_type").map_err(|e| map_err(e.into()))?,
        needs_roc_research: row.try_get::<i64, _>("needs_roc_research").map_err(|e| map_err(e.into()))?
            != 0,
        notes: row.try_get("notes").map_err(|e| map_err(e.into()))?,
        is_active: row
            .try_get::<i64, _>("is_active")
            .map_err(|e| map_err(e.into()))?
            != 0,
        lookthrough: LookthroughResearch::from_json_str(
            &row.try_get::<String, _>("lookthrough_json")
                .unwrap_or_else(|_| "{}".into()),
        ),
    })
}

pub async fn position_characteristic_list(
    pool: &SqlitePool,
) -> Result<Vec<PositionCharacteristicRecord>, PlatformError> {
    let rows = sqlx::query(
        "SELECT security_id, payment_frequency, risk_tier, provider, underlying,
                roc_pct_2025_actual_minor, roc_pct_2026_estimate_minor, roc_pct_2026_actual_minor,
                roc_pct_2024_actual_minor, roc_scale, div_type, needs_roc_research, notes, is_active,
                lookthrough_json
         FROM position_characteristic ORDER BY security_id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    rows.iter().map(characteristic_from_row).collect()
}
