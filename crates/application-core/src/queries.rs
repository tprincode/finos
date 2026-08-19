//! Query/command dispatch owned by application-core (no Tauri, no database).

use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::contracts::{
    AllocationGetBody, CommandRequest, CommandResult, DashboardBody, ImportCandidate, QueryRequest,
    QueryResult, RoiBody, TrendPoint, TrendsBody, UpdaterCheckBody, FINANCE_CLIENT_CONTRACT_VERSION,
};
use crate::ports::canonical::Canonical;
use crate::ports::platform::{Platform, PlatformError};

const HEALTH_QUERY: &str = "HealthGet";

const ORDINARY_WRITES: &[&str] = &[
    "ConfigSet",
    "AccountRegister",
    "AccountUpdate",
    "SnapshotImport",
    "SecurityRegister",
    "EvidenceStore",
    "ImportStage",
    "ImportValidate",
    "ImportApprove",
    "ImportPost",
    "ActivityPost",
    "ActivityCorrect",
    "ExceptionAcknowledge",
    "DividendDeclare",
    "DividendActualRecord",
    "IncomePlanUpdate",
    "LotOpen",
    "LotAssign",
    "MagiRuleSet",
    "MagiFactRecord",
    "MagiCoverageSet",
    "MagiAdjustmentRecord",
    "PlanApprove",
    "AllocationTargetSet",
    "CartItemAdd",
    "CartItemRemove",
    "BacktestRun",
    "ClassificationReviewRecord",
    "AiAnalyze",
];

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestBody {
    snapshot_id: Option<Uuid>,
    action: Option<String>,
    device_name: Option<String>,
}

fn parse_body(body_json: Option<&str>) -> RequestBody {
    body_json
        .and_then(|raw| serde_json::from_str(raw).ok())
        .unwrap_or_default()
}

fn parse_json(body_json: Option<&str>) -> Value {
    body_json
        .and_then(|raw| serde_json::from_str(raw).ok())
        .unwrap_or(Value::Object(serde_json::Map::new()))
}

fn jstr(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
}

fn juuid(v: &Value, key: &str) -> Option<Uuid> {
    jstr(v, key).and_then(|s| Uuid::parse_str(&s).ok())
}

fn ji64(v: &Value, key: &str) -> Option<i64> {
    v.get(key).and_then(|x| {
        x.as_i64()
            .or_else(|| x.as_u64().map(|n| n as i64))
            .or_else(|| x.as_str().and_then(|s| s.parse().ok()))
    })
}

fn ju8(v: &Value, key: &str, default: u8) -> u8 {
    v.get(key)
        .and_then(|x| x.as_u64())
        .map(|n| n as u8)
        .unwrap_or(default)
}

fn jstrings(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|i| i.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn content_bytes(v: &Value) -> Vec<u8> {
    if let Some(s) = jstr(v, "content") {
        return s.into_bytes();
    }
    Vec::new()
}

fn candidates_from(v: &Value) -> Vec<ImportCandidate> {
    let Some(arr) = v.get("candidates").and_then(|x| x.as_array()) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|item| {
            Some(ImportCandidate {
                account_name: jstr(item, "accountName")?,
                symbol: jstr(item, "symbol"),
                activity_type: jstr(item, "activityType").unwrap_or_else(|| "unknown".into()),
                amount_minor: ji64(item, "amountMinor"),
                scale: ju8(item, "scale", 2),
                occurred_on: jstr(item, "occurredOn").unwrap_or_default(),
            })
        })
        .collect()
}

fn health_body() -> String {
    format!("{{\"status\":\"ok\",\"contractVersion\":\"{FINANCE_CLIENT_CONTRACT_VERSION}\"}}")
}

fn updater_check_body() -> UpdaterCheckBody {
    let check = financial_domain::updater::check_for_update();
    UpdaterCheckBody {
        applied: check.applied,
        posted: check.posted,
        status: check.status.to_string(),
    }
}

fn query_ok(request: &QueryRequest, body_json: String) -> QueryResult {
    QueryResult {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        query_name: request.query_name.clone(),
        correlation_id: request.correlation_id,
        ok: true,
        error_code: None,
        body_json: Some(body_json),
    }
}

