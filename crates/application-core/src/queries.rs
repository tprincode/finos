//! Query/command dispatch owned by application-core (no Tauri, no database).

use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::contracts::{
    AllocationGetBody, CalculatorGetBody, CalculatorRowBody, CommandRequest, CommandResult,
    DashboardBody, DashboardBurndownBody, DashboardBurndownLineBody, HoldingsGetBody,
    HoldingsLotBody, HouseholdSummaryBody, ImportCandidate, IncomePlanDrillBody,
    IncomePlanLineBody, IncomePlanWeekBody, ProductionSeedDocument, QueryRequest, QueryResult,
    RoiBody, TrendPoint, TrendsBody, UpdaterCheckBody, FINANCE_CLIENT_CONTRACT_VERSION,
    PlanReviewBody, PositionCharacteristicRecord, RetrievalTemplateRecord, InvestmentGetBody,
    InvestmentLotBody,
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
    "SecurityUpdate",
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
    "ProductionSeedLoad",
    "DistributionCharacterize",
    "PlanHistoryConfirm",
    "IssuerDeclarationRecord",
    "PriceQuoteRecord",
    "ManualPriceOverride",
    "RetrievalTemplateSet",
    "PositionCharacteristicUpsert",
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

fn json_has(v: &Value, key: &str) -> bool {
    v.as_object().is_some_and(|o| o.contains_key(key))
}

fn jstr_keep(v: &Value, key: &str, keep: String) -> String {
    match v.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) => String::new(),
        _ => keep,
    }
}

fn ji64_keep(v: &Value, key: &str, keep: Option<i64>) -> Option<i64> {
    if !json_has(v, key) {
        return keep;
    }
    ji64(v, key)
}

fn ju8_opt_keep(v: &Value, key: &str, keep: Option<u8>) -> Option<u8> {
    if !json_has(v, key) {
        return keep;
    }
    v.get(key).and_then(|x| x.as_u64()).map(|n| n as u8)
}

