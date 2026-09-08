//! MAGI rule, facts, and projection (table-isolated from ledger).

use application_core::contracts::{DataCompleteness, DecisionState, MagiProjection};
use application_core::ports::platform::PlatformError;
use financial_domain::magi::{project, MagiCompleteness, MagiDecision, MagiInputs, MagiRule};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::StorageError;

fn map_err(err: StorageError) -> PlatformError {
    PlatformError::new("storage_error", err.to_string())
}

fn completeness_from(raw: &str) -> MagiCompleteness {
    match raw {
        "incomplete" => MagiCompleteness::Incomplete,
        "pending_review" => MagiCompleteness::PendingReview,
        _ => MagiCompleteness::Complete,
    }
}

fn group_thousands(n: i64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn usd_cents(minor: i64) -> String {
    let sign = if minor < 0 { "-" } else { "" };
    let n = minor.abs();
    format!("{sign}{}.{:02}", group_thousands(n / 100), n % 100)
}

fn completeness_to(c: MagiCompleteness) -> DataCompleteness {
    match c {
        MagiCompleteness::Complete => DataCompleteness::Complete,
        MagiCompleteness::Incomplete => DataCompleteness::Incomplete,
        MagiCompleteness::PendingReview => DataCompleteness::PendingReview,
    }
}

fn decision_to(d: MagiDecision) -> DecisionState {
    match d {
        MagiDecision::Safe => DecisionState::Safe,
        MagiDecision::Watch => DecisionState::Watch,
        MagiDecision::LikelyOver => DecisionState::LikelyOver,
        MagiDecision::Over => DecisionState::Over,
        MagiDecision::Indeterminate => DecisionState::Indeterminate,
    }
}

pub async fn rule_set(
    pool: &PgPool,
    threshold_minor: i64,
    safety_reserve_minor: i64,
    scale: u8,
) -> Result<(), PlatformError> {
    sqlx::query(
        "INSERT INTO magi_rule (singleton, threshold_minor, safety_reserve_minor, scale)
         VALUES (1, $1, $2, $3)
         ON CONFLICT (singleton) DO UPDATE SET
            threshold_minor = excluded.threshold_minor,
            safety_reserve_minor = excluded.safety_reserve_minor,
            scale = excluded.scale",
    )
    .bind(threshold_minor)
    .bind(safety_reserve_minor)
    .bind(scale as i32)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(())
}

pub async fn fact_record(
    pool: &PgPool,
    source_id: String,
    treatment: String,
    amount_minor: i64,
    scale: u8,
    category: String,
) -> Result<(), PlatformError> {
    sqlx::query(
        "INSERT INTO magi_fact (fact_id, source_id, treatment, amount_minor, scale, category)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (source_id) DO UPDATE SET
            treatment = excluded.treatment,
            amount_minor = excluded.amount_minor,
            scale = excluded.scale,
            category = excluded.category",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&source_id)
    .bind(&treatment)
    .bind(amount_minor)
    .bind(scale as i32)
    .bind(&category)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(())
}

pub async fn coverage_set(
    pool: &PgPool,
    completeness: String,
    remaining_minor: i64,
    withholding_minor: i64,
    form_total_minor: i64,
    warnings: Vec<String>,
) -> Result<(), PlatformError> {
    let warnings_json = serde_json::to_string(&warnings).unwrap_or_else(|_| "[]".into());
    sqlx::query(
        "INSERT INTO magi_coverage (
            singleton, completeness, remaining_minor, withholding_minor, form_total_minor, warnings_json
         ) VALUES (1, $1, $2, $3, $4, $5)
         ON CONFLICT (singleton) DO UPDATE SET
            completeness = excluded.completeness,
            remaining_minor = excluded.remaining_minor,
            withholding_minor = excluded.withholding_minor,
            form_total_minor = excluded.form_total_minor,
            warnings_json = excluded.warnings_json",
    )
    .bind(&completeness)
    .bind(remaining_minor)
    .bind(withholding_minor)
    .bind(form_total_minor)
    .bind(&warnings_json)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(())
}

pub async fn withholding_get(pool: &PgPool) -> Result<i64, PlatformError> {
    let n: i64 = sqlx::query_scalar(
        "SELECT COALESCE((SELECT withholding_minor FROM magi_coverage WHERE singleton = 1), 0)",
    )
    .fetch_one(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(n)
}

pub async fn adjustment_record(
    pool: &PgPool,
    adjustment_id: String,
    amount_minor: i64,
    scale: u8,
    status: String,
    reason: String,
) -> Result<(), PlatformError> {
    sqlx::query(
        "INSERT INTO magi_adjustment (adjustment_id, amount_minor, scale, status, reason)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (adjustment_id) DO UPDATE SET
            amount_minor = excluded.amount_minor,
            scale = excluded.scale,
            status = excluded.status,
            reason = excluded.reason",
    )
    .bind(&adjustment_id)
    .bind(amount_minor)
    .bind(scale as i32)
    .bind(&status)
    .bind(&reason)
    .execute(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    Ok(())
}

pub async fn projection(pool: &PgPool) -> Result<MagiProjection, PlatformError> {
    let rule_row = sqlx::query(
        "SELECT threshold_minor, safety_reserve_minor, scale FROM magi_rule WHERE singleton = 1",
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| map_err(e.into()))?
    .ok_or_else(|| PlatformError::new("not_found", "MAGI rule is not set"))?;
    let rule = MagiRule {
        threshold_minor: rule_row.try_get("threshold_minor").map_err(|e| map_err(e.into()))?,
        safety_reserve_minor: rule_row
            .try_get("safety_reserve_minor")
            .map_err(|e| map_err(e.into()))?,
        scale: rule_row.try_get::<i32, _>("scale").map_err(|e| map_err(e.into()))? as u8,
    };
    let facts = sqlx::query("SELECT source_id, treatment, amount_minor FROM magi_fact")
        .fetch_all(pool)
        .await
        .map_err(|e| map_err(e.into()))?;
    let mut actual = 0i64;
    let mut uncertain = 0i64;
    let mut include_trace = Vec::new();
    for row in &facts {
        let source: String = row.try_get("source_id").map_err(|e| map_err(e.into()))?;
        let treatment: String = row.try_get("treatment").map_err(|e| map_err(e.into()))?;
        let amount: i64 = row.try_get("amount_minor").map_err(|e| map_err(e.into()))?;
        match treatment.as_str() {
            "include" => {
                actual += amount;
                include_trace.push(source);
            }
            "uncertain" => uncertain += amount,
            _ => {}
        }
    }
    let cov = sqlx::query(
        "SELECT completeness, remaining_minor, form_total_minor, warnings_json
         FROM magi_coverage WHERE singleton = 1",
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| map_err(e.into()))?;
    let (completeness, remaining, form_total, mut warnings) = if let Some(row) = cov {
        let raw: String = row.try_get("completeness").map_err(|e| map_err(e.into()))?;
        let remaining: i64 = row.try_get("remaining_minor").map_err(|e| map_err(e.into()))?;
        let form_total: i64 = row.try_get("form_total_minor").map_err(|e| map_err(e.into()))?;
        let warnings_json: String = row.try_get("warnings_json").map_err(|e| map_err(e.into()))?;
        let warnings: Vec<String> = serde_json::from_str(&warnings_json).unwrap_or_default();
        (completeness_from(&raw), remaining, form_total, warnings)
    } else {
        (MagiCompleteness::Complete, 0, 0, Vec::new())
    };
    let adj_rows = sqlx::query("SELECT adjustment_id, amount_minor, status FROM magi_adjustment")
        .fetch_all(pool)
        .await
        .map_err(|e| map_err(e.into()))?;
    let mut adjusted_form = form_total;
    let mut unapproved = Vec::new();
    for row in &adj_rows {
        let id: String = row.try_get("adjustment_id").map_err(|e| map_err(e.into()))?;
        let amount: i64 = row.try_get("amount_minor").map_err(|e| map_err(e.into()))?;
        let status: String = row.try_get("status").map_err(|e| map_err(e.into()))?;
        if status.eq_ignore_ascii_case("approved") {
            adjusted_form += amount;
        } else {
            unapproved.push(id);
        }
    }
    if form_total != 0 && actual != adjusted_form {
        let diff = actual - adjusted_form;
        if unapproved.is_empty() {
            warnings.push(format!(
                "Posted MAGI ${} vs adjusted forms ${}; difference ${} remains visible",
                usd_cents(actual),
                usd_cents(adjusted_form),
                usd_cents(diff.abs())
            ));
        } else {
            warnings.push(format!(
                "Posted MAGI ${} vs forms ${}; difference ${} held visible until {} is owner-approved",
                usd_cents(actual),
                usd_cents(form_total),
                usd_cents((actual - form_total).abs()),
                unapproved.join(", ")
            ));
        }
    }
    let snap = project(
        rule,
        MagiInputs {
            actual_included_ytd_minor: actual,
            known_remaining_minor: remaining,
            uncertain_amount_minor: uncertain,
            completeness,
            include_trace,
        },
        &warnings,
    );
    Ok(MagiProjection {
        applicable_threshold: snap.threshold,
        actual_included_ytd: snap.actual_included_ytd,
        known_remaining: snap.known_remaining,
        base_forecast: snap.base_forecast,
        conservative_forecast: snap.conservative_forecast,
        uncertain_amount: snap.uncertain_amount,
        raw_headroom: snap.raw_headroom,
        protected_headroom: snap.protected_headroom,
        data_completeness: completeness_to(snap.completeness),
        decision_state: decision_to(snap.decision),
        warnings: snap.warnings,
        calculation_trace: snap.trace,
    })
}