fn query_err(request: &QueryRequest, code: &str) -> QueryResult {
    QueryResult {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        query_name: request.query_name.clone(),
        correlation_id: request.correlation_id,
        ok: false,
        error_code: Some(code.to_string()),
        body_json: None,
    }
}

fn command_ok(request: &CommandRequest, body_json: String) -> CommandResult {
    CommandResult {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        command_name: request.command_name.clone(),
        correlation_id: request.correlation_id,
        ok: true,
        error_code: None,
        body_json: Some(body_json),
    }
}

fn command_err(request: &CommandRequest, code: &str) -> CommandResult {
    CommandResult {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        command_name: request.command_name.clone(),
        correlation_id: request.correlation_id,
        ok: false,
        error_code: Some(code.to_string()),
        body_json: None,
    }
}

fn to_json<T: serde::Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| e.to_string())
}

async fn income_plan_view(
    canonical: &dyn Canonical,
) -> Result<crate::contracts::IncomePlanBody, PlatformError> {
    let mut plan = canonical.income_plan_get().await?;
    let div = canonical.dividend_get().await?;
    plan.actual_minor = div.actual_total_minor;
    plan.scale = div.scale;
    Ok(plan)
}

async fn allocation_view(
    canonical: &dyn Canonical,
) -> Result<AllocationGetBody, PlatformError> {
    let mut alloc = canonical.allocation_get().await?;
    let pos = canonical.position_details_get().await?;
    alloc.open_performance_minor = pos.open_performance_minor;
    alloc.open_tax_minor = pos.open_tax_minor;
    alloc.scale = pos.scale;
    Ok(alloc)
}

async fn roi_view(canonical: &dyn Canonical, request: &QueryRequest) -> QueryResult {
    match (canonical.roi_get().await, canonical.dividend_get().await) {
        (Ok(mut roi), Ok(div)) => {
            roi.dividend_actual_minor = div.actual_total_minor;
            map_q(request, Ok::<RoiBody, PlatformError>(roi))
        }
        (Err(err), _) | (_, Err(err)) => query_err(request, &err.code),
    }
}

fn map_q<T: serde::Serialize>(request: &QueryRequest, result: Result<T, PlatformError>) -> QueryResult {
    match result {
        Ok(body) => match to_json(&body) {
            Ok(json) => query_ok(request, json),
            Err(_) => query_err(request, "serialize_failed"),
        },
        Err(err) => query_err(request, &err.code),
    }
}

fn map_c<T: serde::Serialize>(
    request: &CommandRequest,
    result: Result<T, PlatformError>,
) -> CommandResult {
    match result {
        Ok(body) => match to_json(&body) {
            Ok(json) => command_ok(request, json),
            Err(_) => command_err(request, "serialize_failed"),
        },
        Err(err) => command_err(request, &err.code),
    }
}

/// Stateless dispatch used by library tests (HealthGet / UpdaterCheckGet; no SQLite/Tauri).
pub fn execute_query(request: QueryRequest) -> QueryResult {
    match request.query_name.as_str() {
        HEALTH_QUERY => query_ok(&request, health_body()),
        "UpdaterCheckGet" => match to_json(&updater_check_body()) {
            Ok(json) => query_ok(&request, json),
            Err(_) => query_err(&request, "serialize_failed"),
        },
        _ => query_err(&request, "unknown_query"),
    }
}

/// Command dispatch without a platform adapter.
pub fn execute_command(request: CommandRequest) -> CommandResult {
    command_err(&request, "not_implemented")
}