fn jbool_keep(v: &Value, key: &str, keep: bool) -> bool {
    if !json_has(v, key) {
        return keep;
    }
    jbool(v, key, keep)
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

fn jbool(v: &Value, key: &str, default: bool) -> bool {
    v.get(key)
        .and_then(|x| x.as_bool().or_else(|| x.as_i64().map(|n| n != 0)))
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

async fn income_plan_week_view(
    canonical: &dyn Canonical,
    as_of_date: String,
) -> Result<IncomePlanWeekBody, PlatformError> {
    let accounts = canonical.account_list().await?;
    let securities = canonical.security_list().await?;
    let dividend = canonical.dividend_get().await?;
    let latest_actual_on = dividend
        .actuals
        .iter()
        .map(|a| a.occurred_on.clone())
        .max();
    let yield_count = dividend.actuals.len() as u64;
    let as_of = if as_of_date.trim().is_empty() {
        latest_actual_on
            .clone()
            .unwrap_or_else(|| "2026-08-18".into())
    } else {
        as_of_date
    };
    let week = canonical.canonical_week_get(as_of).await?;
    let mut actuals: std::collections::HashMap<&'static str, i64> = financial_domain::income_plan::CONTROL_ACCOUNTS
        .iter()
        .map(|name| (*name, 0i64))
        .collect();
    let mut drilldown = Vec::new();
    for actual in &dividend.actuals {
        if !financial_domain::income_plan::occurred_in_week(
            &actual.occurred_on,
            &week.start,
            &week.end,
        ) {
            continue;
        }
        let account = accounts
            .iter()
            .find(|a| a.account_id == actual.account_id);
        let Some(account) = account else {
            continue;
        };
        let Some(control) = financial_domain::income_plan::map_control_account(&account.name) else {
            continue;
        };
        *actuals.entry(control).or_insert(0) += actual.amount_minor;
        let symbol = actual
            .security_id
            .and_then(|id| {
                securities
                    .iter()
                    .find(|s| s.security_id == id)
                    .map(|s| s.symbol.clone())
            })
            .unwrap_or_else(|| "—".into());
        drilldown.push(IncomePlanDrillBody {
            account_name: control.to_string(),
            symbol,
            occurred_on: actual.occurred_on.clone(),
            amount_minor: actual.amount_minor,
            scale: actual.scale,
        });
    }
    let plans = canonical.plan_history_list().await?;
    let characteristics = canonical.position_characteristic_list().await?;
    let basis = canonical.basis_get().await?;
    let mut last_actual: std::collections::HashMap<uuid::Uuid, String> =
        std::collections::HashMap::new();
    for actual in &dividend.actuals {
        if let Some(security_id) = actual.security_id {
            last_actual
                .entry(security_id)
                .and_modify(|on| {
                    if actual.occurred_on > *on {
                        *on = actual.occurred_on.clone();
                    }
                })
                .or_insert_with(|| actual.occurred_on.clone());
        }
    }
    let mut planned: std::collections::HashMap<&'static str, i64> = financial_domain::income_plan::CONTROL_ACCOUNTS
        .iter()
        .map(|name| (*name, 0i64))
        .collect();
    let plan_catalog: std::collections::HashMap<uuid::Uuid, _> = plans
        .iter()
        .map(|p| (p.security_id, p))
        .collect();
    let freq_catalog: std::collections::HashMap<uuid::Uuid, String> = characteristics
        .iter()
        .map(|c| (c.security_id, c.payment_frequency.clone()))
        .collect();
    for lot in &basis.lots {
        if lot.remaining_quantity_minor <= 0 {
            continue;
        }
        let Some(plan) = plan_catalog.get(&lot.security_id) else {
            continue;
        };
        let freq = freq_catalog
            .get(&lot.security_id)
            .map(|s| s.as_str())
            .unwrap_or("");
        let periods = financial_domain::calculator::periods_from_frequency(freq)
            .unwrap_or(plan.planning_periods_per_year);
        if !financial_domain::calculator::expected_in_week(
            last_actual.get(&lot.security_id).map(|s| s.as_str()),
            periods,
            &week.start,
            &week.end,
        ) {
            continue;
        }
        let account = accounts.iter().find(|a| a.account_id == lot.account_id);
        let Some(account) = account else {
            continue;
        };
        let Some(control) = financial_domain::income_plan::map_control_account(&account.name) else {
            continue;
        };
        *planned.entry(control).or_insert(0) += financial_domain::calculator::plan_payment_cents(
            lot.remaining_quantity_minor,
            lot.quantity_scale,
            plan.amount_per_share_minor,
            plan.amount_scale,
        );
    }
    let plan_known = !plans.is_empty();
    let lines = financial_domain::income_plan::CONTROL_ACCOUNTS
        .iter()
        .map(|name| IncomePlanLineBody {
            account_name: (*name).to_string(),
            actual_minor: *actuals.get(name).unwrap_or(&0),
            plan_known,
            planned_minor: *planned.get(name).unwrap_or(&0),
            scale: 2,
        })
        .collect();
    Ok(IncomePlanWeekBody {
        as_of_date: week.as_of_date,
        start: week.start,
        end: week.end,
        status: "Open".into(),
        lines,
        drilldown,
        latest_actual_on,
        yield_count,
        scale: 2,
    })
}

fn is_disbursement_type(activity_type: &str) -> bool {
    matches!(
        activity_type,
        "IRA_Distribution" | "Withdrawal" | "Form_1099" | "SSA" | "Roth_Distribution"
    )
}

async fn dashboard_burndown_view(
    canonical: &dyn Canonical,
    as_of_date: String,
) -> Result<DashboardBurndownBody, PlatformError> {
    let week = income_plan_week_view(canonical, as_of_date).await?;
    let accounts = canonical.account_list().await?;
    let activities = canonical.activity_list().await?;
    let mut outflows: std::collections::HashMap<&'static str, i64> =
        financial_domain::income_plan::BURNDOWN_ACCOUNTS
            .iter()
            .map(|name| (*name, 0i64))
            .collect();
    for activity in &activities {
        if !is_disbursement_type(&activity.activity_type) {
            continue;
        }
        if !financial_domain::income_plan::occurred_in_week(
            &activity.occurred_on,
            &week.start,
            &week.end,
        ) {
            continue;
        }
        let account = accounts
            .iter()
            .find(|a| a.account_id == activity.account_id);
        let Some(account) = account else {
            continue;
        };
        let Some(control) = financial_domain::income_plan::map_control_account(&account.name) else {
            continue;
        };
        if !financial_domain::income_plan::is_burndown_account(control) {
            continue;
        }
        *outflows.entry(control).or_insert(0) += activity.amount_minor;
    }
    let lines = financial_domain::income_plan::BURNDOWN_ACCOUNTS
        .iter()
        .map(|name| {
            let inflow = week
                .lines
                .iter()
                .find(|line| line.account_name == *name)
                .map(|line| line.actual_minor)
                .unwrap_or(0);
            DashboardBurndownLineBody {
                account_name: (*name).to_string(),
                inflow_minor: inflow,
                outflow_minor: *outflows.get(name).unwrap_or(&0),
                floor_known: false,
                scale: 2,
            }
        })
        .collect();
    Ok(DashboardBurndownBody {
        as_of_date: week.as_of_date,
        start: week.start,
        end: week.end,
        status: week.status,
        note: "operational (week not closed); Account 9 excluded from burndown".into(),
        lines,
        scale: 2,
    })
}

async fn holdings_view(canonical: &dyn Canonical) -> Result<HoldingsGetBody, PlatformError> {
    let basis = canonical.basis_get().await?;
    let accounts = canonical.account_list().await?;
    let securities = canonical.security_list().await?;
    let lots = basis
        .lots
        .into_iter()
        .filter(|lot| lot.remaining_quantity_minor > 0)
        .map(|lot| {
            let account_name = accounts
                .iter()
                .find(|a| a.account_id == lot.account_id)
                .map(|a| a.name.clone())
                .unwrap_or_else(|| "—".into());
            let symbol = securities
                .iter()
                .find(|s| s.security_id == lot.security_id)
                .map(|s| s.symbol.clone())
                .unwrap_or_else(|| "—".into());
            HoldingsLotBody {
                lot_id: lot.lot_id,
                account_name,
                symbol,
                opened_on: lot.opened_on,
                remaining_quantity_minor: lot.remaining_quantity_minor,
                quantity_scale: lot.quantity_scale,
                remaining_performance_minor: lot.remaining_performance_minor,
                remaining_tax_minor: lot.remaining_tax_minor,
                scale: lot.scale,
            }
        })
        .collect();
    Ok(HoldingsGetBody { lots, scale: 2 })
}

fn is_household_account(name: &str) -> bool {
    !name.eq_ignore_ascii_case("External")
}

async fn household_summary_view(
    canonical: &dyn Canonical,
) -> Result<HouseholdSummaryBody, PlatformError> {
    let accounts = canonical.account_list().await?;
    let basis = canonical.basis_get().await?;
    let dividend = canonical.dividend_get().await?;
    let activities = canonical.activity_list().await?;
    let open_lot_count = basis
        .lots
        .iter()
        .filter(|lot| lot.remaining_quantity_minor > 0)
        .count() as u64;
    let latest_yield_on = dividend
        .actuals
        .iter()
        .map(|a| a.occurred_on.clone())
        .max();
    let disbursement_count = activities
        .iter()
        .filter(|a| is_disbursement_type(&a.activity_type))
        .count() as u64;
    Ok(HouseholdSummaryBody {
        account_count: accounts
            .iter()
            .filter(|a| is_household_account(&a.name))
            .count() as u64,
        open_lot_count,
        yield_count: dividend.actuals.len() as u64,
        disbursement_count,
        latest_yield_on,
        plan_count: canonical.plan_history_list().await?.len() as u64,
    })
}

async fn calculator_view(canonical: &dyn Canonical) -> Result<CalculatorGetBody, PlatformError> {
    let securities = canonical.security_list().await?;
    let plans = canonical.plan_history_list().await?;
    let characteristics = canonical.position_characteristic_list().await?;
    let basis = canonical.basis_get().await?;
    let plan_by_sec: std::collections::HashMap<_, _> =
        plans.iter().map(|p| (p.security_id, p)).collect();
    let char_by_sec: std::collections::HashMap<_, _> = characteristics
        .iter()
        .map(|c| (c.security_id, c))
        .collect();
    let mut qty: std::collections::HashMap<uuid::Uuid, (i64, u8, i64)> =
        std::collections::HashMap::new();
    for lot in &basis.lots {
        if lot.remaining_quantity_minor <= 0 {
            continue;
        }
        let entry = qty.entry(lot.security_id).or_insert((0, lot.quantity_scale, 0));
        entry.0 += lot.remaining_quantity_minor;
        entry.1 = lot.quantity_scale;
        entry.2 += lot.remaining_performance_minor;
    }
    let mut rows = Vec::new();
    for security in &securities {
        let plan = plan_by_sec.get(&security.security_id);
        let ch = char_by_sec.get(&security.security_id);
        let (remaining_quantity_minor, quantity_scale, remaining_performance_minor) = qty
            .get(&security.security_id)
            .copied()
            .unwrap_or((0, 0, 0));
        if remaining_quantity_minor <= 0 {
            continue;
        }
        let plan_known = plan.is_some();
        let (plan_per_share_minor, plan_scale, periods) = plan
            .map(|p| {
                (
                    p.amount_per_share_minor,
                    p.amount_scale,
                    p.planning_periods_per_year,
                )
            })
            .unwrap_or((0, 2, 0));
        let plan_payment_minor = if plan_known {
            financial_domain::calculator::plan_payment_cents(
                remaining_quantity_minor,
                quantity_scale,
                plan_per_share_minor,
                plan_scale,
            )
        } else {
            0
        };
        rows.push(CalculatorRowBody {
            symbol: security.symbol.clone(),
            payment_frequency: ch
                .map(|c| c.payment_frequency.clone())
                .unwrap_or_default(),
            plan_known,
            plan_per_share_minor,
            plan_scale,
            planning_periods_per_year: periods,
            remaining_quantity_minor,
            quantity_scale,
            plan_payment_minor,
            remaining_performance_minor,
            roc_pct_2025_actual_minor: ch.and_then(|c| c.roc_pct_2025_actual_minor),
            roc_scale: ch.and_then(|c| c.roc_scale),
            scale: 2,
        });
    }
    rows.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    Ok(CalculatorGetBody {
        plan_count: plans.len() as u64,
        rows,
        scale: 2,
    })
}

async fn plan_review_view(
    canonical: &dyn Canonical,
    security_id: uuid::Uuid,
) -> Result<PlanReviewBody, PlatformError> {
    let decls = canonical.issuer_declaration_list(security_id).await?;
    let amounts: Vec<Option<i64>> = decls.iter().map(|d| d.amount_per_share_minor).collect();
    let review = financial_domain::plan_review::plan_review(&amounts);
    Ok(PlanReviewBody {
        security_id,
        observation_count: review.observation_count as u64,
        most_current_minor: review.most_current_minor,
        avg6_minor: review.avg6_minor,
        min_minor: review.min_minor,
        max_minor: review.max_minor,
        average_minor: review.average_minor,
        eighty_pct_of_avg_minor: review.eighty_pct_of_avg_minor,
        avg6_complete: review.avg6_complete,
        full_analysis_possible: review.full_analysis_possible,
        confirm_blocked: review.confirm_blocked,
        incomplete_reason_required: review.incomplete_reason_required,
        amount_scale: decls.first().map(|d| d.amount_scale).unwrap_or(4),
    })
}

async fn plan_history_confirm_on(
    canonical: &dyn Canonical,
    security_id: uuid::Uuid,
    json: &Value,
) -> Result<crate::contracts::PlanHistoryRecord, PlatformError> {
    let review = plan_review_view(canonical, security_id).await?;
    match financial_domain::plan_review::plan_confirm_gate(
        review.observation_count as usize,
        jstr(json, "incompleteAnalysisReason")
            .as_deref()
            .unwrap_or(""),
    ) {
        Ok(()) => {}
        Err(financial_domain::error::DomainError::PlanConfirmBlocked) => {
            return Err(PlatformError::new(
                "plan_confirm_blocked",
                "plan confirm is blocked until at least one declaration observation exists",
            ));
        }
        Err(financial_domain::error::DomainError::IncompleteAnalysisRequired) => {
            return Err(PlatformError::new(
                "incomplete_analysis_required",
                "incomplete analysis requires an explicit reason before plan confirm",
            ));
        }
        Err(err) => {
            return Err(PlatformError::new("domain_error", err.to_string()));
        }
    }
    let mut reason = jstr(json, "decisionReason").unwrap_or_default();
    if review.incomplete_reason_required {
        if let Some(extra) = jstr(json, "incompleteAnalysisReason") {
            if !reason.is_empty() {
                reason.push_str("; ");
            }
            reason.push_str("incomplete analysis: ");
            reason.push_str(&extra);
        }
    }
    canonical
        .plan_history_confirm(
            security_id,
            ji64(json, "amountPerShareMinor").unwrap_or(0),
            ju8(json, "amountScale", 2),
            ju8(json, "planningPeriodsPerYear", 12),
            jstr(json, "effectiveFrom").unwrap_or_else(|| "2026-08-21".into()),
            reason,
        )
        .await
}

async fn investment_view(
    canonical: &dyn Canonical,
    security_id: uuid::Uuid,
    as_of: String,
) -> Result<InvestmentGetBody, PlatformError> {
    let security = canonical.security_get(security_id).await?;
    let characteristics = canonical.position_characteristic_list().await?;
    let ch = characteristics.iter().find(|c| c.security_id == security_id);
    let plans = canonical.plan_history_list().await?;
    let plan = plans.iter().find(|p| p.security_id == security_id);
    let review = plan_review_view(canonical, security_id).await?;
    let price = canonical.current_price_get(security_id, as_of).await?;
    let template = canonical.retrieval_template_get(security_id).await?;
    let decls = canonical.issuer_declaration_list(security_id).await?;
    let accounts = canonical.account_list().await?;
    let basis = canonical.basis_get().await?;
    let mut lots = Vec::new();
    let mut remaining_quantity_minor = 0i64;
    let mut quantity_scale = 0u8;
    let mut remaining_performance_minor = 0i64;
    for lot in &basis.lots {
        if lot.security_id != security_id || lot.remaining_quantity_minor <= 0 {
            continue;
        }
        remaining_quantity_minor += lot.remaining_quantity_minor;
        quantity_scale = lot.quantity_scale;
        remaining_performance_minor += lot.remaining_performance_minor;
        let account_name = accounts
            .iter()
            .find(|a| a.account_id == lot.account_id)
            .map(|a| a.name.clone())
            .unwrap_or_default();
        lots.push(InvestmentLotBody {
            lot_id: lot.lot_id,
            account_name,
            opened_on: lot.opened_on.clone(),
            remaining_quantity_minor: lot.remaining_quantity_minor,
            quantity_scale: lot.quantity_scale,
            remaining_performance_minor: lot.remaining_performance_minor,
            remaining_tax_minor: lot.remaining_tax_minor,
            scale: lot.scale,
        });
    }
    Ok(InvestmentGetBody {
        security_id,
        symbol: security.symbol,
        name: security.name,
        payment_frequency: ch.map(|c| c.payment_frequency.clone()).unwrap_or_default(),
        risk_tier: ch.map(|c| c.risk_tier.clone()).unwrap_or_default(),
        provider: ch.map(|c| c.provider.clone()).unwrap_or_default(),
        underlying: ch.map(|c| c.underlying.clone()).unwrap_or_default(),
        plan_known: plan.is_some(),
        plan_per_share_minor: plan.map(|p| p.amount_per_share_minor).unwrap_or(0),
        plan_scale: plan.map(|p| p.amount_scale).unwrap_or(2),
        planning_periods_per_year: plan.map(|p| p.planning_periods_per_year).unwrap_or(0),
        plan_reason: plan.map(|p| p.decision_reason.clone()).unwrap_or_default(),
        plan_effective_from: plan.map(|p| p.effective_from.clone()).unwrap_or_default(),
        remaining_quantity_minor,
        quantity_scale,
        remaining_performance_minor,
        roc_pct_2025_actual_minor: ch.and_then(|c| c.roc_pct_2025_actual_minor),
        roc_scale: ch.and_then(|c| c.roc_scale),
        price,
        review,
        template,
        lots,
        declaration_count: decls.len() as u64,
        first_lot_complete: remaining_quantity_minor > 0,
        scale: 2,
    })
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
        "IncomePlanWeekGet" => {
            let as_of = jstr(&json, "asOfDate").unwrap_or_default();
            map_q(&request, income_plan_week_view(canonical, as_of).await)
        }
        "DashboardBurndownGet" => {
            let as_of = jstr(&json, "asOfDate").unwrap_or_default();
            map_q(&request, dashboard_burndown_view(canonical, as_of).await)
        }
        "HoldingsGet" => map_q(&request, holdings_view(canonical).await),
        "HouseholdSummaryGet" => map_q(&request, household_summary_view(canonical).await),
        "CalculatorGet" => map_q(&request, calculator_view(canonical).await),
        "PlanReviewGet" => match juuid(&json, "securityId") {
            Some(id) => map_q(&request, plan_review_view(canonical, id).await),
            None => query_err(&request, "missing_security_id"),
        },
        "CurrentPriceGet" => match juuid(&json, "securityId") {
            Some(id) => {
                let as_of = jstr(&json, "asOfDate").unwrap_or_else(|| "2026-08-21".into());
                map_q(&request, canonical.current_price_get(id, as_of).await)
            }
            None => query_err(&request, "missing_security_id"),
        },
        "InvestmentGet" => {
            let as_of = jstr(&json, "asOfDate").unwrap_or_else(|| "2026-08-21".into());
            if let Some(id) = juuid(&json, "securityId") {
                map_q(&request, investment_view(canonical, id, as_of).await)
            } else if let Some(symbol) = jstr(&json, "symbol") {
                match canonical.security_list().await {
                    Ok(list) => {
                        match list.into_iter().find(|s| s.symbol.eq_ignore_ascii_case(&symbol)) {
                            Some(sec) => map_q(
                                &request,
                                investment_view(canonical, sec.security_id, as_of).await,
                            ),
                            None => query_err(&request, "not_found"),
                        }
                    }
                    Err(err) => query_err(&request, &err.code),
                }
            } else {
                query_err(&request, "missing_security_id")
            }
        },
        "PriceRetrievalSetGet" => map_q(&request, canonical.price_retrieval_set().await),
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
        "DistributionGet" => map_q(&request, canonical.distribution_get().await),
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
            map_c(&request, canonical.security_register(symbol, name, jbool(&json, "crf", false)).await)
        }
        "SecurityUpdate" => match juuid(&json, "securityId") {
            Some(id) => map_c(
                &request,
                canonical
                    .security_update(id, jstr(&json, "name"), jstr(&json, "symbol"))
                    .await,
            ),
            None => command_err(&request, "missing_security_id"),
        },
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
                        jbool(&json, "isOpen", true),
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
        "DistributionCharacterize" => match juuid(&json, "activityId") {
            Some(activity_id) => map_c(
                &request,
                canonical
                    .distribution_characterize(
                        activity_id,
                        jstr(&json, "category").unwrap_or_else(|| "ordinary".into()),
                        ji64(&json, "amountMinor").unwrap_or(0),
                        ju8(&json, "scale", 2),
                    )
                    .await,
            ),
            None => command_err(&request, "missing_activity_id"),
        },
        "ProductionSeedLoad" => {
            match serde_json::from_value::<ProductionSeedDocument>(json.clone()) {
                Ok(doc) => map_c(
                    &request,
                    crate::production_seed::apply_production_seed(canonical, doc).await,
                ),
                Err(_) => command_err(&request, "invalid_seed_document"),
            }
        },
        "PlanHistoryConfirm" => match juuid(&json, "securityId") {
            Some(security_id) => {
                match plan_history_confirm_on(canonical, security_id, &json).await {
                    Ok(body) => map_c(&request, Ok(body)),
                    Err(err) => command_err(&request, &err.code),
                }
            }
            None => command_err(&request, "missing_security_id"),
        },
        "IssuerDeclarationRecord" => match juuid(&json, "securityId") {
            Some(security_id) => map_c(
                &request,
                canonical
                    .issuer_declaration_record(
                        security_id,
                        json.get("amountPerShareMinor")
                            .and_then(|x| {
                                if x.is_null() {
                                    None
                                } else {
                                    ji64(&json, "amountPerShareMinor")
                                }
                            }),
                        ju8(&json, "amountScale", 2),
                        jstr(&json, "paymentPeriod").unwrap_or_default(),
                        jstr(&json, "source").unwrap_or_default(),
                        jstr(&json, "enteredAt").unwrap_or_else(|| "2026-08-21".into()),
                    )
                    .await,
            ),
            None => command_err(&request, "missing_security_id"),
        },
        "PriceQuoteRecord" => match juuid(&json, "securityId") {
            Some(security_id) => map_c(
                &request,
                canonical
                    .price_quote_record(
                        security_id,
                        ji64(&json, "priceMinor").unwrap_or(0),
                        ju8(&json, "scale", 2),
                        jstr(&json, "asOfAt").unwrap_or_else(|| "2026-08-21".into()),
                        jstr(&json, "source").unwrap_or_else(|| "live".into()),
                    )
                    .await,
            ),
            None => command_err(&request, "missing_security_id"),
        },
        "ManualPriceOverride" => match juuid(&json, "securityId") {
            Some(security_id) => map_c(
                &request,
                canonical
                    .manual_price_override(
                        security_id,
                        ji64(&json, "priceMinor").unwrap_or(0),
                        ju8(&json, "scale", 2),
                        jstr(&json, "reason").unwrap_or_else(|| "prompt".into()),
                        jstr(&json, "effectiveFrom").unwrap_or_else(|| "2026-08-21".into()),
                    )
                    .await,
            ),
            None => command_err(&request, "missing_security_id"),
        },
        "RetrievalTemplateSet" => match juuid(&json, "securityId") {
            Some(security_id) => map_c(
                &request,
                canonical
                    .retrieval_template_set(RetrievalTemplateRecord {
                        security_id,
                        price_source: jstr(&json, "priceSource").unwrap_or_else(|| "public".into()),
                        source_symbol: jstr(&json, "sourceSymbol").unwrap_or_default(),
                        declaration_source: jstr(&json, "declarationSource").unwrap_or_default(),
                        lookback_count: ju8(&json, "lookbackCount", 12),
                        payment_source: jstr(&json, "paymentSource").unwrap_or_default(),
                    })
                    .await,
            ),
            None => command_err(&request, "missing_security_id"),
        },
        "PositionCharacteristicUpsert" => match juuid(&json, "securityId") {
            Some(security_id) => {
                let existing = match canonical.position_characteristic_list().await {
                    Ok(list) => list.into_iter().find(|c| c.security_id == security_id),
                    Err(err) => return command_err(&request, &err.code),
                };
                let base = existing.unwrap_or(PositionCharacteristicRecord {
                    security_id,
                    payment_frequency: String::new(),
                    risk_tier: String::new(),
                    provider: String::new(),
                    underlying: String::new(),
                    roc_pct_2025_actual_minor: None,
                    roc_pct_2026_estimate_minor: None,
                    roc_pct_2026_actual_minor: None,
                    roc_pct_2024_actual_minor: None,
                    roc_scale: None,
                    div_type: String::new(),
                    needs_roc_research: false,
                    notes: String::new(),
                });
                map_c(
                    &request,
                    canonical
                        .position_characteristic_upsert(PositionCharacteristicRecord {
                            security_id,
                            payment_frequency: jstr_keep(
                                &json,
                                "paymentFrequency",
                                base.payment_frequency,
                            ),
                            risk_tier: jstr_keep(&json, "riskTier", base.risk_tier),
                            provider: jstr_keep(&json, "provider", base.provider),
                            underlying: jstr_keep(&json, "underlying", base.underlying),
                            roc_pct_2025_actual_minor: ji64_keep(
                                &json,
                                "rocPct2025ActualMinor",
                                base.roc_pct_2025_actual_minor,
                            ),
                            roc_pct_2026_estimate_minor: ji64_keep(
                                &json,
                                "rocPct2026EstimateMinor",
                                base.roc_pct_2026_estimate_minor,
                            ),
                            roc_pct_2026_actual_minor: ji64_keep(
                                &json,
                                "rocPct2026ActualMinor",
                                base.roc_pct_2026_actual_minor,
                            ),
                            roc_pct_2024_actual_minor: ji64_keep(
                                &json,
                                "rocPct2024ActualMinor",
                                base.roc_pct_2024_actual_minor,
                            ),
                            roc_scale: ju8_opt_keep(&json, "rocScale", base.roc_scale),
                            div_type: jstr_keep(&json, "divType", base.div_type),
                            needs_roc_research: jbool_keep(
                                &json,
                                "needsRocResearch",
                                base.needs_roc_research,
                            ),
                            notes: jstr_keep(&json, "notes", base.notes),
                        })
                        .await,
                )
            }
            None => command_err(&request, "missing_security_id"),
        },
        "DeclarationRetrieve" => command_ok(
            &request,
            json.get("candidates")
                .cloned()
                .map(|c| serde_json::json!({ "candidates": c }).to_string())
                .unwrap_or_else(|| "{\"candidates\":[]}".into()),
        ),
        "PriceQuoteRetrieve" => command_ok(
            &request,
            json.get("candidates")
                .cloned()
                .map(|c| serde_json::json!({ "candidates": c }).to_string())
                .unwrap_or_else(|| "{\"candidates\":[]}".into()),
        ),
        "MarketRetrieve" => command_ok(&request, json.to_string()),
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
