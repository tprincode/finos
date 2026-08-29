//! Query/command dispatch owned by application-core (no Tauri, no database).

use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::contracts::{
    AllocationGetBody, CalculatorGetBody, CalculatorRowBody, CommandRequest, CommandResult,
    DashboardBody, DashboardBurndownBody, DashboardBurndownLineBody, HoldingsGetBody,
    HoldingsLotBody, DataSummaryBody, ImportCandidate, IncomePlanDrillBody,
    IncomePlanLineBody, IncomePlanWeekBody, ProductionSeedDocument, QueryRequest, QueryResult,
    RoiBody, TrendPoint, TrendsBody, TrendsWeekPoint, UpdaterCheckBody, FINANCE_CLIENT_CONTRACT_VERSION,
    PlanReviewBody, PositionCharacteristicRecord, LookthroughResearch, RetrievalTemplateRecord, InvestmentGetBody,
    InvestmentLotBody, InvestmentDeclarationBody, AccountPositionTotalBody, PositionDetailsBody,
    BacktestPeriodRecord, PositionBacktestResultBody, EvidenceDimensionsBody, TierSuggestionBody,
    RocResearchBody, RocCandidateBody, RocResearchObservation, RemainingYearIncomeBody,
    RemainingPaymentBody, RemainingMonthBody, RemainingPaymentDateOverride,
    LastPriceRefreshBody, DeclarationRefreshBody, PositionMasterGetBody, PositionMasterRowBody,
    ExpectedPaymentPattern, PositionTaxProfile, CurrentPriceBody,
    PositionDetailsCoverageBody, PositionDetailsCoverageRow, Div1ComplianceSummaryBody,
    Div1ComplianceSummaryRow, AccountRecord, ActivityRecord,
    DistributionRecord, LotRecord, IssuerDeclarationRecord, IssuerPayDateRecord,
    ProviderDeclarationSourcesApplyBody, RetrieveRunRecord, CollectorRetrieveBody,
    PositionResearchSeedBody, RetrieveRunListBody,
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
    "IssuerPayDateReplace",
    "PositionCharacteristicUpsert",
    "BacktestPeriodRecord",
    "PositionBacktestCalculate",
    "ClassificationApply",
    "RocPlanConfirm",
    "RemainingPaymentDateOverride",
    "LastPriceRefresh",
    "DeclarationRefresh",
    "ProviderDeclarationSourcesApply",
    "CollectorRetrieve",
    "PositionResearchSeed",
    "ExpectedPaymentPatternUpsert",
    "PositionTaxProfileUpsert",
    "TrendsWeekSave",
    "TrendsWeekCorrect",
    "TrendsWeekClose",
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

fn jlookthrough_keep(v: &Value, key: &str, keep: LookthroughResearch) -> LookthroughResearch {
    match v.get(key) {
        None => keep,
        Some(Value::Null) => LookthroughResearch::default(),
        Some(value) => serde_json::from_value(value.clone()).unwrap_or(keep),
    }
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

fn ju8_opt(v: &Value, key: &str) -> Option<u8> {
    v.get(key).and_then(|x| {
        x.as_u64()
            .map(|n| n as u8)
            .or_else(|| x.as_i64().and_then(|n| u8::try_from(n).ok()))
            .or_else(|| x.as_str().and_then(|s| s.parse().ok()))
    })
}

fn ju8(v: &Value, key: &str, default: u8) -> u8 {
    ju8_opt(v, key).unwrap_or(default)
}

fn locked_cadence(raw: &str) -> Result<financial_domain::calculator::PaymentCadence, PlatformError> {
    financial_domain::calculator::PaymentCadence::parse(raw).ok_or_else(|| {
        PlatformError::new(
            "payment_cadence_required",
            "Weekly (52), Monthly (12), Quarterly (4), or None (does not pay) must be identified; there is no default",
        )
    })
}

fn cadence_periods(freq: &str) -> Option<u8> {
    financial_domain::calculator::PaymentCadence::parse(freq)
        .and_then(financial_domain::calculator::PaymentCadence::periods)
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

async fn remaining_pay_dates_for(
    canonical: &dyn Canonical,
    security_id: Uuid,
    as_of: &str,
    periods: u8,
    cache: &mut std::collections::HashMap<Uuid, Vec<String>>,
) -> Result<Vec<String>, PlatformError> {
    if let Some(dates) = cache.get(&security_id) {
        return Ok(dates.clone());
    }
    let schedule = remaining_year_schedule_for(canonical, security_id, as_of, periods, None, &[]).await?;
    let dates = if schedule.known {
        schedule.payments.iter().map(|p| p.pay_on.clone()).collect()
    } else {
        Vec::new()
    };
    cache.insert(security_id, dates.clone());
    Ok(dates)
}

async fn remaining_year_schedule_for(
    canonical: &dyn Canonical,
    security_id: Uuid,
    as_of: &str,
    periods: u8,
    extra_lots: Option<&[financial_domain::schedule::OpenLotQty]>,
    draft_overrides: &[financial_domain::schedule::DateOverride],
) -> Result<financial_domain::schedule::RemainingYearSchedule, PlatformError> {
    let overrides = merge_date_overrides(
        stored_date_overrides(canonical, security_id).await?,
        draft_overrides.to_vec(),
    );
    let latest = latest_declaration_period(canonical, security_id).await?;
    let issuer = canonical.issuer_pay_date_list(security_id).await?;
    let issuer_pay_ons: Vec<String> = issuer.into_iter().map(|d| d.pay_on).collect();
    let template = canonical.retrieval_template_get(security_id).await?;
    let policy = financial_domain::schedule::CalendarPolicy::resolve(
        template.as_ref().map(|t| t.calendar_policy.as_str()).unwrap_or(""),
        periods,
        issuer_pay_ons.len(),
    );
    let basis = canonical.basis_get().await?;
    let mut lots: Vec<financial_domain::schedule::OpenLotQty> = basis
        .lots
        .iter()
        .filter(|l| l.security_id == security_id && l.remaining_quantity_minor > 0)
        .map(|l| financial_domain::schedule::OpenLotQty {
            opened_on: l.opened_on.clone(),
            quantity_minor: l.remaining_quantity_minor,
            quantity_scale: l.quantity_scale,
        })
        .collect();
    if let Some(extra) = extra_lots {
        lots.extend_from_slice(extra);
    }
    let plans = canonical.plan_history_list().await?;
    let (plan_minor, plan_scale) = plans
        .iter()
        .find(|p| p.security_id == security_id)
        .map(|p| (p.amount_per_share_minor, p.amount_scale))
        .unwrap_or((0, 2));
    Ok(financial_domain::schedule::remaining_year_from_spec(
        financial_domain::schedule::RemainingYearSpec {
            as_of,
            latest_payment_period: latest.as_deref(),
            periods_per_year: periods,
            lots: &lots,
            plan_minor,
            plan_scale,
            overrides: &overrides,
            issuer_pay_ons: &issuer_pay_ons,
            calendar_policy: policy,
        },
    ))
}

async fn latest_declaration_period(
    canonical: &dyn Canonical,
    security_id: Uuid,
) -> Result<Option<String>, PlatformError> {
    let decls = canonical.issuer_declaration_list(security_id).await?;
    Ok(financial_domain::schedule::latest_parseable_period(
        decls.iter().map(|d| d.payment_period.as_str()),
    ))
}

async fn stored_date_overrides(
    canonical: &dyn Canonical,
    security_id: Uuid,
) -> Result<Vec<financial_domain::schedule::DateOverride>, PlatformError> {
    let stored = canonical
        .remaining_payment_date_override_list(security_id)
        .await?;
    Ok(stored
        .into_iter()
        .map(|o| financial_domain::schedule::DateOverride {
            original_pay_on: o.original_pay_on,
            pay_on: o.pay_on,
        })
        .collect())
}

fn draft_date_overrides(json: &Value) -> Vec<financial_domain::schedule::DateOverride> {
    json.get("overrides")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|row| {
                    Some(financial_domain::schedule::DateOverride {
                        original_pay_on: row.get("originalPayOn")?.as_str()?.to_string(),
                        pay_on: row.get("payOn")?.as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn fetched_source_url_for(json: &Value, security_id: Option<Uuid>) -> String {
    if let Some(url) = jstr(json, "fetchedSourceUrl").filter(|u| !u.is_empty()) {
        return url;
    }
    let Some(security_id) = security_id else {
        return String::new();
    };
    for key in ["candidates", "declarations"] {
        let Some(arr) = json.get(key).and_then(|v| v.as_array()) else {
            continue;
        };
        for row in arr {
            if juuid(row, "securityId") == Some(security_id) {
                if let Some(url) = jstr(row, "fetchedSourceUrl").filter(|u| !u.is_empty()) {
                    return url;
                }
            }
        }
    }
    String::new()
}

async fn promote_vendor_calendar_template(
    canonical: &dyn Canonical,
    security_id: Uuid,
    pay_date_count: usize,
    payment_calendar_url: &str,
) {
    if pay_date_count == 0 && payment_calendar_url.is_empty() {
        return;
    }
    let Ok(Some(mut template)) = canonical.retrieval_template_get(security_id).await else {
        return;
    };
    if pay_date_count > 0 {
        template.calendar_policy = financial_domain::schedule::CalendarPolicy::IssuerCalendar.label().to_string();
    }
    if !payment_calendar_url.is_empty() {
        template.payment_source = payment_calendar_url.to_string();
    }
    let _ = canonical.retrieval_template_set(template).await;
}

async fn raise_retrieve_misses(
    canonical: &dyn Canonical,
    misses: &[Value],
    ran_at: &str,
) -> Result<(), PlatformError> {
    for miss in misses {
        let code = jstr(miss, "code").unwrap_or_else(|| "retrieve_miss".into());
        let message = jstr(miss, "reason")
            .or_else(|| jstr(miss, "message"))
            .unwrap_or_else(|| "retrieve miss".into());
        let symbol = jstr(miss, "symbol").unwrap_or_default();
        let message = if symbol.is_empty() || message.contains(&symbol) {
            message
        } else {
            format!("{symbol}: {message}")
        };
        let _ = canonical.exception_raise(code, message.clone()).await;
        if let Some(id) = juuid(miss, "securityId") {
            let hash = jstr(miss, "contentHash").unwrap_or_default();
            let _ = canonical
                .retrieval_template_touch_run(id, false, message, ran_at.to_string(), hash, "")
                .await;
        }
    }
    Ok(())
}

async fn persist_retrieve_run(
    canonical: &dyn Canonical,
    security_id: Uuid,
    kind: &str,
    requested_at: &str,
    ok: bool,
    code: &str,
    message: &str,
    attempted: u64,
    recorded: u64,
    skipped: u64,
    unchanged: u64,
    payload: &Value,
) {
    let payload_json = serde_json::to_string(payload).unwrap_or_else(|_| "{}".into());
    let _ = canonical
        .retrieve_run_record(RetrieveRunRecord {
            run_id: Uuid::new_v4(),
            security_id,
            kind: kind.into(),
            requested_at: requested_at.into(),
            ok,
            code: code.into(),
            message: message.into(),
            attempted,
            recorded,
            skipped,
            unchanged,
            payload_json,
        })
        .await;
}

/// CASH-only: rate change → new PlanHistory; paid months → actuals when broker yield is absent.
async fn apply_cash_moneymarket_followups(
    canonical: &dyn Canonical,
    security_id: Uuid,
    candidates: &[Value],
    source: &str,
    as_of: &str,
) -> u64 {
    let Ok(security) = canonical.security_get(security_id).await else {
        return 0;
    };
    let chars = canonical
        .position_characteristic_list()
        .await
        .unwrap_or_default();
    let div_type = chars
        .iter()
        .find(|c| c.security_id == security_id)
        .map(|c| c.div_type.as_str())
        .unwrap_or("");
    if !financial_domain::current_price::uses_cash_par(div_type, &security.symbol) {
        return 0;
    }

    let mut posted = 0u64;
    let paid: Vec<&Value> = candidates
        .iter()
        .filter(|c| {
            let amount = c.get("amountPerShareMinor").and_then(|x| {
                if x.is_null() {
                    None
                } else {
                    ji64(c, "amountPerShareMinor")
                }
            });
            amount.unwrap_or(0) > 0
        })
        .collect();
    let Some(latest) = paid.first() else {
        return 0;
    };
    let Some(rate_minor) = ji64(latest, "amountPerShareMinor").filter(|a| *a > 0) else {
        return 0;
    };
    let rate_scale = ju8(latest, "amountScale", 5);
    let plans = canonical.plan_history_list().await.unwrap_or_default();
    let current = plans.iter().find(|p| p.security_id == security_id);
    let reason = format!("money-market rate from {source}");
    let needs_plan = match current {
        None => true,
        Some(p) => p.amount_per_share_minor != rate_minor || p.amount_scale != rate_scale,
    };
    if needs_plan {
        let result = if current.is_none() {
            canonical
                .plan_history_record(
                    security_id,
                    rate_minor,
                    rate_scale,
                    12,
                    as_of.to_string(),
                    reason.clone(),
                )
                .await
        } else {
            canonical
                .plan_history_confirm(
                    security_id,
                    rate_minor,
                    rate_scale,
                    12,
                    as_of.to_string(),
                    reason,
                )
                .await
        };
        if result.is_ok() {
            posted += 1;
        }
    }

    let existing = canonical.dividend_get().await.ok();
    let lots = canonical
        .basis_get()
        .await
        .map(|b| b.lots)
        .unwrap_or_default();
    for cand in paid {
        let Some(amount_minor) = ji64(cand, "amountPerShareMinor").filter(|a| *a > 0) else {
            continue;
        };
        let amount_scale = ju8(cand, "amountScale", 5);
        let occurred_on = jstr(cand, "paymentPeriod")
            .or_else(|| jstr(cand, "payOn"))
            .unwrap_or_default();
        if occurred_on.is_empty() {
            continue;
        }
        let broker_exists = existing
            .as_ref()
            .map(|d| {
                d.actuals.iter().any(|a| {
                    a.security_id == Some(security_id) && a.occurred_on == occurred_on
                })
            })
            .unwrap_or(false);
        if broker_exists {
            continue;
        }
        for lot in lots
            .iter()
            .filter(|l| l.security_id == security_id && l.remaining_quantity_minor > 0)
        {
            let cents = financial_domain::calculator::plan_payment_cents(
                lot.remaining_quantity_minor,
                lot.quantity_scale,
                amount_minor,
                amount_scale,
            );
            if cents <= 0 {
                continue;
            }
            let key = format!(
                "mm-{}-{}-{}-{}",
                security_id, lot.account_id, occurred_on, cents
            );
            if canonical
                .dividend_actual_record(
                    lot.account_id,
                    Some(security_id),
                    occurred_on.clone(),
                    Some(cents),
                    2,
                    Some(key),
                )
                .await
                .is_ok()
            {
                posted += 1;
            }
        }
    }
    posted
}

fn declarations_for_security(json: &Value, security_id: Uuid) -> Vec<Value> {
    json.get("declarations")
        .and_then(|q| q.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|row| juuid(row, "securityId") == Some(security_id))
        .collect()
}

fn pay_dates_for_security(json: &Value, security_id: Uuid) -> Vec<Value> {
    json.get("payDates")
        .and_then(|q| q.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|row| juuid(row, "securityId") == Some(security_id))
        .collect()
}

fn payment_calendar_url_for(json: &Value, security_id: Uuid) -> String {
    let top = jstr(json, "fetchedPaymentCalendarUrl").unwrap_or_default();
    if !top.is_empty() {
        return top;
    }
    pay_dates_for_security(json, security_id)
        .iter()
        .find_map(|row| jstr(row, "paymentCalendarUrl").filter(|u| !u.is_empty()))
        .unwrap_or_default()
}

fn merge_date_overrides(
    stored: Vec<financial_domain::schedule::DateOverride>,
    draft: Vec<financial_domain::schedule::DateOverride>,
) -> Vec<financial_domain::schedule::DateOverride> {
    let mut out = stored;
    for ov in draft {
        if let Some(existing) = out
            .iter_mut()
            .find(|s| s.original_pay_on == ov.original_pay_on)
        {
            *existing = ov;
        } else {
            out.push(ov);
        }
    }
    out
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
            .unwrap_or_else(today_iso)
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
    let mut decl_dates: std::collections::HashMap<uuid::Uuid, Vec<String>> =
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
        let opened = lot.opened_on.get(..10).unwrap_or(lot.opened_on.as_str());
        if opened > week.end.as_str() {
            continue;
        }
        let Some(plan) = plan_catalog.get(&lot.security_id) else {
            continue;
        };
        let freq = freq_catalog
            .get(&lot.security_id)
            .map(|s| s.as_str())
            .unwrap_or("");
        let Some(periods) = cadence_periods(freq) else {
            continue;
        };
        let has_actual = last_actual.contains_key(&lot.security_id);
        let scheduled = if has_actual || periods == 52 {
            financial_domain::calculator::expected_in_week(
                last_actual.get(&lot.security_id).map(|s| s.as_str()),
                periods,
                &week.start,
                &week.end,
            )
        } else {
            remaining_pay_dates_for(canonical, lot.security_id, &week.start, periods, &mut decl_dates)
                .await?
                .iter()
                .any(|d| financial_domain::schedule::pay_on_in_week(d, &week.start, &week.end))
        };
        if !scheduled {
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
    let snapshots = canonical.account_balance_snapshot_list().await?;
    let mut latest_balance: std::collections::HashMap<&'static str, i64> =
        std::collections::HashMap::new();
    for snap in &snapshots {
        let Some(account) = accounts.iter().find(|a| a.account_id == snap.account_id) else {
            continue;
        };
        let Some(control) = financial_domain::income_plan::map_control_account(&account.name) else {
            continue;
        };
        if !financial_domain::income_plan::is_burndown_account(control) {
            continue;
        }
        // snapshots are ordered by period_end ASC — last write wins as latest
        latest_balance.insert(control, snap.balance_minor);
    }
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
            let ending = latest_balance.get(name).copied();
            DashboardBurndownLineBody {
                account_name: (*name).to_string(),
                inflow_minor: inflow,
                outflow_minor: *outflows.get(name).unwrap_or(&0),
                floor_known: false,
                ending_balance_minor: ending,
                ending_balance_known: ending.is_some(),
                scale: 2,
            }
        })
        .collect();
    Ok(DashboardBurndownBody {
        as_of_date: week.as_of_date,
        start: week.start,
        end: week.end,
        status: week.status,
        note: "operational (week not closed); Account 9 excluded from burndown; ending balances from Trends snapshots".into(),
        lines,
        scale: 2,
    })
}

fn acct9_etf_proxy_minor(etf_value_minor: i64) -> i64 {
    // Trends tab column "Acct 9 70% ETF" is already the ×70% amount; store and use as-is.
    etf_value_minor
}

async fn account_trends_weeks(
    canonical: &dyn Canonical,
) -> Result<Vec<TrendsWeekPoint>, PlatformError> {
    let sources = canonical.trends_week_list().await?;
    let accounts = canonical.account_list().await?;
    let snapshots = canonical.account_balance_snapshot_list().await?;
    let mut by_period: std::collections::HashMap<String, std::collections::HashMap<&'static str, i64>> =
        std::collections::HashMap::new();
    for snap in &snapshots {
        let Some(account) = accounts.iter().find(|a| a.account_id == snap.account_id) else {
            continue;
        };
        let control = if account.name.eq_ignore_ascii_case("speculation") {
            "Speculation"
        } else {
            match financial_domain::income_plan::map_control_account(&account.name) {
                Some(c) => c,
                None => continue,
            }
        };
        by_period
            .entry(snap.period_end.clone())
            .or_default()
            .insert(control, snap.balance_minor);
    }
    let mut weeks = Vec::with_capacity(sources.len());
    let mut prev_divs: Option<i64> = None;
    let mut prev_combined: Option<i64> = None;
    for src in &sources {
        let combined = src.fidelity_total_minor + src.schwab_total_minor;
        let proxy = acct9_etf_proxy_minor(src.acct9_etf_value_minor);
        let total_cash = src.income_cash_minor + src.acct9_cash_minor + proxy;
        let div_delta = match prev_divs {
            Some(prev) => src.monthly_divs_minor - prev,
            None => 0,
        };
        let wk_change = match prev_combined {
            Some(prev) => combined - prev,
            None => 0,
        };
        let bals = by_period.get(&src.period_end);
        weeks.push(TrendsWeekPoint {
            period_end: src.period_end.clone(),
            profit_minor: src.profit_minor,
            monthly_divs_minor: src.monthly_divs_minor,
            div_delta_minor: div_delta,
            fidelity_total_minor: src.fidelity_total_minor,
            schwab_total_minor: src.schwab_total_minor,
            fid_sch_combined_minor: combined,
            wk_to_wk_change_minor: wk_change,
            income_cash_minor: src.income_cash_minor,
            acct9_cash_minor: src.acct9_cash_minor,
            acct9_etf_proxy_minor: proxy,
            total_cash_minor: total_cash,
            car_balance_minor: bals.and_then(|m| m.get("Car").copied()),
            income_balance_minor: bals.and_then(|m| m.get("Income").copied()),
            health_balance_minor: bals.and_then(|m| m.get("Health").copied()),
            roth_balance_minor: bals.and_then(|m| m.get("Roth").copied()),
            speculation_balance_minor: bals.and_then(|m| m.get("Speculation").copied()),
            closed: src.closed,
            scale: src.scale,
        });
        prev_divs = Some(src.monthly_divs_minor);
        prev_combined = Some(combined);
    }
    Ok(weeks)
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

fn is_data_account(name: &str) -> bool {
    !name.eq_ignore_ascii_case("External")
}

fn today_iso() -> String {
    chrono::Utc::now().date_naive().to_string()
}

fn count_paid_declarations(decls: &[IssuerDeclarationRecord]) -> u8 {
    decls
        .iter()
        .filter(|d| d.amount_per_share_minor.map(|a| a > 0).unwrap_or(false))
        .count()
        .min(u8::MAX as usize) as u8
}

/// After a collector write, confirm SQLite matches parsed candidates (read-after-write).
fn declaration_store_verify_issues(
    candidates: &[Value],
    stored: &[IssuerDeclarationRecord],
) -> Vec<String> {
    use std::collections::HashMap;
    let mut issues = Vec::new();
    let mut period_counts: HashMap<String, u32> = HashMap::new();
    for d in stored {
        if d.amount_per_share_minor.map(|a| a > 0).unwrap_or(false) {
            *period_counts.entry(d.payment_period.clone()).or_default() += 1;
        }
    }
    for (period, count) in period_counts {
        if count > 1 {
            issues.push(format!("duplicate stored period {period} ({count} rows)"));
        }
    }
    for c in candidates {
        let period = jstr(c, "paymentPeriod").unwrap_or_default();
        if period.is_empty() {
            continue;
        }
        let amount = match c.get("amountPerShareMinor").and_then(|x| {
            if x.is_null() {
                None
            } else {
                ji64(c, "amountPerShareMinor")
            }
        }) {
            Some(a) if a > 0 => a,
            _ => continue,
        };
        let scale = ju8(c, "amountScale", 2);
        let rows: Vec<_> = stored
            .iter()
            .filter(|d| d.payment_period == period)
            .collect();
        if rows.is_empty() {
            issues.push(format!("missing stored declaration for {period}"));
            continue;
        }
        if !rows.iter().any(|d| {
            d.amount_per_share_minor == Some(amount) && d.amount_scale == scale
        }) {
            issues.push(format!(
                "amount mismatch for {period}: stored {:?}, expected {amount} scale {scale}",
                rows.iter()
                    .map(|d| (d.amount_per_share_minor, d.amount_scale))
                    .collect::<Vec<_>>()
            ));
        }
    }
    issues
}

struct DeclarationGate {
    ok: bool,
    code: String,
    message: String,
}

fn declaration_retrieve_gate(
    paid_count: u8,
    inception_on: &str,
    payment_frequency: &str,
    div_type: &str,
) -> DeclarationGate {
    if !financial_domain::div1::is_div1(div_type) {
        return DeclarationGate {
            ok: true,
            code: String::new(),
            message: String::new(),
        };
    }
    use financial_domain::declaration_lookback::{
        validate_paid_lookback, LookbackValidation, DECLARATION_LOOKBACK_TARGET,
    };
    let as_of = today_iso();
    match validate_paid_lookback(paid_count, inception_on, &as_of, payment_frequency) {
        LookbackValidation::Complete | LookbackValidation::CompleteViaInception { .. } => {
            DeclarationGate {
                ok: true,
                code: String::new(),
                message: "declaration retrieve complete".into(),
            }
        }
        LookbackValidation::ShortWithoutInception { paid } => DeclarationGate {
            ok: false,
            code: "declaration_lookback_short".into(),
            message: format!(
                "Adapter returned {paid} of {DECLARATION_LOOKBACK_TARGET} required paid declarations. Set optional inception on Settings only if this name is too new for a full lookback."
            ),
        },
        LookbackValidation::ShortWithInception { paid, expected } => DeclarationGate {
            ok: false,
            code: "declaration_lookback_short".into(),
            message: format!(
                "Adapter returned {paid} of {expected} paid declarations expected since inception {inception_on}."
            ),
        },
    }
}

fn required_paid_count(inception_on: &str, payment_frequency: &str) -> u8 {
    use financial_domain::declaration_lookback::{
        expected_declaration_lookback, DECLARATION_LOOKBACK_TARGET,
    };
    if inception_on.trim().is_empty() {
        DECLARATION_LOOKBACK_TARGET
    } else {
        expected_declaration_lookback(inception_on, &today_iso(), payment_frequency)
    }
}

async fn div1_compliance_summary(
    canonical: &dyn Canonical,
) -> Result<Div1ComplianceSummaryBody, PlatformError> {
    let set = canonical.collector_set().await?;
    let as_of = today_iso();
    let mut rows = Vec::new();
    for item in set.items {
        if !financial_domain::div1::is_div1(&item.div_type) {
            continue;
        }
        if !item.collector_enabled || item.declaration_source.trim().is_empty() {
            continue;
        }
        let decls = canonical.issuer_declaration_list(item.security_id).await?;
        let pay_dates = canonical.issuer_pay_date_list(item.security_id).await?;
        let future_pay_dates_qty = pay_dates
            .iter()
            .filter(|d| d.pay_on.as_str() > as_of.as_str())
            .count() as u64;
        let paid: Vec<_> = decls
            .iter()
            .filter(|d| d.amount_per_share_minor.map(|a| a > 0).unwrap_or(false))
            .collect();
        let prior_declarations_qty = paid.len() as u64;
        let latest = paid.iter().max_by(|a, b| a.payment_period.cmp(&b.payment_period));
        rows.push(Div1ComplianceSummaryRow {
            security_id: item.security_id,
            symbol: item.symbol,
            future_pay_dates_qty,
            prior_declarations_qty,
            current_declaration_amount_minor: latest.and_then(|d| d.amount_per_share_minor),
            current_declaration_amount_scale: latest.map(|d| d.amount_scale),
            current_declaration_date: latest
                .map(|d| d.payment_period.clone())
                .unwrap_or_default(),
            last_run_ok: item.last_run_ok,
            required_paid: required_paid_count(&item.inception_on, &item.payment_frequency),
        });
    }
    rows.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    Ok(Div1ComplianceSummaryBody { rows, as_of_date: as_of })
}

async fn data_summary_view(
    canonical: &dyn Canonical,
) -> Result<DataSummaryBody, PlatformError> {
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
    let mut qty: std::collections::HashMap<uuid::Uuid, (i64, u8)> =
        std::collections::HashMap::new();
    for lot in &basis.lots {
        if lot.remaining_quantity_minor <= 0 {
            continue;
        }
        let entry = qty.entry(lot.security_id).or_insert((0, lot.quantity_scale));
        let qty_scale = entry.1.max(lot.quantity_scale);
        entry.0 = financial_domain::money::rescale(entry.0, entry.1, qty_scale)
            + financial_domain::money::rescale(
                lot.remaining_quantity_minor,
                lot.quantity_scale,
                qty_scale,
            );
        entry.1 = qty_scale;
    }
    let today = today_iso();
    let mut last_price_count = 0u64;
    let mut market_value_minor = 0i64;
    let mut any_market_value = false;
    let mut market_value_complete = !qty.is_empty();
    for (security_id, (remaining_quantity_minor, quantity_scale)) in &qty {
        let price = canonical
            .current_price_get(*security_id, today.clone())
            .await?;
        if price.price_derived_valid {
            last_price_count += 1;
            if let Some(px) = price.price_minor {
                market_value_minor += financial_domain::calculator::plan_payment_cents(
                    *remaining_quantity_minor,
                    *quantity_scale,
                    px,
                    price.scale,
                );
                any_market_value = true;
            }
        } else {
            market_value_complete = false;
        }
    }
    if qty.is_empty() {
        market_value_complete = true;
    }
    Ok(DataSummaryBody {
        account_count: accounts
            .iter()
            .filter(|a| is_data_account(&a.name))
            .count() as u64,
        open_lot_count,
        yield_count: dividend.actuals.len() as u64,
        disbursement_count,
        latest_yield_on,
        plan_count: canonical.plan_history_list().await?.len() as u64,
        symbol_count: qty.len() as u64,
        open_performance_minor: basis.open_performance_minor,
        open_tax_minor: basis.open_tax_minor,
        last_price_count,
        market_value_minor: if any_market_value {
            Some(market_value_minor)
        } else {
            None
        },
        market_value_complete,
        income_earned_minor: dividend
            .actuals
            .iter()
            .map(|a| financial_domain::money::to_usd_cents(a.amount_minor, a.scale))
            .sum(),
        scale: 2,
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
        let qty_scale = entry.1.max(lot.quantity_scale);
        entry.0 = financial_domain::money::rescale(entry.0, entry.1, qty_scale)
            + financial_domain::money::rescale(
                lot.remaining_quantity_minor,
                lot.quantity_scale,
                qty_scale,
            );
        entry.1 = qty_scale;
        entry.2 += financial_domain::money::to_usd_cents(lot.remaining_performance_minor, lot.scale);
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
        let periods = ch
            .and_then(|c| cadence_periods(&c.payment_frequency))
            .unwrap_or(0);
        let (plan_per_share_minor, plan_scale) = plan
            .map(|p| (p.amount_per_share_minor, p.amount_scale))
            .unwrap_or((0, 2));
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
        let price = canonical
            .current_price_get(security.security_id, today_iso())
            .await?;
        let last_price_ok = price.price_derived_valid && price.price_minor.is_some();
        let market_value_minor = match (price.price_minor, remaining_quantity_minor > 0) {
            (Some(px), true) if price.price_derived_valid => Some(
                financial_domain::calculator::plan_payment_cents(
                    remaining_quantity_minor,
                    quantity_scale,
                    px,
                    price.scale,
                ),
            ),
            _ => None,
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
            roc_pct_2026_estimate_minor: ch.and_then(|c| c.roc_pct_2026_estimate_minor),
            roc_pct_2026_actual_minor: ch.and_then(|c| c.roc_pct_2026_actual_minor),
            roc_scale: ch.and_then(|c| c.roc_scale),
            last_price_minor: if last_price_ok { price.price_minor } else { None },
            last_price_scale: if last_price_ok { Some(price.scale) } else { None },
            price_freshness: price.freshness,
            market_value_minor,
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
    let chars = canonical.position_characteristic_list().await?;
    let freq = chars
        .iter()
        .find(|c| c.security_id == security_id)
        .map(|c| c.payment_frequency.as_str())
        .unwrap_or("");
    let cadence = locked_cadence(freq)?;
    if let Some(n) = ju8_opt(json, "planningPeriodsPerYear") {
        let matches = match cadence.periods() {
            Some(p) => n == p,
            None => n == 0,
        };
        if !matches {
            return Err(PlatformError::new(
                "payment_cadence_mismatch",
                "frequency and period count are one value; they cannot disagree",
            ));
        }
    }
    canonical
        .plan_history_confirm(
            security_id,
            ji64(json, "amountPerShareMinor").unwrap_or(0),
            ju8(json, "amountScale", 2),
            cadence.periods().unwrap_or(0),
            jstr(json, "effectiveFrom").unwrap_or_else(today_iso),
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
    let price = canonical.current_price_get(security_id, as_of.clone()).await?;
    let template = canonical.retrieval_template_get(security_id).await?;
    let decls = canonical.issuer_declaration_list(security_id).await?;
    let accounts = canonical.account_list().await?;
    let basis = canonical.basis_get().await?;
    let mut lots = Vec::new();
    let mut remaining_quantity_minor = 0i64;
    let mut quantity_scale = 0u8;
    let mut remaining_performance_minor = 0i64;
    let mut remaining_tax_minor = 0i64;
    for lot in &basis.lots {
        if lot.security_id != security_id || lot.remaining_quantity_minor <= 0 {
            continue;
        }
        let next_qty_scale = quantity_scale.max(lot.quantity_scale);
        remaining_quantity_minor = financial_domain::money::rescale(
            remaining_quantity_minor,
            quantity_scale,
            next_qty_scale,
        ) + financial_domain::money::rescale(
            lot.remaining_quantity_minor,
            lot.quantity_scale,
            next_qty_scale,
        );
        quantity_scale = next_qty_scale;
        remaining_performance_minor +=
            financial_domain::money::to_usd_cents(lot.remaining_performance_minor, lot.scale);
        remaining_tax_minor +=
            financial_domain::money::to_usd_cents(lot.remaining_tax_minor, lot.scale);
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
    let market_value_minor = match (price.price_minor, remaining_quantity_minor > 0) {
        (Some(px), true) if price.price_derived_valid => Some(
            financial_domain::calculator::plan_payment_cents(
                remaining_quantity_minor,
                quantity_scale,
                px,
                price.scale,
            ),
        ),
        _ => None,
    };
    let unrealized_performance_minor =
        market_value_minor.map(|mv| mv - remaining_performance_minor);
    let (annual_plan_minor, plan_yoc_bps, most_current_vs_plan_bps) = if let Some(plan) = plan {
        let period = financial_domain::calculator::plan_payment_cents(
            remaining_quantity_minor,
            quantity_scale,
            plan.amount_per_share_minor,
            plan.amount_scale,
        );
        let periods = ch.and_then(|c| cadence_periods(&c.payment_frequency)).unwrap_or(0);
        let annual = if remaining_quantity_minor > 0 && periods > 0 {
            Some(period.saturating_mul(periods as i64))
        } else {
            None
        };
        let yoc = annual.and_then(|yr| {
            if remaining_performance_minor > 0 {
                Some((yr.saturating_mul(10_000)) / remaining_performance_minor)
            } else {
                None
            }
        });
        let vs = review.most_current_minor.map(|mc| {
            let plan_as_review = scale_amount(
                plan.amount_per_share_minor,
                plan.amount_scale,
                review.amount_scale,
            );
            if plan_as_review == 0 {
                0
            } else {
                ((mc - plan_as_review).saturating_mul(10_000)) / plan_as_review
            }
        });
        (annual, yoc, vs)
    } else {
        (None, None, None)
    };
    let declarations = decls
        .iter()
        .map(|d| InvestmentDeclarationBody {
            payment_period: d.payment_period.clone(),
            amount_per_share_minor: d.amount_per_share_minor,
            amount_scale: d.amount_scale,
            source: d.source.clone(),
            entered_at: d.entered_at.clone(),
        })
        .collect();
    let results = canonical.position_backtest_result_list(security_id).await?;
    let periods = canonical.backtest_period_list().await?;
    let (evidence, suggestion) = match results.last() {
        Some(latest) => {
            let domain = financial_domain::regime::RegimeResult {
                price_return_bps: latest.price_return_bps,
                total_return_bps: latest.total_return_bps,
                cushion_bps: latest.cushion_bps,
                max_drawdown_bps: latest.max_drawdown_bps,
                recovery_ratio_bps: latest.recovery_ratio_bps,
                recovery_days: latest.recovery_days,
                income_reliability_bps: latest.income_reliability_bps,
                bear_relative_bps: latest.bear_relative_bps,
                downside_capture_bps: latest.downside_capture_bps,
                upside_capture_bps: latest.upside_capture_bps,
                completeness: latest.completeness.clone(),
            };
            let dim = financial_domain::regime::dimensions(&domain, None);
            let kind = periods
                .iter()
                .find(|p| p.period_id == latest.period_id)
                .map(|p| p.kind.as_str())
                .unwrap_or("");
            let sug = financial_domain::regime::suggest_tier(&dim, kind);
            (
                Some(EvidenceDimensionsBody {
                    income_reliability: dim.income_reliability,
                    downside_resilience: dim.downside_resilience,
                    recovery_upside: dim.recovery_upside,
                    nav_persistence: dim.nav_persistence,
                    diversification: dim.diversification,
                    data_confidence: dim.data_confidence,
                    known_components: dim.known_components,
                }),
                Some(TierSuggestionBody {
                    suggested_tier: sug.suggested_tier,
                    ruleset: sug.ruleset,
                    reason: sug.reason,
                    complete: sug.complete,
                }),
            )
        }
        None => (None, None),
    };
    let tax_handling = canonical
        .position_tax_profile_list()
        .await?
        .into_iter()
        .find(|t| t.security_id == security_id)
        .map(|t| t.expected_handling)
        .unwrap_or_default();
    let pattern = canonical
        .expected_payment_pattern_list()
        .await?
        .into_iter()
        .find(|p| p.security_id == security_id);
    let activities = canonical.activity_list().await?;
    let distributions = canonical.distribution_get().await?;
    let lifetime = lifetime_slice(
        security_id,
        &activities,
        &distributions.characterizations,
        &basis.lots,
    );
    let observations = canonical.roc_observation_list(security_id).await?;
    let car_id = car_account_id(&accounts);
    let mut data_mv = 0i64;
    let mut any_data = false;
    let mut seen = std::collections::HashSet::new();
    for lot in &basis.lots {
        if lot.remaining_quantity_minor <= 0 || !seen.insert(lot.security_id) {
            continue;
        }
        let px = canonical
            .current_price_get(lot.security_id, as_of.clone())
            .await?;
        let mut q = 0i64;
        let mut qs = 0u8;
        for other in &basis.lots {
            if other.security_id != lot.security_id || other.remaining_quantity_minor <= 0 {
                continue;
            }
            let scale = qs.max(other.quantity_scale);
            q = financial_domain::money::rescale(q, qs, scale)
                + financial_domain::money::rescale(
                    other.remaining_quantity_minor,
                    other.quantity_scale,
                    scale,
                );
            qs = scale;
        }
        if let (Some(p), true) = (px.price_minor, px.price_derived_valid) {
            data_mv += financial_domain::calculator::plan_payment_cents(q, qs, p, px.scale);
            any_data = true;
        }
    }
    let data_market_value_minor = if any_data { Some(data_mv) } else { None };
    let (car_mv, car_share_symbol, car_share_data) = car_metrics(
        security_id,
        &basis.lots,
        car_id,
        if price.price_derived_valid {
            price.price_minor
        } else {
            None
        },
        price.scale,
        price.price_derived_valid && price.price_minor.is_some(),
        market_value_minor,
        data_market_value_minor,
    );
    let roc_research_status = roc_status_for(
        ch.map(|c| c.needs_roc_research).unwrap_or(false),
        ch,
        &observations,
        has_open_car_lots(security_id, &basis.lots, car_id),
    );
    let declaration_freshness = freshness_for(&decls, &as_of, template.as_ref());
    let roc_est = latest_roc_estimate(&observations);
    let roc_research_completed_at =
        roc_research_completed_at(&roc_research_status, &observations);
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
        planning_periods_per_year: ch
            .and_then(|c| cadence_periods(&c.payment_frequency))
            .unwrap_or(0),
        plan_reason: plan.map(|p| p.decision_reason.clone()).unwrap_or_default(),
        plan_effective_from: plan.map(|p| p.effective_from.clone()).unwrap_or_default(),
        remaining_quantity_minor,
        quantity_scale,
        remaining_performance_minor,
        remaining_tax_minor,
        roc_pct_2024_actual_minor: ch.and_then(|c| c.roc_pct_2024_actual_minor),
        roc_pct_2025_actual_minor: ch.and_then(|c| c.roc_pct_2025_actual_minor),
        roc_pct_2026_estimate_minor: ch.and_then(|c| c.roc_pct_2026_estimate_minor),
        roc_pct_2026_actual_minor: ch.and_then(|c| c.roc_pct_2026_actual_minor),
        roc_scale: ch.and_then(|c| c.roc_scale),
        notes: ch.map(|c| c.notes.clone()).unwrap_or_default(),
        div_type: ch.map(|c| c.div_type.clone()).unwrap_or_default(),
        is_active: ch.map(|c| c.is_active).unwrap_or(true),
        needs_roc_research: ch.map(|c| c.needs_roc_research).unwrap_or(false),
        tax_handling,
        declaration_weekday: pattern
            .as_ref()
            .map(|p| p.declaration_weekday.clone())
            .unwrap_or_default(),
        exdate_weekday: pattern
            .as_ref()
            .map(|p| p.exdate_weekday.clone())
            .unwrap_or_default(),
        payday_weekday: pattern
            .as_ref()
            .map(|p| p.payday_weekday.clone())
            .unwrap_or_default(),
        unit_cost_minor: unit_cost_cents(
            remaining_performance_minor,
            remaining_quantity_minor,
            quantity_scale,
        ),
        plan_fwd_yield_bps: match (plan, price.price_minor, ch.and_then(|c| cadence_periods(&c.payment_frequency))) {
            (Some(p), Some(px), Some(periods)) if price.price_derived_valid && periods > 0 => {
                fwd_yield_bps(
                    p.amount_per_share_minor
                        .saturating_mul(periods as i64),
                    p.amount_scale,
                    px,
                    price.scale,
                )
            }
            _ => None,
        },
        most_current_fwd_yield_bps: match (
            review.most_current_minor,
            price.price_minor,
            ch.and_then(|c| cadence_periods(&c.payment_frequency)),
        ) {
            (Some(mc), Some(px), Some(periods)) if price.price_derived_valid && periods > 0 => {
                fwd_yield_bps(
                    mc.saturating_mul(periods as i64),
                    review.amount_scale,
                    px,
                    price.scale,
                )
            }
            _ => None,
        },
        unrealized_pnl_bps: market_value_minor.and_then(|v| {
            if remaining_performance_minor > 0 {
                Some(((v - remaining_performance_minor).saturating_mul(10_000)) / remaining_performance_minor)
            } else {
                None
            }
        }),
        price,
        review,
        template,
        lots,
        declarations,
        declaration_count: decls.len() as u64,
        first_lot_complete: remaining_quantity_minor > 0,
        market_value_minor,
        unrealized_performance_minor,
        annual_plan_minor,
        plan_yoc_bps,
        most_current_vs_plan_bps,
        periods,
        results,
        evidence,
        suggestion,
        total_distributions_received_minor: lifetime.total_distributions_received_minor,
        roc_distributions_minor: lifetime.roc_distributions_minor,
        cost_recovery_bps: lifetime.cost_recovery_bps,
        distributions_scope: lifetime.distributions_scope,
        car_market_value_minor: car_mv,
        car_share_of_symbol_bps: car_share_symbol,
        car_share_of_data_bps: car_share_data,
        roc_research_status,
        declaration_freshness,
        roc_estimate_method: roc_est.map(|o| o.method.clone()).unwrap_or_default(),
        roc_estimate_source_url: roc_est.map(|o| o.source_url.clone()).unwrap_or_default(),
        roc_estimate_as_of: roc_est.map(|o| o.as_of.clone()).unwrap_or_default(),
        roc_estimate_established_how: roc_est
            .map(|o| o.established_how.clone())
            .unwrap_or_default(),
        roc_research_completed_at,
        lookthrough: ch.map(|c| c.lookthrough.clone()).unwrap_or_default(),
        scale: 2,
    })
}

async fn position_details_summary(
    canonical: &dyn Canonical,
    json: &Value,
) -> Result<PositionDetailsBody, PlatformError> {
    let mut body = canonical.position_details_get().await?;
    let as_of = jstr(json, "asOfDate").unwrap_or_else(today_iso);
    let mut data_mv = 0i64;
    let mut mv_complete = true;
    let mut per_line_mv: std::collections::HashMap<(Uuid, Uuid), i64> =
        std::collections::HashMap::new();
    for line in &body.positions {
        let price = canonical
            .current_price_get(line.security_id, as_of.clone())
            .await?;
        match (price.price_minor, price.price_derived_valid) {
            (Some(px), true) => {
                let mv = financial_domain::calculator::plan_payment_cents(
                    line.remaining_quantity_minor,
                    line.quantity_scale,
                    px,
                    price.scale,
                );
                data_mv += mv;
                per_line_mv.insert((line.account_id, line.security_id), mv);
            }
            _ => {
                mv_complete = false;
            }
        }
    }
    let mut account_map: std::collections::BTreeMap<
        Uuid,
        AccountPositionTotalBody,
    > = std::collections::BTreeMap::new();
    for line in &body.positions {
        let entry = account_map.entry(line.account_id).or_insert(AccountPositionTotalBody {
            account_id: line.account_id,
            account_name: line.account_name.clone(),
            symbol_count: 0,
            open_lot_count: 0,
            open_performance_minor: 0,
            open_tax_minor: 0,
            market_value_minor: Some(0),
            scale: 2,
        });
        entry.symbol_count += 1;
        entry.open_lot_count += line.lot_count;
        entry.open_performance_minor += line.remaining_performance_minor;
        entry.open_tax_minor += line.remaining_tax_minor;
        match per_line_mv.get(&(line.account_id, line.security_id)) {
            Some(mv) => {
                if let Some(cur) = entry.market_value_minor.as_mut() {
                    *cur += *mv;
                }
            }
            None => entry.market_value_minor = None,
        }
    }
    for line in &mut body.positions {
        line.market_value_minor = per_line_mv
            .get(&(line.account_id, line.security_id))
            .copied();
    }
    body.account_totals = account_map.into_values().collect();
    body.market_value_minor = if mv_complete && !body.positions.is_empty() {
        Some(data_mv)
    } else if data_mv > 0 {
        Some(data_mv)
    } else {
        None
    };
    body.market_value_complete = mv_complete && !body.positions.is_empty();
    let basis = canonical.basis_get().await?;
    let line_qtys: Vec<(Uuid, i64, u8)> = body
        .positions
        .iter()
        .map(|l| (l.security_id, l.remaining_quantity_minor, l.quantity_scale))
        .collect();
    let lot_qtys: Vec<(Uuid, i64, u8)> = basis
        .lots
        .iter()
        .filter(|l| l.remaining_quantity_minor > 0)
        .map(|l| (l.security_id, l.remaining_quantity_minor, l.quantity_scale))
        .collect();
    match financial_domain::lifetime::account_qty_matches_open_lots(&line_qtys, &lot_qtys) {
        Ok(()) => body.qty_reconcile_ok = true,
        Err(_) => {
            body.qty_reconcile_ok = false;
            let _ = canonical
                .exception_raise(
                    "qty_reconcile_mismatch".into(),
                    "account quantities do not reconcile to open lots".into(),
                )
                .await;
        }
    }
    Ok(body)
}

fn empty_characteristic(security_id: Uuid) -> PositionCharacteristicRecord {
    PositionCharacteristicRecord {
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
        is_active: true,
        lookthrough: LookthroughResearch::default(),
    }
}

/// Process A: persist suggested frequency when paid history or issuer label supports 52/12/4.
/// Does not overwrite an already-set cadence. Does not confirm Plan or open lots.
async fn persist_inferred_frequency_if_unknown(
    canonical: &dyn Canonical,
    security_id: Uuid,
    page_label: Option<&str>,
) -> String {
    let existing = match canonical.position_characteristic_list().await {
        Ok(list) => list.into_iter().find(|c| c.security_id == security_id),
        Err(_) => None,
    };
    if let Some(ch) = existing.as_ref() {
        if financial_domain::calculator::PaymentCadence::parse(&ch.payment_frequency)
            .and_then(financial_domain::calculator::PaymentCadence::periods)
            .is_some()
        {
            return ch.payment_frequency.clone();
        }
    }
    let decls = canonical
        .issuer_declaration_list(security_id)
        .await
        .unwrap_or_default();
    let periods: Vec<String> = decls
        .iter()
        .filter(|d| d.amount_per_share_minor.unwrap_or(0) > 0)
        .map(|d| d.payment_period.clone())
        .collect();
    let period_refs: Vec<&str> = periods.iter().map(String::as_str).collect();
    let Some(cadence) =
        financial_domain::calculator::infer_payment_cadence(&period_refs, page_label)
    else {
        return existing
            .map(|c| c.payment_frequency)
            .unwrap_or_default();
    };
    let mut rec = existing.unwrap_or_else(|| empty_characteristic(security_id));
    rec.payment_frequency = cadence.label().to_string();
    let _ = canonical.position_characteristic_upsert(rec).await;
    cadence.label().to_string()
}

fn unit_cost_cents(cost_cents: i64, qty_minor: i64, qty_scale: u8) -> Option<i64> {
    if qty_minor <= 0 {
        return None;
    }
    Some(cost_cents.saturating_mul(10i64.pow(qty_scale as u32)) / qty_minor)
}

fn car_account_id(accounts: &[AccountRecord]) -> Option<Uuid> {
    accounts
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case("Car"))
        .map(|a| a.account_id)
}

fn is_roc_category(category: &str) -> bool {
    let c = category.to_ascii_lowercase();
    c == "roc" || c == "return_of_capital" || c.contains("return of capital")
}

struct LifetimeSlice {
    total_distributions_received_minor: Option<i64>,
    roc_distributions_minor: Option<i64>,
    cost_recovery_bps: Option<i64>,
    distributions_scope: String,
}

fn lifetime_slice(
    security_id: Uuid,
    activities: &[ActivityRecord],
    characterizations: &[DistributionRecord],
    lots: &[LotRecord],
) -> LifetimeSlice {
    let mut total = 0i64;
    let mut dividend_ids = Vec::new();
    for activity in activities {
        if activity.security_id != Some(security_id) {
            continue;
        }
        if !activity.activity_type.eq_ignore_ascii_case("dividend") {
            continue;
        }
        total += financial_domain::money::to_usd_cents(activity.amount_minor, activity.scale);
        dividend_ids.push(activity.activity_id);
    }
    let roc = characterizations
        .iter()
        .filter(|c| dividend_ids.contains(&c.activity_id) && is_roc_category(&c.category))
        .map(|c| financial_domain::money::to_usd_cents(c.amount_minor, c.scale))
        .sum::<i64>();
    let open = lots
        .iter()
        .any(|l| l.security_id == security_id && l.remaining_quantity_minor > 0);
    if open && dividend_ids.is_empty() {
        return LifetimeSlice {
            total_distributions_received_minor: None,
            roc_distributions_minor: None,
            cost_recovery_bps: None,
            distributions_scope: "incomplete".into(),
        };
    }
    if dividend_ids.is_empty() {
        return LifetimeSlice {
            total_distributions_received_minor: None,
            roc_distributions_minor: None,
            cost_recovery_bps: None,
            distributions_scope: "incomplete".into(),
        };
    }
    let original: i64 = lots
        .iter()
        .filter(|l| l.security_id == security_id)
        .map(|l| financial_domain::money::to_usd_cents(l.performance_basis_minor, l.scale))
        .sum();
    let cost_recovery = financial_domain::lifetime::cost_recovery_bps(total, original, false)
        .ok()
        .flatten();
    LifetimeSlice {
        total_distributions_received_minor: Some(total),
        roc_distributions_minor: Some(roc),
        cost_recovery_bps: cost_recovery,
        distributions_scope: "complete".into(),
    }
}

fn evidence_from_result(latest: &PositionBacktestResultBody) -> EvidenceDimensionsBody {
    let domain = financial_domain::regime::RegimeResult {
        price_return_bps: latest.price_return_bps,
        total_return_bps: latest.total_return_bps,
        cushion_bps: latest.cushion_bps,
        max_drawdown_bps: latest.max_drawdown_bps,
        recovery_ratio_bps: latest.recovery_ratio_bps,
        recovery_days: latest.recovery_days,
        income_reliability_bps: latest.income_reliability_bps,
        bear_relative_bps: latest.bear_relative_bps,
        downside_capture_bps: latest.downside_capture_bps,
        upside_capture_bps: latest.upside_capture_bps,
        completeness: latest.completeness.clone(),
    };
    let dim = financial_domain::regime::dimensions(&domain, None);
    EvidenceDimensionsBody {
        income_reliability: dim.income_reliability,
        downside_resilience: dim.downside_resilience,
        recovery_upside: dim.recovery_upside,
        nav_persistence: dim.nav_persistence,
        diversification: dim.diversification,
        data_confidence: dim.data_confidence,
        known_components: dim.known_components,
    }
}

fn car_metrics(
    security_id: Uuid,
    lots: &[LotRecord],
    car_id: Option<Uuid>,
    last_price_minor: Option<i64>,
    last_price_scale: u8,
    last_ok: bool,
    symbol_mv: Option<i64>,
    data_mv: Option<i64>,
) -> (Option<i64>, Option<i64>, Option<i64>) {
    let Some(car_id) = car_id else {
        // No account named Car — cannot compute Car metrics.
        return (None, None, None);
    };
    let mut qty = 0i64;
    let mut qty_scale = 0u8;
    let mut any = false;
    for lot in lots {
        if lot.account_id != car_id || lot.security_id != security_id || lot.remaining_quantity_minor <= 0
        {
            continue;
        }
        any = true;
        let scale = qty_scale.max(lot.quantity_scale);
        qty = financial_domain::money::rescale(qty, qty_scale, scale)
            + financial_domain::money::rescale(
                lot.remaining_quantity_minor,
                lot.quantity_scale,
                scale,
            );
        qty_scale = scale;
    }
    // Known empty: not held in Car → $0, not unknown.
    if !any {
        return (Some(0), Some(0), Some(0));
    }
    // Held in Car but last price missing/invalid → MV unknown (never invent $0).
    let mv = match (last_price_minor, last_ok) {
        (Some(px), true) => Some(financial_domain::calculator::plan_payment_cents(
            qty,
            qty_scale,
            px,
            last_price_scale,
        )),
        _ => None,
    };
    let share_symbol = match (mv, symbol_mv) {
        (Some(v), Some(s)) if s > 0 => Some((v.saturating_mul(10_000)) / s),
        (Some(0), _) => Some(0),
        _ => None,
    };
    let share_data = match (mv, data_mv) {
        (Some(v), Some(h)) if h > 0 => Some((v.saturating_mul(10_000)) / h),
        (Some(0), _) => Some(0),
        _ => None,
    };
    (mv, share_symbol, share_data)
}

fn roc_status_for(
    needs_roc: bool,
    ch: Option<&PositionCharacteristicRecord>,
    observations: &[RocResearchObservation],
    has_open_car: bool,
) -> String {
    let any_pct = ch
        .map(|c| {
            c.roc_pct_2024_actual_minor.is_some()
                || c.roc_pct_2025_actual_minor.is_some()
                || c.roc_pct_2026_estimate_minor.is_some()
                || c.roc_pct_2026_actual_minor.is_some()
        })
        .unwrap_or(false);
    let in_scope = financial_domain::lifetime::roc_in_scope(needs_roc, any_pct, has_open_car);
    let views: Vec<financial_domain::lifetime::RocObservationView<'_>> = observations
        .iter()
        .map(|o| financial_domain::lifetime::RocObservationView {
            tax_year: o.tax_year.as_str(),
            kind: o.kind.as_str(),
            source: o.source.as_str(),
            roc_pct_minor: o.roc_pct_minor,
            established_how: o.established_how.as_str(),
        })
        .collect();
    financial_domain::lifetime::roc_research_status(in_scope, &views).to_string()
}

fn freshness_for(
    decls: &[IssuerDeclarationRecord],
    today: &str,
    template: Option<&RetrievalTemplateRecord>,
) -> String {
    let stamps: Vec<&str> = decls
        .iter()
        .flat_map(|d| [d.entered_at.as_str(), d.payment_period.as_str()])
        .collect();
    financial_domain::lifetime::declaration_freshness(
        &stamps,
        today,
        template.map(|t| t.last_run_at.as_str()).unwrap_or(""),
        template.and_then(|t| t.last_run_ok),
    )
    .to_string()
}

fn latest_roc_estimate(observations: &[RocResearchObservation]) -> Option<&RocResearchObservation> {
    observations
        .iter()
        .filter(|o| {
            o.kind.eq_ignore_ascii_case("estimate")
                || o.source.eq_ignore_ascii_case("19a-1")
                || o.method.contains("19a-1")
                || o.method.contains("table-roc")
        })
        .max_by(|a, b| a.as_of.cmp(&b.as_of).then(a.recorded_at.cmp(&b.recorded_at)))
}

fn roc_research_completed_at(
    status: &str,
    observations: &[RocResearchObservation],
) -> Option<String> {
    if status != "complete" {
        return None;
    }
    observations
        .iter()
        .map(|o| o.recorded_at.as_str())
        .filter(|s| !s.is_empty())
        .max()
        .map(|s| s.to_string())
}

fn has_open_car_lots(security_id: Uuid, lots: &[LotRecord], car_id: Option<Uuid>) -> bool {
    let Some(car_id) = car_id else {
        return false;
    };
    lots.iter().any(|l| {
        l.account_id == car_id && l.security_id == security_id && l.remaining_quantity_minor > 0
    })
}

fn fwd_yield_bps(
    annual_per_share_minor: i64,
    annual_scale: u8,
    price_minor: i64,
    price_scale: u8,
) -> Option<i64> {
    let price_cents = financial_domain::money::to_usd_cents(price_minor, price_scale);
    if price_cents <= 0 {
        return None;
    }
    let annual_cents = financial_domain::money::to_usd_cents(annual_per_share_minor, annual_scale);
    Some(annual_cents.saturating_mul(10_000) / price_cents)
}

async fn position_master_view(
    canonical: &dyn Canonical,
) -> Result<PositionMasterGetBody, PlatformError> {
    let securities = canonical.security_list().await?;
    let characteristics = canonical.position_characteristic_list().await?;
    let plans = canonical.plan_history_list().await?;
    let basis = canonical.basis_get().await?;
    let periods = canonical.backtest_period_list().await?;
    let patterns = canonical.expected_payment_pattern_list().await?;
    let taxes = canonical.position_tax_profile_list().await?;
    let accounts = canonical.account_list().await?;
    let activities = canonical.activity_list().await?;
    let distributions = canonical.distribution_get().await?;
    let car_id = car_account_id(&accounts);
    let today = today_iso();
    let char_by: std::collections::HashMap<_, _> = characteristics
        .iter()
        .map(|c| (c.security_id, c))
        .collect();
    let plan_by: std::collections::HashMap<_, _> =
        plans.iter().map(|p| (p.security_id, p)).collect();
    let pattern_by: std::collections::HashMap<_, _> =
        patterns.iter().map(|p| (p.security_id, p)).collect();
    let tax_by: std::collections::HashMap<_, _> =
        taxes.iter().map(|t| (t.security_id, t)).collect();
    let mut qty: std::collections::HashMap<Uuid, (i64, u8, i64, i64)> =
        std::collections::HashMap::new();
    for lot in &basis.lots {
        if lot.remaining_quantity_minor <= 0 {
            continue;
        }
        let entry = qty
            .entry(lot.security_id)
            .or_insert((0, lot.quantity_scale, 0, 0));
        let qty_scale = entry.1.max(lot.quantity_scale);
        entry.0 = financial_domain::money::rescale(entry.0, entry.1, qty_scale)
            + financial_domain::money::rescale(
                lot.remaining_quantity_minor,
                lot.quantity_scale,
                qty_scale,
            );
        entry.1 = qty_scale;
        entry.2 += financial_domain::money::to_usd_cents(lot.remaining_performance_minor, lot.scale);
        entry.3 += financial_domain::money::to_usd_cents(lot.remaining_tax_minor, lot.scale);
    }
    let mut ids: std::collections::HashSet<Uuid> = qty.keys().copied().collect();
    ids.extend(char_by.keys().copied());
    let mut data_mv = 0i64;
    let mut any_mv = false;
    let mut mv_complete = !ids.is_empty();
    let mut staged: Vec<(Uuid, i64, u8, i64, i64, Option<i64>, CurrentPriceBody)> = Vec::new();
    for security_id in &ids {
        let (q, qs, cost, tax) = qty.get(security_id).copied().unwrap_or((0, 0, 0, 0));
        let price = canonical
            .current_price_get(*security_id, today.clone())
            .await?;
        let mv = match (price.price_minor, q > 0) {
            (Some(px), true) if price.price_derived_valid => Some(
                financial_domain::calculator::plan_payment_cents(q, qs, px, price.scale),
            ),
            _ => None,
        };
        if let Some(v) = mv {
            data_mv += v;
            any_mv = true;
        } else if q > 0 {
            mv_complete = false;
        }
        staged.push((*security_id, q, qs, cost, tax, mv, price));
    }
    let data_market_value_minor = if any_mv { Some(data_mv) } else { None };
    let mut rows = Vec::new();
    for (security_id, q, qs, cost, tax, mv, price) in staged {
        let security = securities
            .iter()
            .find(|s| s.security_id == security_id);
        let Some(security) = security else {
            continue;
        };
        let ch = char_by.get(&security_id).copied();
        let plan = plan_by.get(&security_id).copied();
        let pattern = pattern_by.get(&security_id).copied();
        let tax_profile = tax_by.get(&security_id).copied();
        let review = plan_review_view(canonical, security_id).await?;
        let decls = canonical.issuer_declaration_list(security_id).await?;
        let results = canonical.position_backtest_result_list(security_id).await?;
        let periods_per_year = ch.and_then(|c| cadence_periods(&c.payment_frequency)).unwrap_or(0);
        let annual_plan_minor = if let Some(plan) = plan {
            if q > 0 && periods_per_year > 0 {
                Some(
                    financial_domain::calculator::plan_payment_cents(
                        q,
                        qs,
                        plan.amount_per_share_minor,
                        plan.amount_scale,
                    )
                    .saturating_mul(periods_per_year as i64),
                )
            } else {
                None
            }
        } else {
            None
        };
        let plan_yoc_bps = annual_plan_minor.and_then(|yr| {
            if cost > 0 {
                Some((yr.saturating_mul(10_000)) / cost)
            } else {
                None
            }
        });
        let last_ok = price.price_derived_valid && price.price_minor.is_some();
        let plan_fwd_yield_bps = match (plan, price.price_minor) {
            (Some(p), Some(px)) if last_ok && periods_per_year > 0 => {
                let annual_share = p
                    .amount_per_share_minor
                    .saturating_mul(periods_per_year as i64);
                fwd_yield_bps(annual_share, p.amount_scale, px, price.scale)
            }
            _ => None,
        };
        let most_current_fwd_yield_bps = match (review.most_current_minor, price.price_minor) {
            (Some(mc), Some(px)) if last_ok && periods_per_year > 0 => {
                let annual_share = mc.saturating_mul(periods_per_year as i64);
                fwd_yield_bps(annual_share, review.amount_scale, px, price.scale)
            }
            _ => None,
        };
        let unrealized_pnl_bps = mv.and_then(|v| {
            if cost > 0 {
                Some(((v - cost).saturating_mul(10_000)) / cost)
            } else {
                None
            }
        });
        let allocation_bps = match (mv, data_market_value_minor) {
            (Some(v), Some(hh)) if hh > 0 => Some((v.saturating_mul(10_000)) / hh),
            _ => None,
        };
        let dated = |kind: &str| {
            periods.iter().any(|p| {
                p.kind.eq_ignore_ascii_case(kind)
                    && !p.start_on.is_empty()
                    && !p.end_on.is_empty()
                    && !p.method.is_empty()
            })
        };
        let latest_for = |kind: &str| -> Option<&PositionBacktestResultBody> {
            if !dated(kind) {
                return None;
            }
            let period_ids: std::collections::HashSet<_> = periods
                .iter()
                .filter(|p| p.kind.eq_ignore_ascii_case(kind) && !p.start_on.is_empty())
                .map(|p| p.period_id)
                .collect();
            results
                .iter()
                .rev()
                .find(|r| period_ids.contains(&r.period_id))
        };
        let bear = latest_for("Bear");
        let bull = latest_for("Bull");
        let evidence = results.last().map(evidence_from_result);
        let lifetime = lifetime_slice(
            security_id,
            &activities,
            &distributions.characterizations,
            &basis.lots,
        );
        let (car_mv, car_share_symbol, car_share_data) = car_metrics(
            security_id,
            &basis.lots,
            car_id,
            if last_ok { price.price_minor } else { None },
            price.scale,
            last_ok,
            mv,
            data_market_value_minor,
        );
        let observations = canonical.roc_observation_list(security_id).await?;
        let template = canonical.retrieval_template_get(security_id).await?;
        let roc_research_status = roc_status_for(
            ch.map(|c| c.needs_roc_research).unwrap_or(false),
            ch,
            &observations,
            has_open_car_lots(security_id, &basis.lots, car_id),
        );
        let declaration_freshness = freshness_for(&decls, &today, template.as_ref());
        let roc_known = ch.map(|c| {
            c.roc_pct_2025_actual_minor.is_some()
                || c.roc_pct_2026_estimate_minor.is_some()
                || c.roc_pct_2026_actual_minor.is_some()
                || c.roc_pct_2024_actual_minor.is_some()
        })
        .unwrap_or(false);
        let period_dated = dated("Bull") || dated("Bear");
        let completeness = format!(
            "price:{};plan:{};decl:{};roc:{};period:{}",
            if last_ok {
                price.freshness.as_str()
            } else {
                "unavailable"
            },
            if plan.is_some() { "yes" } else { "no" },
            decls.len(),
            if roc_known { "known" } else { "unknown" },
            if period_dated { "dated" } else { "none" },
        );
        rows.push(PositionMasterRowBody {
            security_id,
            symbol: security.symbol.clone(),
            name: security.name.clone(),
            risk_tier: ch.map(|c| c.risk_tier.clone()).unwrap_or_default(),
            provider: ch.map(|c| c.provider.clone()).unwrap_or_default(),
            underlying: ch.map(|c| c.underlying.clone()).unwrap_or_default(),
            payment_frequency: ch.map(|c| c.payment_frequency.clone()).unwrap_or_default(),
            div_type: ch.map(|c| c.div_type.clone()).unwrap_or_default(),
            needs_roc_research: ch.map(|c| c.needs_roc_research).unwrap_or(false),
            is_active: ch.map(|c| c.is_active).unwrap_or(true),
            notes: ch.map(|c| c.notes.clone()).unwrap_or_default(),
            tax_handling: tax_profile
                .map(|t| t.expected_handling.clone())
                .unwrap_or_default(),
            declaration_weekday: pattern
                .map(|p| p.declaration_weekday.clone())
                .unwrap_or_default(),
            exdate_weekday: pattern
                .map(|p| p.exdate_weekday.clone())
                .unwrap_or_default(),
            payday_weekday: pattern
                .map(|p| p.payday_weekday.clone())
                .unwrap_or_default(),
            remaining_quantity_minor: q,
            quantity_scale: qs,
            remaining_performance_minor: cost,
            remaining_tax_minor: tax,
            unit_cost_minor: unit_cost_cents(cost, q, qs),
            last_price_minor: if last_ok { price.price_minor } else { None },
            last_price_scale: if last_ok { Some(price.scale) } else { None },
            price_freshness: price.freshness.clone(),
            price_derived_valid: price.price_derived_valid,
            market_value_minor: mv,
            allocation_bps,
            plan_known: plan.is_some(),
            plan_per_share_minor: plan.map(|p| p.amount_per_share_minor).unwrap_or(0),
            plan_scale: plan.map(|p| p.amount_scale).unwrap_or(2),
            annual_plan_minor,
            plan_yoc_bps,
            plan_fwd_yield_bps,
            most_current_fwd_yield_bps,
            unrealized_pnl_bps,
            roc_pct_2024_actual_minor: ch.and_then(|c| c.roc_pct_2024_actual_minor),
            roc_pct_2025_actual_minor: ch.and_then(|c| c.roc_pct_2025_actual_minor),
            roc_pct_2026_estimate_minor: ch.and_then(|c| c.roc_pct_2026_estimate_minor),
            roc_pct_2026_actual_minor: ch.and_then(|c| c.roc_pct_2026_actual_minor),
            roc_scale: ch.and_then(|c| c.roc_scale),
            declaration_count: decls.len() as u64,
            period_dated,
            bear_price_return_bps: bear.and_then(|r| r.price_return_bps),
            bear_total_return_bps: bear.and_then(|r| r.total_return_bps),
            bull_total_return_bps: bull.and_then(|r| r.total_return_bps),
            completeness,
            total_distributions_received_minor: lifetime.total_distributions_received_minor,
            roc_distributions_minor: lifetime.roc_distributions_minor,
            cost_recovery_bps: lifetime.cost_recovery_bps,
            distributions_scope: lifetime.distributions_scope,
            evidence,
            bear_cushion_bps: bear.and_then(|r| r.cushion_bps),
            bull_price_return_bps: bull.and_then(|r| r.price_return_bps),
            bull_cushion_bps: bull.and_then(|r| r.cushion_bps),
            car_market_value_minor: car_mv,
            car_share_of_symbol_bps: car_share_symbol,
            car_share_of_data_bps: car_share_data,
            roc_research_status,
            declaration_freshness,
            scale: 2,
        });
    }
    rows.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    Ok(PositionMasterGetBody {
        rows,
        data_market_value_minor,
        market_value_complete: mv_complete && any_mv,
        scale: 2,
    })
}

async fn position_details_coverage(
    canonical: &dyn Canonical,
) -> Result<PositionDetailsCoverageBody, PlatformError> {
    let securities = canonical.security_list().await?;
    let characteristics = canonical.position_characteristic_list().await?;
    let basis = canonical.basis_get().await?;
    let accounts = canonical.account_list().await?;
    let activities = canonical.activity_list().await?;
    let distributions = canonical.distribution_get().await?;
    let car_id = car_account_id(&accounts);
    let today = today_iso();
    let char_by: std::collections::HashMap<_, _> = characteristics
        .iter()
        .map(|c| (c.security_id, c))
        .collect();
    let mut ids: std::collections::HashSet<Uuid> = char_by.keys().copied().collect();
    for lot in &basis.lots {
        if lot.remaining_quantity_minor > 0 {
            ids.insert(lot.security_id);
        }
    }
    let mut rows = Vec::new();
    for security_id in ids {
        let Some(security) = securities.iter().find(|s| s.security_id == security_id) else {
            continue;
        };
        let ch = char_by.get(&security_id).copied();
        let open_lots = basis
            .lots
            .iter()
            .any(|l| l.security_id == security_id && l.remaining_quantity_minor > 0);
        let had_lots = basis.lots.iter().any(|l| l.security_id == security_id);
        let is_active = ch.map(|c| c.is_active).unwrap_or(true);
        let decls = canonical.issuer_declaration_list(security_id).await?;
        let observations = canonical.roc_observation_list(security_id).await?;
        let template = canonical.retrieval_template_get(security_id).await?;
        let lifetime = lifetime_slice(
            security_id,
            &activities,
            &distributions.characterizations,
            &basis.lots,
        );
        rows.push(PositionDetailsCoverageRow {
            security_id,
            symbol: security.symbol.clone(),
            active: is_active,
            open_lots,
            recorded_status: financial_domain::lifetime::recorded_status(
                is_active,
                open_lots,
                had_lots,
                ch.is_some(),
            )
            .to_string(),
            roc_research_status: roc_status_for(
                ch.map(|c| c.needs_roc_research).unwrap_or(false),
                ch,
                &observations,
                has_open_car_lots(security_id, &basis.lots, car_id),
            ),
            declaration_freshness: freshness_for(&decls, &today, template.as_ref()),
            distributions_scope: lifetime.distributions_scope,
            declaration_source: template
                .as_ref()
                .map(|t| t.declaration_source.clone())
                .unwrap_or_default(),
            last_run_at: template
                .as_ref()
                .map(|t| t.last_run_at.clone())
                .unwrap_or_default(),
            last_run_ok: template.as_ref().and_then(|t| t.last_run_ok),
            last_run_message: template
                .as_ref()
                .map(|t| t.last_run_message.clone())
                .unwrap_or_default(),
        });
    }
    rows.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    Ok(PositionDetailsCoverageBody { rows })
}

fn scale_amount(amount: i64, from_scale: u8, to_scale: u8) -> i64 {
    if from_scale == to_scale {
        return amount;
    }
    if from_scale < to_scale {
        amount.saturating_mul(10i64.pow((to_scale - from_scale) as u32))
    } else {
        amount / 10i64.pow((from_scale - to_scale) as u32)
    }
}

fn parse_price_points(json: &Value, key: &str) -> Vec<financial_domain::regime::PricePoint> {
    json.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let on = item
                        .get("asOfAt")
                        .or_else(|| item.get("on"))
                        .and_then(|s| s.as_str())?
                        .to_string();
                    let price_minor = item.get("priceMinor").and_then(|p| {
                        p.as_i64()
                            .or_else(|| p.as_u64().map(|n| n as i64))
                            .or_else(|| p.as_str().and_then(|s| s.parse().ok()))
                    })?;
                    if on.is_empty() {
                        return None;
                    }
                    Some(financial_domain::regime::PricePoint { on, price_minor })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn date_in_window(on: &str, start: &str, end: &str) -> bool {
    let d = if on.len() >= 10 { &on[..10] } else { on };
    let s = if start.len() >= 10 { &start[..10] } else { start };
    let e = if end.len() >= 10 { &end[..10] } else { end };
    d >= s && d <= e
}

async fn position_backtest_calculate(
    canonical: &dyn Canonical,
    security_id: Uuid,
    period_id: Uuid,
    json: &Value,
) -> Result<PositionBacktestResultBody, PlatformError> {
    let period = canonical.backtest_period_get(period_id).await?;
    if !financial_domain::regime::period_ready(
        &period.start_on,
        &period.end_on,
        &period.kind,
    ) {
        return Err(PlatformError::new(
            "regime_period_incomplete",
            "owner period is missing dates, kind, method, or reason",
        ));
    }
    let prices = parse_price_points(json, "candidates");
    let benchmark_prices = parse_price_points(json, "benchmarkCandidates");
    let basis = canonical.basis_get().await?;
    let mut remaining_quantity_minor = 0i64;
    let mut quantity_scale = 0u8;
    for lot in &basis.lots {
        if lot.security_id == security_id && lot.remaining_quantity_minor > 0 {
            remaining_quantity_minor += lot.remaining_quantity_minor;
            quantity_scale = lot.quantity_scale;
        }
    }
    let decls = canonical.issuer_declaration_list(security_id).await?;
    let decl_cash: i64 = decls
        .iter()
        .filter(|d| date_in_window(&d.payment_period, &period.start_on, &period.end_on))
        .filter_map(|d| {
            d.amount_per_share_minor.map(|px| {
                financial_domain::calculator::plan_payment_cents(
                    remaining_quantity_minor,
                    quantity_scale,
                    px,
                    d.amount_scale,
                )
            })
        })
        .sum();
    let decl_count = decls
        .iter()
        .filter(|d| date_in_window(&d.payment_period, &period.start_on, &period.end_on))
        .count();
    let dividend = canonical.dividend_get().await?;
    let actual_cash: i64 = dividend
        .actuals
        .iter()
        .filter(|a| {
            a.security_id == Some(security_id)
                && date_in_window(&a.occurred_on, &period.start_on, &period.end_on)
        })
        .map(|a| a.amount_minor)
        .sum();
    let actual_count = dividend
        .actuals
        .iter()
        .filter(|a| {
            a.security_id == Some(security_id)
                && date_in_window(&a.occurred_on, &period.start_on, &period.end_on)
        })
        .count();
    let distribution_minor = if json_has(json, "distributionMinor") {
        ji64(json, "distributionMinor")
    } else if actual_count > 0 {
        Some(actual_cash)
    } else if decl_count > 0 {
        Some(decl_cash)
    } else {
        None
    };
    let plans = canonical.plan_history_list().await?;
    let plan = plans.iter().find(|p| p.security_id == security_id);
    let chars = canonical.position_characteristic_list().await?;
    let periods = chars
        .iter()
        .find(|c| c.security_id == security_id)
        .and_then(|c| cadence_periods(&c.payment_frequency))
        .unwrap_or(0);
    let planned_income_minor = if json_has(json, "plannedIncomeMinor") {
        ji64(json, "plannedIncomeMinor")
    } else if let Some(plan) = plan {
        financial_domain::regime::expected_observation_count(
            &period.start_on,
            &period.end_on,
            periods,
        )
        .map(|n| {
            financial_domain::calculator::plan_payment_cents(
                remaining_quantity_minor,
                quantity_scale,
                plan.amount_per_share_minor,
                plan.amount_scale,
            )
            .saturating_mul(n)
        })
    } else {
        None
    };
    let observed_income_minor = if json_has(json, "observedIncomeMinor") {
        ji64(json, "observedIncomeMinor")
    } else if decl_count > 0 {
        Some(decl_cash)
    } else if actual_count > 0 {
        Some(actual_cash)
    } else {
        None
    };
    let inputs = financial_domain::regime::RegimeInputs {
        start_on: period.start_on.clone(),
        end_on: period.end_on.clone(),
        kind: period.kind.clone(),
        method: period.method.clone(),
        reason: period.selection_reason.clone(),
        prices,
        benchmark_prices,
        distribution_minor,
        planned_income_minor,
        observed_income_minor,
    };
    let domain = financial_domain::regime::calculate(&inputs).map_err(|err| {
        PlatformError::new(
            match err {
                financial_domain::error::DomainError::RegimePeriodIncomplete => {
                    "regime_period_incomplete"
                }
                _ => "regime_calculate_failed",
            },
            err.to_string(),
        )
    })?;
    let record = PositionBacktestResultBody {
        result_id: Uuid::new_v4(),
        security_id,
        period_id,
        price_return_bps: domain.price_return_bps,
        total_return_bps: domain.total_return_bps,
        cushion_bps: domain.cushion_bps,
        max_drawdown_bps: domain.max_drawdown_bps,
        recovery_ratio_bps: domain.recovery_ratio_bps,
        recovery_days: domain.recovery_days,
        income_reliability_bps: domain.income_reliability_bps,
        bear_relative_bps: domain.bear_relative_bps,
        downside_capture_bps: domain.downside_capture_bps,
        upside_capture_bps: domain.upside_capture_bps,
        completeness: domain.completeness,
        source: period.method,
        calculated_at: jstr(json, "calculatedAt").unwrap_or_else(today_iso),
    };
    canonical.position_backtest_result_record(record).await
}

async fn classification_suggest(
    canonical: &dyn Canonical,
    security_id: Uuid,
) -> Result<TierSuggestionBody, PlatformError> {
    let results = canonical.position_backtest_result_list(security_id).await?;
    let Some(latest) = results.last() else {
        return Ok(TierSuggestionBody {
            suggested_tier: String::new(),
            ruleset: financial_domain::regime::TIER_RULESET.into(),
            reason: "no calculated window".into(),
            complete: false,
        });
    };
    let domain = financial_domain::regime::RegimeResult {
        price_return_bps: latest.price_return_bps,
        total_return_bps: latest.total_return_bps,
        cushion_bps: latest.cushion_bps,
        max_drawdown_bps: latest.max_drawdown_bps,
        recovery_ratio_bps: latest.recovery_ratio_bps,
        recovery_days: latest.recovery_days,
        income_reliability_bps: latest.income_reliability_bps,
        bear_relative_bps: latest.bear_relative_bps,
        downside_capture_bps: latest.downside_capture_bps,
        upside_capture_bps: latest.upside_capture_bps,
        completeness: latest.completeness.clone(),
    };
    let dim = financial_domain::regime::dimensions(&domain, None);
    let periods = canonical.backtest_period_list().await?;
    let kind = periods
        .iter()
        .find(|p| p.period_id == latest.period_id)
        .map(|p| p.kind.as_str())
        .unwrap_or("");
    let sug = financial_domain::regime::suggest_tier(&dim, kind);
    Ok(TierSuggestionBody {
        suggested_tier: sug.suggested_tier,
        ruleset: sug.ruleset,
        reason: sug.reason,
        complete: sug.complete,
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

async fn remaining_year_income_view(
    canonical: &dyn Canonical,
    security_id: Uuid,
    json: &Value,
) -> Result<RemainingYearIncomeBody, PlatformError> {
    let as_of = jstr(json, "asOfDate").unwrap_or_else(today_iso);
    let chars = canonical.position_characteristic_list().await?;
    let freq = chars
        .iter()
        .find(|c| c.security_id == security_id)
        .map(|c| c.payment_frequency.clone())
        .unwrap_or_default();
    let periods = cadence_periods(&freq).unwrap_or(0);
    let plans = canonical.plan_history_list().await?;
    let plan = plans.iter().find(|p| p.security_id == security_id);
    let plan_known = plan.is_some();
    let (plan_minor, plan_scale) = plan
        .map(|p| (p.amount_per_share_minor, p.amount_scale))
        .unwrap_or((0, 2));
    let basis = canonical.basis_get().await?;
    let mut existing_qty = 0i64;
    let mut qty_scale = ju8(json, "quantityScale", 0);
    for lot in &basis.lots {
        if lot.security_id == security_id && lot.remaining_quantity_minor > 0 {
            existing_qty += lot.remaining_quantity_minor;
            qty_scale = lot.quantity_scale;
        }
    }
    let this_lot_qty = ji64(json, "thisLotQuantityMinor").or_else(|| ji64(json, "quantityMinor"));
    let this_lot_opened = jstr(json, "thisLotOpenedOn")
        .or_else(|| jstr(json, "openedOn"))
        .unwrap_or_else(|| as_of.clone());
    let position_qty = existing_qty.saturating_add(this_lot_qty.unwrap_or(0));
    let latest = latest_declaration_period(canonical, security_id).await?;
    let stored = stored_date_overrides(canonical, security_id).await?;
    let overrides = merge_date_overrides(stored, draft_date_overrides(json));
    let issuer = canonical.issuer_pay_date_list(security_id).await?;
    let issuer_pay_ons: Vec<String> = issuer.into_iter().map(|d| d.pay_on).collect();
    let template = canonical.retrieval_template_get(security_id).await?;
    let policy = financial_domain::schedule::CalendarPolicy::resolve(
        template.as_ref().map(|t| t.calendar_policy.as_str()).unwrap_or(""),
        periods,
        issuer_pay_ons.len(),
    );
    let existing_lots: Vec<financial_domain::schedule::OpenLotQty> = basis
        .lots
        .iter()
        .filter(|l| l.security_id == security_id && l.remaining_quantity_minor > 0)
        .map(|l| financial_domain::schedule::OpenLotQty {
            opened_on: l.opened_on.clone(),
            quantity_minor: l.remaining_quantity_minor,
            quantity_scale: l.quantity_scale,
        })
        .collect();
    let this_lot = this_lot_qty.and_then(|q| {
        (q > 0).then(|| financial_domain::schedule::OpenLotQty {
            opened_on: this_lot_opened.clone(),
            quantity_minor: q,
            quantity_scale: qty_scale,
        })
    });
    let mut position_lots = existing_lots.clone();
    if let Some(lot) = this_lot.clone() {
        position_lots.push(lot);
    }
    let spec = |lots: &[financial_domain::schedule::OpenLotQty]| {
        financial_domain::schedule::remaining_year_from_spec(financial_domain::schedule::RemainingYearSpec {
            as_of: &as_of,
            latest_payment_period: latest.as_deref(),
            periods_per_year: periods,
            lots,
            plan_minor,
            plan_scale,
            overrides: &overrides,
            issuer_pay_ons: &issuer_pay_ons,
            calendar_policy: policy,
        })
    };
    let schedule = spec(&position_lots);
    let lot_schedule = this_lot.as_ref().filter(|_| plan_known).map(|lot| spec(std::slice::from_ref(lot)));
    let cash_ok = plan_known && position_qty > 0;
    let lot_ok = plan_known && this_lot_qty.unwrap_or(0) > 0;
    let payments = schedule
        .payments
        .iter()
        .map(|p| RemainingPaymentBody {
            pay_on: p.pay_on.clone(),
            original_pay_on: p.original_pay_on.clone(),
            month: p.month.clone(),
            cash_minor: cash_ok.then_some(p.cash_minor),
            this_lot_cash_minor: lot_schedule.as_ref().and_then(|s| {
                lot_ok.then_some(
                    s.payments
                        .iter()
                        .find(|lp| lp.pay_on == p.pay_on)
                        .map(|lp| lp.cash_minor)
                        .unwrap_or(0),
                )
            }),
            position_after_cash_minor: cash_ok.then_some(p.cash_minor),
            owner_override: p.owner_override,
            date_provenance: p.date_provenance.clone(),
        })
        .collect();
    let months = schedule
        .months
        .iter()
        .map(|m| RemainingMonthBody {
            month: m.month.clone(),
            cash_minor: cash_ok.then_some(m.cash_minor),
            this_lot_cash_minor: lot_schedule.as_ref().and_then(|s| {
                lot_ok.then_some(
                    s.months
                        .iter()
                        .find(|lm| lm.month == m.month)
                        .map(|lm| lm.cash_minor)
                        .unwrap_or(0),
                )
            }),
            position_after_cash_minor: cash_ok.then_some(m.cash_minor),
        })
        .collect();
    Ok(RemainingYearIncomeBody {
        security_id,
        as_of_date: as_of,
        known: schedule.known,
        provenance: schedule.provenance,
        payment_frequency: if freq.is_empty() {
            financial_domain::schedule::frequency_label(periods).to_string()
        } else {
            freq
        },
        latest_declaration_period: latest,
        remaining_periods: schedule.remaining_periods,
        year_to_go_minor: cash_ok.then_some(schedule.year_to_go_minor.unwrap_or(0)),
        this_lot_year_to_go_minor: lot_schedule
            .as_ref()
            .and_then(|s| lot_ok.then_some(s.year_to_go_minor.unwrap_or(0))),
        position_after_year_to_go_minor: cash_ok.then_some(schedule.year_to_go_minor.unwrap_or(0)),
        existing_quantity_minor: existing_qty,
        this_lot_quantity_minor: this_lot_qty,
        hypothetical: existing_qty == 0 || this_lot_qty.is_some(),
        plan_known,
        payments,
        months,
        orphaned_overrides: schedule
            .orphaned_overrides
            .iter()
            .map(|o| RemainingPaymentDateOverride {
                override_id: Uuid::nil(),
                security_id,
                original_pay_on: o.original_pay_on.clone(),
                pay_on: o.pay_on.clone(),
                recorded_at: String::new(),
            })
            .collect(),
        calendar_policy: policy.label().to_string(),
        scale: 2,
    })
}

async fn roc_research_view(
    canonical: &dyn Canonical,
    security_id: Option<Uuid>,
    account_id: Option<Uuid>,
    as_of: String,
    injected: &[RocCandidateBody],
) -> Result<RocResearchBody, PlatformError> {
    let mut candidates = injected.to_vec();
    let mut observations = Vec::new();
    let mut history: Vec<Option<i64>> = Vec::new();
    let mut scale = financial_domain::roc::ROC_PCT_SCALE;
    let mut confirmed: Option<i64> = None;
    let mut needs_roc_research = false;
    if let Some(security_id) = security_id {
        observations = canonical.roc_observation_list(security_id).await?;
        for obs in &observations {
            candidates.push(candidate_from_observation(obs));
        }
        let chars = canonical.position_characteristic_list().await?;
        if let Some(ch) = chars.iter().find(|c| c.security_id == security_id) {
            scale = ch.roc_scale.unwrap_or(scale);
            needs_roc_research = ch.needs_roc_research;
            // Proposed Process A 19a-1 estimate keeps needs_roc_research=true — not confirmed.
            if !ch.needs_roc_research {
                confirmed = ch.roc_pct_2026_estimate_minor;
            }
            history.push(ch.roc_pct_2025_actual_minor);
            history.push(ch.roc_pct_2024_actual_minor);
            if let Some(pct) = ch.roc_pct_2026_estimate_minor {
                candidates.push(stored_year_candidate(pct, scale, "2026", "stored-2026-estimate"));
            }
            if let Some(pct) = ch.roc_pct_2025_actual_minor {
                candidates.push(stored_year_candidate(pct, scale, "2025", "stored-2025-actual"));
            }
            if let Some(pct) = ch.roc_pct_2024_actual_minor {
                candidates.push(stored_year_candidate(pct, scale, "2024", "stored-2024-actual"));
            }
        }
    }
    let system = candidates.iter().find(|c| is_system_19a1(c)).cloned();
    let owner_override = observations.iter().any(|o| o.owner_override)
        || injected.iter().any(|c| c.owner_override);
    let sug = if let Some(pct) = confirmed {
        let source = if owner_override {
            "owner-override".into()
        } else if let Some(sys) = &system {
            sys.source.clone()
        } else {
            "stored-2026-estimate".into()
        };
        financial_domain::roc::RocSuggestion {
            roc_pct_minor: Some(pct),
            scale: system.as_ref().map(|s| s.scale).unwrap_or(scale),
            source,
            complete: true,
            reason: if owner_override {
                "owner override of system 19a-1/1099 percent".into()
            } else {
                "confirmed working percent; owner may override".into()
            },
        }
    } else if let Some(sys) = &system {
        let mut sug = financial_domain::roc::suggest_current_year(
            sys.roc_pct_minor.unwrap_or(0),
            sys.scale,
            sys.source.clone(),
            if sys.established_how.is_empty() {
                "current-year 19a-1 estimate".into()
            } else {
                sys.established_how.clone()
            },
        );
        // Process A proposes estimate only — not research-complete, not 1099 actual.
        if needs_roc_research {
            sug.complete = false;
            sug.reason = format!(
                "{}; proposed only — not research-complete, not 1099 actual",
                if sys.established_how.is_empty() {
                    "current-year 19a-1 estimate"
                } else {
                    sys.established_how.as_str()
                }
            );
        }
        sug
    } else {
        financial_domain::roc::suggest_from_history(&history, scale)
    };
    let mut remaining_periods = None;
    let mut remaining_total_minor = None;
    let mut remaining_ordinary_minor = None;
    let mut remaining_roc_minor = None;
    let mut magi_eligible = false;
    if let Some(security_id) = security_id {
        if let Some(account_id) = account_id {
            let account = canonical.account_get(account_id).await?;
            magi_eligible = financial_domain::roc::account_is_car_magi(&account.name);
        }
        let plans = canonical.plan_history_list().await?;
        let plan = plans.iter().find(|p| p.security_id == security_id);
        let basis = canonical.basis_get().await?;
        let mut qty = 0i64;
        let mut qscale = 0u8;
        for lot in &basis.lots {
            if lot.security_id == security_id && lot.remaining_quantity_minor > 0 {
                qty += lot.remaining_quantity_minor;
                qscale = lot.quantity_scale;
            }
        }
        if let Some(plan) = plan {
            let chars = canonical.position_characteristic_list().await?;
            let periods = chars
                .iter()
                .find(|c| c.security_id == security_id)
                .and_then(|c| cadence_periods(&c.payment_frequency))
                .unwrap_or(0);
            let latest = latest_declaration_period(canonical, security_id).await?;
            let overrides = stored_date_overrides(canonical, security_id).await?;
            if let Some(year) = financial_domain::roc::remaining_year_plan(
                &as_of,
                periods,
                qty,
                qscale,
                plan.amount_per_share_minor,
                plan.amount_scale,
                sug.roc_pct_minor,
                sug.scale,
                latest.as_deref(),
                &overrides,
            ) {
                remaining_periods = Some(year.remaining_periods);
                remaining_total_minor = Some(year.total_minor);
                remaining_ordinary_minor = year.ordinary_minor;
                remaining_roc_minor = year.roc_minor;
            }
        }
    }
    Ok(RocResearchBody {
        security_id,
        roc_pct_minor: sug.roc_pct_minor,
        scale: sug.scale,
        source: sug.source,
        complete: sug.complete,
        reason: sug.reason,
        candidates,
        remaining_periods,
        remaining_total_minor,
        remaining_ordinary_minor,
        remaining_roc_minor,
        magi_eligible,
        system_roc_pct_minor: system.as_ref().and_then(|s| s.roc_pct_minor),
        source_url: system.as_ref().map(|s| s.source_url.clone()).unwrap_or_default(),
        method: system.as_ref().map(|s| s.method.clone()).unwrap_or_default(),
        as_of: system.as_ref().map(|s| s.as_of.clone()).unwrap_or_default(),
        kind: system.as_ref().map(|s| s.kind.clone()).unwrap_or_default(),
        established_how: system
            .as_ref()
            .map(|s| s.established_how.clone())
            .unwrap_or_default(),
        owner_override,
        observations,
    })
}

fn stored_year_candidate(pct: i64, scale: u8, year: &str, source: &str) -> RocCandidateBody {
    RocCandidateBody {
        roc_pct_minor: Some(pct),
        scale,
        tax_year: year.into(),
        source: source.into(),
        source_url: String::new(),
        method: source.into(),
        as_of: String::new(),
        kind: if source.contains("actual") {
            "actual".into()
        } else {
            "estimate".into()
        },
        established_how: source.into(),
        owner_override: false,
    }
}

fn candidate_from_observation(obs: &RocResearchObservation) -> RocCandidateBody {
    RocCandidateBody {
        roc_pct_minor: obs.roc_pct_minor,
        scale: obs.scale,
        tax_year: obs.tax_year.clone(),
        source: obs.source.clone(),
        source_url: obs.source_url.clone(),
        method: obs.method.clone(),
        as_of: obs.as_of.clone(),
        kind: obs.kind.clone(),
        established_how: obs.established_how.clone(),
        owner_override: obs.owner_override,
    }
}

fn is_system_19a1(c: &RocCandidateBody) -> bool {
    !c.owner_override
        && c.roc_pct_minor.is_some()
        && (c.method == "19a-1-current-year" || c.source == "19a-1")
}

fn parse_roc_candidates(v: &Value) -> Vec<RocCandidateBody> {
    let Some(arr) = v.as_array() else {
        return Vec::new();
    };
    arr.iter()
        .map(|c| RocCandidateBody {
            roc_pct_minor: c.get("rocPctMinor").and_then(|x| x.as_i64()),
            scale: c.get("scale").and_then(|x| x.as_u64()).unwrap_or(2) as u8,
            tax_year: c.get("taxYear").and_then(|x| x.as_str()).unwrap_or("").into(),
            source: c.get("source").and_then(|x| x.as_str()).unwrap_or("").into(),
            source_url: c.get("sourceUrl").and_then(|x| x.as_str()).unwrap_or("").into(),
            method: c.get("method").and_then(|x| x.as_str()).unwrap_or("").into(),
            as_of: c.get("asOf").and_then(|x| x.as_str()).unwrap_or("").into(),
            kind: c.get("kind").and_then(|x| x.as_str()).unwrap_or("").into(),
            established_how: c
                .get("establishedHow")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .into(),
            owner_override: c
                .get("ownerOverride")
                .and_then(|x| x.as_bool())
                .unwrap_or(false),
        })
        .collect()
}

async fn persist_roc_candidates(
    canonical: &dyn Canonical,
    security_id: Uuid,
    candidates: &[RocCandidateBody],
    recorded_at: &str,
) -> Result<(), PlatformError> {
    let existing = canonical.roc_observation_list(security_id).await?;
    for c in candidates {
        if c.source_url.is_empty() && c.method.is_empty() && !c.owner_override {
            continue;
        }
        let already = existing.iter().any(|o| {
            o.source_url == c.source_url
                && o.method == c.method
                && o.roc_pct_minor == c.roc_pct_minor
                && o.owner_override == c.owner_override
        });
        if already {
            continue;
        }
        canonical
            .roc_observation_record(RocResearchObservation {
                observation_id: Uuid::new_v4(),
                security_id,
                roc_pct_minor: c.roc_pct_minor,
                scale: c.scale,
                tax_year: if c.tax_year.is_empty() {
                    "2026".into()
                } else {
                    c.tax_year.clone()
                },
                source: if c.source.is_empty() {
                    "19a-1".into()
                } else {
                    c.source.clone()
                },
                source_url: c.source_url.clone(),
                method: c.method.clone(),
                as_of: c.as_of.clone(),
                kind: if c.kind.is_empty() {
                    "estimate".into()
                } else {
                    c.kind.clone()
                },
                established_how: c.established_how.clone(),
                owner_override: c.owner_override,
                recorded_at: recorded_at.to_string(),
            })
            .await?;
    }
    Ok(())
}

async fn roc_plan_confirm(
    canonical: &dyn Canonical,
    security_id: Uuid,
    account_id: Option<Uuid>,
    json: &Value,
) -> Result<RocResearchBody, PlatformError> {
    let as_of = jstr(json, "asOfDate").unwrap_or_else(today_iso);
    let pct = ji64(json, "rocPctMinor");
    let chars = canonical.position_characteristic_list().await?;
    let mut rec = chars
        .into_iter()
        .find(|c| c.security_id == security_id)
        .unwrap_or_else(|| empty_characteristic(security_id));
    rec.roc_pct_2026_estimate_minor = pct;
    rec.roc_scale = Some(ju8(json, "rocScale", financial_domain::roc::ROC_PCT_SCALE));
    rec.needs_roc_research = pct.is_none();
    if let Some(src) = jstr(json, "source") {
        if !src.is_empty() {
            rec.notes = format!("roc source: {src}");
        }
    }
    if financial_domain::calculator::PaymentCadence::parse(&rec.payment_frequency).is_some() {
        canonical.position_characteristic_upsert(rec).await?;
    }
    let owner_override = jbool(json, "ownerOverride", false);
    let scale = ju8(json, "rocScale", financial_domain::roc::ROC_PCT_SCALE);
    let mut provenance = Vec::new();
    if jstr(json, "sourceUrl").is_some() || jstr(json, "method").is_some() {
        provenance.push(RocCandidateBody {
            roc_pct_minor: ji64(json, "systemRocPctMinor").or(pct),
            scale,
            tax_year: financial_domain::roc::tax_year(&as_of),
            source: jstr(json, "source").unwrap_or_else(|| "19a-1".into()),
            source_url: jstr(json, "sourceUrl").unwrap_or_default(),
            method: jstr(json, "method").unwrap_or_default(),
            as_of: jstr(json, "asOf").unwrap_or_else(|| as_of.clone()),
            kind: jstr(json, "kind").unwrap_or_else(|| "estimate".into()),
            established_how: jstr(json, "establishedHow").unwrap_or_default(),
            owner_override: false,
        });
    }
    if owner_override {
        provenance.push(RocCandidateBody {
            roc_pct_minor: pct,
            scale,
            tax_year: financial_domain::roc::tax_year(&as_of),
            source: "owner-override".into(),
            source_url: String::new(),
            method: "owner-override".into(),
            as_of: as_of.clone(),
            kind: "estimate".into(),
            established_how: "owner typed percent".into(),
            owner_override: true,
        });
    }
    persist_roc_candidates(canonical, security_id, &provenance, &as_of).await?;
    let research = roc_research_view(canonical, Some(security_id), account_id, as_of.clone(), &[]).await?;
    if research.magi_eligible {
        let year = financial_domain::roc::tax_year(&as_of);
        let source = financial_domain::roc::magi_remaining_source(&security_id.to_string(), &year);
        if let Some(ordinary) = research.remaining_ordinary_minor {
            canonical
                .magi_fact_record(
                    source,
                    "uncertain".into(),
                    ordinary,
                    2,
                    "ordinary-remaining".into(),
                )
                .await?;
        }
    }
    roc_research_view(canonical, Some(security_id), account_id, as_of, &[]).await
}

async fn roc_on_dividend(
    canonical: &dyn Canonical,
    actual: &crate::contracts::DividendActual,
) -> Result<(), PlatformError> {
    let Some(security_id) = actual.security_id else {
        return Ok(());
    };
    let account = canonical.account_get(actual.account_id).await?;
    if !financial_domain::roc::account_is_car_magi(&account.name) {
        return Ok(());
    }
    let chars = canonical.position_characteristic_list().await?;
    let Some(ch) = chars.iter().find(|c| c.security_id == security_id) else {
        return Ok(());
    };
    let pct = ch
        .roc_pct_2026_actual_minor
        .or(ch.roc_pct_2026_estimate_minor);
    let scale = ch.roc_scale.unwrap_or(financial_domain::roc::ROC_PCT_SCALE);
    let split = financial_domain::roc::split_cash(actual.amount_minor, pct, scale);
    let Some(ordinary) = split.ordinary_minor else {
        return Ok(());
    };
    canonical
        .magi_fact_record(
            financial_domain::roc::magi_actual_source(
                &security_id.to_string(),
                &actual.occurred_on,
            ),
            "include".into(),
            ordinary,
            actual.scale,
            "ordinary-actual".into(),
        )
        .await?;
    let year = financial_domain::roc::tax_year(&actual.occurred_on);
    let research = roc_research_view(
        canonical,
        Some(security_id),
        Some(actual.account_id),
        actual.occurred_on.clone(),
        &[],
    )
    .await?;
    if let Some(remaining) = research.remaining_ordinary_minor {
        canonical
            .magi_fact_record(
                financial_domain::roc::magi_remaining_source(&security_id.to_string(), &year),
                "uncertain".into(),
                financial_domain::roc::remaining_after_actual(remaining, ordinary),
                2,
                "ordinary-remaining".into(),
            )
            .await?;
    }
    if let (Some(roc), Some(activity_id)) = (split.roc_minor, actual.activity_id) {
        let _ = canonical
            .distribution_characterize(activity_id, "roc".into(), roc, actual.scale)
            .await;
    }
    Ok(())
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

/// Process A: owner supplies symbol + distribution URL only.
/// Persists investment identity and the standing issuer-declaration retrieval template,
/// then runs CollectorRetrieve (pass-through candidates/misses/quote when present).
/// Infers Weekly/Monthly/Quarterly from paid spacing or issuer label when supported.
/// Does not write PlanHistory, ClassificationApply, LotOpen, or RocPlanConfirm.
async fn position_research_seed(
    platform: &dyn Platform,
    canonical: &dyn Canonical,
    request: &CommandRequest,
    json: &Value,
) -> CommandResult {
    let symbol = jstr(json, "symbol")
        .map(|s| s.trim().to_ascii_uppercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_default();
    if symbol.is_empty() {
        return command_err(request, "missing_symbol");
    }
    let source_url = jstr(json, "sourceUrl")
        .or_else(|| jstr(json, "distributionUrl"))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_default();
    if source_url.is_empty() {
        return command_err(request, "missing_source_url");
    }

    let security = match canonical.security_list().await {
        Ok(list) => match list
            .into_iter()
            .find(|s| s.symbol.eq_ignore_ascii_case(&symbol))
        {
            Some(existing) => existing,
            None => {
                let name = jstr(json, "name").unwrap_or_else(|| symbol.clone());
                match canonical
                    .security_register(symbol.clone(), name, jbool(json, "crf", false))
                    .await
                {
                    Ok(rec) => rec,
                    Err(err) => return command_err(request, &err.code),
                }
            }
        },
        Err(err) => return command_err(request, &err.code),
    };

    let declaration_source = jstr(json, "declarationSource")
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            financial_domain::div1::declaration_source_from_url(&source_url)
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "issuer".into());
    let calendar_policy = jstr(json, "calendarPolicy")
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            financial_domain::div1::calendar_policy_for_source(&declaration_source).to_string()
        });
    let collector_enabled = json
        .get("collectorEnabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    if let Err(err) = canonical
        .retrieval_template_set(RetrievalTemplateRecord {
            security_id: security.security_id,
            price_source: jstr(json, "priceSource").unwrap_or_else(|| "public".into()),
            source_symbol: jstr(json, "sourceSymbol").unwrap_or_else(|| symbol.clone()),
            declaration_source: declaration_source.clone(),
            lookback_count: ju8(json, "lookbackCount", 12),
            payment_source: "import".into(),
            source_url: source_url.clone(),
            calendar_policy: calendar_policy.clone(),
            last_run_at: String::new(),
            last_run_ok: None,
            last_run_message: String::new(),
            last_content_hash: String::new(),
            collector_enabled,
            inception_on: jstr(json, "inceptionOn").unwrap_or_default(),
        })
        .await
    {
        return command_err(request, &err.code);
    }

    let mut retrieve_body = serde_json::json!({
        "securityId": security.security_id,
        "symbol": symbol,
        "declarationSource": declaration_source,
        "sourceUrl": source_url,
        "sourceSymbol": jstr(json, "sourceSymbol").unwrap_or_else(|| symbol.clone()),
        "inceptionOn": jstr(json, "inceptionOn").unwrap_or_default(),
        "paymentFrequency": jstr(json, "paymentFrequency").unwrap_or_default(),
        "divType": jstr(json, "divType").unwrap_or_default(),
    });
    // Pass-through injected retrieve payloads (golden / desktop fill); never invent $0 amounts.
    for key in [
        "candidates",
        "declarations",
        "misses",
        "upcomingPays",
        "payDates",
        "quote",
        "contentHash",
        "unchanged",
        "forceRefresh",
        "lastContentHash",
        "lastRunOk",
        "lastRunAt",
        "knownPaymentPeriods",
        "missExplanation",
        "suggestedFrequency",
    ] {
        if let Some(v) = json.get(key) {
            retrieve_body[key] = v.clone();
        }
    }
    // No invented amounts: empty candidates leave declarations unknown (never $0).
    // Desktop fill / golden injects candidates or misses when available.

    let retrieve = Box::pin(execute_command_on(
        platform,
        canonical,
        CommandRequest {
            contract_version: request.contract_version.clone(),
            command_name: "CollectorRetrieve".into(),
            correlation_id: request.correlation_id,
            body_json: Some(retrieve_body.to_string()),
            expected_version: None,
        },
    ))
    .await;

    let (retrieve_ok, retrieve_code, retrieve_message) = if retrieve.ok {
        let body: CollectorRetrieveBody = serde_json::from_str(retrieve.body_json.as_deref().unwrap_or("{}"))
            .unwrap_or(CollectorRetrieveBody {
                security_id: security.security_id,
                symbol: symbol.clone(),
                run_id: Uuid::nil(),
                attempted: 0,
                recorded: 0,
                skipped: 0,
                unchanged: 0,
                ok: false,
                code: "retrieve_parse_failed".into(),
                message: String::new(),
                payload_json: String::new(),
            });
        (body.ok, body.code, body.message)
    } else {
        (
            false,
            retrieve.error_code.clone().unwrap_or_else(|| "retrieve_failed".into()),
            String::new(),
        )
    };

    let page_label = jstr(json, "suggestedFrequency")
        .or_else(|| jstr(json, "paymentFrequency"))
        .filter(|s| !s.trim().is_empty());
    let payment_frequency = persist_inferred_frequency_if_unknown(
        canonical,
        security.security_id,
        page_label.as_deref(),
    )
    .await;

    // After declarations: propose current-year 19a-1 ROC estimate (not complete, not 1099).
    let roc = process_a_propose_roc_estimate(
        platform,
        canonical,
        request,
        security.security_id,
        &symbol,
        &declaration_source,
        json,
    )
    .await;

    // LastPriceRefresh when a quote was injected; otherwise no-op empty set (miss stays unknown).
    if json.get("quote").is_some() || json.get("quotes").is_some() {
        let mut price_body = serde_json::json!({});
        if let Some(quotes) = json.get("quotes") {
            price_body["quotes"] = quotes.clone();
        } else if let Some(quote) = json.get("quote") {
            let mut q = quote.clone();
            if q.get("securityId").is_none() {
                q["securityId"] = serde_json::json!(security.security_id);
            }
            if q.get("symbol").is_none() {
                q["symbol"] = serde_json::json!(symbol);
            }
            price_body["quotes"] = serde_json::json!([q]);
        }
        let _ = Box::pin(execute_command_on(
            platform,
            canonical,
            CommandRequest {
                contract_version: request.contract_version.clone(),
                command_name: "LastPriceRefresh".into(),
                correlation_id: request.correlation_id,
                body_json: Some(price_body.to_string()),
                expected_version: None,
            },
        ))
        .await;
    }

    command_ok(
        request,
        serde_json::to_string(&PositionResearchSeedBody {
            security_id: security.security_id,
            symbol,
            declaration_source,
            source_url,
            calendar_policy,
            payment_frequency,
            retrieve_ok,
            retrieve_code,
            retrieve_message,
            roc_pct_minor: roc.roc_pct_minor,
            roc_scale: roc.roc_scale,
            roc_source_url: roc.roc_source_url,
            roc_method: roc.roc_method,
            roc_kind: roc.roc_kind,
            roc_as_of: roc.roc_as_of,
            roc_established_how: roc.roc_established_how,
            roc_complete: false,
        })
        .unwrap_or_else(|_| "{}".into()),
    )
}

#[derive(Default)]
struct ProcessARocPropose {
    roc_pct_minor: Option<i64>,
    roc_scale: u8,
    roc_source_url: String,
    roc_method: String,
    roc_kind: String,
    roc_as_of: String,
    roc_established_how: String,
}

/// Retrieve / persist a current-year 19a-1 estimate after declarations.
/// Does not call RocPlanConfirm, does not mark research complete, never invents 0%.
async fn process_a_propose_roc_estimate(
    platform: &dyn Platform,
    canonical: &dyn Canonical,
    request: &CommandRequest,
    security_id: Uuid,
    symbol: &str,
    declaration_source: &str,
    json: &Value,
) -> ProcessARocPropose {
    let mut out = ProcessARocPropose {
        roc_scale: financial_domain::roc::ROC_PCT_SCALE,
        ..ProcessARocPropose::default()
    };
    let paid = canonical
        .issuer_declaration_list(security_id)
        .await
        .unwrap_or_default()
        .iter()
        .filter(|d| d.amount_per_share_minor.unwrap_or(0) > 0)
        .count();
    let injected = json
        .get("rocCandidates")
        .cloned()
        .filter(|v| v.as_array().map(|a| !a.is_empty()).unwrap_or(false));
    if paid == 0 && injected.is_none() {
        return out;
    }

    let mut roc_body = serde_json::json!({
        "securityId": security_id,
        "symbol": symbol,
        "declarationSource": declaration_source,
        "asOfDate": today_iso(),
    });
    if let Some(c) = injected {
        roc_body["candidates"] = c;
    }

    let retrieve = Box::pin(execute_command_on(
        platform,
        canonical,
        CommandRequest {
            contract_version: request.contract_version.clone(),
            command_name: "RocResearchRetrieve".into(),
            correlation_id: request.correlation_id,
            body_json: Some(roc_body.to_string()),
            expected_version: None,
        },
    ))
    .await;
    if !retrieve.ok {
        return out;
    }
    let body: Value =
        serde_json::from_str(retrieve.body_json.as_deref().unwrap_or("{}")).unwrap_or_default();
    let parsed = parse_roc_candidates(body.get("candidates").unwrap_or(&Value::Null));
    let Some(system) = parsed.iter().find(|c| is_system_19a1(c)).cloned() else {
        return out;
    };
    propose_characteristic_roc_estimate(canonical, security_id, &parsed).await;
    out.roc_pct_minor = system.roc_pct_minor;
    out.roc_scale = system.scale;
    out.roc_source_url = system.source_url;
    out.roc_method = system.method;
    out.roc_kind = if system.kind.is_empty() {
        "estimate".into()
    } else {
        system.kind
    };
    out.roc_as_of = system.as_of;
    out.roc_established_how = system.established_how;
    out
}

/// Persist current-year 19a-1 % as characteristic 2026 estimate. Keeps needsRocResearch=true
/// (estimate is not 1099 actual; research is not complete).
async fn propose_characteristic_roc_estimate(
    canonical: &dyn Canonical,
    security_id: Uuid,
    candidates: &[RocCandidateBody],
) {
    let Some(system) = candidates.iter().find(|c| is_system_19a1(c)) else {
        return;
    };
    let Ok(list) = canonical.position_characteristic_list().await else {
        return;
    };
    let mut rec = list
        .into_iter()
        .find(|c| c.security_id == security_id)
        .unwrap_or_else(|| empty_characteristic(security_id));
    if financial_domain::calculator::PaymentCadence::parse(&rec.payment_frequency).is_none() {
        return;
    }
    // Do not overwrite a confirmed 2026 actual.
    if rec.roc_pct_2026_actual_minor.is_some() {
        return;
    }
    rec.roc_pct_2026_estimate_minor = system.roc_pct_minor;
    rec.roc_scale = Some(system.scale);
    rec.needs_roc_research = true;
    let _ = canonical.position_characteristic_upsert(rec).await;
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
            let as_of = jstr(&json, "asOfDate").unwrap_or_else(today_iso);
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
        "DataSummaryGet" => map_q(&request, data_summary_view(canonical).await),
        "CalculatorGet" => map_q(&request, calculator_view(canonical).await),
        "PlanReviewGet" => match juuid(&json, "securityId") {
            Some(id) => map_q(&request, plan_review_view(canonical, id).await),
            None => query_err(&request, "missing_security_id"),
        },
        "CurrentPriceGet" => match juuid(&json, "securityId") {
            Some(id) => {
                let as_of = jstr(&json, "asOfDate").unwrap_or_else(today_iso);
                map_q(&request, canonical.current_price_get(id, as_of).await)
            }
            None => query_err(&request, "missing_security_id"),
        },
        "InvestmentGet" => {
            let as_of = jstr(&json, "asOfDate").unwrap_or_else(today_iso);
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
        "CollectorSetGet" => map_q(&request, canonical.collector_set().await),
        "CollectorStatsGet" => {
            let as_of = jstr(&json, "asOfDate").unwrap_or_else(today_iso);
            map_q(&request, canonical.collector_stats(as_of).await)
        }
        "RetrieveRunList" => {
            let security_id = juuid(&json, "securityId");
            let limit = json
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(50) as u32;
            map_q(
                &request,
                canonical
                    .retrieve_run_list(security_id, limit)
                    .await
                    .map(|runs| RetrieveRunListBody { runs }),
            )
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
        "TrendsGet" => {
            let as_of = jstr(&json, "asOfDate").unwrap_or_else(|| chrono::Utc::now().date_naive().to_string());
            match (
                canonical.dividend_get().await,
                account_trends_weeks(canonical).await,
            ) {
                (Ok(div), Ok(weeks)) => {
                    let overview = crate::trends_app::overview_from_weeks(&weeks);
                    let distributions = crate::trends_app::distributions_ytd(canonical, &as_of)
                        .await
                        .ok();
                    let tax_monitor = crate::trends_app::tax_monitor(canonical).await.ok();
                    let capture = crate::trends_app::trends_week_capture_view(canonical, &as_of)
                        .await
                        .ok();
                    map_q(
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
                            weeks,
                            note: "Sat–Fri weeks; capture on Trends; dividend points remain M3 actuals"
                                .into(),
                            overview: Some(overview),
                            distributions,
                            tax_monitor,
                            missing_required: capture
                                .map(|c| c.missing_required)
                                .unwrap_or_default(),
                            scale: div.scale,
                        }),
                    )
                }
                (Err(err), _) | (_, Err(err)) => query_err(&request, &err.code),
            }
        }
        "TrendsWeekGet" => {
            let as_of = jstr(&json, "asOfDate").unwrap_or_else(|| chrono::Utc::now().date_naive().to_string());
            map_q(
                &request,
                crate::trends_app::trends_week_capture_view(canonical, &as_of).await,
            )
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
        "PositionDetailsGet" => map_q(&request, position_details_summary(canonical, &json).await),
        "PositionMasterGet" => map_q(&request, position_master_view(canonical).await),
        "PositionDetailsCoverageGet" => {
            map_q(&request, position_details_coverage(canonical).await)
        }
        "Div1ComplianceSummaryGet" => {
            map_q(&request, div1_compliance_summary(canonical).await)
        }
        "ClassificationSuggestGet" => match juuid(&json, "securityId") {
            Some(security_id) => map_q(
                &request,
                classification_suggest(canonical, security_id).await,
            ),
            None => query_err(&request, "missing_security_id"),
        },
        "RocResearchGet" => {
            let injected = json
                .get("candidates")
                .and_then(|v| serde_json::from_value::<Vec<RocCandidateBody>>(v.clone()).ok())
                .unwrap_or_default();
            let as_of = jstr(&json, "asOfDate").unwrap_or_else(today_iso);
            map_q(
                &request,
                roc_research_view(
                    canonical,
                    juuid(&json, "securityId"),
                    juuid(&json, "accountId"),
                    as_of,
                    &injected,
                )
                .await,
            )
        }
        "RemainingYearIncomeGet" => match juuid(&json, "securityId") {
            Some(id) => map_q(&request, remaining_year_income_view(canonical, id, &json).await),
            None => query_err(&request, "missing_security_id"),
        },
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
        "PositionResearchSeed" => {
            position_research_seed(platform, canonical, &request, &json).await
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
            Some(account_id) => {
                let recorded = canonical
                    .dividend_actual_record(
                        account_id,
                        juuid(&json, "securityId"),
                        jstr(&json, "occurredOn").unwrap_or_default(),
                        ji64(&json, "amountMinor"),
                        ju8(&json, "scale", 2),
                        jstr(&json, "idempotencyKey"),
                    )
                    .await;
                match recorded {
                    Ok(actual) => {
                        let _ = roc_on_dividend(canonical, &actual).await;
                        map_c(&request, Ok(actual))
                    }
                    Err(err) => command_err(&request, &err.code),
                }
            }
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
            (Some(account_id), Some(security_id)) => {
                // Process B: lots only on a researched identity (standing retrieval template).
                match canonical.retrieval_template_get(security_id).await {
                    Ok(None) => return command_err(&request, "not_researched"),
                    Err(err) => return command_err(&request, &err.code),
                    Ok(Some(_)) => {}
                }
                let opened_on = jstr(&json, "openedOn").unwrap_or_default();
                if opened_on.trim().is_empty() {
                    return command_err(&request, "missing_opened_on");
                }
                let mut origin = jstr(&json, "origin").unwrap_or_else(|| "purchase".into());
                let origin_key = origin.trim().to_ascii_lowercase();
                if !matches!(
                    origin_key.as_str(),
                    "purchase" | "drip" | "transfer" | "option"
                ) {
                    return command_err(&request, "invalid_lot_origin");
                }
                origin = origin_key;
                let performance = ji64(&json, "performanceBasisMinor")
                    .or_else(|| ji64(&json, "originalCostMinor"))
                    .unwrap_or(0);
                let tax = ji64(&json, "taxBasisMinor")
                    .or_else(|| ji64(&json, "taxCostMinor"))
                    .unwrap_or(performance);
                map_c(
                    &request,
                    canonical
                        .lot_open(
                            account_id,
                            security_id,
                            opened_on,
                            origin,
                            ji64(&json, "quantityMinor").unwrap_or(0),
                            ju8(&json, "quantityScale", 0),
                            performance,
                            tax,
                            ju8(&json, "scale", 2),
                            juuid(&json, "openingActivityId"),
                            jbool(&json, "isOpen", true),
                        )
                        .await,
                )
            }
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
                    jstr(&json, "approvedOn").unwrap_or_else(today_iso),
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
                    jstr(&json, "completedAt").unwrap_or_else(today_iso),
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
        "TrendsWeekSave" | "TrendsWeekCorrect" => {
            let period_end = jstr(&json, "periodEnd").unwrap_or_default();
            let period_start = jstr(&json, "periodStart").unwrap_or_default();
            if period_end.is_empty() {
                return command_err(&request, "missing_period_end");
            }
            let allow_closed = request.command_name == "TrendsWeekCorrect";
            let captured_at = jstr(&json, "capturedAt")
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
            let record = crate::contracts::TrendsWeekSourceRecord {
                period_end: period_end.clone(),
                period_start,
                profit_minor: ji64(&json, "profitMinor").unwrap_or(0),
                monthly_divs_minor: ji64(&json, "monthlyDivsMinor").unwrap_or(0),
                fidelity_total_minor: ji64(&json, "fidelityTotalMinor").unwrap_or(0),
                schwab_total_minor: ji64(&json, "schwabTotalMinor").unwrap_or(0),
                income_cash_minor: ji64(&json, "incomeCashMinor").unwrap_or(0),
                acct9_cash_minor: ji64(&json, "acct9CashMinor").unwrap_or(0),
                acct9_etf_value_minor: ji64(&json, "acct9EtfValueMinor").unwrap_or(0),
                scale: ju8(&json, "scale", 2),
                captured_at,
                closed: jbool(&json, "closed", false),
            };
            let balances = [
                ("Car", ji64(&json, "carBalanceMinor")),
                ("Income", ji64(&json, "incomeBalanceMinor")),
                ("Health", ji64(&json, "healthBalanceMinor")),
                ("FI Roth", ji64(&json, "rothBalanceMinor")),
                ("Speculation", ji64(&json, "speculationBalanceMinor")),
            ];
            let bal_refs: Vec<(&str, Option<i64>)> = balances
                .iter()
                .map(|(n, v)| (*n, *v))
                .collect();
            map_c(
                &request,
                crate::trends_app::save_trends_week(canonical, record, &bal_refs, allow_closed)
                    .await,
            )
        }
        "TrendsWeekClose" => {
            let period_end = jstr(&json, "periodEnd").unwrap_or_default();
            if period_end.is_empty() {
                return command_err(&request, "missing_period_end");
            }
            match canonical
                .trends_week_set_closed(period_end.clone(), true)
                .await
            {
                Ok(()) => map_c(
                    &request,
                    crate::trends_app::trends_week_capture_view(canonical, &period_end).await,
                ),
                Err(err) => command_err(&request, &err.code),
            }
        }
        "ProductionSeedLoad" => {
            match serde_json::from_value::<ProductionSeedDocument>(json.clone()) {
                Ok(doc) => map_c(
                    &request,
                    crate::production_seed::apply_production_seed(canonical, doc).await,
                ),
                Err(_) => command_err(&request, "invalid_seed_document"),
            }
        },
        "ProviderDeclarationSourcesApply" => map_c(
            &request,
            crate::production_seed::apply_provider_declaration_sources(canonical)
                .await
                .map(|updated| ProviderDeclarationSourcesApplyBody { updated }),
        ),
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
                        jstr(&json, "enteredAt").unwrap_or_else(today_iso),
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
                        jstr(&json, "asOfAt").unwrap_or_else(today_iso),
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
                        jstr(&json, "effectiveFrom").unwrap_or_else(today_iso),
                    )
                    .await,
            ),
            None => command_err(&request, "missing_security_id"),
        },
        "RetrievalTemplateSet" => match juuid(&json, "securityId") {
            Some(security_id) => {
                let declaration_source =
                    jstr(&json, "declarationSource").unwrap_or_default();
                let collector_enabled = json
                    .get("collectorEnabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or_else(|| {
                        financial_domain::div1::is_registered_declaration_source(
                            &declaration_source,
                        )
                    });
                map_c(
                    &request,
                    canonical
                        .retrieval_template_set(RetrievalTemplateRecord {
                            security_id,
                            price_source: jstr(&json, "priceSource")
                                .unwrap_or_else(|| "public".into()),
                            source_symbol: jstr(&json, "sourceSymbol").unwrap_or_default(),
                            declaration_source,
                            lookback_count: ju8(&json, "lookbackCount", 12),
                            payment_source: "import".into(),
                            source_url: jstr(&json, "sourceUrl").unwrap_or_default(),
                            calendar_policy: jstr(&json, "calendarPolicy").unwrap_or_default(),
                            last_run_at: String::new(),
                            last_run_ok: None,
                            last_run_message: String::new(),
                            last_content_hash: String::new(),
                            collector_enabled,
                            inception_on: jstr(&json, "inceptionOn").unwrap_or_default(),
                        })
                        .await,
                )
            }
            None => command_err(&request, "missing_security_id"),
        },
        "PositionCharacteristicUpsert" => match juuid(&json, "securityId") {
            Some(security_id) => {
                let existing = match canonical.position_characteristic_list().await {
                    Ok(list) => list.into_iter().find(|c| c.security_id == security_id),
                    Err(err) => return command_err(&request, &err.code),
                };
                let base = existing.unwrap_or_else(|| empty_characteristic(security_id));
                let freq_raw = if json.get("paymentFrequency").is_some() {
                    jstr(&json, "paymentFrequency").unwrap_or_default()
                } else {
                    base.payment_frequency.clone()
                };
                let cadence = match locked_cadence(&freq_raw) {
                    Ok(c) => c,
                    Err(err) => return command_err(&request, &err.code),
                };
                map_c(
                    &request,
                    canonical
                        .position_characteristic_upsert(PositionCharacteristicRecord {
                            security_id,
                            payment_frequency: cadence.label().to_string(),
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
                            is_active: jbool_keep(&json, "isActive", base.is_active),
                            lookthrough: jlookthrough_keep(&json, "lookthrough", base.lookthrough),
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
        "BacktestPeriodRecord" => {
            let kind = jstr(&json, "kind").unwrap_or_default();
            let start_on = jstr(&json, "startOn").unwrap_or_default();
            let end_on = jstr(&json, "endOn").unwrap_or_default();
            if !financial_domain::regime::period_ready(&start_on, &end_on, &kind) {
                return command_err(&request, "regime_period_incomplete");
            }
            map_c(
                &request,
                canonical
                    .backtest_period_record(BacktestPeriodRecord {
                        period_id: Uuid::new_v4(),
                        kind: kind.clone(),
                        name: jstr(&json, "name").unwrap_or_default(),
                        start_on,
                        end_on,
                        benchmark_symbol: jstr(&json, "benchmarkSymbol").unwrap_or_default(),
                        selection_reason: financial_domain::regime::locked_selection_reason(&kind),
                        method: financial_domain::regime::REGIME_PRICE_METHOD.into(),
                        status: jstr(&json, "status").unwrap_or_else(|| "approved".into()),
                        recorded_at: jstr(&json, "recordedAt")
                            .unwrap_or_else(today_iso),
                    })
                    .await,
            )
        }
        "PositionBacktestCalculate" => match (juuid(&json, "securityId"), juuid(&json, "periodId")) {
            (Some(security_id), Some(period_id)) => {
                map_c(
                    &request,
                    position_backtest_calculate(canonical, security_id, period_id, &json).await,
                )
            }
            _ => command_err(&request, "missing_security_or_period"),
        },
        "ClassificationApply" => match juuid(&json, "securityId") {
            Some(security_id) => {
                let tier = jstr(&json, "riskTier").unwrap_or_default();
                if !matches!(tier.as_str(), "Foundation" | "Core" | "Risk On") {
                    return command_err(&request, "invalid_risk_tier");
                }
                let chars = canonical.position_characteristic_list().await;
                let existing = match chars {
                    Ok(list) => list.into_iter().find(|c| c.security_id == security_id),
                    Err(err) => return command_err(&request, &err.code),
                };
                let mut rec = existing.unwrap_or_else(|| empty_characteristic(security_id));
                rec.risk_tier = financial_domain::plan_review::normalize_risk_tier(&tier);
                if let Err(err) = canonical.position_characteristic_upsert(rec).await {
                    return command_err(&request, &err.code);
                }
                map_c(
                    &request,
                    canonical
                        .classification_review_record(
                            security_id.to_string(),
                            tier,
                            "applied".into(),
                        )
                        .await,
                )
            }
            None => command_err(&request, "missing_security_id"),
        },
        "PeriodSeriesRetrieve" => command_ok(
            &request,
            serde_json::json!({
                "candidates": json.get("candidates").cloned().unwrap_or(serde_json::json!([])),
                "benchmarkCandidates": json.get("benchmarkCandidates").cloned().unwrap_or(serde_json::json!([])),
                "posted": false
            })
            .to_string(),
        ),
        "RocResearchRetrieve" => {
            let candidates = json
                .get("candidates")
                .cloned()
                .unwrap_or(serde_json::json!([]));
            let parsed = parse_roc_candidates(&candidates);
            if let Some(security_id) = juuid(&json, "securityId") {
                let as_of = jstr(&json, "asOfDate").unwrap_or_else(today_iso);
                if let Err(err) =
                    persist_roc_candidates(canonical, security_id, &parsed, &as_of).await
                {
                    return command_err(&request, &err.code);
                }
                // Process A propose: mirror 19a-1 onto 2026 estimate without completing research.
                propose_characteristic_roc_estimate(canonical, security_id, &parsed).await;
            }
            command_ok(
                &request,
                serde_json::json!({
                    "candidates": candidates,
                    "posted": false
                })
                .to_string(),
            )
        }
        "RocPlanConfirm" => match juuid(&json, "securityId") {
            Some(security_id) => map_c(
                &request,
                roc_plan_confirm(canonical, security_id, juuid(&json, "accountId"), &json).await,
            ),
            None => command_err(&request, "missing_security_id"),
        },
        "RemainingPaymentDateOverride" => match juuid(&json, "securityId") {
            Some(security_id) => {
                let pay_on = jstr(&json, "payOn").unwrap_or_default();
                if pay_on.is_empty() {
                    command_err(&request, "missing_pay_on")
                } else {
                    map_c(
                        &request,
                        canonical
                            .remaining_payment_date_override_record(RemainingPaymentDateOverride {
                                override_id: Uuid::new_v4(),
                                security_id,
                                original_pay_on: jstr(&json, "originalPayOn").unwrap_or_default(),
                                pay_on,
                                recorded_at: jstr(&json, "recordedAt")
                                    .or_else(|| jstr(&json, "asOfDate"))
                                    .unwrap_or_else(today_iso),
                            })
                            .await,
                    )
                }
            }
            None => command_err(&request, "missing_security_id"),
        },
        "ExpectedPaymentPatternUpsert" => match juuid(&json, "securityId") {
            Some(security_id) => map_c(
                &request,
                canonical
                    .expected_payment_pattern_upsert(ExpectedPaymentPattern {
                        security_id,
                        declaration_weekday: jstr(&json, "declarationWeekday").unwrap_or_default(),
                        exdate_weekday: jstr(&json, "exdateWeekday").unwrap_or_default(),
                        payday_weekday: jstr(&json, "paydayWeekday").unwrap_or_default(),
                    })
                    .await,
            ),
            None => command_err(&request, "missing_security_id"),
        },
        "PositionTaxProfileUpsert" => match juuid(&json, "securityId") {
            Some(security_id) => map_c(
                &request,
                canonical
                    .position_tax_profile_upsert(PositionTaxProfile {
                        security_id,
                        expected_handling: jstr(&json, "expectedHandling").unwrap_or_default(),
                    })
                    .await,
            ),
            None => command_err(&request, "missing_security_id"),
        },
        "AiAnalyze" => map_c(
            &request,
            canonical
                .ai_analyze(jstr(&json, "prompt").unwrap_or_default())
                .await,
        ),
        "LastPriceRefresh" => {
            let quotes = json
                .get("quotes")
                .and_then(|q| q.as_array())
                .cloned()
                .unwrap_or_default();
            let attempted = quotes.len() as u64;
            let mut recorded = 0u64;
            let mut skipped = 0u64;
            for quote in quotes {
                let Some(security_id) = juuid(&quote, "securityId") else {
                    skipped += 1;
                    continue;
                };
                let price_minor = ji64(&quote, "priceMinor").unwrap_or(0);
                if price_minor <= 0 {
                    skipped += 1;
                    continue;
                }
                match canonical
                    .price_quote_record(
                        security_id,
                        price_minor,
                        ju8(&quote, "scale", 2),
                        jstr(&quote, "asOfAt").unwrap_or_else(today_iso),
                        jstr(&quote, "source").unwrap_or_else(|| "yahoo".into()),
                    )
                    .await
                {
                    Ok(_) => recorded += 1,
                    Err(_) => skipped += 1,
                }
            }
            let ran_at = today_iso();
            let misses = json
                .get("misses")
                .and_then(|m| m.as_array())
                .cloned()
                .unwrap_or_default();
            let _ = raise_retrieve_misses(canonical, &misses, &ran_at).await;
            for quote in json
                .get("quotes")
                .and_then(|q| q.as_array())
                .cloned()
                .unwrap_or_default()
            {
                if let Some(security_id) = juuid(&quote, "securityId") {
                    persist_retrieve_run(
                        canonical,
                        security_id,
                        "price",
                        &ran_at,
                        true,
                        "",
                        "last price recorded",
                        1,
                        1,
                        0,
                        0,
                        &quote,
                    )
                    .await;
                }
            }
            for miss in &misses {
                if let Some(security_id) = juuid(miss, "securityId") {
                    persist_retrieve_run(
                        canonical,
                        security_id,
                        "price",
                        &ran_at,
                        false,
                        &jstr(miss, "code").unwrap_or_else(|| "price_retrieve_miss".into()),
                        &jstr(miss, "reason").unwrap_or_else(|| "price miss".into()),
                        1,
                        0,
                        1,
                        0,
                        miss,
                    )
                    .await;
                }
            }
            command_ok(
                &request,
                serde_json::to_string(&LastPriceRefreshBody {
                    attempted: attempted + misses.len() as u64,
                    recorded,
                    skipped: skipped + misses.len() as u64,
                })
                .unwrap_or_else(|_| "{\"attempted\":0,\"recorded\":0,\"skipped\":0}".into()),
            )
        }
        "DeclarationRefresh" => {
            let declarations = json
                .get("declarations")
                .and_then(|q| q.as_array())
                .cloned()
                .unwrap_or_default();
            let attempted = declarations.len() as u64;
            let mut recorded = 0u64;
            let mut skipped = 0u64;
            let mut ok_ids: std::collections::HashSet<Uuid> = std::collections::HashSet::new();
            let mut hashes: std::collections::HashMap<Uuid, String> =
                std::collections::HashMap::new();
            let capture_hash = |row: &Value, hashes: &mut std::collections::HashMap<Uuid, String>| {
                if let Some(security_id) = juuid(row, "securityId") {
                    if let Some(hash) = jstr(row, "contentHash").filter(|h| !h.is_empty()) {
                        hashes.insert(security_id, hash);
                    }
                }
            };
            for decl in &declarations {
                capture_hash(decl, &mut hashes);
            }
            for decl in declarations {
                let Some(security_id) = juuid(&decl, "securityId") else {
                    skipped += 1;
                    continue;
                };
                let amount = decl.get("amountPerShareMinor").and_then(|x| {
                    if x.is_null() {
                        None
                    } else {
                        ji64(&decl, "amountPerShareMinor")
                    }
                });
                if amount.unwrap_or(0) <= 0 {
                    skipped += 1;
                    continue;
                }
                match canonical
                    .issuer_declaration_record(
                        security_id,
                        amount,
                        ju8(&decl, "amountScale", 2),
                        jstr(&decl, "paymentPeriod").unwrap_or_default(),
                        jstr(&decl, "source").unwrap_or_else(|| "roundhill".into()),
                        jstr(&decl, "enteredAt").unwrap_or_else(today_iso),
                    )
                    .await
                {
                    Ok(_) => {
                        recorded += 1;
                        ok_ids.insert(security_id);
                    }
                    Err(_) => skipped += 1,
                }
            }
            let ran_at = today_iso();
            let pay_dates = json
                .get("payDates")
                .and_then(|q| q.as_array())
                .cloned()
                .unwrap_or_default();
            for row in &pay_dates {
                capture_hash(row, &mut hashes);
            }
            let mut by_security: std::collections::HashMap<Uuid, Vec<IssuerPayDateRecord>> =
                std::collections::HashMap::new();
            for row in pay_dates {
                let Some(security_id) = juuid(&row, "securityId") else {
                    continue;
                };
                let pay_on = jstr(&row, "payOn")
                    .or_else(|| jstr(&row, "paymentPeriod"))
                    .unwrap_or_default();
                if pay_on.is_empty() {
                    continue;
                }
                by_security.entry(security_id).or_default().push(IssuerPayDateRecord {
                    pay_date_id: Uuid::new_v4(),
                    security_id,
                    pay_on,
                    source: jstr(&row, "source").unwrap_or_default(),
                    recorded_at: ran_at.clone(),
                });
            }
            for (security_id, dates) in by_security {
                if dates.is_empty() {
                    continue;
                }
                let as_of = dates
                    .iter()
                    .map(|d| d.pay_on.as_str())
                    .min()
                    .unwrap_or(ran_at.as_str())
                    .to_string();
                let pay_count = dates.len();
                let calendar_url = payment_calendar_url_for(&json, security_id);
                match canonical
                    .issuer_pay_date_replace(security_id, as_of, dates)
                    .await
                {
                    Ok(_) => {
                        ok_ids.insert(security_id);
                        promote_vendor_calendar_template(
                            canonical,
                            security_id,
                            pay_count,
                            &calendar_url,
                        )
                        .await;
                    }
                    Err(_) => skipped += 1,
                }
            }
            for security_id in &ok_ids {
                let hash = hashes.get(security_id).cloned().unwrap_or_default();
                let source_url = fetched_source_url_for(&json, Some(*security_id));
                let _ = canonical
                    .retrieval_template_touch_run(
                        *security_id,
                        true,
                        "declaration retrieve recorded".into(),
                        ran_at.clone(),
                        hash,
                        &source_url,
                    )
                    .await;
            }
            let unchanged = json
                .get("unchanged")
                .and_then(|m| m.as_array())
                .cloned()
                .unwrap_or_default();
            for row in &unchanged {
                capture_hash(row, &mut hashes);
                let Some(security_id) = juuid(row, "securityId") else {
                    continue;
                };
                if ok_ids.contains(&security_id) {
                    continue;
                }
                let hash = hashes.get(&security_id).cloned().unwrap_or_default();
                let existing = canonical
                    .retrieval_template_get(security_id)
                    .await
                    .ok()
                    .flatten();
                let characteristics = canonical.position_characteristic_list().await.unwrap_or_default();
                let ch = characteristics.iter().find(|c| c.security_id == security_id);
                let div_type = ch.map(|c| c.div_type.as_str()).unwrap_or("");
                let inception_on = existing
                    .as_ref()
                    .map(|t| t.inception_on.as_str())
                    .unwrap_or("");
                let payment_frequency = ch
                    .map(|c| c.payment_frequency.as_str())
                    .unwrap_or("");
                let decls = canonical
                    .issuer_declaration_list(security_id)
                    .await
                    .unwrap_or_default();
                let paid_count = count_paid_declarations(&decls);
                let gate = declaration_retrieve_gate(
                    paid_count,
                    inception_on,
                    payment_frequency,
                    div_type,
                );
                let ok = gate.ok;
                let message = if gate.message.is_empty() {
                    "declaration retrieve unchanged".into()
                } else {
                    gate.message
                };
                let _ = canonical
                    .retrieval_template_touch_run(security_id, ok, message.clone(), ran_at.clone(), hash, "")
                    .await;
            }
            let misses = json
                .get("misses")
                .and_then(|m| m.as_array())
                .cloned()
                .unwrap_or_default();
            let _ = raise_retrieve_misses(canonical, &misses, &ran_at).await;
            for security_id in &ok_ids {
                let payload = serde_json::json!({
                    "declarations": declarations_for_security(&json, *security_id),
                    "payDates": pay_dates_for_security(&json, *security_id),
                });
                let decls = declarations_for_security(&json, *security_id);
                let stored_declarations = canonical
                    .issuer_declaration_list(*security_id)
                    .await
                    .unwrap_or_default();
                let verify_issues = declaration_store_verify_issues(&decls, &stored_declarations);
                if !verify_issues.is_empty() {
                    let message = verify_issues.join("; ");
                    let _ = canonical
                        .retrieval_template_touch_run(
                            *security_id,
                            false,
                            message.clone(),
                            ran_at.clone(),
                            hashes.get(security_id).cloned().unwrap_or_default(),
                            &fetched_source_url_for(&json, Some(*security_id)),
                        )
                        .await;
                }
                let source = decls
                    .first()
                    .and_then(|d| jstr(d, "source"))
                    .unwrap_or_else(|| "issuer".into());
                let _ = apply_cash_moneymarket_followups(
                    canonical,
                    *security_id,
                    &decls,
                    &source,
                    &ran_at,
                )
                .await;
                persist_retrieve_run(
                    canonical,
                    *security_id,
                    "declaration",
                    &ran_at,
                    true,
                    "",
                    "declaration retrieve recorded",
                    1,
                    1,
                    0,
                    0,
                    &payload,
                )
                .await;
            }
            for row in &unchanged {
                if let Some(security_id) = juuid(row, "securityId") {
                    if ok_ids.contains(&security_id) {
                        continue;
                    }
                    persist_retrieve_run(
                        canonical,
                        security_id,
                        "declaration",
                        &ran_at,
                        true,
                        "",
                        "declaration retrieve unchanged",
                        1,
                        0,
                        0,
                        1,
                        row,
                    )
                    .await;
                }
            }
            for miss in &misses {
                if let Some(security_id) = juuid(miss, "securityId") {
                    persist_retrieve_run(
                        canonical,
                        security_id,
                        "declaration",
                        &ran_at,
                        false,
                        &jstr(miss, "code").unwrap_or_else(|| "declaration_retrieve_miss".into()),
                        &jstr(miss, "reason").unwrap_or_else(|| "declaration miss".into()),
                        1,
                        0,
                        1,
                        0,
                        miss,
                    )
                    .await;
                }
            }
            command_ok(
                &request,
                serde_json::to_string(&DeclarationRefreshBody {
                    attempted: attempted + misses.len() as u64,
                    recorded,
                    skipped: skipped + misses.len() as u64,
                })
                .unwrap_or_else(|_| "{\"attempted\":0,\"recorded\":0,\"skipped\":0}".into()),
            )
        }
        "CollectorRetrieve" => match juuid(&json, "securityId") {
            Some(security_id) => {
                let symbol = jstr(&json, "symbol").unwrap_or_default();
                let ran_at = today_iso();
                let mut recorded = 0u64;
                let mut skipped = 0u64;
                let mut unchanged = 0u64;
                let mut ok = true;
                let mut code = String::new();
                let mut message = String::new();
                let hash = jstr(&json, "contentHash")
                    .or_else(|| {
                        json.get("candidates")
                            .and_then(|c| c.as_array())
                            .and_then(|a| a.first())
                            .and_then(|c| jstr(c, "contentHash"))
                    })
                    .unwrap_or_default();
                let source_url = fetched_source_url_for(
                    &json,
                    Some(security_id),
                );

                let div_type = jstr(&json, "divType").unwrap_or_default();
                let inception_on = jstr(&json, "inceptionOn").unwrap_or_default();
                let payment_frequency = jstr(&json, "paymentFrequency").unwrap_or_default();
                let is_unchanged = json
                    .get("unchanged")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if is_unchanged {
                    unchanged = 1;
                    message = "declaration retrieve unchanged".into();
                }

                let declarations = json
                    .get("candidates")
                    .or_else(|| json.get("declarations"))
                    .and_then(|q| q.as_array())
                    .cloned()
                    .unwrap_or_default();
                for decl in &declarations {
                    let amount = decl.get("amountPerShareMinor").and_then(|x| {
                        if x.is_null() {
                            None
                        } else {
                            ji64(decl, "amountPerShareMinor")
                        }
                    });
                    if amount.unwrap_or(0) <= 0 {
                        skipped += 1;
                        continue;
                    }
                    match canonical
                        .issuer_declaration_record(
                            security_id,
                            amount,
                            ju8(decl, "amountScale", 2),
                            jstr(decl, "paymentPeriod").unwrap_or_default(),
                            jstr(decl, "source").unwrap_or_else(|| "issuer".into()),
                            jstr(decl, "enteredAt").unwrap_or_else(|| ran_at.clone()),
                        )
                        .await
                    {
                        Ok(_) => recorded += 1,
                        Err(_) => skipped += 1,
                    }
                }

                let pay_dates = json
                    .get("upcomingPays")
                    .or_else(|| json.get("payDates"))
                    .and_then(|q| q.as_array())
                    .cloned()
                    .unwrap_or_default();
                if !pay_dates.is_empty() {
                    let dates: Vec<IssuerPayDateRecord> = pay_dates
                        .iter()
                        .filter_map(|row| {
                            let pay_on = jstr(row, "payOn")
                                .or_else(|| jstr(row, "paymentPeriod"))
                                .unwrap_or_default();
                            if pay_on.is_empty() {
                                return None;
                            }
                            Some(IssuerPayDateRecord {
                                pay_date_id: Uuid::new_v4(),
                                security_id,
                                pay_on,
                                source: jstr(row, "source").unwrap_or_default(),
                                recorded_at: ran_at.clone(),
                            })
                        })
                        .collect();
                    if !dates.is_empty() {
                        let as_of = dates
                            .iter()
                            .map(|d| d.pay_on.as_str())
                            .min()
                            .unwrap_or(ran_at.as_str())
                            .to_string();
                        let pay_count = dates.len();
                        let calendar_url = payment_calendar_url_for(&json, security_id);
                        if canonical
                            .issuer_pay_date_replace(security_id, as_of, dates)
                            .await
                            .is_ok()
                        {
                            recorded = recorded.saturating_add(1);
                            promote_vendor_calendar_template(
                                canonical,
                                security_id,
                                pay_count,
                                &calendar_url,
                            )
                            .await;
                        }
                    }
                }

                if let Some(quote) = json.get("quote") {
                    if let Some(price_minor) = ji64(quote, "priceMinor").filter(|p| *p > 0) {
                        let source = jstr(quote, "source").unwrap_or_else(|| "yahoo".into());
                        // CASH par is authoritative via CurrentPriceGet; skip writing Yahoo-style quotes.
                        if source != "par" {
                            let _ = canonical
                                .price_quote_record(
                                    security_id,
                                    price_minor,
                                    ju8(quote, "scale", 2),
                                    jstr(quote, "asOfAt").unwrap_or_else(|| ran_at.clone()),
                                    source,
                                )
                                .await;
                            recorded = recorded.saturating_add(1);
                        }
                    }
                }

                let cash_posted = apply_cash_moneymarket_followups(
                    canonical,
                    security_id,
                    &declarations,
                    &jstr(&json, "declarationSource").unwrap_or_else(|| "issuer".into()),
                    &ran_at,
                )
                .await;
                recorded = recorded.saturating_add(cash_posted);

                let misses = json
                    .get("misses")
                    .and_then(|m| m.as_array())
                    .cloned()
                    .unwrap_or_default();
                if !misses.is_empty() {
                    ok = false;
                    code = jstr(&misses[0], "code")
                        .unwrap_or_else(|| "declaration_retrieve_miss".into());
                    message = jstr(&misses[0], "reason")
                        .unwrap_or_else(|| "retrieve miss".into());
                    let _ = raise_retrieve_misses(canonical, &misses, &ran_at).await;
                    skipped = skipped.saturating_add(misses.len() as u64);
                } else if jstr(&json, "declarationSource")
                    .map(|s| {
                        matches!(
                            s.to_ascii_lowercase().as_str(),
                            "fidelity" | "schwab"
                        )
                    })
                    .unwrap_or(false)
                {
                    message = jstr(&json, "missExplanation").unwrap_or_else(|| {
                        "money-market standing (par); issuer distribution page unavailable"
                            .into()
                    });
                    unchanged = unchanged.saturating_add(1);
                }

                let stored_declarations = canonical
                    .issuer_declaration_list(security_id)
                    .await
                    .unwrap_or_default();
                let page_label = jstr(&json, "suggestedFrequency")
                    .or_else(|| jstr(&json, "paymentFrequency"))
                    .filter(|s| !s.trim().is_empty());
                let _ = persist_inferred_frequency_if_unknown(
                    canonical,
                    security_id,
                    page_label.as_deref(),
                )
                .await;
                let verify_issues =
                    declaration_store_verify_issues(&declarations, &stored_declarations);
                if !verify_issues.is_empty() {
                    ok = false;
                    code = "declaration_stored_mismatch".into();
                    message = verify_issues.join("; ");
                }
                let paid_count = count_paid_declarations(&stored_declarations);
                let gate = declaration_retrieve_gate(
                    paid_count,
                    &inception_on,
                    &payment_frequency,
                    &div_type,
                );
                if financial_domain::div1::is_div1(&div_type) {
                    ok = gate.ok;
                    if !gate.code.is_empty() {
                        code = gate.code;
                    }
                    if !gate.message.is_empty() {
                        message = gate.message;
                    }
                    let _ = canonical
                        .retrieval_template_touch_run(
                            security_id,
                            gate.ok,
                            message.clone(),
                            ran_at.clone(),
                            hash.clone(),
                            &source_url,
                        )
                        .await;
                } else if recorded > 0 {
                    let _ = canonical
                        .retrieval_template_touch_run(
                            security_id,
                            true,
                            "collector retrieve recorded".into(),
                            ran_at.clone(),
                            hash.clone(),
                            &source_url,
                        )
                        .await;
                    message = "collector retrieve recorded".into();
                }

                let attempted = recorded + skipped + unchanged;
                let run_id = Uuid::new_v4();
                let payload_json = serde_json::to_string(&json).unwrap_or_else(|_| "{}".into());
                let _ = canonical
                    .retrieve_run_record(RetrieveRunRecord {
                        run_id,
                        security_id,
                        kind: "declaration".into(),
                        requested_at: ran_at,
                        ok,
                        code: code.clone(),
                        message: message.clone(),
                        attempted,
                        recorded,
                        skipped,
                        unchanged,
                        payload_json: payload_json.clone(),
                    })
                    .await;
                command_ok(
                    &request,
                    serde_json::to_string(&CollectorRetrieveBody {
                        security_id,
                        symbol,
                        run_id,
                        attempted,
                        recorded,
                        skipped,
                        unchanged,
                        ok,
                        code,
                        message,
                        payload_json,
                    })
                    .unwrap_or_else(|_| "{}".into()),
                )
            }
            None => command_err(&request, "missing_security_id"),
        },
        "IssuerPayDateReplace" => match juuid(&json, "securityId") {
            Some(security_id) => {
                let as_of = jstr(&json, "asOfDate").unwrap_or_else(today_iso);
                let dates = json
                    .get("dates")
                    .or_else(|| json.get("payDates"))
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|row| {
                        let pay_on = jstr(&row, "payOn")
                            .or_else(|| jstr(&row, "paymentPeriod"))
                            .unwrap_or_default();
                        if pay_on.is_empty() {
                            return None;
                        }
                        Some(IssuerPayDateRecord {
                            pay_date_id: Uuid::new_v4(),
                            security_id,
                            pay_on,
                            source: jstr(&row, "source").unwrap_or_default(),
                            recorded_at: as_of.clone(),
                        })
                    })
                    .collect::<Vec<_>>();
                if dates.is_empty() {
                    command_err(&request, "missing_pay_dates")
                } else {
                    map_c(
                        &request,
                        canonical
                            .issuer_pay_date_replace(security_id, as_of, dates)
                            .await,
                    )
                }
            }
            None => command_err(&request, "missing_security_id"),
        },
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