/// Query dispatch against local adapters (desktop host).
pub async fn execute_query_on(
    platform: &dyn Platform,
    canonical: &dyn Canonical,
    request: QueryRequest,
) -> QueryResult {
    let json = parse_json(request.body_json.as_deref());
    match request.query_name.as_str() {
        HEALTH_QUERY => query_ok(&request, health_body()),
        "UpdaterCheckGet" => map_q(&request, Ok::<UpdaterCheckBody, PlatformError>(updater_check_body())),
        "ConfigGet" => map_q(&request, platform.config_get().await),
        "SnapshotHeadGet" => map_q(&request, platform.snapshot_head_get().await),
        "HandoffStatusGet" => map_q(&request, platform.handoff_status_get().await),
        "AccountGet" => match juuid(&json, "accountId") {
            Some(id) => map_q(&request, canonical.account_get(id).await),
            None => query_err(&request, "missing_account_id"),
        },
        "AccountList" => map_q(&request, canonical.account_list().await),
        "SecurityGet" => match juuid(&json, "securityId") {
            Some(id) => map_q(&request, canonical.security_get(id).await),
            None => query_err(&request, "missing_security_id"),
        },
        "SecurityList" => map_q(&request, canonical.security_list().await),
        "EvidenceGet" => match juuid(&json, "evidenceId") {
            Some(id) => map_q(&request, canonical.evidence_get(id).await),
            None => query_err(&request, "missing_evidence_id"),
        },
        "ImportBatchGet" => match juuid(&json, "batchId") {
            Some(id) => map_q(&request, canonical.import_batch_get(id).await),
            None => query_err(&request, "missing_batch_id"),
        },
        "ActivityGet" => match juuid(&json, "activityId") {
            Some(id) => map_q(&request, canonical.activity_get(id).await),
            None => query_err(&request, "missing_activity_id"),
        },
        "ActivityList" => map_q(&request, canonical.activity_list().await),
        "AuditList" => map_q(&request, canonical.audit_list().await),
        "ExceptionList" => map_q(&request, canonical.exception_list().await),
        "CanonicalWeekGet" => {
            let as_of = jstr(&json, "asOfDate").unwrap_or_else(|| "2026-08-18".into());
            map_q(&request, canonical.canonical_week_get(as_of).await)
        }
        "ReconcileCountsGet" => map_q(&request, canonical.reconcile_counts().await),
        "DividendGet" => map_q(&request, canonical.dividend_get().await),
        "IncomePlanGet" => map_q(&request, income_plan_view(canonical).await),
        "DashboardGet" => {
            match (
                canonical.dividend_get().await,
                income_plan_view(canonical).await,
            ) {
                (Ok(div), Ok(plan)) => map_q(
                    &request,
                    Ok(DashboardBody {
                        actual_dividend_minor: div.actual_total_minor,
                        planned_income_minor: plan.planned_minor,
                        scale: div.scale,
                    }),
                ),
                (Err(err), _) | (_, Err(err)) => query_err(&request, &err.code),
            }
        }
        "TrendsGet" => match canonical.dividend_get().await {
            Ok(div) => map_q(
                &request,
                Ok(TrendsBody {
                    points: div
                        .actuals
                        .into_iter()
                        .map(|a| TrendPoint {
                            occurred_on: a.occurred_on,
                            amount_minor: a.amount_minor,
                        })
                        .collect(),
                    total_minor: div.actual_total_minor,
                    scale: div.scale,
                }),
            ),
            Err(err) => query_err(&request, &err.code),
        },
        "LotGet" => match juuid(&json, "lotId") {
            Some(id) => map_q(&request, canonical.lot_get(id).await),
            None => query_err(&request, "missing_lot_id"),
        },
        "BasisGet" => map_q(&request, canonical.basis_get().await),
        "RoiGet" => roi_view(canonical, &request).await,
        "LotRecommendGet" => match (juuid(&json, "accountId"), juuid(&json, "securityId")) {
            (Some(account_id), Some(security_id)) => {
                map_q(&request, canonical.lot_recommend(account_id, security_id).await)
            }
            _ => query_err(&request, "missing_account_or_security"),
        },
        "BrokerLotReconcileGet" => map_q(&request, canonical.broker_lot_reconcile().await),
        "PositionDetailsGet" => map_q(&request, canonical.position_details_get().await),
        "TaxProjectionGet" => map_q(&request, canonical.tax_projection_get().await),
        "MagiProjectionGet" => map_q(&request, canonical.magi_projection_get().await),
        "MagiTaxPaymentGet" => map_q(&request, canonical.magi_tax_payment_get().await),
        "PlanGet" => map_q(&request, canonical.plan_get().await),
        "BurndownGet" => map_q(&request, canonical.burndown_get().await),
        "AllocationGet" => map_q(&request, allocation_view(canonical).await),
        "CartGet" => map_q(&request, canonical.cart_get().await),
        "BacktestGet" => map_q(&request, canonical.backtest_get().await),
        "ClassificationReviewGet" => map_q(&request, canonical.classification_review_get().await),
        "AiRunGet" => match juuid(&json, "runId") {
            Some(run_id) => map_q(&request, canonical.ai_run_get(run_id).await),
            None => query_err(&request, "missing_run_id"),
        },
        "AnalysisRunList" => map_q(&request, canonical.analysis_run_list().await),
        _ => query_err(&request, "unknown_query"),
    }
}

/// Command dispatch against local adapters (desktop host).
pub async fn execute_command_on(
    platform: &dyn Platform,
    canonical: &dyn Canonical,
    request: CommandRequest,
) -> CommandResult {
    if ORDINARY_WRITES.contains(&request.command_name.as_str()) {
        match platform.writes_allowed().await {
            Ok(false) => return command_err(&request, "writes_blocked"),
            Ok(true) => {}
            Err(err) => return command_err(&request, &err.code),
        }
    }
    let body = parse_body(request.body_json.as_deref());
    let json = parse_json(request.body_json.as_deref());
    match request.command_name.as_str() {
        "ConfigSet" => map_c(&request, platform.config_set(body.device_name).await),
        "SnapshotCreate" => map_c(&request, platform.snapshot_create().await),
        "SnapshotRestore" => map_c(&request, platform.snapshot_restore(body.snapshot_id).await),
        "HandoffResolve" => {
            let action = body.action.as_deref().unwrap_or("restore");
            map_c(
                &request,
                platform.handoff_resolve(action, body.snapshot_id).await,
            )
        }
        "AccountRegister" => {
            let name = jstr(&json, "name").unwrap_or_default();
            let kind = jstr(&json, "kind").unwrap_or_else(|| "unknown".into());
            map_c(&request, canonical.account_register(name, kind).await)
        }
        "AccountUpdate" => match juuid(&json, "accountId") {
            Some(id) => map_c(
                &request,
                canonical
                    .account_update(
                        id,
                        jstr(&json, "name"),
                        jstr(&json, "kind"),
                        request.expected_version,
                    )
                    .await,
            ),
            None => command_err(&request, "missing_account_id"),
        },
        "SnapshotImport" => {
            let path = jstr(&json, "sqlitePath").unwrap_or_default();
            map_c(&request, canonical.snapshot_import_sqlite(path).await)
        }
        "SecurityRegister" => {
            let symbol = jstr(&json, "symbol").unwrap_or_default();
            let name = jstr(&json, "name").unwrap_or_else(|| symbol.clone());
            map_c(&request, canonical.security_register(symbol, name).await)
        }
        "EvidenceStore" => {
            let filename = jstr(&json, "filename").unwrap_or_else(|| "evidence.bin".into());
            map_c(
                &request,
                canonical.evidence_store(filename, content_bytes(&json)).await,
            )
        }
        "ImportStage" => {
            let source_id = jstr(&json, "sourceId").unwrap_or_default();
            let filename = jstr(&json, "filename").unwrap_or_else(|| "import.txt".into());
            map_c(
                &request,
                canonical
                    .import_stage(
                        source_id,
                        filename,
                        content_bytes(&json),
                        candidates_from(&json),
                        jstr(&json, "accountName"),
                    )
                    .await,
            )
        }
        "ImportValidate" => match juuid(&json, "batchId") {
            Some(id) => map_c(&request, canonical.import_validate(id).await),
            None => command_err(&request, "missing_batch_id"),
        },
        "ImportApprove" => match juuid(&json, "batchId") {
            Some(id) => map_c(&request, canonical.import_approve(id).await),
            None => command_err(&request, "missing_batch_id"),
        },
        "ImportPost" => match juuid(&json, "batchId") {
            Some(id) => map_c(&request, canonical.import_post(id).await),
            None => command_err(&request, "missing_batch_id"),
        },
        "ActivityPost" => match juuid(&json, "accountId") {
            Some(account_id) => map_c(
                &request,
                canonical
                    .activity_post(
                        account_id,
                        juuid(&json, "securityId"),
                        jstr(&json, "activityType").unwrap_or_else(|| "deposit".into()),
                        ji64(&json, "amountMinor"),
                        ju8(&json, "scale", 2),
                        jstr(&json, "occurredOn").unwrap_or_default(),
                        juuid(&json, "correctsActivityId"),
                        juuid(&json, "importBatchId"),
                        jstr(&json, "idempotencyKey"),
                    )
                    .await,
            ),
            None => command_err(&request, "missing_account_id"),
        },
        "ActivityCorrect" => match juuid(&json, "activityId") {
            Some(id) => map_c(
                &request,
                canonical
                    .activity_correct(
                        id,
                        ji64(&json, "amountMinor"),
                        ju8(&json, "scale", 2),
                        jstr(&json, "occurredOn").unwrap_or_default(),
                    )
                    .await,
            ),
            None => command_err(&request, "missing_activity_id"),
        },
        "ExceptionAcknowledge" => match juuid(&json, "exceptionId") {
            Some(id) => map_c(&request, canonical.exception_acknowledge(id).await),
            None => command_err(&request, "missing_exception_id"),
        },
        "DividendDeclare" => map_c(
            &request,
            canonical
                .dividend_declare(
                    jstr(&json, "securitySymbol").unwrap_or_else(|| "CASH".into()),
                    jstr(&json, "declaredOn").unwrap_or_default(),
                    ji64(&json, "amountMinor").unwrap_or(0),
                    ju8(&json, "scale", 2),
                )
                .await,
        ),
        "DividendActualRecord" => match juuid(&json, "accountId") {
            Some(account_id) => map_c(
                &request,
                canonical
                    .dividend_actual_record(
                        account_id,
                        juuid(&json, "securityId"),
                        jstr(&json, "occurredOn").unwrap_or_default(),
                        ji64(&json, "amountMinor"),
                        ju8(&json, "scale", 2),
                        jstr(&json, "idempotencyKey"),
                    )
                    .await,
            ),
            None => command_err(&request, "missing_account_id"),
        },
        "IncomePlanUpdate" => {
            let planned = ji64(&json, "plannedMinor").unwrap_or(0);
            let scale = ju8(&json, "scale", 2);
            match canonical.income_plan_update(planned, scale).await {
                Ok(_) => map_c(&request, income_plan_view(canonical).await),
                Err(err) => command_err(&request, &err.code),
            }
        }
        "LotOpen" => match (juuid(&json, "accountId"), juuid(&json, "securityId")) {
            (Some(account_id), Some(security_id)) => map_c(
                &request,
                canonical
                    .lot_open(
                        account_id,
                        security_id,
                        jstr(&json, "openedOn").unwrap_or_default(),
                        jstr(&json, "origin").unwrap_or_else(|| "purchase".into()),
                        ji64(&json, "quantityMinor").unwrap_or(0),
                        ju8(&json, "quantityScale", 0),
                        ji64(&json, "performanceBasisMinor").unwrap_or(0),
                        ji64(&json, "taxBasisMinor").unwrap_or(0),
                        ju8(&json, "scale", 2),
                        juuid(&json, "openingActivityId"),
                    )
                    .await,
            ),
            _ => command_err(&request, "missing_account_or_security"),
        },
        "LotAssign" => match (juuid(&json, "lotId"), juuid(&json, "activityId")) {
            (Some(lot_id), Some(activity_id)) => map_c(
                &request,
                canonical
                    .lot_assign(
                        lot_id,
                        activity_id,
                        ji64(&json, "quantityMinor").unwrap_or(0),
                        ju8(&json, "quantityScale", 0),
                    )
                    .await,
            ),
            _ => command_err(&request, "missing_lot_or_activity"),
        },
        "MagiRuleSet" => map_c(
            &request,
            canonical
                .magi_rule_set(
                    ji64(&json, "thresholdMinor").unwrap_or(0),
                    ji64(&json, "safetyReserveMinor").unwrap_or(0),
                    ju8(&json, "scale", 2),
                )
                .await,
        ),
        "MagiFactRecord" => map_c(
            &request,
            canonical
                .magi_fact_record(
                    jstr(&json, "sourceId").unwrap_or_default(),
                    jstr(&json, "treatment").unwrap_or_else(|| "include".into()),
                    ji64(&json, "amountMinor").unwrap_or(0),
                    ju8(&json, "scale", 2),
                    jstr(&json, "category").unwrap_or_default(),
                )
                .await,
        ),
        "MagiCoverageSet" => map_c(
            &request,
            canonical
                .magi_coverage_set(
                    jstr(&json, "completeness").unwrap_or_else(|| "complete".into()),
                    ji64(&json, "remainingMinor").unwrap_or(0),
                    ji64(&json, "withholdingMinor").unwrap_or(0),
                    ji64(&json, "formTotalMinor").unwrap_or(0),
                    jstrings(&json, "warnings"),
                )
                .await,
        ),
        "MagiAdjustmentRecord" => map_c(
            &request,
            canonical
                .magi_adjustment_record(
                    jstr(&json, "adjustmentId").unwrap_or_default(),
                    ji64(&json, "amountMinor").unwrap_or(0),
                    ju8(&json, "scale", 2),
                    jstr(&json, "status").unwrap_or_else(|| "proposed".into()),
                    jstr(&json, "reason").unwrap_or_default(),
                )
                .await,
        ),
        "PlanApprove" => map_c(
            &request,
            canonical
                .plan_approve(
                    ji64(&json, "remainingMinor").unwrap_or(0),
                    ju8(&json, "scale", 2),
                    jstr(&json, "approvedOn").unwrap_or_else(|| "2026-08-18".into()),
                )
                .await,
        ),
        "AllocationTargetSet" => match canonical
            .allocation_target_set(
                jstr(&json, "name").unwrap_or_default(),
                ji64(&json, "targetMinor").unwrap_or(0),
                ju8(&json, "scale", 2),
            )
            .await
        {
            Ok(_) => map_c(&request, allocation_view(canonical).await),
            Err(err) => command_err(&request, &err.code),
        },
        "CartItemAdd" => map_c(
            &request,
            canonical
                .cart_item_add(
                    jstr(&json, "symbol").unwrap_or_default(),
                    ji64(&json, "quantityMinor").unwrap_or(0),
                    ju8(&json, "quantityScale", 2),
                )
                .await,
        ),
        "CartItemRemove" => match juuid(&json, "itemId") {
            Some(item_id) => map_c(&request, canonical.cart_item_remove(item_id).await),
            None => command_err(&request, "missing_item_id"),
        },
        "BacktestRun" => map_c(
            &request,
            canonical
                .backtest_run(
                    jstr(&json, "scenario").unwrap_or_default(),
                    ji64(&json, "hypotheticalPnlMinor").unwrap_or(0),
                    ju8(&json, "scale", 2),
                    jstr(&json, "completedAt").unwrap_or_else(|| "2026-08-18".into()),
                )
                .await,
        ),
        "ClassificationReviewRecord" => map_c(
            &request,
            canonical
                .classification_review_record(
                    jstr(&json, "factKey").unwrap_or_default(),
                    jstr(&json, "classification").unwrap_or_default(),
                    jstr(&json, "status").unwrap_or_else(|| "reviewed".into()),
                )
                .await,
        ),
        "AiAnalyze" => map_c(
            &request,
            canonical
                .ai_analyze(jstr(&json, "prompt").unwrap_or_default())
                .await,
        ),
        _ => command_err(&request, "not_implemented"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::contracts::{
        DeviceConfig, HandoffDecision, HandoffStatusBody, SnapshotIdentity, APP_VERSION,
        CALCULATION_VERSION, SCHEMA_VERSION,
    };
    use crate::ports::canonical::UnimplementedCanonical;
    use crate::ports::platform::PlatformError;

    fn query(name: &str) -> QueryRequest {
        QueryRequest {
            contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
            query_name: name.to_string(),
            correlation_id: Uuid::nil(),
            body_json: None,
        }
    }

    fn command(name: &str, body_json: Option<String>) -> CommandRequest {
        CommandRequest {
            contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
            command_name: name.to_string(),
            correlation_id: Uuid::nil(),
            body_json,
            expected_version: None,
        }
    }

    struct MockPlatform {
        writes: bool,
        status: HandoffStatusBody,
        config: DeviceConfig,
    }

    #[async_trait]
    impl Platform for MockPlatform {
        async fn config_get(&self) -> Result<DeviceConfig, PlatformError> {
            Ok(self.config.clone())
        }
        async fn config_set(
            &self,
            _device_name: Option<String>,
        ) -> Result<DeviceConfig, PlatformError> {
            Ok(self.config.clone())
        }
        async fn snapshot_head_get(&self) -> Result<Option<SnapshotIdentity>, PlatformError> {
            Ok(None)
        }
        async fn handoff_status_get(&self) -> Result<HandoffStatusBody, PlatformError> {
            Ok(self.status.clone())
        }
        async fn snapshot_create(&self) -> Result<SnapshotIdentity, PlatformError> {
            Err(PlatformError::new("not_used", "not used"))
        }
        async fn snapshot_restore(
            &self,
            _snapshot_id: Option<Uuid>,
        ) -> Result<SnapshotIdentity, PlatformError> {
            Err(PlatformError::new("not_used", "not used"))
        }
        async fn handoff_resolve(
            &self,
            _action: &str,
            _snapshot_id: Option<Uuid>,
        ) -> Result<HandoffStatusBody, PlatformError> {
            Ok(self.status.clone())
        }
        async fn writes_allowed(&self) -> Result<bool, PlatformError> {
            Ok(self.writes)
        }
    }

    fn mock_blocked() -> MockPlatform {
        MockPlatform {
            writes: false,
            status: HandoffStatusBody {
                decision: HandoffDecision::BlockUntilRestore,
                writes_allowed: false,
                message: "blocked".to_string(),
                local_head: None,
                published_head: None,
            },
            config: DeviceConfig {
                device_id: Uuid::nil(),
                device_name: "test".to_string(),
                database_id: Uuid::nil(),
                schema_version: SCHEMA_VERSION.to_string(),
                calculation_version: CALCULATION_VERSION.to_string(),
                app_version: APP_VERSION.to_string(),
            },
        }
    }

    #[test]
    fn health_get_returns_ok_body_without_desktop_or_db() {
        let result = execute_query(query("HealthGet"));
        assert!(result.ok);
        assert_eq!(result.error_code, None);
        let body = result.body_json.expect("health body");
        assert!(body.contains("\"status\":\"ok\""));
        assert!(body.contains(FINANCE_CLIENT_CONTRACT_VERSION));
    }

    #[test]
    fn unknown_query_fails() {
        let result = execute_query(query("NotARealQuery"));
        assert!(!result.ok);
        assert_eq!(result.error_code.as_deref(), Some("unknown_query"));
        assert!(result.body_json.is_none());
    }

    #[tokio::test]
    async fn handoff_status_get_uses_platform_without_tauri() {
        let platform = mock_blocked();
        let canonical = UnimplementedCanonical;
        let result = execute_query_on(&platform, &canonical, query("HandoffStatusGet")).await;
        assert!(result.ok);
        let body = result.body_json.expect("handoff body");
        assert!(body.contains("block_until_restore"));
        assert!(body.contains("\"writesAllowed\":false"));
    }

    #[tokio::test]
    async fn config_set_blocked_when_handoff_blocks_writes() {
        let platform = mock_blocked();
        let canonical = UnimplementedCanonical;
        let result = execute_command_on(&platform, &canonical, command("ConfigSet", None)).await;
        assert!(!result.ok);
        assert_eq!(result.error_code.as_deref(), Some("writes_blocked"));
    }

    #[tokio::test]
    async fn import_post_blocked_when_handoff_blocks_writes() {
        let platform = mock_blocked();
        let canonical = UnimplementedCanonical;
        let result = execute_command_on(
            &platform,
            &canonical,
            command("ImportPost", Some("{\"batchId\":\"00000000-0000-0000-0000-000000000001\"}".into())),
        )
        .await;
        assert!(!result.ok);
        assert_eq!(result.error_code.as_deref(), Some("writes_blocked"));
    }
}
