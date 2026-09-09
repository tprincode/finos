//! Query/command dispatch owned by application-core (no Tauri, no database).

use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::contracts::{
    AllocationGetBody, CalculatorGetBody, CalculatorRowBody, CommandRequest, CommandResult,
    DeclarationHistoryCellBody, DeclarationHistoryGetBody, DeclarationHistoryRowBody,
    DashboardBody, DashboardBurndownBody, DashboardBurndownLineBody, HoldingsGetBody,
    HoldingsLotBody, DataSummaryBody, ImportCandidate,     IncomePlanDrillBody,
    IncomePlanExportBody, IncomePlanGridBody, IncomePlanLineBody, IncomePlanPositionAccountBody, IncomePlanPositionBody, IncomePlanWeekBody, ProductionSeedDocument, QueryRequest, QueryResult,
    RoiBody, TrendPoint, TrendsBody, TrendsWeekPoint, UpdaterCheckBody, FINANCE_CLIENT_CONTRACT_VERSION,
    PlanReviewBody, PositionCharacteristicRecord, LookthroughResearch, RetrievalTemplateRecord, InvestmentGetBody,
    InvestmentLotBody, InvestmentDeclarationBody, AccountPositionTotalBody, PositionDetailsBody,
    AccountMarketValueDailyRecord, AccountValueHomeBody, AccountValuePointBody,
    AccountValueSeriesBody,
    BacktestPeriodRecord, PositionBacktestResultBody, EvidenceDimensionsBody, TierSuggestionBody,
    RocResearchBody, RocCandidateBody, RocResearchObservation, RemainingYearIncomeBody,
    RemainingPaymentBody, RemainingMonthBody, RemainingPaymentDateOverride,
    LastPriceRefreshBody, DeclarationRefreshBody, PositionMasterGetBody, PositionMasterRowBody,
    ExpectedPaymentPattern, PositionTaxProfile, CurrentPriceBody,
    PositionDetailsCoverageBody, PositionDetailsCoverageRow, Div1ComplianceSummaryBody,
    Div1ComplianceSummaryRow, AccountRecord, ActivityRecord,
    DistributionRecord, LotRecord, IssuerDeclarationRecord, IssuerPayDateRecord,
    ProviderDeclarationSourcesApplyBody, RetrieveRunRecord, CollectorRetrieveBody,
    PositionResearchSeedBody, PositionResearchRefreshBody, ResearchGapItem, ResearchGapsGetBody,
    CollectorSetBody,
    CollectorFieldDecisionRecord,
    RetrieveRunListBody, CashDividendCoverageBody, CashDividendCoverageRow,
    DividendPerformanceBody, DividendPerformanceWeekBody, DividendPerformanceSummaryBody,
    DividendPerformancePositionBody, SecurityRecord, DividendGetBody, BasisGetBody,
    PlanHistoryRecord, WorkTicketRecord, WorkTicketListBody, WorkTicketSyncMissesBody,
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
    "CollectorAlignFutureToPlan",
    "PositionResearchSeed",
    "PositionResearchRefresh",
    "ExpectedPaymentPatternUpsert",
    "PositionTaxProfileUpsert",
    "TrendsWeekSave",
    "TrendsWeekCorrect",
    "TrendsWeekClose",
    "AccountValueSnapshotRecord",
    "WorkTicketResolve",
    "WorkTicketFile",
    "WorkTicketSyncMisses",
    "CollectorFieldDecisionSet",
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
                candidate_id: None,
                validation: String::new(),
                issue: String::new(),
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

/// January 1 of the week-ending year. Remaining issuer dates must cover historical
/// pay weeks in that year; `week.start` of the Jan 1 week is often in December and
/// would truncate remaining to the prior year (dropping MON1 on 2026-08-28).
fn income_plan_remaining_as_of(week_end: &str) -> String {
    match week_end.get(..4) {
        Some(year) if year.len() == 4 && year.bytes().all(|b| b.is_ascii_digit()) => {
            format!("{year}-01-01")
        }
        _ => week_end.to_string(),
    }
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

fn issuer_declaration_for_week<'a>(
    decls: &'a [IssuerDeclarationRecord],
    week_start: &str,
    week_end: &str,
    pay_on: &str,
) -> Option<&'a IssuerDeclarationRecord> {
    decls
        .iter()
        .filter(|d| {
            d.amount_per_share_minor.unwrap_or(0) > 0
                && !d.source.eq_ignore_ascii_case("derived_walk")
                && financial_domain::income_plan::declaration_belongs_in_week(
                    &d.payment_period,
                    week_start,
                    week_end,
                    pay_on,
                )
        })
        .max_by(|a, b| a.entered_at.as_str().cmp(b.entered_at.as_str()))
}

fn declaration_share_fields(
    decl_row: Option<&IssuerDeclarationRecord>,
    as_of: &str,
) -> (Option<i64>, u8, Option<String>, bool) {
    let Some(row) = decl_row else {
        return (None, 0, None, false);
    };
    let per_share = row.amount_per_share_minor.filter(|amt| *amt > 0);
    let entered_on = row
        .entered_at
        .get(..10)
        .unwrap_or(row.entered_at.as_str())
        .to_string();
    let current = per_share.is_some()
        && financial_domain::income_plan::declaration_is_current(&row.entered_at, as_of);
    (per_share, row.amount_scale, Some(entered_on), current)
}

fn income_plan_position_accounts(
    symbol: &str,
    lots: &[&LotRecord],
    accounts: &[AccountRecord],
    plan: Option<&PlanHistoryRecord>,
    pay_on: &str,
    decl_row: Option<&IssuerDeclarationRecord>,
    account_actuals: &std::collections::HashMap<(String, String), i64>,
) -> Vec<IncomePlanPositionAccountBody> {
    let mut lots_by_control: std::collections::BTreeMap<&str, Vec<&LotRecord>> =
        std::collections::BTreeMap::new();
    for lot in lots {
        let Some(account) = accounts.iter().find(|a| a.account_id == lot.account_id) else {
            continue;
        };
        let Some(control) = financial_domain::income_plan::map_income_plan_account(&account.name) else {
            continue;
        };
        lots_by_control.entry(control).or_default().push(*lot);
    }
    let decl_amt = decl_row.and_then(|d| d.amount_per_share_minor.map(|amt| (amt, d.amount_scale)));
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (control, acc_lots) in &lots_by_control {
        let mut planned_minor = 0i64;
        let mut unknown = 0u32;
        for lot in acc_lots {
            if let Some(p) = plan {
                planned_minor += financial_domain::calculator::plan_payment_cents(
                    lot.remaining_quantity_minor,
                    lot.quantity_scale,
                    p.amount_per_share_minor,
                    p.amount_scale,
                );
            } else {
                unknown = unknown.saturating_add(1);
            }
        }
        let declaration_minor = decl_amt
            .map(|(amt, scale)| lot_cash_for_pay(acc_lots, pay_on, amt, scale))
            .unwrap_or(0);
        let display = financial_domain::income_plan::display_account_label(control);
        let key = (symbol.to_string(), display.clone());
        let actual_known = account_actuals.contains_key(&key);
        out.push(IncomePlanPositionAccountBody {
            account_name: display.clone(),
            actual_minor: actual_known
                .then(|| *account_actuals.get(&key).unwrap_or(&0))
                .unwrap_or(0),
            actual_known,
            planned_minor,
            plan_known: !acc_lots.is_empty() && unknown == 0 && plan.is_some(),
            declaration_minor,
            declaration_known: decl_amt.is_some(),
        });
        seen.insert(display);
    }
    let mut extras: Vec<(String, i64)> = account_actuals
        .iter()
        .filter(|((sym, control), _)| sym == symbol && !seen.contains(control))
        .map(|((_, control), amt)| (control.clone(), *amt))
        .collect();
    extras.sort_by(|a, b| a.0.cmp(&b.0));
    for (control, actual_minor) in extras {
        out.push(IncomePlanPositionAccountBody {
            account_name: financial_domain::income_plan::display_account_label(&control),
            actual_minor,
            actual_known: true,
            planned_minor: 0,
            plan_known: false,
            declaration_minor: 0,
            declaration_known: false,
        });
    }
    out
}

fn lot_cash_for_pay(lots: &[&LotRecord], pay_on: &str, amount_per_share_minor: i64, amount_scale: u8) -> i64 {
    lots.iter()
        .filter(|lot| {
            let opened = lot.opened_on.get(..10).unwrap_or(lot.opened_on.as_str());
            opened <= pay_on
        })
        .map(|lot| {
            financial_domain::calculator::plan_payment_cents(
                lot.remaining_quantity_minor,
                lot.quantity_scale,
                amount_per_share_minor,
                amount_scale,
            )
        })
        .sum()
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
    let latest = latest_declaration_period(canonical, security_id, as_of).await?;
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
    as_of: &str,
) -> Result<Option<String>, PlatformError> {
    let decls = canonical.issuer_declaration_list(security_id).await?;
    Ok(financial_domain::schedule::latest_parseable_period(
        decls.iter().filter_map(|d| {
            financial_domain::schedule::period_has_occurred(&d.payment_period, as_of)
                .then_some(d.payment_period.as_str())
        }),
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
            let tmpl = canonical
                .retrieval_template_get(id)
                .await
                .ok()
                .flatten();
            let src = jstr(miss, "declarationSource").unwrap_or_else(|| {
                tmpl.as_ref()
                    .map(|t| t.declaration_source.clone())
                    .unwrap_or_default()
            });
            let stored_url = tmpl.as_ref().map(|t| t.source_url.as_str()).unwrap_or("");
            if financial_domain::mlp_sec::is_adapter_kind(&src)
                || financial_domain::mlp_sec::routes_fetch(&src, Some(stored_url))
            {
                let outcome = mlp_sec_outcome_from_misses(std::slice::from_ref(miss));
                let _ = persist_mlp_sec_last_run(
                    canonical,
                    id,
                    &today_iso(),
                    outcome,
                    ran_at.to_string(),
                    hash,
                )
                .await;
            } else {
                let _ = canonical
                    .retrieval_template_touch_run(
                        id,
                        false,
                        message,
                        ran_at.to_string(),
                        hash,
                        "",
                    )
                    .await;
            }
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

fn is_cash_rate_candidate(row: &Value) -> bool {
    jstr(row, "kind")
        .map(|k| k.eq_ignore_ascii_case("cash_rate"))
        .unwrap_or(false)
        || row.get("planOnly").and_then(|v| v.as_bool()).unwrap_or(false)
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
    let rate_only: Vec<&Value> = candidates
        .iter()
        .filter(|c| is_cash_rate_candidate(c))
        .collect();
    let paid: Vec<&Value> = if !rate_only.is_empty() {
        rate_only
    } else {
        candidates
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
            .collect()
    };
    let Some(latest) = paid.first() else {
        return 0;
    };
    let Some(rate_minor) = ji64(latest, "amountPerShareMinor").filter(|a| *a > 0) else {
        return 0;
    };
    let rate_scale = ju8(latest, "amountScale", 5);
    let plans = canonical.plan_history_list().await.unwrap_or_default();
    let current = plans.iter().find(|p| p.security_id == security_id);
    let yield_pct = jstr(latest, "sevenDayYield")
        .or_else(|| {
            ji64(latest, "annualYieldBps").map(|bps| format!("{:.2}", bps as f64 / 100.0))
        })
        .unwrap_or_default();
    let reason = if yield_pct.is_empty() {
        format!("money-market 7-day yield from {source}")
    } else {
        format!("money-market 7-day yield {yield_pct}% from {source}")
    };
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
        if is_cash_rate_candidate(cand) {
            continue;
        }
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
    pick_date: String,
    closed_as_of: String,
) -> Result<IncomePlanWeekBody, PlatformError> {
    let accounts = canonical.account_list().await?;
    let securities = canonical.security_list().await?;
    let dividend = canonical.dividend_get().await?;
    let plans = canonical.plan_history_list().await?;
    let characteristics = canonical.position_characteristic_list().await?;
    let basis = canonical.basis_get().await?;
    let mut decl_dates = std::collections::HashMap::new();
    income_plan_week_with(
        canonical,
        &accounts,
        &securities,
        &dividend,
        &plans,
        &characteristics,
        &basis,
        pick_date,
        &mut decl_dates,
        if closed_as_of.trim().is_empty() {
            None
        } else {
            Some(closed_as_of)
        },
    )
    .await
}

async fn income_plan_grid_view(
    canonical: &dyn Canonical,
    as_of_date: String,
    historical_weeks: u32,
    future_weeks: u32,
    accounts: Vec<String>,
    selected_week_end: String,
) -> Result<IncomePlanGridBody, PlatformError> {
    let as_of = parse_iso_date(&as_of_date).unwrap_or_else(|| {
        chrono::Local::now().date_naive()
    });
    let hist = financial_domain::income_plan::clamp_week_count(historical_weeks as i64);
    let fut = financial_domain::income_plan::clamp_week_count(future_weeks as i64);
    let visible = financial_domain::income_plan::visible_grid_weeks(as_of, hist, fut);
    let year = crate::income_plan_display::year_weeks_for(as_of);
    let accounts_list = canonical.account_list().await?;
    let securities = canonical.security_list().await?;
    let dividend = canonical.dividend_get().await?;
    let plans = canonical.plan_history_list().await?;
    let characteristics = canonical.position_characteristic_list().await?;
    let basis = canonical.basis_get().await?;
    let mut cache = std::collections::HashMap::new();
    // Sequential load (cache is shared). Year-start first so remaining dates cover the year.
    let mut year_bodies = Vec::new();
    for gw in &year {
        year_bodies.push(
            income_plan_week_with(
                canonical,
                &accounts_list,
                &securities,
                &dividend,
                &plans,
                &characteristics,
                &basis,
                gw.week.end.to_string(),
                &mut cache,
                Some(as_of_date.clone()),
            )
            .await
            .map(|body| (gw.kind, body))?,
        );
    }
    let mut week_bodies = Vec::new();
    for gw in &visible {
        if let Some(existing) = year_bodies.iter().find(|(_, b)| b.end == gw.week.end.to_string())
        {
            week_bodies.push((gw.kind, existing.1.clone()));
            continue;
        }
        week_bodies.push(
            income_plan_week_with(
                canonical,
                &accounts_list,
                &securities,
                &dividend,
                &plans,
                &characteristics,
                &basis,
                gw.week.end.to_string(),
                &mut cache,
                Some(as_of_date.clone()),
            )
            .await
            .map(|body| (gw.kind, body))?,
        );
    }
    let selected_end = if selected_week_end.trim().is_empty() {
        visible
            .iter()
            .find(|w| matches!(w.kind, financial_domain::income_plan::GridWeekKind::InProgress))
            .map(|w| w.week.end.to_string())
            .unwrap_or_else(|| visible.last().map(|w| w.week.end.to_string()).unwrap_or_default())
    } else {
        selected_week_end
    };
    Ok(crate::income_plan_display::assemble_grid(
        as_of.format("%Y-%m-%d").to_string(),
        hist,
        fut,
        crate::income_plan_display::normalize_selected_accounts(&accounts),
        selected_end,
        &week_bodies,
        &year_bodies,
    ))
}

async fn income_plan_export_view(
    canonical: &dyn Canonical,
    json: &Value,
) -> Result<IncomePlanExportBody, PlatformError> {
    let pattern = jstr(json, "pattern").unwrap_or_else(|| "A".into());
    let format = jstr(json, "format").unwrap_or_else(|| "pdf".into());
    let as_of = jstr(json, "asOfDate").unwrap_or_else(today_iso);
    let hist = json
        .get("historicalWeeks")
        .and_then(|v| v.as_i64())
        .unwrap_or(6);
    let fut = json
        .get("futureWeeks")
        .and_then(|v| v.as_i64())
        .unwrap_or(6);
    let accounts: Vec<String> = json
        .get("accounts")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let selected_week = jstr(json, "weekEnding").unwrap_or_default();
    let printed_at = jstr(json, "printedAt").unwrap_or_else(|| {
        chrono::Utc::now().format("%Y-%m-%dT%H:%MZ").to_string()
    });
    let grid = income_plan_grid_view(
        canonical,
        as_of.clone(),
        financial_domain::income_plan::clamp_week_count(hist),
        financial_domain::income_plan::clamp_week_count(fut),
        accounts.clone(),
        selected_week.clone(),
    )
    .await?;
    let week = if pattern.eq_ignore_ascii_case("B") {
        let week_as_of = if selected_week.trim().is_empty() {
            as_of.clone()
        } else {
            selected_week.clone()
        };
        Some(income_plan_week_view(canonical, week_as_of, as_of.clone()).await?)
    } else {
        None
    };
    crate::income_plan_display::build_export(crate::income_plan_display::ExportRequest {
        pattern: &pattern,
        format: &format,
        printed_at: &printed_at,
        grid: Some(&grid),
        week: week.as_ref(),
        selected_accounts: &grid.selected_accounts,
        historical_weeks: grid.historical_weeks,
        future_weeks: grid.future_weeks,
    })
    .map_err(|e| PlatformError::new("export_failed", e))
}

async fn income_plan_week_with(
    canonical: &dyn Canonical,
    accounts: &[AccountRecord],
    securities: &[SecurityRecord],
    dividend: &DividendGetBody,
    plans: &[PlanHistoryRecord],
    characteristics: &[PositionCharacteristicRecord],
    basis: &BasisGetBody,
    as_of_date: String,
    decl_dates: &mut std::collections::HashMap<uuid::Uuid, Vec<String>>,
    closed_as_of: Option<String>,
) -> Result<IncomePlanWeekBody, PlatformError> {
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
    let week = canonical.canonical_week_get(as_of.clone()).await?;
    let mut actuals: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut position_actuals: std::collections::HashMap<String, i64> =
        std::collections::HashMap::new();
    let mut position_account_actuals: std::collections::HashMap<(String, String), i64> =
        std::collections::HashMap::new();
    let mut drilldown = Vec::new();
    let mut add_actual = |symbol: String,
                          control: &str,
                          occurred_on: &str,
                          amount_minor: i64,
                          scale: u8| {
        let display = financial_domain::income_plan::display_account_label(control);
        *actuals.entry(display.clone()).or_insert(0) += amount_minor;
        *position_actuals.entry(symbol.clone()).or_insert(0) += amount_minor;
        *position_account_actuals
            .entry((symbol.clone(), display.clone()))
            .or_insert(0) += amount_minor;
        drilldown.push(IncomePlanDrillBody {
            account_name: display,
            symbol,
            occurred_on: occurred_on.to_string(),
            amount_minor,
            scale,
        });
    };
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
        let Some(control) = financial_domain::income_plan::map_income_plan_account(&account.name) else {
            continue;
        };
        let symbol = actual
            .security_id
            .and_then(|id| {
                securities
                    .iter()
                    .find(|s| s.security_id == id)
                    .map(|s| s.symbol.clone())
            })
            .unwrap_or_else(|| "—".into());
        add_actual(
            symbol,
            control,
            &actual.occurred_on,
            actual.amount_minor,
            actual.scale,
        );
    }
    if let Ok(activities) = canonical.activity_list().await {
        for act in activities {
            if !financial_domain::income_plan::is_income_cash_activity(&act.activity_type) {
                continue;
            }
            if !act.activity_type.eq_ignore_ascii_case("interest") {
                continue;
            }
            if !financial_domain::income_plan::occurred_in_week(
                &act.occurred_on,
                &week.start,
                &week.end,
            ) {
                continue;
            }
            let Some(account) = accounts.iter().find(|a| a.account_id == act.account_id) else {
                continue;
            };
            let Some(control) = financial_domain::income_plan::map_income_plan_account(&account.name)
            else {
                continue;
            };
            let symbol = act
                .security_id
                .and_then(|id| {
                    securities
                        .iter()
                        .find(|s| s.security_id == id)
                        .map(|s| s.symbol.clone())
                })
                .unwrap_or_else(|| "CASH".into());
            add_actual(symbol, control, &act.occurred_on, act.amount_minor, act.scale);
        }
    }
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
    struct AccountWeek {
        planned: i64,
        scheduled: u32,
        unknown_plan: u32,
    }
    let mut account_week: std::collections::HashMap<String, AccountWeek> =
        financial_domain::income_plan::CONTROL_ACCOUNTS
            .iter()
            .map(|name| {
                (
                    financial_domain::income_plan::display_account_label(name),
                    AccountWeek {
                        planned: 0,
                        scheduled: 0,
                        unknown_plan: 0,
                    },
                )
            })
            .collect();
    let plan_versions = canonical
        .plan_history_version_list()
        .await
        .unwrap_or_else(|_| plans.to_vec());
    let plan_catalog: std::collections::HashMap<uuid::Uuid, _> = plans
        .iter()
        .map(|p| (p.security_id, p))
        .collect();
    let freq_catalog: std::collections::HashMap<uuid::Uuid, String> = characteristics
        .iter()
        .map(|c| (c.security_id, c.payment_frequency.clone()))
        .collect();
    let mut lots_by_security: std::collections::HashMap<uuid::Uuid, Vec<&LotRecord>> =
        std::collections::HashMap::new();
    for lot in &basis.lots {
        if lot.remaining_quantity_minor <= 0 {
            continue;
        }
        let opened = lot.opened_on.get(..10).unwrap_or(lot.opened_on.as_str());
        if opened > week.end.as_str() {
            continue;
        }
        lots_by_security
            .entry(lot.security_id)
            .or_default()
            .push(lot);
    }
    let last_update_by_sec: std::collections::HashMap<uuid::Uuid, Option<String>> =
        match canonical.collector_set().await {
            Ok(set) => set
                .items
                .into_iter()
                .map(|item| {
                    (
                        item.security_id,
                        financial_domain::income_plan::last_update_success(
                            item.last_run_ok,
                            &item.last_run_at,
                        ),
                    )
                })
                .collect(),
            Err(_) => std::collections::HashMap::new(),
        };
    let mut positions: Vec<IncomePlanPositionBody> = Vec::new();
    let mut listed: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut decl_cache: std::collections::HashMap<Uuid, Vec<IssuerDeclarationRecord>> =
        std::collections::HashMap::new();
    for (security_id, lots) in &lots_by_security {
        let freq = freq_catalog
            .get(security_id)
            .map(|s| s.as_str())
            .unwrap_or("");
        let cadence = financial_domain::calculator::PaymentCadence::parse(freq);
        if cadence == Some(financial_domain::calculator::PaymentCadence::None) {
            continue;
        }
        let Some(periods) = cadence.and_then(|c| c.periods()) else {
            continue;
        };
        let remaining = remaining_pay_dates_for(
            canonical,
            *security_id,
            &income_plan_remaining_as_of(&week.end),
            periods,
            &mut *decl_dates,
        )
        .await?;
        let remaining_refs: Vec<&str> = remaining.iter().map(|s| s.as_str()).collect();
        let last = last_actual.get(security_id).map(|s| s.as_str());
        let calendar_pay = financial_domain::schedule::pay_on_for_week(
            periods,
            &week.start,
            &week.end,
            &remaining_refs,
            last,
        );
        let Some(sec) = securities.iter().find(|s| s.security_id == *security_id) else {
            continue;
        };
        let actual_known = position_actuals.contains_key(&sec.symbol);
        let actual = actual_known
            .then(|| *position_actuals.get(&sec.symbol).unwrap_or(&0))
            .unwrap_or(0);
        if !decl_cache.contains_key(security_id) {
            decl_cache.insert(
                *security_id,
                canonical
                    .issuer_declaration_list(*security_id)
                    .await
                    .unwrap_or_default(),
            );
        }
        let decls = decl_cache.get(security_id).map(|v| v.as_slice()).unwrap_or(&[]);
        let pay_hint = calendar_pay.clone().unwrap_or_default();
        let decl_row = issuer_declaration_for_week(decls, &week.start, &week.end, &pay_hint);
        let declaration_known = decl_row.is_some();
        if calendar_pay.is_none() && !actual_known && !declaration_known {
            continue;
        }
        let pay_on = calendar_pay
            .or_else(|| {
                decl_row.map(|d| {
                    let p = d.payment_period.as_str();
                    if p.len() >= 10 { p[..10].to_string() } else { p.to_string() }
                })
            })
            .unwrap_or_else(|| {
                drilldown
                    .iter()
                    .filter(|d| d.symbol == sec.symbol)
                    .map(|d| d.occurred_on.clone())
                    .min()
                    .unwrap_or_default()
            });
        let declaration_minor = decl_row
            .and_then(|d| d.amount_per_share_minor.map(|amt| (amt, d.amount_scale)))
            .map(|(amt, scale)| lot_cash_for_pay(lots, &pay_on, amt, scale))
            .unwrap_or(0);
        let (
            declaration_per_share_minor,
            declaration_per_share_scale,
            declaration_entered_on,
            declaration_current,
        ) = declaration_share_fields(decl_row, &as_of);
        let plan = plan_catalog.get(security_id);
        let windows: Vec<financial_domain::plan::PlanAmountWindow<'_>> = plan_versions
            .iter()
            .filter(|p| p.security_id == *security_id)
            .map(|p| financial_domain::plan::PlanAmountWindow {
                effective_from: p.effective_from.as_str(),
                effective_to: p.effective_to.as_str(),
                amount_per_share_minor: p.amount_per_share_minor,
                amount_scale: p.amount_scale,
            })
            .collect();
        let mut plan_for_week = plan.map(|p| (*p).clone());
        if let Some((amt, scale)) =
            financial_domain::income_plan::plan_amount_on_pay_date(&windows, &pay_on)
        {
            if let Some(p) = plan_for_week.as_mut() {
                p.amount_per_share_minor = amt;
                p.amount_scale = scale;
            }
        }
        let plan_known = plan_for_week.is_some();
        let mut planned_minor = 0i64;
        for lot in lots {
            let Some(account) = accounts.iter().find(|a| a.account_id == lot.account_id) else {
                continue;
            };
            let Some(control) = financial_domain::income_plan::map_income_plan_account(&account.name)
            else {
                continue;
            };
            let display = financial_domain::income_plan::display_account_label(control);
            let entry = account_week.entry(display).or_insert(AccountWeek {
                planned: 0,
                scheduled: 0,
                unknown_plan: 0,
            });
            entry.scheduled = entry.scheduled.saturating_add(1);
            if let Some(p) = plan_for_week.as_ref() {
                let pay = financial_domain::calculator::plan_payment_cents(
                    lot.remaining_quantity_minor,
                    lot.quantity_scale,
                    p.amount_per_share_minor,
                    p.amount_scale,
                );
                planned_minor += pay;
                entry.planned += pay;
            } else {
                entry.unknown_plan = entry.unknown_plan.saturating_add(1);
            }
        }
        listed.insert(sec.symbol.clone());
        let account_slices = income_plan_position_accounts(
            &sec.symbol,
            lots,
            accounts,
            plan_for_week.as_ref(),
            &pay_on,
            decl_row,
            &position_account_actuals,
        );
        positions.push(IncomePlanPositionBody {
            symbol: sec.symbol.clone(),
            cadence: cadence
                .map(|c| c.label().to_string())
                .unwrap_or_default(),
            pay_on,
            actual_minor: actual,
            actual_known,
            planned_minor,
            plan_known,
            declaration_minor,
            declaration_known,
            declaration_per_share_minor,
            declaration_per_share_scale,
            declaration_entered_on,
            declaration_current,
            scale: 2,
            accounts: account_slices,
            last_update: last_update_by_sec.get(security_id).cloned().flatten(),
        });
    }
    for (symbol, actual) in &position_actuals {
        if *actual == 0 || listed.contains(symbol) {
            continue;
        }
        let account_slices = income_plan_position_accounts(
            symbol,
            &[],
            accounts,
            None,
            "",
            None,
            &position_account_actuals,
        );
        positions.push(IncomePlanPositionBody {
            symbol: symbol.clone(),
            cadence: String::new(),
            pay_on: drilldown
                .iter()
                .filter(|d| d.symbol == *symbol)
                .map(|d| d.occurred_on.clone())
                .min()
                .unwrap_or_default(),
            actual_minor: *actual,
            actual_known: true,
            planned_minor: 0,
            plan_known: false,
            declaration_minor: 0,
            declaration_known: false,
            declaration_per_share_minor: None,
            declaration_per_share_scale: 0,
            declaration_entered_on: None,
            declaration_current: false,
            scale: 2,
            accounts: account_slices,
            last_update: None,
        });
    }
    positions.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    let mut line_names: Vec<String> = financial_domain::income_plan::CONTROL_ACCOUNTS
        .iter()
        .map(|name| financial_domain::income_plan::display_account_label(name))
        .collect();
    for extra in ["Speculation", "Energy", "Robinhood"] {
        if (account_week.contains_key(extra) || actuals.contains_key(extra))
            && !line_names.iter().any(|n| n == extra)
        {
            line_names.push(extra.to_string());
        }
    }
    let lines: Vec<IncomePlanLineBody> = line_names
        .iter()
        .map(|name| {
            let row = account_week.get(name);
            let scheduled = row.map(|r| r.scheduled).unwrap_or(0);
            let unknown = row.map(|r| r.unknown_plan).unwrap_or(0);
            let plan_known = scheduled > 0 && unknown == 0;
            IncomePlanLineBody {
                account_name: name.clone(),
                actual_minor: *actuals.get(name).unwrap_or(&0),
                plan_known,
                planned_minor: if plan_known {
                    row.map(|r| r.planned).unwrap_or(0)
                } else {
                    0
                },
                scale: 2,
            }
        })
        .collect();
    let cutoff = closed_as_of.as_deref().unwrap_or(as_of.as_str());
    let week_closed = week.end.as_str() < cutoff;
    let mut miss_count = 0u64;
    let mut amount_exception_count = 0u64;
    let mut variance_minor = None;
    if week_closed {
        let mut planned = 0i64;
        let mut actual = 0i64;
        for pos in &positions {
            if pos.plan_known {
                planned += pos.planned_minor;
            }
            let paid = pos.actual_known && pos.actual_minor != 0;
            if pos.plan_known && pos.planned_minor != 0 && !paid {
                miss_count = miss_count.saturating_add(1);
            } else if pos.plan_known && paid {
                let d = pos.actual_minor - pos.planned_minor;
                if d.abs() > financial_domain::income_plan::TABLE2_OK_TOLERANCE_MINOR {
                    amount_exception_count = amount_exception_count.saturating_add(1);
                }
            }
            if pos.actual_known {
                actual += pos.actual_minor;
            }
        }
        variance_minor = Some(actual - planned);
    }
    Ok(IncomePlanWeekBody {
        as_of_date: week.as_of_date,
        start: week.start.clone(),
        end: week.end.clone(),
        week_year: week.week_year,
        week_number: week.week_number,
        status: "Open".into(),
        lines,
        positions,
        drilldown,
        latest_actual_on,
        yield_count,
        scale: 2,
        miss_count,
        amount_exception_count,
        variance_minor,
    })
}

fn parse_iso_date(s: &str) -> Option<chrono::NaiveDate> {
    let day = if s.len() >= 10 { &s[..10] } else { s };
    chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()
}

fn performance_week_from_plan(week: &IncomePlanWeekBody) -> DividendPerformanceWeekBody {
    let plan_known = !week.positions.is_empty() && week.positions.iter().all(|p| p.plan_known);
    let planned_minor = if plan_known {
        week.positions.iter().map(|p| p.planned_minor).sum()
    } else {
        0
    };
    let actual_known = week.positions.iter().any(|p| p.actual_known);
    let actual_minor: i64 = week
        .positions
        .iter()
        .filter(|p| p.actual_known)
        .map(|p| p.actual_minor)
        .sum();
    let declaration_known = week.positions.iter().any(|p| p.declaration_known);
    let declaration_minor: i64 = week
        .positions
        .iter()
        .filter(|p| p.declaration_known)
        .map(|p| p.declaration_minor)
        .sum();
    DividendPerformanceWeekBody {
        start: week.start.clone(),
        end: week.end.clone(),
        week_year: week.week_year,
        week_number: week.week_number,
        actual_minor,
        actual_known,
        planned_minor,
        plan_known,
        declaration_minor,
        declaration_known,
        pct_of_plan_minor: financial_domain::income_plan::pct_of_plan_minor(
            actual_minor,
            plan_known && actual_known,
            planned_minor,
        ),
        positions: week
            .positions
            .iter()
            .map(|p| DividendPerformancePositionBody {
                symbol: p.symbol.clone(),
                actual_minor: p.actual_minor,
                actual_known: p.actual_known,
                planned_minor: p.planned_minor,
                plan_known: p.plan_known,
                declaration_minor: p.declaration_minor,
                declaration_known: p.declaration_known,
                pct_of_plan_minor: financial_domain::income_plan::pct_of_plan_minor(
                    p.actual_minor,
                    p.plan_known && p.actual_known,
                    p.planned_minor,
                ),
                scale: p.scale,
            })
            .collect(),
        scale: week.scale,
    }
}

fn empty_performance_summary(scale: u8) -> DividendPerformanceSummaryBody {
    DividendPerformanceSummaryBody {
        actual_minor: 0,
        planned_minor: 0,
        plan_known: false,
        pct_of_plan_minor: None,
        avg_weekly_actual_minor: None,
        avg_weekly_plan_minor: None,
        week_count: 0,
        known_plan_week_count: 0,
        scale,
    }
}

async fn dividend_performance_view(
    canonical: &dyn Canonical,
    as_of_date: String,
    range: String,
) -> Result<DividendPerformanceBody, PlatformError> {
    let accounts = canonical.account_list().await?;
    let securities = canonical.security_list().await?;
    let dividend = canonical.dividend_get().await?;
    let plans = canonical.plan_history_list().await?;
    let characteristics = canonical.position_characteristic_list().await?;
    let basis = canonical.basis_get().await?;
    let latest_actual_on = dividend.actuals.iter().map(|a| a.occurred_on.clone()).max();
    let as_of = if as_of_date.trim().is_empty() {
        latest_actual_on.clone().unwrap_or_else(today_iso)
    } else {
        as_of_date
    };
    let as_of_d = parse_iso_date(&as_of).ok_or_else(|| {
        PlatformError::new("invalid_date", format!("invalid asOfDate {as_of}"))
    })?;
    let range_key = if range.trim().is_empty() {
        "ytd".to_string()
    } else {
        range.trim().to_ascii_lowercase()
    };
    let range_start_d = financial_domain::income_plan::performance_range_start(as_of_d, &range_key)
        .map_err(|code| PlatformError::new(code, format!("unsupported range {range_key}")))?;
    let this_week = financial_domain::week::week_containing(as_of_d);
    let mut earliest: Option<chrono::NaiveDate> = None;
    for actual in &dividend.actuals {
        if let Some(d) = parse_iso_date(&actual.occurred_on) {
            earliest = Some(earliest.map_or(d, |e| e.min(d)));
        }
    }
    for lot in &basis.lots {
        if lot.remaining_quantity_minor <= 0 {
            continue;
        }
        if let Some(d) = parse_iso_date(&lot.opened_on) {
            earliest = Some(earliest.map_or(d, |e| e.min(d)));
        }
    }
    let walk_from = match (range_start_d, earliest) {
        (Some(rs), Some(e)) => rs.max(e),
        (Some(rs), None) => rs,
        (None, Some(e)) => e,
        (None, None) => {
            return Ok(DividendPerformanceBody {
                as_of_date: as_of,
                range: range_key,
                range_start: None,
                this_week_start: this_week.start.to_string(),
                weeks: Vec::new(),
                summary: empty_performance_summary(2),
                scale: 2,
            });
        }
    };
    let mut decl_dates = std::collections::HashMap::new();
    let mut last_year: Option<i32> = None;
    let mut weeks: Vec<DividendPerformanceWeekBody> = Vec::new();
    let mut cursor = financial_domain::week::week_containing(walk_from).start;
    while cursor < this_week.start {
        let w = financial_domain::week::week_containing(cursor);
        if w.end >= this_week.start {
            break;
        }
        let year = chrono::Datelike::year(&w.start);
        if last_year != Some(year) {
            decl_dates.clear();
            last_year = Some(year);
        }
        let in_range = range_start_d.map(|rs| w.end >= rs).unwrap_or(true);
        if in_range {
            let view = income_plan_week_with(
                canonical,
                &accounts,
                &securities,
                &dividend,
                &plans,
                &characteristics,
                &basis,
                w.end.to_string(),
                &mut decl_dates,
                None,
            )
            .await?;
            let actual_minor: i64 = view.positions.iter().map(|p| p.actual_minor).sum();
            if !view.positions.is_empty() || actual_minor > 0 {
                weeks.push(performance_week_from_plan(&view));
            }
        }
        cursor += chrono::Duration::days(7);
    }
    weeks.sort_by(|a, b| a.end.cmp(&b.end));
    let week_count = weeks.len() as u32;
    let actual_total: i64 = weeks.iter().map(|w| w.actual_minor).sum();
    let known: Vec<&DividendPerformanceWeekBody> =
        weeks.iter().filter(|w| w.plan_known).collect();
    let known_plan_week_count = known.len() as u32;
    let planned_total: i64 = known.iter().map(|w| w.planned_minor).sum();
    let known_actual: i64 = known.iter().map(|w| w.actual_minor).sum();
    let summary = DividendPerformanceSummaryBody {
        actual_minor: actual_total,
        planned_minor: planned_total,
        plan_known: known_plan_week_count > 0,
        pct_of_plan_minor: financial_domain::income_plan::pct_of_plan_minor(
            known_actual,
            known_plan_week_count > 0,
            planned_total,
        ),
        avg_weekly_actual_minor: if known_plan_week_count == 0 {
            None
        } else {
            Some(known_actual / i64::from(known_plan_week_count))
        },
        avg_weekly_plan_minor: if known_plan_week_count == 0 {
            None
        } else {
            Some(planned_total / i64::from(known_plan_week_count))
        },
        week_count,
        known_plan_week_count,
        scale: 2,
    };
    Ok(DividendPerformanceBody {
        as_of_date: as_of,
        range: range_key,
        range_start: range_start_d.map(|d| d.to_string()),
        this_week_start: this_week.start.to_string(),
        weeks,
        summary,
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
    let week = income_plan_week_view(canonical, as_of_date.clone(), as_of_date).await?;
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
        start: week.start.clone(),
        end: week.end.clone(),
        week_year: week.week_year,
        week_number: week.week_number,
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
            period_start: {
                financial_domain::week::parse_iso_day(&src.period_end)
                    .map(|d| financial_domain::week::week_containing(d).start.to_string())
                    .unwrap_or_default()
            },
            week_year: financial_domain::week::parse_iso_day(&src.period_end)
                .map(|d| financial_domain::week::week_id_containing(d).year)
                .unwrap_or(0),
            week_number: financial_domain::week::parse_iso_day(&src.period_end)
                .map(|d| financial_domain::week::week_id_containing(d).number)
                .unwrap_or(0),
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

fn today_local_iso() -> String {
    chrono::Local::now().date_naive().to_string()
}

async fn record_account_value_snapshot(
    canonical: &dyn crate::ports::canonical::Canonical,
    as_of: &str,
) -> Result<AccountValueHomeBody, crate::ports::platform::PlatformError> {
    let details = position_details_summary(
        canonical,
        &serde_json::json!({ "asOfDate": as_of }),
    )
    .await?;
    let accounts = canonical.account_list().await?;
    let captured_at = chrono::Utc::now().to_rfc3339();
    let mut live: Vec<(String, String, Option<i64>)> = Vec::new();
    for acct in accounts.iter().filter(|a| is_data_account(&a.name)) {
        let mv = details
            .account_totals
            .iter()
            .find(|t| t.account_id == acct.account_id)
            .map(|t| t.market_value_minor)
            .unwrap_or(Some(0));
        live.push((acct.account_id.to_string(), acct.name.clone(), mv));
        canonical
            .account_market_value_daily_upsert(AccountMarketValueDailyRecord {
                snapshot_id: uuid::Uuid::new_v4().to_string(),
                account_id: acct.account_id.to_string(),
                account_name: acct.name.clone(),
                as_of: as_of.to_string(),
                market_value_minor: mv,
                market_value_complete: mv.is_some(),
                scale: 2,
                captured_at: captured_at.clone(),
            })
            .await?;
    }
    let fid_rows: Vec<(String, Option<i64>)> = live
        .iter()
        .map(|(_, name, mv)| (name.clone(), *mv))
        .collect();
    let (fid_mv, fid_ok) = financial_domain::account_value::fidelity_total_from_accounts(&fid_rows);
    let (sch_mv, sch_ok) = financial_domain::account_value::schwab_total_from_accounts(&fid_rows);
    canonical
        .account_market_value_daily_upsert(AccountMarketValueDailyRecord {
            snapshot_id: uuid::Uuid::new_v4().to_string(),
            account_id: financial_domain::account_value::FIDELITY_TOTAL_ID.to_string(),
            account_name: financial_domain::account_value::FIDELITY_TOTAL_NAME.to_string(),
            as_of: as_of.to_string(),
            market_value_minor: fid_mv,
            market_value_complete: fid_ok,
            scale: 2,
            captured_at: captured_at.clone(),
        })
        .await?;
    canonical
        .account_market_value_daily_upsert(AccountMarketValueDailyRecord {
            snapshot_id: uuid::Uuid::new_v4().to_string(),
            account_id: financial_domain::account_value::SCHWAB_TOTAL_ID.to_string(),
            account_name: financial_domain::account_value::SCHWAB_TOTAL_NAME.to_string(),
            as_of: as_of.to_string(),
            market_value_minor: sch_mv,
            market_value_complete: sch_ok,
            scale: 2,
            captured_at,
        })
        .await?;
    account_value_home_view(canonical, as_of).await
}

fn overlay_live_point(
    mut points: Vec<AccountValuePointBody>,
    as_of: &str,
    current: Option<i64>,
) -> Vec<AccountValuePointBody> {
    if let Some(point) = points.iter_mut().find(|p| p.as_of == as_of) {
        point.market_value_minor = current;
        point.market_value_complete = current.is_some();
        return points;
    }
    points.push(AccountValuePointBody {
        as_of: as_of.to_string(),
        market_value_minor: current,
        market_value_complete: current.is_some(),
    });
    points
}

fn history_points_for(
    history: &[AccountMarketValueDailyRecord],
    account_id: &str,
) -> Vec<AccountValuePointBody> {
    history
        .iter()
        .filter(|h| h.account_id == account_id)
        .map(|h| AccountValuePointBody {
            as_of: h.as_of.clone(),
            market_value_minor: h.market_value_minor,
            market_value_complete: h.market_value_complete,
        })
        .collect()
}

fn trends_points_for(name: &str, weeks: &[TrendsWeekPoint]) -> Vec<AccountValuePointBody> {
    weeks
        .iter()
        .filter_map(|week| {
            financial_domain::account_value::trends_balance_for_account(
                name,
                week.fidelity_total_minor,
                week.schwab_total_minor,
                week.income_balance_minor,
                week.car_balance_minor,
                week.health_balance_minor,
                week.roth_balance_minor,
                week.speculation_balance_minor,
            )
            .map(|market_value_minor| AccountValuePointBody {
                as_of: week.period_end.clone(),
                market_value_minor: Some(market_value_minor),
                market_value_complete: true,
            })
        })
        .collect()
}

async fn account_value_home_view(
    canonical: &dyn crate::ports::canonical::Canonical,
    as_of: &str,
) -> Result<AccountValueHomeBody, crate::ports::platform::PlatformError> {
    let details = position_details_summary(
        canonical,
        &serde_json::json!({ "asOfDate": as_of }),
    )
    .await?;
    let accounts = canonical.account_list().await?;
    let history = canonical.account_market_value_daily_list().await?;
    let weeks = account_trends_weeks(canonical).await?;
    let mut series = Vec::new();
    let mut live_rows: Vec<(String, Option<i64>)> = Vec::new();
    for acct in accounts.iter().filter(|a| is_data_account(&a.name)) {
        let current = details
            .account_totals
            .iter()
            .find(|t| t.account_id == acct.account_id)
            .map(|t| t.market_value_minor)
            .unwrap_or(Some(0));
        live_rows.push((acct.name.clone(), current));
        let points = overlay_live_point(
            history_points_for(&history, &acct.account_id.to_string()),
            as_of,
            current,
        );
        series.push(AccountValueSeriesBody {
            account_id: acct.account_id.to_string(),
            account_name: acct.name.clone(),
            custodian: financial_domain::account_value::account_custodian(&acct.name).to_string(),
            current_minor: current,
            current_complete: current.is_some(),
            points,
            trends_points: trends_points_for(&acct.name, &weeks),
            scale: 2,
        });
    }
    series.sort_by(|a, b| {
        a.account_name
            .to_ascii_lowercase()
            .cmp(&b.account_name.to_ascii_lowercase())
    });
    let (fid_mv, fid_ok) = financial_domain::account_value::fidelity_total_from_accounts(&live_rows);
    let (sch_mv, sch_ok) = financial_domain::account_value::schwab_total_from_accounts(&live_rows);
    Ok(AccountValueHomeBody {
        as_of: as_of.to_string(),
        accounts: series,
        fidelity: AccountValueSeriesBody {
            account_id: financial_domain::account_value::FIDELITY_TOTAL_ID.to_string(),
            account_name: financial_domain::account_value::FIDELITY_TOTAL_NAME.to_string(),
            custodian: "Fidelity".into(),
            current_minor: fid_mv,
            current_complete: fid_ok,
            points: overlay_live_point(
                history_points_for(
                    &history,
                    financial_domain::account_value::FIDELITY_TOTAL_ID,
                ),
                as_of,
                fid_mv,
            ),
            trends_points: trends_points_for(
                financial_domain::account_value::FIDELITY_TOTAL_NAME,
                &weeks,
            ),
            scale: 2,
        },
        schwab: AccountValueSeriesBody {
            account_id: financial_domain::account_value::SCHWAB_TOTAL_ID.to_string(),
            account_name: financial_domain::account_value::SCHWAB_TOTAL_NAME.to_string(),
            custodian: "Schwab".into(),
            current_minor: sch_mv,
            current_complete: sch_ok,
            points: overlay_live_point(
                history_points_for(&history, financial_domain::account_value::SCHWAB_TOTAL_ID),
                as_of,
                sch_mv,
            ),
            trends_points: trends_points_for(
                financial_domain::account_value::SCHWAB_TOTAL_NAME,
                &weeks,
            ),
            scale: 2,
        },
        note: "Solid line is live holdings (qty × last price). Dashed line is stored Trends weeks. Missing stays unknown."
            .into(),
        scale: 2,
    })
}

/// Local date and time for collector last-run / retrieve_run stamps.
fn run_stamp_local() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

fn calendar_month_prefix(as_of: &str) -> Option<String> {
    let trimmed = as_of.trim();
    if trimmed.len() >= 7 && trimmed.as_bytes()[4] == b'-' {
        Some(trimmed[..7].to_string())
    } else {
        None
    }
}

fn is_broker_sourced_dividend(activity: &ActivityRecord) -> bool {
    activity.activity_type.eq_ignore_ascii_case("dividend")
        && activity.amount_minor > 0
        && !activity.idempotency_key.starts_with("mm-")
}

async fn as_of_from_import_batch(canonical: &dyn Canonical, batch_id: Uuid) -> String {
    let Ok(activities) = canonical.activity_list().await else {
        return today_iso();
    };
    activities
        .iter()
        .filter(|a| a.import_batch_id == Some(batch_id))
        .map(|a| a.occurred_on.as_str())
        .max()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(today_iso)
}

/// BR-CASH-01: flag open SPAXX/FDRXX/SWVXX lots with no broker cash in the as-of month.
async fn cash_dividend_coverage_refresh(
    canonical: &dyn Canonical,
    as_of: &str,
) -> Result<CashDividendCoverageBody, PlatformError> {
    let month = calendar_month_prefix(as_of).unwrap_or_else(|| as_of.to_string());
    let lots = canonical.basis_get().await?.lots;
    let activities = canonical.activity_list().await.unwrap_or_default();
    let exceptions = canonical.exception_list().await.unwrap_or_default();
    let mut seen = std::collections::HashSet::<(Uuid, Uuid)>::new();
    let mut positions = Vec::new();
    let mut missing_count = 0u64;
    let mut raised_count = 0u64;
    let mut acknowledged_count = 0u64;

    for lot in lots.iter().filter(|l| l.remaining_quantity_minor > 0) {
        if !seen.insert((lot.account_id, lot.security_id)) {
            continue;
        }
        let Ok(security) = canonical.security_get(lot.security_id).await else {
            continue;
        };
        if !financial_domain::current_price::is_cash_par_symbol(&security.symbol) {
            continue;
        }
        let Ok(account) = canonical.account_get(lot.account_id).await else {
            continue;
        };
        let present = activities.iter().any(|a| {
            is_broker_sourced_dividend(a)
                && a.account_id == lot.account_id
                && a.security_id == Some(lot.security_id)
                && a.occurred_on.starts_with(&month)
        });
        let message = financial_domain::current_price::missing_cash_dividend_message(
            &account.name,
            &security.symbol,
            &month,
        );
        if present {
            for ex in exceptions
                .iter()
                .filter(|e| !e.acknowledged && e.code == "missing_cash_dividend" && e.message == message)
            {
                if canonical.exception_acknowledge(ex.exception_id).await.is_ok() {
                    acknowledged_count += 1;
                }
            }
        } else {
            missing_count += 1;
            let already = exceptions.iter().any(|e| {
                !e.acknowledged && e.code == "missing_cash_dividend" && e.message == message
            });
            if !already && canonical.exception_raise("missing_cash_dividend".into(), message).await.is_ok()
            {
                raised_count += 1;
            }
        }
        positions.push(CashDividendCoverageRow {
            account_id: lot.account_id,
            account_name: account.name,
            security_id: lot.security_id,
            symbol: security.symbol,
            present,
        });
    }

    Ok(CashDividendCoverageBody {
        as_of_date: as_of.to_string(),
        month,
        missing_count,
        raised_count,
        acknowledged_count,
        positions,
    })
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
    as_of: &str,
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
        if is_cash_rate_candidate(c) {
            continue;
        }
        let period = jstr(c, "paymentPeriod").unwrap_or_default();
        if period.is_empty() {
            continue;
        }
        if !financial_domain::schedule::period_has_occurred(&period, as_of) {
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
            d.amount_per_share_minor
                .is_some_and(|stored| financial_domain::money::amounts_equal(stored, d.amount_scale, amount, scale))
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

fn declaration_lookback_applies(div_type: &str, declaration_source: &str, symbol: &str) -> bool {
    if financial_domain::mlp_sec::is_adapter_kind(declaration_source) {
        return false;
    }
    if financial_domain::current_price::uses_cash_par(div_type, symbol) {
        return false;
    }
    financial_domain::div1::is_div1(div_type)
        || financial_domain::div1::is_registered_declaration_source(declaration_source)
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
    apply_lookback: bool,
    lookback_owner_accepted: bool,
) -> DeclarationGate {
    if !apply_lookback {
        return DeclarationGate {
            ok: true,
            code: String::new(),
            message: String::new(),
        };
    }
    if lookback_owner_accepted && paid_count > 0 {
        return DeclarationGate {
            ok: true,
            code: String::new(),
            message: "declaration retrieve complete (owner accepted issuer series)".into(),
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
                "Adapter returned {paid} of {DECLARATION_LOOKBACK_TARGET} required paid declarations. Confirm inception Yes/No — do not defer to Settings."
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

fn paid_view_from_record(d: &IssuerDeclarationRecord) -> financial_domain::declaration_post::PaidDeclarationView<'_> {
    financial_domain::declaration_post::PaidDeclarationView {
        payment_period: d.payment_period.as_str(),
        amount_per_share_minor: d.amount_per_share_minor,
        amount_scale: d.amount_scale,
    }
}

fn paid_json_period(c: &Value) -> String {
    jstr(c, "paymentPeriod")
        .or_else(|| jstr(c, "payOn"))
        .unwrap_or_default()
}

fn parse_owner_per_unit_amount(raw: &str) -> Option<(i64, u8)> {
    let s = raw.trim().trim_start_matches('$').replace(',', "");
    if s.is_empty() {
        return None;
    }
    let (whole, frac) = match s.split_once('.') {
        Some((w, f)) => (w, f),
        None => (s.as_str(), ""),
    };
    let whole_n: i64 = whole.parse().ok()?;
    let scale = frac.len() as u8;
    let frac_n: i64 = if frac.is_empty() {
        0
    } else {
        frac.parse().ok()?
    };
    let pow = 10i64.checked_pow(u32::from(scale))?;
    let minor = whole_n.checked_mul(pow)?.checked_add(frac_n)?;
    if minor <= 0 {
        return None;
    }
    Some((minor, scale.max(2)))
}

fn payment_period_from_ticket_reason(reason: &str) -> Option<String> {
    for token in reason.split_whitespace() {
        let key = payment_period_key(token.trim_end_matches(';'));
        if financial_domain::week::parse_iso_day(&key).is_some() {
            return Some(key);
        }
    }
    None
}

fn payment_period_key(raw: &str) -> String {
    let s = raw.trim();
    if s.len() >= 10 {
        s[..10].to_string()
    } else {
        s.to_string()
    }
}

fn paid_json_amount(c: &Value) -> Option<i64> {
    c.get("amountPerShareMinor").and_then(|x| {
        if x.is_null() {
            None
        } else {
            ji64(c, "amountPerShareMinor")
        }
    })
}

fn declaration_post_check_issues(
    stored_before: &[IssuerDeclarationRecord],
    stored_after: &[IssuerDeclarationRecord],
    newly_posted: &[Value],
    page_paid: &[Value],
    locked_frequency: &str,
    stored_paid_count: u8,
) -> Vec<(String, String)> {
    use financial_domain::declaration_post::{
        amount_variation_applies, cadence_matches_locked, new_amount_variation_issues,
        overlap_amount_change_issues, previous_periods_retained, PaidDeclarationView,
        AMOUNT_VARIATION_PCT,
    };
    let mut out = Vec::new();
    let variation = amount_variation_applies(stored_paid_count);
    if !page_paid.is_empty() {
        let page_hold: Vec<(String, Option<i64>, u8)> = page_paid
            .iter()
            .map(|c| {
                (
                    paid_json_period(c),
                    paid_json_amount(c),
                    ju8(c, "amountScale", 2),
                )
            })
            .collect();
        let page_views: Vec<PaidDeclarationView<'_>> = page_hold
            .iter()
            .map(|(p, a, s)| PaidDeclarationView {
                payment_period: p.as_str(),
                amount_per_share_minor: *a,
                amount_scale: *s,
            })
            .collect();
        let before_views: Vec<_> = stored_before.iter().map(paid_view_from_record).collect();
        for issue in previous_periods_retained(&before_views, &page_views) {
            out.push((issue.code.to_string(), issue.message));
        }
        if variation {
            let after_overlap: Vec<_> = stored_after.iter().map(paid_view_from_record).collect();
            for issue in overlap_amount_change_issues(&after_overlap, &page_views) {
                out.push((issue.code.to_string(), issue.message));
            }
        }
        // Issuer page only. An empty retrieve must not infer Monthly from leftover stored dates.
        let periods: Vec<&str> = page_views.iter().map(|v| v.payment_period).collect();
        for issue in cadence_matches_locked(&periods, locked_frequency) {
            out.push((issue.code.to_string(), issue.message));
        }
    }
    let new_hold: Vec<(String, Option<i64>, u8)> = newly_posted
        .iter()
        .map(|c| {
            (
                paid_json_period(c),
                paid_json_amount(c),
                ju8(c, "amountScale", 2),
            )
        })
        .collect();
    let new_views: Vec<PaidDeclarationView<'_>> = new_hold
        .iter()
        .map(|(p, a, s)| PaidDeclarationView {
            payment_period: p.as_str(),
            amount_per_share_minor: *a,
            amount_scale: *s,
        })
        .collect();
    let after_views: Vec<_> = stored_after.iter().map(paid_view_from_record).collect();
    if variation {
        for issue in new_amount_variation_issues(&new_views, &after_views, AMOUNT_VARIATION_PCT) {
            out.push((issue.code.to_string(), issue.message));
        }
    }
    out
}

fn append_tried_url(existing: &str, url: &str) -> String {
    let url = url.trim();
    if url.is_empty() {
        return existing.to_string();
    }
    let mut urls: Vec<String> = serde_json::from_str(existing).unwrap_or_default();
    if !urls.iter().any(|u| u == url) {
        urls.push(url.to_string());
    }
    serde_json::to_string(&urls).unwrap_or_else(|_| "[]".into())
}

async fn work_ticket_raise_or_bump(
    canonical: &dyn Canonical,
    security_id: Uuid,
    symbol: &str,
    code: &str,
    reason: &str,
    url: &str,
) -> Result<WorkTicketRecord, PlatformError> {
    let Some(tool) = financial_domain::work_ticket::tool_for_code(code) else {
        return Err(PlatformError::new(
            "ticket_missing_tool",
            format!("no fix tool for code {code}"),
        ));
    };
    let field = financial_domain::work_ticket::field_for_code(code).to_string();
    let today = today_iso();
    let open = canonical
        .work_ticket_list(Some(security_id), Some("open".into()))
        .await
        .unwrap_or_default();
    if let Some(mut existing) = open.into_iter().find(|t| t.code == code) {
        existing.reason = reason.to_string();
        existing.last_seen_on = today;
        existing.urls_tried = append_tried_url(&existing.urls_tried, url);
        existing.tool = tool.to_string();
        existing.field = field;
        return canonical.work_ticket_update(existing).await;
    }
    canonical
        .work_ticket_raise(WorkTicketRecord {
            ticket_id: Uuid::new_v4(),
            security_id,
            symbol: symbol.to_string(),
            field,
            code: code.to_string(),
            tool: tool.to_string(),
            reason: reason.to_string(),
            urls_tried: append_tried_url("[]", url),
            opened_on: today.clone(),
            last_seen_on: today,
            status: "open".into(),
            filed_on: String::new(),
            completed_how: String::new(),
            owner_note: String::new(),
            retrieve_run_id: String::new(),
        })
        .await
}

async fn raise_collector_identity_tickets(
    canonical: &dyn Canonical,
    security_id: Uuid,
    symbol: &str,
    json: &Value,
) {
    let url = jstr(json, "sourceUrl")
        .or_else(|| jstr(json, "source_url"))
        .unwrap_or_default();
    let chars = canonical
        .position_characteristic_list()
        .await
        .unwrap_or_default();
    let rec = chars.into_iter().find(|c| c.security_id == security_id);
    let cash = rec
        .as_ref()
        .map(|c| financial_domain::current_price::uses_cash_par(&c.div_type, symbol))
        .unwrap_or_else(|| financial_domain::current_price::is_cash_par_symbol(symbol));
    let div_type = rec.as_ref().map(|c| c.div_type.as_str()).unwrap_or("");
    if div_type.trim().is_empty() {
        let _ = work_ticket_raise_or_bump(
            canonical,
            security_id,
            symbol,
            "div_type",
            "DIV-1 (or CASH) is empty — owner must set it.",
            &url,
        )
        .await;
    }
    if !cash {
        let freq = rec
            .as_ref()
            .map(|c| c.payment_frequency.as_str())
            .unwrap_or("");
        if financial_domain::calculator::PaymentCadence::parse(freq)
            .and_then(financial_domain::calculator::PaymentCadence::periods)
            .is_none()
        {
            let _ = work_ticket_raise_or_bump(
                canonical,
                security_id,
                symbol,
                "frequency",
                "Dividend frequency cannot be derived.",
                &url,
            )
            .await;
        }
        let underlying = rec.as_ref().map(|c| c.underlying.as_str()).unwrap_or("");
        if underlying.trim().is_empty() || underlying.eq_ignore_ascii_case(symbol) {
            let _ = work_ticket_raise_or_bump(
                canonical,
                security_id,
                symbol,
                "underlying",
                "Underlying is unknown after retrieve.",
                &url,
            )
            .await;
        }
        let roc_ok = rec
            .as_ref()
            .and_then(|c| c.roc_pct_2026_estimate_minor)
            .is_some();
        if !roc_ok {
            let probe = json
                .get("rocProbes")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|p| p.get("url").and_then(|u| u.as_str()))
                .unwrap_or(url.as_str());
            let _ = work_ticket_raise_or_bump(
                canonical,
                security_id,
                symbol,
                "roc_estimate",
                "19a-1 search miss — ROC estimate unknown. Never invent 0%.",
                probe,
            )
            .await;
        }
    }
}

async fn collector_status_for(
    canonical: &dyn Canonical,
    security_id: Uuid,
    symbol: &str,
    as_of: &str,
) -> financial_domain::collector::CollectorCompleteStatus {
    let template = canonical
        .retrieval_template_get(security_id)
        .await
        .ok()
        .flatten();
    let chars = canonical
        .position_characteristic_list()
        .await
        .unwrap_or_default();
    let ch = chars.into_iter().find(|c| c.security_id == security_id);
    let paid = canonical
        .issuer_declaration_list(security_id)
        .await
        .unwrap_or_default()
        .iter()
        .filter(|d| d.amount_per_share_minor.unwrap_or(0) > 0)
        .count()
        .min(255) as u8;
    let obs = canonical
        .roc_observation_list(security_id)
        .await
        .unwrap_or_default();
    let owner_override = obs.iter().any(|o| o.owner_override);
    let roc_pct = ch.as_ref().and_then(|c| c.roc_pct_2026_estimate_minor);
    let needs = ch.as_ref().map(|c| c.needs_roc_research).unwrap_or(true);
    let roc_owner_accepted = roc_pct.is_some() && (!needs || owner_override);
    let freq = ch
        .as_ref()
        .map(|c| c.payment_frequency.clone())
        .unwrap_or_default();
    let periods = financial_domain::calculator::PaymentCadence::parse(&freq)
        .and_then(financial_domain::calculator::PaymentCadence::periods)
        .unwrap_or(0);
    let pays = canonical
        .issuer_pay_date_list(security_id)
        .await
        .unwrap_or_default();
    let pay_ons: Vec<&str> = pays.iter().map(|p| p.pay_on.as_str()).collect();
    let vendor_ons: Vec<&str> = pays
        .iter()
        .filter(|p| !p.source.eq_ignore_ascii_case("derived_walk"))
        .map(|p| p.pay_on.as_str())
        .collect();
    let stored = financial_domain::collector::stored_remaining_planned(&pay_ons, as_of);
    let issuer_remaining = {
        let n = financial_domain::collector::stored_remaining_planned(&vendor_ons, as_of);
        if n > 0 {
            Some(n)
        } else {
            None
        }
    };
    let planned = if periods == 0 {
        None
    } else if stored > 0 || issuer_remaining.is_some() {
        Some(stored)
    } else {
        remaining_year_schedule_for(canonical, security_id, as_of, periods, None, &[])
            .await
            .ok()
            .and_then(|s| s.remaining_periods)
            .or(Some(0))
    };
    let decisions = canonical
        .collector_field_decision_list(security_id)
        .await
        .unwrap_or_default();
    let lookback_owner_accepted = decisions.iter().any(|d| {
        d.field.eq_ignore_ascii_case("paid_history") && d.decision.eq_ignore_ascii_case("accept")
    });
    let skipped: Vec<String> = decisions
        .into_iter()
        .filter(|d| d.decision.eq_ignore_ascii_case("skip"))
        .map(|d| d.field)
        .collect();
    let skipped_refs: Vec<&str> = skipped.iter().map(|s| s.as_str()).collect();
    financial_domain::collector::collector_status(
        &financial_domain::collector::CollectorCompleteSpec {
            has_template: template.is_some(),
            symbol,
            div_type: ch.as_ref().map(|c| c.div_type.as_str()).unwrap_or(""),
            payment_frequency: ch
                .as_ref()
                .map(|c| c.payment_frequency.as_str())
                .unwrap_or(""),
            underlying: ch.as_ref().map(|c| c.underlying.as_str()).unwrap_or(""),
            provider: ch.as_ref().map(|c| c.provider.as_str()).unwrap_or(""),
            risk_tier: ch.as_ref().map(|c| c.risk_tier.as_str()).unwrap_or(""),
            paid_declaration_count: paid,
            inception_on: template
                .as_ref()
                .map(|t| t.inception_on.as_str())
                .unwrap_or(""),
            as_of,
            roc_pct_minor: roc_pct,
            roc_owner_accepted,
            planned_remaining: planned,
            issuer_remaining,
            skipped_required: &skipped_refs,
            lookback_owner_accepted,
            last_run_ok: template.as_ref().and_then(|t| t.last_run_ok),
        },
    )
}

fn ticket_code_from_run(code: &str) -> String {
    let c = code.trim();
    if financial_domain::work_ticket::tool_for_code(c).is_some() {
        c.to_string()
    } else {
        "declaration_retrieve_miss".into()
    }
}

/// Open a ticket for every enabled collector whose latest declaration retrieve
/// missed, even when last_run_ok was later overwritten to true.
async fn work_ticket_sync_misses(
    canonical: &dyn Canonical,
) -> Result<WorkTicketSyncMissesBody, PlatformError> {
    let set = canonical.collector_set().await?;
    let mut scanned = 0u64;
    let mut raised = 0u64;
    for item in &set.items {
        if !item.collector_enabled || item.declaration_source.trim().is_empty() {
            continue;
        }
        scanned += 1;
        let runs = canonical
            .retrieve_run_list(Some(item.security_id), 50)
            .await
            .unwrap_or_default();
        let decl: Vec<_> = runs.iter().filter(|r| r.kind == "declaration").collect();
        let latest = decl.first();
        let last_failed = item.last_run_ok == Some(false);
        let (code, reason) = match latest {
            Some(run) if !run.ok => (
                ticket_code_from_run(&run.code),
                if run.message.trim().is_empty() {
                    "Issuer page empty.".into()
                } else {
                    run.message.clone()
                },
            ),
            Some(_) => continue,
            None if last_failed => (
                "declaration_retrieve_miss".into(),
                if item.last_run_message.trim().is_empty() {
                    "Issuer page empty.".into()
                } else {
                    item.last_run_message.clone()
                },
            ),
            None => continue,
        };
        let existing = canonical
            .work_ticket_list(Some(item.security_id), None)
            .await
            .unwrap_or_default();
        let had_open = existing
            .iter()
            .any(|t| t.status == "open" && t.code == code);
        if !had_open
            && existing.iter().any(|t| {
                t.status == "done" && t.code == code && t.completed_how == "owner_filed"
            })
        {
            continue;
        }
        if work_ticket_raise_or_bump(
            canonical,
            item.security_id,
            &item.symbol,
            &code,
            &reason,
            &item.source_url,
        )
        .await
        .is_ok()
            && !had_open
        {
            raised += 1;
        }
    }
    for item in &set.items {
        if !item.collector_enabled {
            continue;
        }
        close_stale_expected_4_remaining_year(
            canonical,
            item.security_id,
            &item.symbol,
            &today_iso(),
        )
        .await;
    }
    let open = canonical
        .work_ticket_list(None, Some("open".into()))
        .await
        .unwrap_or_default();
    Ok(WorkTicketSyncMissesBody {
        scanned,
        raised,
        open_count: open.len() as u64,
    })
}

async fn close_stale_expected_4_remaining_year(
    canonical: &dyn Canonical,
    security_id: Uuid,
    _symbol: &str,
    as_of: &str,
) {
    let open = canonical
        .work_ticket_list(Some(security_id), Some("open".into()))
        .await
        .unwrap_or_default();
    let pays = canonical
        .issuer_pay_date_list(security_id)
        .await
        .unwrap_or_default();
    let stored_ons: Vec<&str> = pays.iter().map(|p| p.pay_on.as_str()).collect();
    let vendor_ons: Vec<&str> = pays
        .iter()
        .filter(|p| !p.source.eq_ignore_ascii_case("derived_walk"))
        .map(|p| p.pay_on.as_str())
        .collect();
    if financial_domain::collector::remaining_year_dates_disagree(&stored_ons, &vendor_ons, as_of)
    {
        return;
    }
    let today = today_iso();
    for mut t in open {
        if t.code != "remaining_year" {
            continue;
        }
        if !financial_domain::work_ticket::is_stale_expected_4_remaining_year(&t.reason) {
            continue;
        }
        t.status = "done".into();
        t.filed_on = today.clone();
        t.completed_how = "auto_resolved".into();
        t.last_seen_on = today.clone();
        let _ = canonical.work_ticket_update(t).await;
    }
}

async fn work_ticket_auto_file_declaration(
    canonical: &dyn Canonical,
    security_id: Uuid,
    retrieve_run_id: Uuid,
) {
    let Ok(open) = canonical
        .work_ticket_list(Some(security_id), Some("open".into()))
        .await
    else {
        return;
    };
    let today = today_iso();
    for mut t in open {
        if !financial_domain::work_ticket::is_auto_file_on_ok_code(&t.code) {
            continue;
        }
        t.status = "done".into();
        t.filed_on = today.clone();
        t.completed_how = "auto_resolved".into();
        t.last_seen_on = today.clone();
        t.retrieve_run_id = retrieve_run_id.to_string();
        let _ = canonical.work_ticket_update(t).await;
    }
}

async fn work_ticket_file_on(
    canonical: &dyn Canonical,
    ticket_id: Uuid,
    note: &str,
) -> Result<WorkTicketRecord, PlatformError> {
    if note.trim().is_empty() {
        return Err(PlatformError::new(
            "ticket_note_required",
            "File requires a note",
        ));
    }
    let mut t = canonical.work_ticket_get(ticket_id).await?;
    if t.status != "open" {
        return Ok(t);
    }
    let today = today_iso();
    t.status = "done".into();
    t.filed_on = today.clone();
    t.completed_how = "owner_filed".into();
    t.owner_note = note.trim().to_string();
    t.last_seen_on = today;
    canonical.work_ticket_update(t).await
}

async fn persist_template_url_same_adapter(
    canonical: &dyn Canonical,
    security_id: Uuid,
    source_url: &str,
) -> Result<RetrievalTemplateRecord, PlatformError> {
    let mut rec = canonical
        .retrieval_template_get(security_id)
        .await?
        .ok_or_else(|| PlatformError::new("not_found", "retrieval template not found"))?;
    rec.source_url = source_url.trim().to_string();
    canonical.retrieval_template_set(rec).await
}

#[derive(Default)]
struct PhaseIEstablish {
    needs_second_url: bool,
    second_url_tried: bool,
    adapter_failed: bool,
    retrieve_message: String,
    paid_count: u8,
    needs_inception_confirm: bool,
    inception_candidate: String,
    expected_paid_since_inception: Option<u8>,
    inception_search_miss: bool,
}

async fn apply_phase_i_establish(
    canonical: &dyn Canonical,
    security_id: Uuid,
    symbol: &str,
    source_url: &str,
    payment_frequency: &str,
    retrieve_ok: bool,
    retrieve_code: &str,
    retrieve_message: &str,
    json: &Value,
) -> PhaseIEstablish {
    let mut out = PhaseIEstablish {
        retrieve_message: retrieve_message.to_string(),
        ..PhaseIEstablish::default()
    };
    let as_of = jstr(json, "asOfDate").unwrap_or_else(today_iso);
    let Ok(Some(mut template)) = canonical.retrieval_template_get(security_id).await else {
        return out;
    };

    let parse_fail = financial_domain::work_ticket::is_history_parse_fail(retrieve_code);
    let second_attempt = jbool(json, "secondUrlAttempt", false);
    let incoming = jstr(json, "sourceUrl")
        .or_else(|| jstr(json, "distributionUrl"))
        .unwrap_or_default();
    let url_changed = !incoming.is_empty() && incoming != template.source_url;
    let mut attempts = template.history_url_attempts;
    if parse_fail {
        if second_attempt || url_changed {
            attempts = 2;
        } else {
            attempts = attempts.max(1);
        }
        template.history_url_attempts = attempts;
        let _ = canonical.retrieval_template_set(template.clone()).await;
        out.needs_second_url = attempts < 2;
        out.second_url_tried = attempts >= 2;
        out.adapter_failed = attempts >= 2;
        if out.adapter_failed {
            out.retrieve_message = if retrieve_message.trim().is_empty() {
                "Second distribution URL failed. Adapter not built. Manual adapter is parked."
                    .into()
            } else {
                format!(
                    "{retrieve_message} Second distribution URL failed. Adapter not built. Manual adapter is parked."
                )
            };
        } else if out.retrieve_message.trim().is_empty() {
            out.retrieve_message =
                "History parse failed. Paste a second issuer URL once — same adapter only.".into();
        }
    } else if retrieve_ok {
        template.history_url_attempts = 0;
        let _ = canonical.retrieval_template_set(template.clone()).await;
    }

    let paid = canonical
        .issuer_declaration_list(security_id)
        .await
        .unwrap_or_default()
        .iter()
        .filter(|d| d.amount_per_share_minor.unwrap_or(0) > 0)
        .count()
        .min(255) as u8;
    out.paid_count = paid;

    let cash = financial_domain::current_price::is_cash_par_symbol(symbol);
    if !cash && paid < financial_domain::declaration_lookback::DECLARATION_LOOKBACK_TARGET {
        let confirmed = json.get("inceptionConfirmed").and_then(|v| v.as_bool());
        let hits = json
            .get("inceptionHits")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let candidate = jstr(json, "inceptionCandidate")
            .filter(|s| !s.trim().is_empty())
            .or_else(|| {
                if confirmed == Some(true) {
                    jstr(json, "inceptionOn").filter(|s| !s.trim().is_empty())
                } else {
                    None
                }
            })
            .or_else(|| {
                let pairs: Vec<(&str, &str)> = hits
                    .iter()
                    .map(|hit| {
                        (
                            hit.get("title")
                                .or_else(|| hit.get("inceptionOn"))
                                .and_then(|v| v.as_str())
                                .unwrap_or(""),
                            hit.get("snippet")
                                .or_else(|| hit.get("body"))
                                .or_else(|| hit.get("inceptionOn"))
                                .and_then(|v| v.as_str())
                                .unwrap_or(""),
                        )
                    })
                    .collect();
                financial_domain::declaration_lookback::inception_on_from_search_hits(&pairs)
            })
            .unwrap_or_default();
        out.inception_candidate = candidate.clone();
        if confirmed == Some(true) {
            let date = jstr(json, "inceptionOn")
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| candidate.clone());
            if !date.is_empty() {
                template.inception_on = date;
                let _ = canonical.retrieval_template_set(template.clone()).await;
            }
        } else if confirmed == Some(false) || candidate.is_empty() {
            out.inception_search_miss = candidate.is_empty() && confirmed != Some(true);
            let _ = work_ticket_raise_or_bump(
                canonical,
                security_id,
                symbol,
                "declaration_lookback_short",
                if confirmed == Some(false) {
                    "Owner said this name is not too new. Paid history stays short — collector incomplete."
                } else {
                    "Inception search miss. Paid history under 12 — collector incomplete."
                },
                source_url,
            )
            .await;
        }
        let stored = canonical
            .retrieval_template_get(security_id)
            .await
            .ok()
            .flatten()
            .map(|t| t.inception_on)
            .unwrap_or_default();
        let for_expected = if !stored.trim().is_empty() {
            stored.clone()
        } else {
            candidate.clone()
        };
        if !for_expected.is_empty() {
            out.expected_paid_since_inception = Some(
                financial_domain::declaration_lookback::expected_from_inception(
                    &for_expected,
                    &as_of,
                    payment_frequency,
                ),
            );
        }
        out.needs_inception_confirm =
            stored.trim().is_empty() && !candidate.is_empty() && confirmed != Some(false);
    }

    persist_phase_i_remaining_year(canonical, security_id, symbol, &as_of, payment_frequency).await;

    if let Some(roc_url) = jstr(json, "rocSourceUrl").filter(|s| !s.trim().is_empty()) {
        if let Ok(Some(mut rec)) = canonical.retrieval_template_get(security_id).await {
            rec.roc_source_url = roc_url;
            let _ = canonical.retrieval_template_set(rec).await;
        }
    }

    out
}

async fn persist_phase_i_remaining_year(
    canonical: &dyn Canonical,
    security_id: Uuid,
    symbol: &str,
    as_of: &str,
    payment_frequency: &str,
) {
    let Some(periods) = financial_domain::calculator::PaymentCadence::parse(payment_frequency)
        .and_then(financial_domain::calculator::PaymentCadence::periods)
    else {
        return;
    };
    let year = as_of.get(..4).unwrap_or("");
    let year_end = format!("{year}-12-31");
    let issuer = canonical
        .issuer_pay_date_list(security_id)
        .await
        .unwrap_or_default();
    let vendor: Vec<_> = issuer
        .iter()
        .filter(|d| {
            !d.source.eq_ignore_ascii_case("derived_walk")
                && d.pay_on.as_str() >= as_of
                && d.pay_on.as_str() <= year_end.as_str()
        })
        .collect();
    if vendor.is_empty() {
        let Ok(schedule) =
            remaining_year_schedule_for(canonical, security_id, as_of, periods, None, &[]).await
        else {
            return;
        };
        if !schedule.known {
            return;
        }
        let dates: Vec<IssuerPayDateRecord> = schedule
            .payments
            .iter()
            .map(|p| IssuerPayDateRecord {
                pay_date_id: Uuid::new_v4(),
                security_id,
                pay_on: p.pay_on.clone(),
                source: "derived_walk".into(),
                recorded_at: as_of.to_string(),
            })
            .collect();
        if !dates.is_empty() {
            let _ = canonical
                .issuer_pay_date_replace(security_id, as_of.to_string(), dates)
                .await;
        }
        return;
    }
    let stored_ons: Vec<&str> = issuer.iter().map(|d| d.pay_on.as_str()).collect();
    let vendor_ons: Vec<&str> = vendor.iter().map(|d| d.pay_on.as_str()).collect();
    if financial_domain::collector::remaining_year_dates_disagree(&stored_ons, &vendor_ons, as_of) {
        let _ = work_ticket_raise_or_bump(
            canonical,
            security_id,
            symbol,
            "remaining_year",
            &format!(
                "Remaining-year stored dates disagree with the issuer list ({} stored, {} issuer) through 31 Dec.",
                stored_ons.len(),
                vendor_ons.len()
            ),
            "",
        )
        .await;
    }
}

/// Stored ex/record leftover → later vendor payable, same dollar. Owner twin cleanup.
async fn apply_leftover_ex_to_payable(
    canonical: &dyn Canonical,
    security_id: Uuid,
    vendor_rows: &[Value],
    recorded_at: &str,
) -> u64 {
    let stored = canonical
        .issuer_declaration_list(security_id)
        .await
        .unwrap_or_default();
    let stored_view: Vec<(String, i64, u8)> = stored
        .iter()
        .filter_map(|d| {
            let amt = d.amount_per_share_minor.filter(|a| *a > 0)?;
            Some((d.payment_period.clone(), amt, d.amount_scale))
        })
        .collect();
    let vendor: Vec<(String, Option<String>, Option<String>, i64, u8)> = vendor_rows
        .iter()
        .filter_map(|row| {
            let pay = jstr(row, "paymentPeriod").or_else(|| jstr(row, "payOn"))?;
            let amt = paid_json_amount(row).filter(|a| *a > 0)?;
            Some((
                pay,
                jstr(row, "exDate"),
                jstr(row, "recordDate"),
                amt,
                ju8(row, "amountScale", 2),
            ))
        })
        .collect();
    let moves = financial_domain::schedule::leftover_ex_to_payable_moves(&stored_view, &vendor);
    let mut n = 0u64;
    for (from, to) in moves {
        if let Some(old) = stored.iter().find(|d| d.payment_period == from) {
            let _ = canonical
                .issuer_declaration_supersede_period(security_id, from.clone())
                .await;
            let already = stored.iter().any(|d| {
                d.payment_period == to && d.amount_per_share_minor.unwrap_or(0) > 0
            });
            if !already {
                let (amt, scale) = vendor
                    .iter()
                    .find(|(pay, _, _, _, _)| pay == &to)
                    .map(|(_, _, _, a, s)| (Some(*a), *s))
                    .unwrap_or((old.amount_per_share_minor, old.amount_scale));
                let _ = canonical
                    .issuer_declaration_record(
                        security_id,
                        amt,
                        scale,
                        to.clone(),
                        "vendor_payable".into(),
                        recorded_at.to_string(),
                    )
                    .await;
            }
            let _ = canonical
                .issuer_pay_date_supersede_one(security_id, from)
                .await;
            let rec = IssuerPayDateRecord {
                pay_date_id: Uuid::new_v4(),
                security_id,
                pay_on: to,
                source: "vendor_payable".into(),
                recorded_at: recorded_at.to_string(),
            };
            let _ = canonical.issuer_pay_date_insert(rec).await;
            n = n.saturating_add(1);
        }
    }
    n
}

/// Runtime collect: apply vendor payables to unoccurred rows only. Never rebuild the year.
/// Plan $ is not touched. Paid months raise a ticket instead of silent-supersede.
async fn apply_mlp_sec_8k_payables(
    canonical: &dyn Canonical,
    security_id: Uuid,
    symbol: &str,
    as_of: &str,
    declarations: &[Value],
    pay_dates: &[Value],
    recorded_at: &str,
) -> u64 {
    let existing = canonical
        .issuer_pay_date_list(security_id)
        .await
        .unwrap_or_default();
    let mut wrote = 0u64;
    for decl in declarations {
        let amount = decl.get("amountPerShareMinor").and_then(|x| {
            if x.is_null() {
                None
            } else {
                ji64(decl, "amountPerShareMinor")
            }
        });
        if amount.unwrap_or(0) <= 0 {
            continue;
        }
        let period = jstr(decl, "paymentPeriod").unwrap_or_default();
        if period.is_empty() {
            continue;
        }
        if financial_domain::schedule::period_has_occurred(&period, as_of) {
            continue;
        }
        let nearby = existing.iter().find(|p| {
            p.source
                .eq_ignore_ascii_case(financial_domain::mlp_sec::SOURCE_DERIVED_TEMPLATE)
                && (p.pay_on == period
                    || financial_domain::mlp_sec::within_three_business_days(&p.pay_on, &period))
        });
        if let Some(old) = nearby {
            if old.pay_on != period {
                let _ = canonical
                    .issuer_pay_date_supersede_one(security_id, old.pay_on.clone())
                    .await;
            }
        } else {
            let far = existing.iter().find(|p| {
                p.source
                    .eq_ignore_ascii_case(financial_domain::mlp_sec::SOURCE_DERIVED_TEMPLATE)
                    && !financial_domain::schedule::period_has_occurred(&p.pay_on, as_of)
                    && p.pay_on != period
            });
            if let Some(old) = far {
                let _ = work_ticket_raise_or_bump(
                    canonical,
                    security_id,
                    symbol,
                    financial_domain::mlp_sec::CODE_PAYABLE_DATE_MOVED,
                    &format!(
                        "Derived payable {} moved to 8-K payable {}.",
                        old.pay_on, period
                    ),
                    "",
                )
                .await;
                let _ = canonical
                    .issuer_pay_date_supersede_one(security_id, old.pay_on.clone())
                    .await;
            }
        }
    }
    for row in pay_dates {
        let Some(pay_on) = jstr(row, "payOn")
            .or_else(|| jstr(row, "paymentPeriod"))
            .filter(|s| !s.trim().is_empty())
        else {
            continue;
        };
        let src = jstr(row, "source").unwrap_or_else(|| {
            financial_domain::mlp_sec::SOURCE_DERIVED_TEMPLATE.to_string()
        });
        if !src.eq_ignore_ascii_case(financial_domain::mlp_sec::SOURCE_DERIVED_TEMPLATE) {
            continue;
        }
        if existing.iter().any(|p| p.pay_on == pay_on) {
            continue;
        }
        if financial_domain::schedule::period_has_occurred(&pay_on, as_of) {
            continue;
        }
        let rec = IssuerPayDateRecord {
            pay_date_id: Uuid::new_v4(),
            security_id,
            pay_on,
            source: financial_domain::mlp_sec::SOURCE_DERIVED_TEMPLATE.into(),
            recorded_at: recorded_at.to_string(),
        };
        if canonical.issuer_pay_date_insert(rec).await.is_ok() {
            wrote = wrote.saturating_add(1);
        }
    }
    wrote
}

fn mlp_sec_outcome_from_misses(misses: &[Value]) -> &'static str {
    if misses.iter().any(|m| {
        jstr(m, "code").as_deref() == Some(financial_domain::mlp_sec::CODE_SEC_403)
            || jstr(m, "reason")
                .unwrap_or_default()
                .contains("sec_403")
    }) {
        "sec_403"
    } else {
        "empty"
    }
}

async fn persist_mlp_sec_last_run(
    canonical: &dyn Canonical,
    security_id: Uuid,
    as_of: &str,
    outcome: &str,
    ran_at: String,
    hash: String,
) -> (bool, String, String) {
    let source_url = financial_domain::mlp_sec::atom_url();
    let stored_now = canonical
        .issuer_declaration_list(security_id)
        .await
        .unwrap_or_default();
    let pays_now = canonical
        .issuer_pay_date_list(security_id)
        .await
        .unwrap_or_default();
    let paid_refs: Vec<&str> = stored_now
        .iter()
        .filter(|d| d.amount_per_share_minor.unwrap_or(0) > 0)
        .map(|d| d.payment_period.as_str())
        .collect();
    let needed = financial_domain::schedule::derive_quarterly_template_pay_ons(as_of, &paid_refs);
    let remaining_ok = needed.iter().all(|need| {
        pays_now.iter().any(|p| {
            p.pay_on == *need
                && p.source
                    .eq_ignore_ascii_case(financial_domain::mlp_sec::SOURCE_DERIVED_TEMPLATE)
        }) || stored_now.iter().any(|d| {
            d.amount_per_share_minor.unwrap_or(0) > 0
                && (d.payment_period == *need
                    || financial_domain::mlp_sec::within_three_business_days(
                        &d.payment_period,
                        need,
                    ))
        })
    });
    let next_pay = needed.first().map(String::as_str).unwrap_or("");
    let has_declared = stored_now.iter().any(|d| {
        d.amount_per_share_minor.unwrap_or(0) > 0
            && (d.payment_period == next_pay
                || financial_domain::mlp_sec::within_three_business_days(
                    &d.payment_period,
                    next_pay,
                ))
    });
    let overdue = financial_domain::mlp_sec::owner_amount_ask_due(as_of, next_pay, has_declared);
    let stamp = if outcome == "sec_403" {
        "sec_403"
    } else if outcome == "page" {
        "page"
    } else if overdue {
        "need_owner"
    } else {
        "waiting"
    };
    let message = financial_domain::mlp_sec::last_run_stamp(stamp);
    let ok = financial_domain::mlp_sec::last_run_ok(&source_url, remaining_ok, overdue);
    let _ = canonical
        .retrieval_template_touch_run(
            security_id,
            ok,
            message.clone(),
            ran_at,
            hash,
            &source_url,
        )
        .await;
    (ok, message, source_url)
}

async fn apply_runtime_vendor_payables(
    canonical: &dyn Canonical,
    security_id: Uuid,
    symbol: &str,
    as_of: &str,
    payment_frequency: &str,
    vendor_rows: &[Value],
    recorded_at: &str,
) -> u64 {
    let vendor: Vec<String> = vendor_rows
        .iter()
        .filter_map(|row| {
            jstr(row, "payOn")
                .or_else(|| jstr(row, "paymentPeriod"))
                .filter(|s| !s.trim().is_empty())
        })
        .collect();
    let _ = canonical.issuer_pay_date_dedupe(security_id).await;
    if vendor.is_empty() {
        ticket_if_remaining_count_mismatch(canonical, security_id, symbol, as_of, payment_frequency)
            .await;
        return 0;
    }
    let existing = canonical
        .issuer_pay_date_list(security_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|d| d.pay_on)
        .collect::<Vec<_>>();
    let decls = canonical
        .issuer_declaration_list(security_id)
        .await
        .unwrap_or_default();
    let paid_periods = decls
        .iter()
        .filter(|d| d.amount_per_share_minor.unwrap_or(0) > 0)
        .map(|d| d.payment_period.clone())
        .collect::<Vec<_>>();
    let plan = financial_domain::schedule::merge_runtime_vendor_payables(
        as_of,
        &existing,
        &paid_periods,
        &vendor,
    );
    let mut wrote = 0u64;
    for (from, to) in &plan.moves {
        if canonical
            .issuer_pay_date_supersede_one(security_id, from.clone())
            .await
            .is_ok()
        {
            let rec = IssuerPayDateRecord {
                pay_date_id: Uuid::new_v4(),
                security_id,
                pay_on: to.clone(),
                source: "vendor_payable".into(),
                recorded_at: recorded_at.to_string(),
            };
            if canonical.issuer_pay_date_insert(rec).await.is_ok() {
                wrote = wrote.saturating_add(1);
            }
        }
        if !financial_domain::schedule::period_has_occurred(from, as_of) {
            if let Some(old) = decls.iter().find(|d| d.payment_period == *from) {
                let occurred: Vec<(i64, u8)> = decls
                    .iter()
                    .filter(|d| {
                        d.amount_per_share_minor.unwrap_or(0) > 0
                            && financial_domain::schedule::period_has_occurred(
                                &d.payment_period,
                                as_of,
                            )
                    })
                    .filter_map(|d| d.amount_per_share_minor.map(|a| (a, d.amount_scale)))
                    .collect();
                let pay_refs: Vec<&str> = existing.iter().map(String::as_str).collect();
                let amt = old.amount_per_share_minor.unwrap_or(0);
                let placeholder = financial_domain::schedule::unoccurred_declaration_is_placeholder(
                    from,
                    amt,
                    old.amount_scale,
                    as_of,
                    &occurred,
                    &pay_refs,
                    &old.entered_at,
                );
                let _ = canonical
                    .issuer_declaration_supersede_period(security_id, from.clone())
                    .await;
                if !placeholder && amt > 0 {
                    let _ = canonical
                        .issuer_declaration_record(
                            security_id,
                            old.amount_per_share_minor,
                            old.amount_scale,
                            to.clone(),
                            "vendor_payable".into(),
                            recorded_at.to_string(),
                        )
                        .await;
                }
            }
        }
    }
    for pay_on in &plan.add {
        let rec = IssuerPayDateRecord {
            pay_date_id: Uuid::new_v4(),
            security_id,
            pay_on: pay_on.clone(),
            source: "vendor_payable".into(),
            recorded_at: recorded_at.to_string(),
        };
        if canonical.issuer_pay_date_insert(rec).await.is_ok() {
            wrote = wrote.saturating_add(1);
        }
    }
    for conflict in &plan.conflicts {
        let _ = work_ticket_raise_or_bump(
            canonical,
            security_id,
            symbol,
            "paid_payable_supersede",
            &format!(
                "Vendor payable {} would silent-supersede paid/occurred {}.",
                conflict.vendor_pay_on,
                if conflict.existing_pay_on.is_empty() {
                    "row"
                } else {
                    conflict.existing_pay_on.as_str()
                }
            ),
            "",
        )
        .await;
    }
    ticket_if_remaining_count_mismatch(canonical, security_id, symbol, as_of, payment_frequency)
        .await;
    if plan.conflicts.is_empty() {
        work_ticket_auto_resolve_codes(canonical, security_id, &["paid_payable_supersede"]).await;
    }
    let _ = align_unoccurred_declarations_to_plan(canonical, security_id, as_of).await;
    wrote
}

async fn align_unoccurred_declarations_to_plan(
    canonical: &dyn Canonical,
    security_id: Uuid,
    as_of: &str,
) -> u64 {
    let decls = canonical
        .issuer_declaration_list(security_id)
        .await
        .unwrap_or_default();
    let pay_ons = canonical
        .issuer_pay_date_list(security_id)
        .await
        .unwrap_or_default();
    let pay_refs: Vec<&str> = pay_ons.iter().map(|p| p.pay_on.as_str()).collect();
    let occurred: Vec<(i64, u8)> = decls
        .iter()
        .filter(|d| {
            d.amount_per_share_minor.unwrap_or(0) > 0
                && financial_domain::schedule::period_has_occurred(&d.payment_period, as_of)
        })
        .filter_map(|d| d.amount_per_share_minor.map(|a| (a, d.amount_scale)))
        .collect();
    let mut n = 0u64;
    for d in &decls {
        let amt = d.amount_per_share_minor.unwrap_or(0);
        if amt <= 0 {
            continue;
        }
        if !financial_domain::schedule::unoccurred_declaration_is_placeholder(
            &d.payment_period,
            amt,
            d.amount_scale,
            as_of,
            &occurred,
            &pay_refs,
            &d.entered_at,
        ) {
            continue;
        }
        if canonical
            .issuer_declaration_supersede_period(security_id, d.payment_period.clone())
            .await
            .ok()
            .unwrap_or(0)
            > 0
        {
            n = n.saturating_add(1);
        }
    }
    n
}

async fn work_ticket_auto_resolve_codes(
    canonical: &dyn Canonical,
    security_id: Uuid,
    codes: &[&str],
) {
    let Ok(open) = canonical
        .work_ticket_list(Some(security_id), Some("open".into()))
        .await
    else {
        return;
    };
    let today = today_iso();
    for mut t in open {
        if !codes.iter().any(|c| t.code == *c) {
            continue;
        }
        t.status = "done".into();
        t.filed_on = today.clone();
        t.completed_how = "auto_resolved".into();
        t.last_seen_on = today.clone();
        let _ = canonical.work_ticket_update(t).await;
    }
}

async fn ticket_if_remaining_count_mismatch(
    canonical: &dyn Canonical,
    security_id: Uuid,
    symbol: &str,
    as_of: &str,
    payment_frequency: &str,
) {
    if financial_domain::calculator::PaymentCadence::parse(payment_frequency)
        .and_then(financial_domain::calculator::PaymentCadence::periods)
        .is_none()
    {
        return;
    }
    let pays = canonical
        .issuer_pay_date_list(security_id)
        .await
        .unwrap_or_default();
    let stored_ons: Vec<&str> = pays.iter().map(|p| p.pay_on.as_str()).collect();
    let vendor_ons: Vec<&str> = pays
        .iter()
        .filter(|p| !p.source.eq_ignore_ascii_case("derived_walk"))
        .map(|p| p.pay_on.as_str())
        .collect();
    if financial_domain::collector::remaining_year_dates_disagree(&stored_ons, &vendor_ons, as_of)
    {
        let _ = work_ticket_raise_or_bump(
            canonical,
            security_id,
            symbol,
            "remaining_year",
            "Remaining-year stored dates disagree with the issuer list through 31 Dec.",
            "",
        )
        .await;
    }
}

fn work_ticket_resolve_body(ticket: &WorkTicketRecord, ok: bool, code: &str, message: &str) -> String {
    serde_json::json!({
        "ok": ok,
        "code": code,
        "message": message,
        "ticketId": ticket.ticket_id,
        "securityId": ticket.security_id,
        "symbol": ticket.symbol,
        "tool": ticket.tool,
        "status": ticket.status,
    })
    .to_string()
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
        let mut future_pay_dates: Vec<String> = pay_dates
            .iter()
            .filter(|d| d.pay_on.as_str() > as_of.as_str())
            .map(|d| d.pay_on.clone())
            .collect();
        future_pay_dates.sort();
        future_pay_dates.dedup();
        let future_pay_dates_qty = future_pay_dates.len() as u64;
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
            future_pay_dates,
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
    let mut last_price_refreshed_on: Option<String> = None;
    let mut market_value_minor = 0i64;
    let mut any_market_value = false;
    let mut market_value_complete = !qty.is_empty();
    for (security_id, (remaining_quantity_minor, quantity_scale)) in &qty {
        let price = canonical
            .current_price_get(*security_id, today.clone())
            .await?;
        if price.price_derived_valid {
            last_price_count += 1;
            if let Some(as_of) = price
                .as_of_at
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                let day = as_of.get(..10).unwrap_or(as_of);
                last_price_refreshed_on = Some(match last_price_refreshed_on {
                    Some(prev) if prev.as_str() >= day => prev,
                    _ => day.to_string(),
                });
            }
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
    let declaration_as_of = chrono::Local::now().format("%Y-%m-%d").to_string();
    let mut declaration_count = 0u64;
    let mut declaration_collector_count = 0u64;
    let mut declaration_refreshed_on: Option<String> = None;
    if let Ok(set) = canonical.collector_set().await {
        for item in set.items {
            if !item.open_lots
                || !item.collector_enabled
                || item.declaration_source.trim().is_empty()
            {
                continue;
            }
            declaration_collector_count += 1;
            if financial_domain::collector::declaration_daily_retrieve_current(
                item.last_run_ok,
                &item.last_run_at,
                &declaration_as_of,
            ) {
                declaration_count += 1;
            }
            let stamp = item.last_run_at.trim();
            if stamp.len() >= 10 {
                let day = stamp.get(..10).unwrap_or(stamp);
                declaration_refreshed_on = Some(match declaration_refreshed_on {
                    Some(prev) if prev.as_str() >= day => prev,
                    _ => day.to_string(),
                });
            }
        }
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
        last_price_refreshed_on,
        declaration_count,
        declaration_collector_count,
        declaration_refreshed_on,
        declaration_as_of,
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
        if financial_domain::calculator::is_non_paying(
            ch.map(|c| c.payment_frequency.as_str()).unwrap_or(""),
        ) {
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

fn cadence_filter_matches(stored: &str, filter: &str) -> bool {
    let filter = filter.trim();
    if filter.is_empty() || filter.eq_ignore_ascii_case("all") {
        return true;
    }
    let Some(want) = financial_domain::calculator::PaymentCadence::parse(filter) else {
        return false;
    };
    financial_domain::calculator::PaymentCadence::parse(stored) == Some(want)
}

fn cadence_sort_rank(freq: &str) -> u8 {
    match financial_domain::calculator::PaymentCadence::parse(freq) {
        Some(financial_domain::calculator::PaymentCadence::Weekly) => 0,
        Some(financial_domain::calculator::PaymentCadence::Monthly) => 1,
        Some(financial_domain::calculator::PaymentCadence::Quarterly) => 2,
        _ => 3,
    }
}

async fn declaration_history_view(
    canonical: &dyn Canonical,
    as_of: &str,
    cadence_filter: &str,
    start_on: Option<&str>,
    end_on: Option<&str>,
    week_count: Option<u32>,
) -> Result<DeclarationHistoryGetBody, PlatformError> {
    let as_of_date = parse_iso_date(as_of)
        .or_else(|| parse_iso_date(&today_iso()))
        .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(2026, 9, 2).unwrap());
    let end_date = end_on
        .and_then(parse_iso_date)
        .unwrap_or(as_of_date);
    let start_date = start_on.and_then(parse_iso_date).unwrap_or_else(|| {
        if let Some(n) = week_count {
            let newest = financial_domain::week::week_containing(end_date).end;
            newest - chrono::Duration::days(7 * (n.clamp(1, 52).saturating_sub(1) as i64))
        } else {
            financial_domain::week::history_window_start(end_date)
        }
    });
    let week_ends: Vec<String> =
        financial_domain::week::friday_week_ends_in_range(start_date, end_date)
            .into_iter()
            .map(|d| d.to_string())
            .collect();
    let week_ids: Vec<financial_domain::week::WeekId> = week_ends
        .iter()
        .filter_map(|friday| {
            financial_domain::week::parse_iso_day(friday)
                .map(financial_domain::week::week_id_containing)
        })
        .collect();
    let week_starts: Vec<String> = week_ids
        .iter()
        .map(|id| id.week.start.to_string())
        .collect();
    let week_years: Vec<i32> = week_ids.iter().map(|id| id.year).collect();
    let week_numbers: Vec<u8> = week_ids.iter().map(|id| id.number).collect();
    let securities = canonical.security_list().await?;
    let characteristics = canonical.position_characteristic_list().await?;
    let char_by_sec: std::collections::HashMap<_, _> = characteristics
        .iter()
        .map(|c| (c.security_id, c))
        .collect();
    let basis = canonical.basis_get().await?;
    let mut open_qty: std::collections::HashSet<uuid::Uuid> = std::collections::HashSet::new();
    for lot in &basis.lots {
        if lot.remaining_quantity_minor > 0 {
            open_qty.insert(lot.security_id);
        }
    }
    let mut rows = Vec::new();
    for security in &securities {
        if !open_qty.contains(&security.security_id) {
            continue;
        }
        let freq = char_by_sec
            .get(&security.security_id)
            .map(|c| c.payment_frequency.as_str())
            .unwrap_or("");
        if financial_domain::calculator::is_non_paying(freq) {
            continue;
        }
        if !cadence_filter_matches(freq, cadence_filter) {
            continue;
        }
        let decls = canonical
            .issuer_declaration_list(security.security_id)
            .await
            .unwrap_or_default();
        let mut by_week: std::collections::HashMap<String, (String, i64, u8)> =
            std::collections::HashMap::new();
        for d in decls {
            let Some(amt) = d.amount_per_share_minor.filter(|a| *a > 0) else {
                continue;
            };
            let Some(pay_on) = financial_domain::week::parse_iso_day(&d.payment_period) else {
                continue;
            };
            let week_end = financial_domain::week::week_end_for_pay_on(pay_on).to_string();
            let key = d.payment_period.clone();
            match by_week.get(&week_end) {
                Some((prev, _, _)) if prev.as_str() >= key.as_str() => {}
                _ => {
                    by_week.insert(week_end, (key, amt, d.amount_scale));
                }
            }
        }
        let cells = week_ends
            .iter()
            .map(|week| match by_week.get(week) {
                Some((_, amt, scale)) => DeclarationHistoryCellBody {
                    amount_per_share_minor: Some(*amt),
                    amount_scale: *scale,
                },
                None => DeclarationHistoryCellBody {
                    amount_per_share_minor: None,
                    amount_scale: 0,
                },
            })
            .collect();
        rows.push(DeclarationHistoryRowBody {
            symbol: security.symbol.clone(),
            payment_frequency: freq.to_string(),
            cells,
        });
    }
    rows.sort_by(|a, b| {
        cadence_sort_rank(&a.payment_frequency)
            .cmp(&cadence_sort_rank(&b.payment_frequency))
            .then_with(|| a.symbol.cmp(&b.symbol))
    });
    Ok(DeclarationHistoryGetBody {
        as_of_date: as_of_date.to_string(),
        start_on: start_date.to_string(),
        end_on: end_date.to_string(),
        cadence_filter: cadence_filter.to_string(),
        week_ends,
        week_starts,
        week_years,
        week_numbers,
        rows,
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
        held_in_2025_from_lots(security_id, &basis.lots),
    );
    let declaration_freshness = freshness_for(&decls, &as_of, template.as_ref());
    let roc_est = latest_roc_estimate(&observations);
    let roc_research_completed_at =
        roc_research_completed_at(&roc_research_status, &observations);
    let collector = collector_status_for(canonical, security_id, &security.symbol, &as_of).await;
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
        collector_complete: collector.complete,
        collector_gaps: collector.gaps,
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

/// Process A / Validate: keep needs_roc_research true even when 19a-1 misses (unknown ≠ 0%).
async fn mark_needs_roc_research(canonical: &dyn Canonical, security_id: Uuid) {
    let Ok(list) = canonical.position_characteristic_list().await else {
        return;
    };
    let mut rec = list
        .into_iter()
        .find(|c| c.security_id == security_id)
        .unwrap_or_else(|| empty_characteristic(security_id));
    let symbol = canonical
        .security_get(security_id)
        .await
        .map(|s| s.symbol)
        .unwrap_or_default();
    if !financial_domain::collector::needs_roc_research_on_create(&rec.div_type, &symbol) {
        rec.needs_roc_research = false;
        let _ = canonical.position_characteristic_upsert(rec).await;
        return;
    }
    if rec.needs_roc_research {
        return;
    }
    rec.needs_roc_research = true;
    let _ = canonical.position_characteristic_upsert(rec).await;
}

/// Persist provider from URL/vendor, optional page title name, underlying + lookthrough.
/// Writes only holes — never overwrites set provider/underlying/lookthrough or applies risk_tier.
/// Income payers get DIV-1 on create. CASH stays CASH and does not need ROC.
async fn persist_process_a_identity(
    canonical: &dyn Canonical,
    security: &crate::contracts::SecurityRecord,
    symbol: &str,
    declaration_source: &str,
    json: &Value,
) {
    let provider = jstr(json, "provider")
        .or_else(|| jstr(json, "suggestedProvider"))
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            let label = financial_domain::div1::source_label(declaration_source);
            if label.is_empty() {
                String::new()
            } else {
                label.to_string()
            }
        });
    let underlying_raw = jstr(json, "underlying").unwrap_or_default();
    // Never persist ticker-as-underlying (HAKY ≠ HACK).
    let underlying = if underlying_raw.eq_ignore_ascii_case(symbol) {
        String::new()
    } else {
        underlying_raw
    };
    let lookthrough = jlookthrough_keep(json, "lookthrough", LookthroughResearch::default());
    let page_name = jstr(json, "name")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    if let Some(name) = page_name.as_ref() {
        let ticker_as_name = security.name.eq_ignore_ascii_case(symbol) || security.name.is_empty();
        if ticker_as_name && !name.eq_ignore_ascii_case(symbol) {
            let _ = canonical
                .security_update(security.security_id, Some(name.clone()), None)
                .await;
        }
    }

    let existing = match canonical.position_characteristic_list().await {
        Ok(list) => list.into_iter().find(|c| c.security_id == security.security_id),
        Err(_) => None,
    };
    let mut rec = existing.unwrap_or_else(|| empty_characteristic(security.security_id));
    // Fill blanks only. Correct ticker-as-underlying hole.
    if rec.provider.trim().is_empty() && !provider.is_empty() {
        rec.provider = provider;
    }
    if rec.underlying.eq_ignore_ascii_case(symbol) {
        rec.underlying.clear();
    }
    if rec.underlying.trim().is_empty() && !underlying.is_empty() {
        rec.underlying = underlying;
    }
    if rec.lookthrough == LookthroughResearch::default()
        && lookthrough != LookthroughResearch::default()
    {
        rec.lookthrough = lookthrough;
    }
    rec.div_type = financial_domain::collector::div_type_on_create(
        symbol,
        jstr(json, "divType").unwrap_or_default().as_str(),
        rec.div_type.as_str(),
    );
    rec.needs_roc_research =
        financial_domain::collector::needs_roc_research_on_create(&rec.div_type, symbol);
    let _ = canonical.position_characteristic_upsert(rec).await;
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

fn held_in_2025_from_lots(security_id: Uuid, lots: &[LotRecord]) -> bool {
    let opened: Vec<&str> = lots
        .iter()
        .filter(|l| l.security_id == security_id)
        .map(|l| l.opened_on.as_str())
        .collect();
    financial_domain::lifetime::held_in_calendar_year(&opened, 2025)
}

fn roc_status_for(
    needs_roc: bool,
    ch: Option<&PositionCharacteristicRecord>,
    observations: &[RocResearchObservation],
    has_open_car: bool,
    held_in_2025: bool,
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
    financial_domain::lifetime::roc_research_status(in_scope, &views, held_in_2025).to_string()
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
    // Owner hub "ROC last update": latest observation write (estimate or complete), not only owner-complete.
    let latest_obs = observations
        .iter()
        .map(|o| o.recorded_at.as_str())
        .filter(|s| !s.is_empty())
        .max()
        .map(|s| s.to_string());
    if latest_obs.is_some() {
        return latest_obs;
    }
    if status == "complete" {
        return None;
    }
    None
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
            held_in_2025_from_lots(security_id, &basis.lots),
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
                held_in_2025_from_lots(security_id, &basis.lots),
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
    let latest = latest_declaration_period(canonical, security_id, &as_of).await?;
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
            let latest = latest_declaration_period(canonical, security_id, &as_of).await?;
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
    if c.owner_override {
        return false;
    }
    let Some(p) = c.roc_pct_minor else {
        return false;
    };
    let from_notice = c.method == "19a-1-current-year";
    let from_table = c.method == "table-roc-current";
    if from_notice {
        return (0..=10_000).contains(&p);
    }
    from_table && p > 0
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
            source_url: jstr(json, "sourceUrl").unwrap_or_default(),
            method: "owner-override".into(),
            as_of: as_of.clone(),
            kind: "estimate".into(),
            established_how: jstr(json, "establishedHow")
                .unwrap_or_else(|| "owner typed percent".into()),
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
/// then runs PositionResearchRefresh (same path as Complete research / Fill research gaps).
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

    let existing_template = canonical
        .retrieval_template_get(security.security_id)
        .await
        .ok()
        .flatten();
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
            last_content_hash: existing_template
                .as_ref()
                .map(|t| t.last_content_hash.clone())
                .unwrap_or_default(),
            collector_enabled,
            inception_on: jstr(json, "inceptionOn")
                .or_else(|| existing_template.as_ref().map(|t| t.inception_on.clone()))
                .unwrap_or_default(),
            roc_source_url: jstr(json, "rocSourceUrl")
                .or_else(|| existing_template.as_ref().map(|t| t.roc_source_url.clone()))
                .unwrap_or_default(),
            history_url_attempts: existing_template
                .as_ref()
                .filter(|t| t.source_url == source_url)
                .map(|t| t.history_url_attempts)
                .unwrap_or(0),
        })
        .await
    {
        return command_err(request, &err.code);
    }

    // Provider from URL/vendor; name/underlying/lookthrough when present (live profile).
    persist_process_a_identity(canonical, &security, &symbol, &declaration_source, json).await;

    // Desktop host runs live enrichers via PositionResearchRefresh after template write.
    if jbool(json, "skipRefresh", false) {
        return command_ok(
            request,
            serde_json::to_string(&PositionResearchSeedBody {
                security_id: security.security_id,
                symbol,
                declaration_source,
                source_url,
                calendar_policy,
                roc_scale: financial_domain::roc::ROC_PCT_SCALE,
                ..PositionResearchSeedBody::default()
            })
            .unwrap_or_else(|_| "{}".into()),
        );
    }

    // Same hole-fill path as Complete research / Fill research gaps.
    let mut refresh_json = json.clone();
    if let Some(obj) = refresh_json.as_object_mut() {
        obj.insert(
            "securityId".into(),
            serde_json::json!(security.security_id),
        );
        obj.insert("symbol".into(), serde_json::json!(symbol));
        obj.insert(
            "declarationSource".into(),
            serde_json::json!(declaration_source),
        );
        obj.insert("sourceUrl".into(), serde_json::json!(source_url));
    }
    let refresh = Box::pin(position_research_refresh(
        platform,
        canonical,
        request,
        &refresh_json,
    ))
    .await;
    if !refresh.ok {
        return refresh;
    }
    let body: PositionResearchRefreshBody =
        serde_json::from_str(refresh.body_json.as_deref().unwrap_or("{}")).unwrap_or(
            PositionResearchRefreshBody {
                security_id: security.security_id,
                symbol: symbol.clone(),
                declaration_source: declaration_source.clone(),
                source_url: source_url.clone(),
                roc_scale: financial_domain::roc::ROC_PCT_SCALE,
                needs_roc_research: true,
                ..PositionResearchRefreshBody::default()
            },
        );

    command_ok(
        request,
        serde_json::to_string(&PositionResearchSeedBody {
            security_id: body.security_id,
            symbol: body.symbol,
            declaration_source: body.declaration_source,
            source_url: body.source_url,
            calendar_policy,
            payment_frequency: body.payment_frequency,
            retrieve_ok: body.retrieve_ok,
            retrieve_code: body.retrieve_code,
            retrieve_message: body.retrieve_message,
            roc_pct_minor: body.roc_pct_minor,
            roc_scale: body.roc_scale,
            roc_source_url: body.roc_source_url,
            roc_method: body.roc_method,
            roc_kind: body.roc_kind,
            roc_as_of: body.roc_as_of,
            roc_established_how: body.roc_established_how,
            roc_complete: false,
            roc_probes: body.roc_probes,
            needs_second_url: body.needs_second_url,
            second_url_tried: body.second_url_tried,
            adapter_failed: body.adapter_failed,
            paid_count: body.paid_count,
            needs_inception_confirm: body.needs_inception_confirm,
            inception_candidate: body.inception_candidate,
            expected_paid_since_inception: body.expected_paid_since_inception,
            inception_search_miss: body.inception_search_miss,
        })
        .unwrap_or_else(|_| "{}".into()),
    )
}

/// Shared research hole-fill: provider / name / underlying / frequency / last price / 19a-1 ROC.
/// Inputs: securityId or symbol. Writes blanks only; never wipes lots, PlanHistory, or decls.
/// Never writes PlanHistory — imported / owner plan amounts (e.g. AMDW $0.55) stay.
/// Does not auto-apply tier. Nested command path accepts injected payloads (golden); live
/// enrichers run in the Tauri host before this dispatch.
async fn position_research_refresh(
    platform: &dyn Platform,
    canonical: &dyn Canonical,
    request: &CommandRequest,
    json: &Value,
) -> CommandResult {
    let security = match resolve_security(canonical, json).await {
        Ok(s) => s,
        Err(code) => return command_err(request, &code),
    };
    let symbol = security.symbol.clone();

    let template = match canonical.retrieval_template_get(security.security_id).await {
        Ok(t) => t,
        Err(err) => return command_err(request, &err.code),
    };

    let source_url = jstr(json, "sourceUrl")
        .or_else(|| jstr(json, "distributionUrl"))
        .or_else(|| template.as_ref().map(|t| t.source_url.clone()))
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_default();
    let declaration_source = jstr(json, "declarationSource")
        .filter(|s| !s.trim().is_empty())
        .or_else(|| template.as_ref().map(|t| t.declaration_source.clone()))
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            if source_url.is_empty() {
                None
            } else {
                financial_domain::div1::declaration_source_from_url(&source_url)
                    .map(|s| s.to_string())
            }
        })
        .unwrap_or_else(|| "issuer".into());

    if !source_url.is_empty() {
        if let Some(existing) = template.as_ref() {
            if !existing.source_url.is_empty()
                && source_url != existing.source_url
                && !financial_domain::work_ticket::retry_url_matches_assigned_adapter(
                    &declaration_source,
                    &source_url,
                )
            {
                return command_ok(
                    request,
                    serde_json::to_string(&PositionResearchRefreshBody {
                        security_id: security.security_id,
                        symbol,
                        declaration_source,
                        source_url: existing.source_url.clone(),
                        retrieve_ok: false,
                        retrieve_code: financial_domain::work_ticket::CODE_ADAPTER_URL_MISMATCH
                            .into(),
                        retrieve_message:
                            "Second URL must stay on the assigned issuer. Manual adapter is parked."
                                .into(),
                        needs_second_url: existing.history_url_attempts < 2,
                        second_url_tried: existing.history_url_attempts >= 1,
                        adapter_failed: false,
                        roc_scale: financial_domain::roc::ROC_PCT_SCALE,
                        needs_roc_research: true,
                        ..PositionResearchRefreshBody::default()
                    })
                    .unwrap_or_else(|_| "{}".into()),
                );
            }
            if source_url != existing.source_url {
                let _ = persist_template_url_same_adapter(
                    canonical,
                    security.security_id,
                    &source_url,
                )
                .await;
            }
        }
    }

    persist_process_a_identity(canonical, &security, &symbol, &declaration_source, json).await;

    let mut retrieve_body = serde_json::json!({
        "securityId": security.security_id,
        "symbol": symbol,
        "declarationSource": declaration_source,
        "sourceUrl": source_url,
        "sourceSymbol": jstr(json, "sourceSymbol")
            .or_else(|| template.as_ref().map(|t| t.source_symbol.clone()))
            .unwrap_or_else(|| symbol.clone()),
        "inceptionOn": jstr(json, "inceptionOn")
            .or_else(|| template.as_ref().map(|t| t.inception_on.clone()))
            .unwrap_or_default(),
        "paymentFrequency": jstr(json, "paymentFrequency").unwrap_or_default(),
        "divType": jstr(json, "divType").unwrap_or_default(),
        "forceRefresh": jbool(json, "forceRefresh", false),
        "lastContentHash": template
            .as_ref()
            .map(|t| t.last_content_hash.clone())
            .unwrap_or_default(),
        "lastRunOk": template.as_ref().and_then(|t| t.last_run_ok).unwrap_or(false),
        "lastRunAt": template
            .as_ref()
            .map(|t| t.last_run_at.clone())
            .unwrap_or_default(),
    });
    for key in [
        "candidates",
        "declarations",
        "misses",
        "upcomingPays",
        "payDates",
        "quote",
        "contentHash",
        "unchanged",
        "knownPaymentPeriods",
        "knownDeclarationAmounts",
        "missExplanation",
        "suggestedFrequency",
        "sourceAnalytics",
        "researchAttempts",
        "fetchedSourceUrl",
        "fetchedPaymentCalendarUrl",
        "fetchedPage",
        "pagePaid",
    ] {
        if let Some(v) = json.get(key) {
            retrieve_body[key] = v.clone();
        }
    }

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
        let body: CollectorRetrieveBody =
            serde_json::from_str(retrieve.body_json.as_deref().unwrap_or("{}")).unwrap_or(
                CollectorRetrieveBody {
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
                },
            );
        (body.ok, body.code, body.message)
    } else {
        (
            false,
            retrieve
                .error_code
                .clone()
                .unwrap_or_else(|| "retrieve_failed".into()),
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

    let mut roc_json = json.clone();
    if roc_json.get("rocSourceUrl").is_none() {
        if let Some(url) = template
            .as_ref()
            .map(|t| t.roc_source_url.clone())
            .filter(|s| !s.trim().is_empty())
        {
            roc_json["rocSourceUrl"] = serde_json::json!(url);
        }
    }
    if let Some(url) = jstr(&roc_json, "rocSourceUrl").filter(|s| !s.trim().is_empty()) {
        roc_json["sourceUrl"] = serde_json::json!(url);
        if let Ok(Some(mut rec)) = canonical.retrieval_template_get(security.security_id).await {
            rec.roc_source_url = url;
            let _ = canonical.retrieval_template_set(rec).await;
        }
    }
    let roc = process_a_propose_roc_estimate(
        platform,
        canonical,
        request,
        security.security_id,
        &symbol,
        &declaration_source,
        &roc_json,
    )
    .await;
    let mut roc_probes = roc.roc_probes.clone();
    if roc_probes.is_empty() {
        if let Some(arr) = json.get("rocProbes").and_then(|v| v.as_array()) {
            roc_probes = arr.clone();
        }
    }
    if roc_probes.is_empty() {
        if let Ok(runs) = canonical
            .retrieve_run_list(Some(security.security_id), 20)
            .await
        {
            if let Some(run) = runs.iter().find(|r| r.kind == "roc-19a1") {
                if let Ok(payload) = serde_json::from_str::<Value>(run.payload_json.as_str()) {
                    if let Some(arr) = payload.get("probes").and_then(|p| p.as_array()) {
                        roc_probes = arr.clone();
                    }
                }
            }
        }
    }
    mark_needs_roc_research(canonical, security.security_id).await;

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

    let chars = canonical
        .position_characteristic_list()
        .await
        .unwrap_or_default();
    let ch = chars
        .into_iter()
        .find(|c| c.security_id == security.security_id);
    let provider = ch
        .as_ref()
        .map(|c| c.provider.clone())
        .unwrap_or_default();
    let underlying = ch
        .as_ref()
        .map(|c| c.underlying.clone())
        .unwrap_or_default();
    let freq = if payment_frequency.is_empty() {
        ch.as_ref()
            .map(|c| c.payment_frequency.clone())
            .unwrap_or_default()
    } else {
        payment_frequency
    };
    let needs = ch.map(|c| c.needs_roc_research).unwrap_or(true);

    let mut ticket_json = json.clone();
    if ticket_json.get("sourceUrl").is_none() {
        ticket_json["sourceUrl"] = serde_json::json!(source_url);
    }
    if !roc_probes.is_empty() {
        ticket_json["rocProbes"] = serde_json::json!(roc_probes.clone());
    }
    raise_collector_identity_tickets(canonical, security.security_id, &symbol, &ticket_json).await;

    let establish = apply_phase_i_establish(
        canonical,
        security.security_id,
        &symbol,
        &source_url,
        &freq,
        retrieve_ok,
        &retrieve_code,
        &retrieve_message,
        json,
    )
    .await;
    if !roc.roc_source_url.trim().is_empty() {
        if let Ok(Some(mut rec)) = canonical.retrieval_template_get(security.security_id).await {
            if rec.roc_source_url.trim().is_empty() {
                rec.roc_source_url = roc.roc_source_url.clone();
                let _ = canonical.retrieval_template_set(rec).await;
            }
        }
    }

    command_ok(
        request,
        serde_json::to_string(&PositionResearchRefreshBody {
            security_id: security.security_id,
            symbol,
            declaration_source,
            source_url,
            provider,
            underlying,
            payment_frequency: freq,
            retrieve_ok,
            retrieve_code,
            retrieve_message: establish.retrieve_message,
            roc_pct_minor: roc.roc_pct_minor,
            roc_scale: roc.roc_scale,
            roc_source_url: roc.roc_source_url,
            roc_method: roc.roc_method,
            roc_kind: roc.roc_kind,
            roc_as_of: roc.roc_as_of,
            roc_established_how: roc.roc_established_how,
            roc_complete: false,
            roc_probes,
            needs_roc_research: needs,
            needs_second_url: establish.needs_second_url,
            second_url_tried: establish.second_url_tried,
            adapter_failed: establish.adapter_failed,
            paid_count: establish.paid_count,
            needs_inception_confirm: establish.needs_inception_confirm,
            inception_candidate: establish.inception_candidate,
            expected_paid_since_inception: establish.expected_paid_since_inception,
            inception_search_miss: establish.inception_search_miss,
        })
        .unwrap_or_else(|_| "{}".into()),
    )
}

async fn resolve_security(
    canonical: &dyn Canonical,
    json: &Value,
) -> Result<crate::contracts::SecurityRecord, String> {
    if let Some(id) = juuid(json, "securityId") {
        return canonical
            .security_get(id)
            .await
            .map_err(|e| e.code.clone());
    }
    let symbol = jstr(json, "symbol")
        .map(|s| s.trim().to_ascii_uppercase())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "missing_security_id".to_string())?;
    let list = canonical
        .security_list()
        .await
        .map_err(|e| e.code.clone())?;
    list.into_iter()
        .find(|s| s.symbol.eq_ignore_ascii_case(&symbol))
        .ok_or_else(|| "security_not_found".to_string())
}

async fn collector_fleet_grid(
    canonical: &dyn Canonical,
    as_of: &str,
) -> Result<CollectorSetBody, PlatformError> {
    let mut set = canonical.collector_set().await?;
    let tickets = canonical
        .work_ticket_list(None, Some("open".into()))
        .await
        .unwrap_or_default();
    let chars = canonical
        .position_characteristic_list()
        .await
        .unwrap_or_default();
    let basis = canonical.basis_get().await.ok();
    for item in &mut set.items {
        let status = collector_status_for(canonical, item.security_id, &item.symbol, as_of).await;
        item.complete = status.complete;
        let mut gaps = status.gaps;

        let ch = chars.iter().find(|c| c.security_id == item.security_id);
        let provider = ch
            .map(|c| c.provider.clone())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| item.provider.clone());
        let frequency = ch
            .map(|c| c.payment_frequency.clone())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| item.payment_frequency.clone());
        let div_type = ch
            .map(|c| c.div_type.clone())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| item.div_type.clone());
        let underlying = ch.map(|c| c.underlying.clone()).unwrap_or_default();
        let roc_pct = ch.and_then(|c| c.roc_pct_2026_estimate_minor);
        let holes = financial_domain::collector::runtime_research_gaps(
            &item.symbol,
            &provider,
            &frequency,
            &div_type,
            roc_pct,
            &underlying,
        );
        item.fill_gaps_provider_blank = holes.provider_blank;
        item.fill_gaps_frequency_blank = holes.frequency_blank;
        item.fill_gaps_div_type_blank = holes.div_type_blank;
        item.fill_gaps_roc_blank = holes.roc_estimate_blank;
        item.underlying = underlying;
        item.roc_estimate_minor = roc_pct;
        item.roc_scale = ch.and_then(|c| c.roc_scale).unwrap_or(2);
        item.roc_tax_year = if roc_pct.is_some() {
            as_of.get(..4).unwrap_or("").to_string()
        } else {
            String::new()
        };
        item.provider = provider;
        item.payment_frequency = frequency.clone();
        item.div_type = div_type.clone();
        let has_miss_ticket = tickets.iter().any(|t| {
            t.security_id == item.security_id
                && t.status == "open"
                && financial_domain::work_ticket::is_retrieve_failure_code(&t.code)
        });
        if financial_domain::collector::needs_owner_seed_url(
            &div_type,
            &item.symbol,
            &item.source_url,
            item.last_run_ok,
            has_miss_ticket,
        ) {
            gaps.push("seed_url".into());
        }
        item.gaps = financial_domain::collector::fleet_gap_labels(&gaps, status.complete);

        let decls = canonical
            .issuer_declaration_list(item.security_id)
            .await
            .unwrap_or_default();
        item.paid_declaration_count = decls
            .iter()
            .filter(|d| d.amount_per_share_minor.unwrap_or(0) > 0)
            .count()
            .min(255) as u8;

        let pays = canonical
            .issuer_pay_date_list(item.security_id)
            .await
            .unwrap_or_default();
        let pay_refs: Vec<&str> = pays.iter().map(|p| p.pay_on.as_str()).collect();
        item.last_payable_on = pay_refs
            .iter()
            .max()
            .map(|s| (*s).to_string())
            .unwrap_or_default();
        let cash = financial_domain::current_price::uses_cash_par(&div_type, &item.symbol);
        if cash {
            item.remaining_planned = None;
            item.remaining_expected = None;
        } else {
            item.remaining_planned = Some(financial_domain::collector::stored_remaining_planned(
                &pay_refs, as_of,
            ));
            let periods = financial_domain::calculator::PaymentCadence::parse(&frequency)
                .and_then(financial_domain::calculator::PaymentCadence::periods);
            item.remaining_expected =
                periods.and_then(|p| financial_domain::schedule::remaining_periods_to_year_end(as_of, p));
        }

        let runs = canonical
            .retrieve_run_list(Some(item.security_id), 200)
            .await
            .unwrap_or_default();
        let decl_runs: Vec<_> = runs.iter().filter(|r| r.kind == "declaration").collect();
        item.successful_run_count = decl_runs.iter().filter(|r| r.ok).count() as u64;
        let open_for: Vec<_> = tickets
            .iter()
            .filter(|t| t.security_id == item.security_id && t.status == "open")
            .collect();
        item.open_ticket_count = open_for.len() as u64;
        item.latest_ticket_field = open_for
            .iter()
            .max_by(|a, b| a.last_seen_on.cmp(&b.last_seen_on))
            .map(|t| {
                if t.field.trim().is_empty() {
                    t.code.clone()
                } else {
                    t.field.clone()
                }
            })
            .unwrap_or_default();
        let miss_runs = decl_runs.iter().filter(|r| !r.ok).count() as u64;
        item.failure_count = miss_runs.saturating_add(item.open_ticket_count);

        if let Ok(price) = canonical.current_price_get(item.security_id, as_of.to_string()).await
        {
            item.last_price_as_of = price.as_of_at.unwrap_or_default();
            item.last_price_freshness = match price.freshness.as_str() {
                "current" | "manual_override" => "Current".into(),
                "stale" => "Stale".into(),
                _ => "Unavailable".into(),
            };
            if price.price_minor.is_none() || !price.price_derived_valid {
                item.last_price_freshness = "Unavailable".into();
            }
        } else {
            item.last_price_freshness = "Unavailable".into();
        }

        item.open_lot_count = basis
            .as_ref()
            .map(|b| {
                b.lots
                    .iter()
                    .filter(|l| l.security_id == item.security_id && l.remaining_quantity_minor > 0)
                    .count() as u64
            })
            .unwrap_or(0);

        if item.declaration_source.eq_ignore_ascii_case("derived_walk") {
            item.declaration_source.clear();
        }
    }
    Ok(set)
}

async fn research_gaps_get(canonical: &dyn Canonical) -> Result<ResearchGapsGetBody, PlatformError> {
    let set = canonical.collector_set().await?;
    let chars = canonical.position_characteristic_list().await?;
    let mut items = Vec::new();
    for item in set.items {
        if !item.collector_enabled || !item.open_lots {
            continue;
        }
        let ch = chars.iter().find(|c| c.security_id == item.security_id);
        let provider = ch
            .map(|c| c.provider.trim().to_string())
            .unwrap_or_else(|| item.provider.trim().to_string());
        let underlying = ch
            .map(|c| c.underlying.trim().to_string())
            .unwrap_or_default();
        let frequency = ch
            .map(|c| c.payment_frequency.trim().to_string())
            .unwrap_or_else(|| item.payment_frequency.trim().to_string());
        let div_type = ch
            .map(|c| c.div_type.as_str())
            .unwrap_or("");
        let holes = financial_domain::collector::runtime_research_gaps(
            &item.symbol,
            &provider,
            &frequency,
            div_type,
            ch.and_then(|c| c.roc_pct_2026_estimate_minor),
            &underlying,
        );
        if holes.is_hole() {
            items.push(ResearchGapItem {
                security_id: item.security_id,
                symbol: item.symbol,
                provider_blank: holes.provider_blank,
                underlying_blank: holes.underlying_blank,
                frequency_blank: holes.frequency_blank,
                roc_observation_blank: holes.roc_estimate_blank,
                div_type_blank: holes.div_type_blank,
                roc_estimate_blank: holes.roc_estimate_blank,
            });
        }
    }
    Ok(ResearchGapsGetBody { items })
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
    roc_probes: Vec<Value>,
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
    if financial_domain::current_price::uses_cash_par("", symbol) {
        return out;
    }
    let chars = canonical
        .position_characteristic_list()
        .await
        .unwrap_or_default();
    if chars
        .iter()
        .find(|c| c.security_id == security_id)
        .is_some_and(|c| financial_domain::current_price::uses_cash_par(&c.div_type, symbol))
    {
        return out;
    }
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
        "sourceUrl": jstr(json, "rocSourceUrl")
            .or_else(|| jstr(json, "sourceUrl"))
            .or_else(|| jstr(json, "distributionUrl"))
            .unwrap_or_default(),
    });
    if let Some(c) = injected {
        roc_body["candidates"] = c;
    }
    if let Some(probes) = json.get("rocProbes") {
        roc_body["rocProbes"] = probes.clone();
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
    if let Some(arr) = body
        .get("rocProbes")
        .or_else(|| body.get("probes"))
        .and_then(|p| p.as_array())
    {
        out.roc_probes = arr.clone();
    }
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
    // Existing identity only — do not insert a blank characteristic. Cadence is not required
    // to persist a parsed 19a-1 estimate. Never write 2025 actual here.
    let Some(mut rec) = list.into_iter().find(|c| c.security_id == security_id) else {
        return;
    };
    // Do not overwrite a 1099 2026 actual. Live 19a-1 replaces seed/stale
    // estimates. Owner-confirmed override (RocPlanConfirm) is kept.
    // Parsed 0% is valid only when the notice said 0.
    if rec.roc_pct_2026_actual_minor.is_some() {
        return;
    }
    let live = system.roc_pct_minor.filter(|p| (0..=10_000).contains(p));
    let Some(pct) = live else {
        return;
    };
    let owner_locked = !rec.needs_roc_research
        && canonical
            .roc_observation_list(security_id)
            .await
            .ok()
            .map(|obs| obs.iter().any(|o| o.owner_override))
            .unwrap_or(false);
    if owner_locked {
        return;
    }
    rec.roc_pct_2026_estimate_minor = Some(pct);
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
        "CoreFunctionsGet" => match crate::core_functions::core_functions_catalog() {
            Ok(body) => match to_json(&body) {
                Ok(json) => query_ok(&request, json),
                Err(_) => query_err(&request, "serialize_failed"),
            },
            Err(_) => query_err(&request, "core_functions_invalid"),
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
        "CoreFunctionsGet" => map_q(
            &request,
            crate::core_functions::core_functions_catalog()
                .map_err(|e| PlatformError {
                    code: "core_functions_invalid".into(),
                    message: e,
                }),
        ),
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
        "ImportPendingGet" => map_q(
            &request,
            canonical.import_pending_get().await.map(|batch| {
                crate::contracts::ImportPendingBody { batch }
            }),
        ),
        "ActivityGet" => match juuid(&json, "activityId") {
            Some(id) => map_q(&request, canonical.activity_get(id).await),
            None => query_err(&request, "missing_activity_id"),
        },
        "ActivityList" => map_q(&request, canonical.activity_list().await),
        "AuditList" => map_q(&request, canonical.audit_list().await),
        "ExceptionList" => map_q(&request, canonical.exception_list().await),
        "WorkTicketList" => {
            let security_id = juuid(&json, "securityId");
            let status = jstr(&json, "status");
            match canonical.work_ticket_list(security_id, status).await {
                Ok(items) => {
                    let open: Vec<_> = items.iter().filter(|t| t.status == "open").collect();
                    let mut symbols = std::collections::HashSet::new();
                    for t in &open {
                        symbols.insert(t.symbol.clone());
                    }
                    map_q(
                        &request,
                        Ok(WorkTicketListBody {
                            open_count: open.len() as u64,
                            symbol_count: symbols.len() as u64,
                            items,
                        }),
                    )
                }
                Err(err) => query_err(&request, &err.code),
            }
        }
        "CashDividendCoverageGet" => {
            let as_of = jstr(&json, "asOfDate").unwrap_or_else(today_iso);
            map_q(&request, cash_dividend_coverage_refresh(canonical, &as_of).await)
        }
        "CanonicalWeekGet" => {
            let as_of = jstr(&json, "asOfDate").unwrap_or_else(today_iso);
            map_q(&request, canonical.canonical_week_get(as_of).await)
        }
        "IncomePlanWeekGet" => {
            let as_of = jstr(&json, "asOfDate").unwrap_or_default();
            let pick = jstr(&json, "weekEnding")
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| as_of.clone());
            map_q(&request, income_plan_week_view(canonical, pick, as_of).await)
        }
        "IncomePlanGridGet" => {
            let as_of = jstr(&json, "asOfDate").unwrap_or_else(today_iso);
            let hist = json
                .get("historicalWeeks")
                .and_then(|v| v.as_i64())
                .unwrap_or(6);
            let fut = json
                .get("futureWeeks")
                .and_then(|v| v.as_i64())
                .unwrap_or(6);
            let accounts: Vec<String> = json
                .get("accounts")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let selected = jstr(&json, "weekEnding").unwrap_or_default();
            map_q(
                &request,
                income_plan_grid_view(
                    canonical,
                    as_of,
                    financial_domain::income_plan::clamp_week_count(hist),
                    financial_domain::income_plan::clamp_week_count(fut),
                    accounts,
                    selected,
                )
                .await,
            )
        }
        "IncomePlanExportGet" => map_q(&request, income_plan_export_view(canonical, &json).await),
        "DividendPerformanceGet" => {
            let as_of = jstr(&json, "asOfDate").unwrap_or_default();
            let range = jstr(&json, "range").unwrap_or_else(|| "ytd".into());
            map_q(
                &request,
                dividend_performance_view(canonical, as_of, range).await,
            )
        }
        "DashboardBurndownGet" => {
            let as_of = jstr(&json, "asOfDate").unwrap_or_default();
            map_q(&request, dashboard_burndown_view(canonical, as_of).await)
        }
        "HoldingsGet" => map_q(&request, holdings_view(canonical).await),
        "DataSummaryGet" => map_q(&request, data_summary_view(canonical).await),
        "CalculatorGet" => map_q(&request, calculator_view(canonical).await),
        "DeclarationHistoryGet" => {
            let as_of = jstr(&json, "asOfDate").unwrap_or_else(today_iso);
            let cadence = jstr(&json, "cadence").unwrap_or_else(|| "all".into());
            let start_on = jstr(&json, "startOn");
            let end_on = jstr(&json, "endOn");
            let week_count = json
                .get("weekCount")
                .and_then(|v| v.as_u64())
                .map(|n| n as u32);
            map_q(
                &request,
                declaration_history_view(
                    canonical,
                    &as_of,
                    &cadence,
                    start_on.as_deref(),
                    end_on.as_deref(),
                    week_count,
                )
                .await,
            )
        }
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
        "PriceQuoteList" => match juuid(&json, "securityId") {
            Some(id) => map_q(&request, canonical.price_quote_list(id).await),
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
        "CollectorSetGet" => {
            let as_of = jstr(&json, "asOfDate").unwrap_or_else(today_iso);
            map_q(&request, collector_fleet_grid(canonical, &as_of).await)
        }
        "ResearchGapsGet" => map_q(&request, research_gaps_get(canonical).await),
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
        "AccountValueHomeGet" => {
            let as_of = jstr(&json, "asOfDate").unwrap_or_else(today_local_iso);
            map_q(&request, account_value_home_view(canonical, &as_of).await)
        }
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
        "PositionResearchRefresh" => {
            position_research_refresh(platform, canonical, &request, &json).await
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
            Some(id) => {
                let posted = canonical.import_post(id).await;
                match posted {
                    Ok(batch) => {
                        let as_of = as_of_from_import_batch(canonical, id).await;
                        let _ = cash_dividend_coverage_refresh(canonical, &as_of).await;
                        map_c(&request, Ok(batch))
                    }
                    Err(err) => command_err(&request, &err.code),
                }
            }
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
        "WorkTicketSyncMisses" => map_c(&request, work_ticket_sync_misses(canonical).await),
        "WorkTicketFile" => match juuid(&json, "ticketId") {
            Some(id) => map_c(
                &request,
                work_ticket_file_on(
                    canonical,
                    id,
                    &jstr(&json, "note").or_else(|| jstr(&json, "ownerNote")).unwrap_or_default(),
                )
                .await,
            ),
            None => command_err(&request, "missing_ticket_id"),
        },
        "WorkTicketResolve" => match juuid(&json, "ticketId") {
            Some(ticket_id) => {
                let ticket = match canonical.work_ticket_get(ticket_id).await {
                    Ok(t) => t,
                    Err(err) => return command_err(&request, &err.code),
                };
                let Some(expected_tool) = financial_domain::work_ticket::tool_for_code(&ticket.code)
                else {
                    return command_err(&request, "ticket_missing_tool");
                };
                let tool = expected_tool.to_string();
                if tool == financial_domain::work_ticket::TOOL_RETRY_RETRIEVE {
                    let template = canonical
                        .retrieval_template_get(ticket.security_id)
                        .await
                        .ok()
                        .flatten();
                    let assigned = template
                        .as_ref()
                        .map(|t| t.declaration_source.clone())
                        .unwrap_or_default();
                    let retry_url = jstr(&json, "sourceUrl").unwrap_or_default();
                    if !retry_url.is_empty()
                        && !financial_domain::work_ticket::retry_url_matches_assigned_adapter(
                            &assigned,
                            &retry_url,
                        )
                    {
                        let mut t = ticket.clone();
                        t.reason = format!(
                            "Assigned adapter {assigned} refused URL {retry_url}. Adapter unchanged."
                        );
                        t.urls_tried = append_tried_url(&t.urls_tried, &retry_url);
                        t.last_seen_on = today_iso();
                        let t = match canonical.work_ticket_update(t).await {
                            Ok(row) => row,
                            Err(err) => return command_err(&request, &err.code),
                        };
                        return command_ok(
                            &request,
                            work_ticket_resolve_body(
                                &t,
                                false,
                                financial_domain::work_ticket::CODE_ADAPTER_URL_MISMATCH,
                                &t.reason,
                            ),
                        );
                    }
                    if !retry_url.is_empty() {
                        if let Err(err) =
                            persist_template_url_same_adapter(canonical, ticket.security_id, &retry_url)
                                .await
                        {
                            return command_err(&request, &err.code);
                        }
                    }
                    let mut t = ticket.clone();
                    t.urls_tried = append_tried_url(&t.urls_tried, &retry_url);
                    t.last_seen_on = today_iso();
                    if !retry_url.is_empty() {
                        t.reason = format!(
                            "Retry URL kept on assigned adapter {assigned}."
                        );
                    }
                    let t = match canonical.work_ticket_update(t).await {
                        Ok(row) => row,
                        Err(err) => return command_err(&request, &err.code),
                    };
                    return command_ok(
                        &request,
                        work_ticket_resolve_body(
                            &t,
                            true,
                            "",
                            "Retrying stored adapter URL.",
                        ),
                    );
                }
                if tool == financial_domain::work_ticket::TOOL_LOCK_CADENCE
                    || tool == financial_domain::work_ticket::TOOL_SET_DIV_TYPE
                    || tool == financial_domain::work_ticket::TOOL_SET_UNDERLYING
                {
                    let mut upsert = serde_json::json!({
                        "securityId": ticket.security_id,
                    });
                    if let Some(freq) = jstr(&json, "paymentFrequency") {
                        upsert["paymentFrequency"] = serde_json::json!(freq);
                    }
                    if let Some(div) = jstr(&json, "divType") {
                        upsert["divType"] = serde_json::json!(div);
                    }
                    if let Some(und) = jstr(&json, "underlying") {
                        upsert["underlying"] = serde_json::json!(und);
                    }
                    let nested = Box::pin(execute_command_on(
                        platform,
                        canonical,
                        CommandRequest {
                            contract_version: request.contract_version.clone(),
                            command_name: "PositionCharacteristicUpsert".into(),
                            correlation_id: request.correlation_id,
                            body_json: Some(upsert.to_string()),
                            expected_version: None,
                        },
                    ))
                    .await;
                    if !nested.ok {
                        return command_err(
                            &request,
                            nested
                                .error_code
                                .as_deref()
                                .unwrap_or("ticket_tool_noop"),
                        );
                    }
                    let latest = canonical
                        .work_ticket_get(ticket_id)
                        .await
                        .unwrap_or(ticket);
                    return command_ok(
                        &request,
                        work_ticket_resolve_body(&latest, true, "", "characteristic saved"),
                    );
                }
                if tool == financial_domain::work_ticket::TOOL_SET_INCEPTION {
                    let inception = jstr(&json, "inceptionOn").unwrap_or_default();
                    let Some(mut rec) = canonical
                        .retrieval_template_get(ticket.security_id)
                        .await
                        .ok()
                        .flatten()
                    else {
                        return command_err(&request, "not_found");
                    };
                    rec.inception_on = inception;
                    if let Err(err) = canonical.retrieval_template_set(rec).await {
                        return command_err(&request, &err.code);
                    }
                    let latest = canonical
                        .work_ticket_get(ticket_id)
                        .await
                        .unwrap_or(ticket);
                    return command_ok(
                        &request,
                        work_ticket_resolve_body(&latest, true, "", "inception saved"),
                    );
                }
                if tool == financial_domain::work_ticket::TOOL_ENTER_DECLARED_AMOUNT {
                    let period = jstr(&json, "paymentPeriod")
                        .or_else(|| payment_period_from_ticket_reason(&ticket.reason))
                        .unwrap_or_default();
                    if period.is_empty() {
                        return command_err(&request, "missing_payment_period");
                    }
                    let (minor, scale) = if let Some(m) = json
                        .get("amountPerShareMinor")
                        .and_then(|x| x.as_i64())
                        .filter(|a| *a > 0)
                    {
                        (m, ju8(&json, "amountScale", 4))
                    } else if let Some(raw) = jstr(&json, "amount").or_else(|| jstr(&json, "dollars"))
                    {
                        match parse_owner_per_unit_amount(&raw) {
                            Some(v) => v,
                            None => return command_err(&request, "missing_amount"),
                        }
                    } else {
                        return command_err(&request, "missing_amount");
                    };
                    if let Err(err) = canonical
                        .issuer_declaration_record(
                            ticket.security_id,
                            Some(minor),
                            scale,
                            period,
                            "owner".into(),
                            today_iso(),
                        )
                        .await
                    {
                        return command_err(&request, &err.code);
                    }
                    match work_ticket_file_on(
                        canonical,
                        ticket_id,
                        &jstr(&json, "note").unwrap_or_else(|| {
                            format!("owner declared {minor} scale {scale}")
                        }),
                    )
                    .await
                    {
                        Ok(t) => {
                            return command_ok(
                                &request,
                                work_ticket_resolve_body(&t, true, "", "declared amount saved"),
                            )
                        }
                        Err(err) => return command_err(&request, &err.code),
                    }
                }
                if tool == financial_domain::work_ticket::TOOL_AMOUNT_CONFIRM
                    || tool == financial_domain::work_ticket::TOOL_PLAN_VS_DECL
                {
                    let action = jstr(&json, "action").unwrap_or_default();
                    if action.eq_ignore_ascii_case("reject")
                        && tool == financial_domain::work_ticket::TOOL_AMOUNT_CONFIRM
                    {
                        if let Some(period) = payment_period_from_ticket_reason(&ticket.reason) {
                            let today = today_iso();
                            if let Err(err) = canonical
                                .issuer_declaration_record(
                                    ticket.security_id,
                                    None,
                                    2,
                                    period,
                                    "owner".into(),
                                    today,
                                )
                                .await
                            {
                                return command_err(&request, &err.code);
                            }
                        }
                        match work_ticket_file_on(
                            canonical,
                            ticket_id,
                            &jstr(&json, "note")
                                .unwrap_or_else(|| "rejected: vendor amount discarded".into()),
                        )
                        .await
                        {
                            Ok(t) => {
                                return command_ok(
                                    &request,
                                    work_ticket_resolve_body(&t, true, "", "rejected"),
                                )
                            }
                            Err(err) => return command_err(&request, &err.code),
                        }
                    }
                    let keep_plan = json
                        .get("keepPlan")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);
                    if !keep_plan {
                        return command_err(&request, "ticket_wrote_plan");
                    }
                    match work_ticket_file_on(
                        canonical,
                        ticket_id,
                        &jstr(&json, "note")
                            .unwrap_or_else(|| "excepted: vendor amount accepted".into()),
                    )
                    .await
                    {
                        Ok(t) => {
                            return command_ok(
                                &request,
                                work_ticket_resolve_body(&t, true, "", "excepted"),
                            )
                        }
                        Err(err) => return command_err(&request, &err.code),
                    }
                }
                if tool == financial_domain::work_ticket::TOOL_RUN_ROC {
                    let mut roc = json.clone();
                    roc["securityId"] = serde_json::json!(ticket.security_id);
                    roc["symbol"] = serde_json::json!(ticket.symbol);
                    let nested = Box::pin(execute_command_on(
                        platform,
                        canonical,
                        CommandRequest {
                            contract_version: request.contract_version.clone(),
                            command_name: "RocResearchRetrieve".into(),
                            correlation_id: request.correlation_id,
                            body_json: Some(roc.to_string()),
                            expected_version: None,
                        },
                    ))
                    .await;
                    let latest = canonical
                        .work_ticket_get(ticket_id)
                        .await
                        .unwrap_or(ticket);
                    return command_ok(
                        &request,
                        work_ticket_resolve_body(
                            &latest,
                            nested.ok,
                            nested.error_code.as_deref().unwrap_or(""),
                            "roc retrieve",
                        ),
                    );
                }
                command_err(&request, "ticket_missing_tool")
            }
            None => command_err(&request, "missing_ticket_id"),
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
                        let as_of = if actual.occurred_on.trim().is_empty() {
                            today_iso()
                        } else {
                            actual.occurred_on.clone()
                        };
                        let _ = cash_dividend_coverage_refresh(canonical, &as_of).await;
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
                let basis = match canonical.basis_get().await {
                    Ok(b) => b,
                    Err(err) => return command_err(&request, &err.code),
                };
                let already_has_lots = basis.lots.iter().any(|l| l.security_id == security_id);
                if !already_has_lots {
                    let symbol = canonical
                        .security_get(security_id)
                        .await
                        .map(|s| s.symbol)
                        .unwrap_or_default();
                    let as_of = today_iso();
                    let status = collector_status_for(canonical, security_id, &symbol, &as_of).await;
                    if !status.complete {
                        return command_err(&request, "collector_incomplete");
                    }
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
                let existing = canonical
                    .retrieval_template_get(security_id)
                    .await
                    .ok()
                    .flatten();
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
                            last_content_hash: jstr(&json, "lastContentHash")
                                .filter(|s| !s.trim().is_empty())
                                .or_else(|| existing.as_ref().map(|e| e.last_content_hash.clone()))
                                .unwrap_or_default(),
                            collector_enabled,
                            inception_on: jstr(&json, "inceptionOn")
                                .or_else(|| existing.as_ref().map(|e| e.inception_on.clone()))
                                .unwrap_or_default(),
                            roc_source_url: jstr(&json, "rocSourceUrl")
                                .or_else(|| existing.as_ref().map(|e| e.roc_source_url.clone()))
                                .unwrap_or_default(),
                            history_url_attempts: json
                                .get("historyUrlAttempts")
                                .and_then(|v| v.as_u64())
                                .map(|n| n as u8)
                                .or_else(|| existing.as_ref().map(|e| e.history_url_attempts))
                                .unwrap_or(0),
                        })
                        .await,
                )
            }
            None => command_err(&request, "missing_security_id"),
        },
        "CollectorFieldDecisionSet" => match juuid(&json, "securityId") {
            Some(security_id) => {
                let field = jstr(&json, "field").unwrap_or_default();
                let decision = jstr(&json, "decision").unwrap_or_default();
                let field_ok = financial_domain::collector::REQUIRED_FIELDS
                    .iter()
                    .any(|r| r.eq_ignore_ascii_case(&field))
                    || field.eq_ignore_ascii_case("backtest");
                let decision_ok = decision.eq_ignore_ascii_case("accept")
                    || decision.eq_ignore_ascii_case("skip");
                if field.trim().is_empty() || !field_ok {
                    command_err(&request, "invalid_field")
                } else if !decision_ok {
                    command_err(&request, "invalid_decision")
                } else {
                    let rec = CollectorFieldDecisionRecord {
                        security_id,
                        field: field.to_ascii_lowercase(),
                        decision: decision.to_ascii_lowercase(),
                        noted_on: today_iso(),
                    };
                    if rec.decision == "skip"
                        && !rec.field.eq_ignore_ascii_case("backtest")
                    {
                        let symbol = resolve_security(canonical, &json)
                            .await
                            .map(|s| s.symbol)
                            .unwrap_or_default();
                        if financial_domain::work_ticket::tool_for_code(&rec.field).is_some()
                        {
                            let _ = work_ticket_raise_or_bump(
                                canonical,
                                security_id,
                                &symbol,
                                &rec.field,
                                &format!("Owner skipped required field {}.", rec.field),
                                "",
                            )
                            .await;
                        }
                    }
                    map_c(
                        &request,
                        canonical.collector_field_decision_set(rec).await,
                    )
                }
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
                let replace_cadence = jbool(&json, "replaceCadence", false);
                let base_locked = financial_domain::calculator::PaymentCadence::parse(
                    &base.payment_frequency,
                )
                .and_then(financial_domain::calculator::PaymentCadence::periods)
                .is_some();
                let payment_frequency = if base_locked && !replace_cadence {
                    base.payment_frequency.clone()
                } else {
                    let freq_raw = if json.get("paymentFrequency").is_some() {
                        jstr(&json, "paymentFrequency").unwrap_or_default()
                    } else {
                        base.payment_frequency.clone()
                    };
                    match locked_cadence(&freq_raw) {
                        Ok(c) => c.label().to_string(),
                        Err(err) => return command_err(&request, &err.code),
                    }
                };
                let rec = PositionCharacteristicRecord {
                    security_id,
                    payment_frequency,
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
                };
                if rec.div_type.trim().is_empty()
                    && !financial_domain::calculator::is_non_paying(&rec.payment_frequency)
                {
                    let symbol = canonical
                        .security_get(security_id)
                        .await
                        .map(|s| s.symbol)
                        .unwrap_or_default();
                    let _ = work_ticket_raise_or_bump(
                        canonical,
                        security_id,
                        &symbol,
                        "div_type",
                        "DIV-1 (or CASH) is empty — owner must set it.",
                        "",
                    )
                    .await;
                }
                map_c(
                    &request,
                    canonical.position_characteristic_upsert(rec).await,
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
            let probes = json
                .get("rocProbes")
                .cloned()
                .unwrap_or(serde_json::json!([]));
            if let Some(security_id) = juuid(&json, "securityId") {
                let as_of = jstr(&json, "asOfDate").unwrap_or_else(today_iso);
                if let Err(err) =
                    persist_roc_candidates(canonical, security_id, &parsed, &as_of).await
                {
                    return command_err(&request, &err.code);
                }
                // Process A propose: mirror 19a-1 onto 2026 estimate without completing research.
                propose_characteristic_roc_estimate(canonical, security_id, &parsed).await;
                // Hit or miss: research still needs ROC (unknown ≠ 0%).
                mark_needs_roc_research(canonical, security_id).await;
                let hit = parsed.iter().any(|c| is_system_19a1(c));
                let probe_count = probes.as_array().map(|a| a.len() as u64).unwrap_or(0);
                persist_retrieve_run(
                    canonical,
                    security_id,
                    "roc-19a1",
                    &as_of,
                    hit,
                    if hit {
                        "roc_estimate"
                    } else {
                        "roc_retrieve_miss"
                    },
                    if hit {
                        "19a-1 estimate proposed"
                    } else {
                        "19a-1 miss — unknown, not 0%"
                    },
                    probe_count,
                    if hit { 1 } else { 0 },
                    if hit { 0 } else { probe_count },
                    0,
                    &serde_json::json!({
                        "symbol": jstr(&json, "symbol").unwrap_or_default(),
                        "declarationSource": jstr(&json, "declarationSource").unwrap_or_default(),
                        "sourceUrl": jstr(&json, "sourceUrl").unwrap_or_default(),
                        "candidates": candidates,
                        "probes": probes,
                    }),
                )
                .await;
            }
            command_ok(
                &request,
                serde_json::json!({
                    "candidates": candidates,
                    "rocProbes": probes,
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
        "AccountValueSnapshotRecord" => {
            let as_of = jstr(&json, "asOfDate").unwrap_or_else(today_local_iso);
            map_c(
                &request,
                record_account_value_snapshot(canonical, &as_of).await,
            )
        }
        "DataSnapshotExport" => {
            let as_of = jstr(&json, "asOfDate").unwrap_or_else(today_local_iso);
            map_c(
                &request,
                crate::data_snapshot::export_data_snapshot(platform, canonical, &as_of).await,
            )
        }
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
            let ran_at = run_stamp_local();
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
                if is_cash_rate_candidate(&decl) {
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
            let ran_at = run_stamp_local();
            let pay_dates = json
                .get("payDates")
                .and_then(|q| q.as_array())
                .cloned()
                .unwrap_or_default();
            for row in &pay_dates {
                capture_hash(row, &mut hashes);
            }
            let mut by_security: std::collections::HashMap<Uuid, Vec<Value>> =
                std::collections::HashMap::new();
            for row in pay_dates {
                let Some(security_id) = juuid(&row, "securityId") else {
                    continue;
                };
                by_security.entry(security_id).or_default().push(row);
            }
            let chars = canonical
                .position_characteristic_list()
                .await
                .unwrap_or_default();
            let as_of = jstr(&json, "asOfDate").unwrap_or_else(today_iso);
            for (security_id, rows) in by_security {
                if rows.is_empty() {
                    continue;
                }
                let symbol = canonical
                    .security_get(security_id)
                    .await
                    .map(|s| s.symbol)
                    .unwrap_or_default();
                let freq = chars
                    .iter()
                    .find(|c| c.security_id == security_id)
                    .map(|c| c.payment_frequency.clone())
                    .unwrap_or_default();
                let tmpl = canonical
                    .retrieval_template_get(security_id)
                    .await
                    .ok()
                    .flatten();
                let src = tmpl
                    .as_ref()
                    .map(|t| t.declaration_source.as_str())
                    .unwrap_or("");
                let decls = json
                    .get("declarations")
                    .or_else(|| json.get("candidates"))
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|d| juuid(d, "securityId") == Some(security_id))
                    .collect::<Vec<_>>();
                let wrote = if financial_domain::mlp_sec::is_adapter_kind(src) {
                    apply_mlp_sec_8k_payables(
                        canonical,
                        security_id,
                        &symbol,
                        &as_of,
                        &decls,
                        &rows,
                        &ran_at,
                    )
                    .await
                } else {
                    apply_runtime_vendor_payables(
                        canonical,
                        security_id,
                        &symbol,
                        &as_of,
                        &freq,
                        &rows,
                        &ran_at,
                    )
                    .await
                };
                if wrote > 0 {
                    ok_ids.insert(security_id);
                }
            }
            for security_id in &ok_ids {
                let hash = hashes.get(security_id).cloned().unwrap_or_default();
                let source_url = fetched_source_url_for(&json, Some(*security_id));
                let tmpl = canonical
                    .retrieval_template_get(*security_id)
                    .await
                    .ok()
                    .flatten();
                let src = tmpl
                    .as_ref()
                    .map(|t| t.declaration_source.as_str())
                    .unwrap_or("");
                if financial_domain::mlp_sec::is_adapter_kind(src)
                    || financial_domain::mlp_sec::routes_fetch(src, Some(source_url.as_str()))
                    || tmpl
                        .as_ref()
                        .is_some_and(|t| {
                            financial_domain::mlp_sec::routes_fetch(
                                &t.declaration_source,
                                Some(t.source_url.as_str()),
                            )
                        })
                {
                    let as_of = jstr(&json, "asOfDate").unwrap_or_else(today_iso);
                    let _ = persist_mlp_sec_last_run(
                        canonical,
                        *security_id,
                        &as_of,
                        "page",
                        ran_at.clone(),
                        hash,
                    )
                    .await;
                } else {
                    let persist_url = if financial_domain::mlp_sec::is_ir_url(&source_url) {
                        String::new()
                    } else {
                        source_url
                    };
                    let _ = canonical
                        .retrieval_template_touch_run(
                            *security_id,
                            true,
                            "declaration retrieve recorded".into(),
                            ran_at.clone(),
                            hash,
                            &persist_url,
                        )
                        .await;
                }
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
                let source = existing
                    .as_ref()
                    .map(|t| t.declaration_source.as_str())
                    .unwrap_or("");
                let lookback_owner_accepted = canonical
                    .collector_field_decision_list(security_id)
                    .await
                    .unwrap_or_default()
                    .iter()
                    .any(|d| {
                        d.field.eq_ignore_ascii_case("paid_history")
                            && d.decision.eq_ignore_ascii_case("accept")
                    });
                let gate = declaration_retrieve_gate(
                    paid_count,
                    inception_on,
                    payment_frequency,
                    declaration_lookback_applies(div_type, source, ""),
                    lookback_owner_accepted,
                );
                let message = if gate.message.is_empty() {
                    "declaration retrieve unchanged".into()
                } else {
                    gate.message
                };
                let _ = canonical
                    .retrieval_template_touch_run(
                        security_id,
                        true,
                        message.clone(),
                        ran_at.clone(),
                        hash,
                        "",
                    )
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
                let as_of = jstr(&json, "asOfDate").unwrap_or_else(today_iso);
                let verify_issues =
                    declaration_store_verify_issues(&decls, &stored_declarations, &as_of);
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
            let mut cash_ids: std::collections::HashSet<Uuid> = ok_ids.clone();
            if let Some(decls) = json.get("declarations").and_then(|q| q.as_array()) {
                for row in decls {
                    if let Some(id) = juuid(row, "securityId") {
                        cash_ids.insert(id);
                    }
                }
            }
            for security_id in &cash_ids {
                let decls = declarations_for_security(&json, *security_id);
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
            let miss_ids: std::collections::HashSet<Uuid> = misses
                .iter()
                .filter_map(|m| juuid(m, "securityId"))
                .collect();
            let mut leftover_ids: std::collections::HashSet<Uuid> =
                std::collections::HashSet::new();
            if let Some(decls) = json.get("declarations").and_then(|q| q.as_array()) {
                for row in decls {
                    capture_hash(row, &mut hashes);
                    if let Some(id) = juuid(row, "securityId") {
                        leftover_ids.insert(id);
                    }
                }
            }
            if let Some(pays) = json.get("payDates").and_then(|q| q.as_array()) {
                for row in pays {
                    capture_hash(row, &mut hashes);
                    if let Some(id) = juuid(row, "securityId") {
                        leftover_ids.insert(id);
                    }
                }
            }
            leftover_ids.retain(|id| {
                !ok_ids.contains(id)
                    && !miss_ids.contains(id)
                    && unchanged.iter().all(|row| juuid(row, "securityId") != Some(*id))
            });
            for security_id in leftover_ids {
                let hash = hashes.get(&security_id).cloned().unwrap_or_default();
                let _ = canonical
                    .retrieval_template_touch_run(
                        security_id,
                        true,
                        "declaration retrieve recorded".into(),
                        ran_at.clone(),
                        hash,
                        &fetched_source_url_for(&json, Some(security_id)),
                    )
                    .await;
                persist_retrieve_run(
                    canonical,
                    security_id,
                    "declaration",
                    &ran_at,
                    true,
                    "",
                    "declaration retrieve recorded",
                    1,
                    0,
                    0,
                    1,
                    &serde_json::json!({
                        "declarations": declarations_for_security(&json, security_id),
                        "payDates": pay_dates_for_security(&json, security_id),
                    }),
                )
                .await;
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
        "CollectorAlignFutureToPlan" => {
            let as_of = jstr(&json, "asOfDate").unwrap_or_else(today_iso);
            let set = canonical
                .collector_set()
                .await
                .unwrap_or(CollectorSetBody { items: Vec::new() });
            let only = juuid(&json, "securityId");
            let mut aligned = 0u64;
            let mut symbols = 0u64;
            for item in set.items {
                if let Some(sid) = only {
                    if item.security_id != sid {
                        continue;
                    }
                }
                if !item.collector_enabled {
                    continue;
                }
                symbols = symbols.saturating_add(1);
                aligned = aligned.saturating_add(
                    align_unoccurred_declarations_to_plan(canonical, item.security_id, &as_of)
                        .await,
                );
            }
            command_ok(
                &request,
                serde_json::json!({ "asOfDate": as_of, "symbols": symbols, "superseded": aligned })
                    .to_string(),
            )
        }
        "CollectorRetrieve" => match juuid(&json, "securityId") {
            Some(security_id) => {
                let symbol = jstr(&json, "symbol").unwrap_or_default();
                let ran_at = run_stamp_local();
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
                let mut source_url = fetched_source_url_for(
                    &json,
                    Some(security_id),
                );

                let declaration_source = jstr(&json, "declarationSource").unwrap_or_default();
                let characteristics = canonical
                    .position_characteristic_list()
                    .await
                    .unwrap_or_default();
                let ch = characteristics.iter().find(|c| c.security_id == security_id);
                let mut div_type = jstr(&json, "divType").unwrap_or_default();
                if div_type.is_empty() {
                    div_type = ch.map(|c| c.div_type.clone()).unwrap_or_default();
                }
                let existing = canonical
                    .retrieval_template_get(security_id)
                    .await
                    .ok()
                    .flatten();
                let mut inception_on = jstr(&json, "inceptionOn").unwrap_or_default();
                if inception_on.is_empty() {
                    inception_on = existing
                        .as_ref()
                        .map(|t| t.inception_on.clone())
                        .unwrap_or_default();
                }
                let mut payment_frequency = jstr(&json, "paymentFrequency").unwrap_or_default();
                if payment_frequency.is_empty() {
                    payment_frequency = ch.map(|c| c.payment_frequency.clone()).unwrap_or_default();
                }
                let is_unchanged = json
                    .get("unchanged")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if is_unchanged {
                    unchanged = 1;
                    message = "declaration retrieve unchanged".into();
                }

                let open_tickets = canonical
                    .work_ticket_list(Some(security_id), Some("open".into()))
                    .await
                    .unwrap_or_default();
                let has_miss_ticket = open_tickets.iter().any(|t| {
                    financial_domain::work_ticket::is_retrieve_failure_code(&t.code)
                });
                let stored_url = existing
                    .as_ref()
                    .map(|t| t.source_url.as_str())
                    .unwrap_or("");
                let last_ok = existing.as_ref().and_then(|t| t.last_run_ok);
                let seed_blocked = !is_unchanged
                    && financial_domain::collector::needs_owner_seed_url(
                        &div_type,
                        &symbol,
                        stored_url,
                        last_ok,
                        has_miss_ticket,
                    );
                if seed_blocked {
                    ok = false;
                    code = financial_domain::work_ticket::CODE_MISSING_SEED_URL.into();
                    message = "Owner must paste an issuer declaration URL. Probe-only is blocked."
                        .into();
                }

                let declarations = if seed_blocked {
                    Vec::new()
                } else {
                    json
                        .get("candidates")
                        .or_else(|| json.get("declarations"))
                        .and_then(|q| q.as_array())
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|d| {
                            let url = jstr(d, "sourceUrl")
                                .or_else(|| jstr(d, "url"))
                                .unwrap_or_default();
                            url.is_empty()
                                || !financial_domain::div1::is_third_party_declaration_url(&url)
                        })
                        .collect::<Vec<_>>()
                };
                let as_of = jstr(&json, "asOfDate").unwrap_or_else(today_iso);
                let leftover_src = json
                    .get("pagePaid")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_else(|| declarations.clone());
                let _ = align_unoccurred_declarations_to_plan(canonical, security_id, &as_of).await;
                let _ = apply_leftover_ex_to_payable(
                    canonical,
                    security_id,
                    &leftover_src,
                    &ran_at,
                )
                .await;
                let stored_before = canonical
                    .issuer_declaration_list(security_id)
                    .await
                    .unwrap_or_default();
                let pay_refs: Vec<String> = canonical
                    .issuer_pay_date_list(security_id)
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .map(|p| p.pay_on)
                    .collect();
                let pay_ref_strs: Vec<&str> = pay_refs.iter().map(String::as_str).collect();
                let occurred_amts: Vec<(i64, u8)> = stored_before
                    .iter()
                    .filter(|d| {
                        d.amount_per_share_minor.unwrap_or(0) > 0
                            && financial_domain::schedule::period_has_occurred(
                                &d.payment_period,
                                &as_of,
                            )
                    })
                    .filter_map(|d| d.amount_per_share_minor.map(|a| (a, d.amount_scale)))
                    .collect();
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
                    if is_cash_rate_candidate(decl) {
                        skipped += 1;
                        continue;
                    }
                    let period = jstr(decl, "paymentPeriod").unwrap_or_default();
                    if !financial_domain::mlp_sec::is_adapter_kind(&declaration_source)
                        && financial_domain::schedule::unoccurred_declaration_is_placeholder(
                            &period,
                            amount.unwrap_or(0),
                            ju8(decl, "amountScale", 2),
                            &as_of,
                            &occurred_amts,
                            &pay_ref_strs,
                            &jstr(decl, "enteredAt").unwrap_or_else(today_iso),
                        )
                    {
                        skipped += 1;
                        continue;
                    }
                    let period_key = payment_period_key(&period);
                    let already_paid = stored_before.iter().any(|d| {
                        payment_period_key(&d.payment_period) == period_key
                            && d.amount_per_share_minor.unwrap_or(0) > 0
                    });
                    if already_paid {
                        if financial_domain::mlp_sec::is_adapter_kind(&declaration_source) {
                            if let Some(old) = stored_before.iter().find(|d| {
                                payment_period_key(&d.payment_period) == period_key
                                    && d.amount_per_share_minor.unwrap_or(0) > 0
                            }) {
                                let new_amt = amount.unwrap_or(0);
                                let new_scale = ju8(decl, "amountScale", 2);
                                if !financial_domain::money::amounts_equal(
                                    old.amount_per_share_minor.unwrap_or(0),
                                    old.amount_scale,
                                    new_amt,
                                    new_scale,
                                ) {
                                    let _ = work_ticket_raise_or_bump(
                                        canonical,
                                        security_id,
                                        &symbol,
                                        "declaration_amount_variation",
                                        &format!(
                                            "Paid row {} amount would change; 8-K not applied.",
                                            old.payment_period
                                        ),
                                        "",
                                    )
                                    .await;
                                } else if !old
                                    .source
                                    .eq_ignore_ascii_case(financial_domain::mlp_sec::SOURCE_SEC_8K)
                                {
                                    if canonical
                                        .issuer_declaration_record(
                                            security_id,
                                            old.amount_per_share_minor,
                                            old.amount_scale,
                                            old.payment_period.clone(),
                                            financial_domain::mlp_sec::SOURCE_SEC_8K.into(),
                                            ran_at.clone(),
                                        )
                                        .await
                                        .is_ok()
                                    {
                                        recorded = recorded.saturating_add(1);
                                    }
                                }
                            }
                        }
                        unchanged = unchanged.saturating_add(1);
                        continue;
                    }
                    match canonical
                        .issuer_declaration_record(
                            security_id,
                            amount,
                            ju8(decl, "amountScale", 2),
                            period,
                            jstr(decl, "source").unwrap_or_else(|| "issuer".into()),
                            jstr(decl, "enteredAt").unwrap_or_else(|| ran_at.clone()),
                        )
                        .await
                    {
                        Ok(_) => recorded += 1,
                        Err(_) => skipped += 1,
                    }
                }

                let pay_dates = if seed_blocked {
                    Vec::new()
                } else {
                    json
                        .get("upcomingPays")
                        .or_else(|| json.get("payDates"))
                        .and_then(|q| q.as_array())
                        .cloned()
                        .unwrap_or_default()
                };
                let _ = canonical.issuer_pay_date_dedupe(security_id).await;
                if financial_domain::mlp_sec::is_adapter_kind(&declaration_source) {
                    let as_of = jstr(&json, "asOfDate").unwrap_or_else(today_iso);
                    let wrote = apply_mlp_sec_8k_payables(
                        canonical,
                        security_id,
                        &symbol,
                        &as_of,
                        &declarations,
                        &pay_dates,
                        &ran_at,
                    )
                    .await;
                    recorded = recorded.saturating_add(wrote);
                } else if !pay_dates.is_empty() {
                    let as_of = jstr(&json, "asOfDate").unwrap_or_else(today_iso);
                    let wrote = apply_runtime_vendor_payables(
                        canonical,
                        security_id,
                        &symbol,
                        &as_of,
                        &payment_frequency,
                        &pay_dates,
                        &ran_at,
                    )
                    .await;
                    recorded = recorded.saturating_add(wrote);
                }

                let cash_posted = if seed_blocked {
                    0
                } else {
                    apply_cash_moneymarket_followups(
                        canonical,
                        security_id,
                        &declarations,
                        &jstr(&json, "declarationSource").unwrap_or_else(|| "issuer".into()),
                        &ran_at,
                    )
                    .await
                };
                recorded = recorded.saturating_add(cash_posted);

                let misses = if seed_blocked {
                    Vec::new()
                } else {
                    json
                        .get("misses")
                        .and_then(|m| m.as_array())
                        .cloned()
                        .unwrap_or_default()
                };
                if !misses.is_empty() {
                    ok = false;
                    code = jstr(&misses[0], "code")
                        .unwrap_or_else(|| "declaration_retrieve_miss".into());
                    message = jstr(&misses[0], "reason")
                        .unwrap_or_else(|| "retrieve miss".into());
                    let _ = raise_retrieve_misses(canonical, &misses, &ran_at).await;
                    skipped = skipped.saturating_add(misses.len() as u64);
                }

                let stored_declarations = canonical
                    .issuer_declaration_list(security_id)
                    .await
                    .unwrap_or_default();
                let page_paid = json
                    .get("pagePaid")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                let mut post_issues = Vec::new();
                if !is_unchanged && !seed_blocked {
                    let as_of = jstr(&json, "asOfDate").unwrap_or_else(today_iso);
                    let verify_issues = declaration_store_verify_issues(
                        &declarations,
                        &stored_declarations,
                        &as_of,
                    );
                    for msg in verify_issues {
                        if msg.starts_with("amount mismatch")
                            || msg.starts_with("missing stored")
                        {
                            continue;
                        }
                        post_issues.push(("declaration_stored_mismatch".into(), msg));
                    }
                    post_issues.extend(declaration_post_check_issues(
                        &stored_before,
                        &stored_declarations,
                        &declarations,
                        &page_paid,
                        &payment_frequency,
                        count_paid_declarations(&stored_before),
                    ));
                }
                if !post_issues.is_empty() {
                    let blocking: Vec<_> = post_issues
                        .iter()
                        .filter(|(c, _)| {
                            !financial_domain::work_ticket::is_amount_confirm_code(c)
                        })
                        .cloned()
                        .collect();
                    let extra = post_issues
                        .iter()
                        .map(|(_, m)| m.as_str())
                        .collect::<Vec<_>>()
                        .join("; ");
                    if !blocking.is_empty() {
                        ok = false;
                        if code.is_empty() {
                            code = blocking[0].0.clone();
                        }
                    } else if code.is_empty() {
                        code = post_issues[0].0.clone();
                    }
                    if message.is_empty() {
                        message = extra;
                    } else {
                        message = format!("{message}; {extra}");
                    }
                }
                let paid_count = count_paid_declarations(&stored_declarations);
                let apply_lookback = declaration_lookback_applies(
                    &div_type,
                    &declaration_source,
                    &symbol,
                );
                let lookback_owner_accepted = canonical
                    .collector_field_decision_list(security_id)
                    .await
                    .unwrap_or_default()
                    .iter()
                    .any(|d| {
                        d.field.eq_ignore_ascii_case("paid_history")
                            && d.decision.eq_ignore_ascii_case("accept")
                    });
                let gate = declaration_retrieve_gate(
                    paid_count,
                    &inception_on,
                    &payment_frequency,
                    apply_lookback && !is_unchanged,
                    lookback_owner_accepted,
                );
                if !gate.ok {
                    ok = false;
                    if code.is_empty() {
                        code = gate.code.clone();
                    }
                    if message.is_empty() {
                        message = gate.message.clone();
                    } else if !gate.message.is_empty() {
                        message = format!("{message}; {}", gate.message);
                    }
                } else if ok && apply_lookback && !is_unchanged && message.is_empty() {
                    message = gate.message;
                }
                if ok
                    && post_issues
                        .iter()
                        .any(|(c, _)| financial_domain::work_ticket::is_amount_confirm_code(c))
                {
                    let first = post_issues
                        .iter()
                        .find(|(c, _)| financial_domain::work_ticket::is_amount_confirm_code(c))
                        .map(|(_, m)| m.as_str())
                        .unwrap_or("amount variation");
                    message = format!(
                        "collector retrieve recorded — Except or Reject amount variation ({first})"
                    );
                }
                let post_checks = serde_json::json!({
                    "c5Match": post_issues.iter().all(|(c, _)| c != "declaration_stored_mismatch"),
                    "previousAligned": post_issues.iter().all(|(c, _)| c != "declaration_history_dropped"),
                    "cadenceOk": post_issues.iter().all(|(c, _)| c != "declaration_cadence_mismatch"),
                    "amountVariationOk": post_issues.iter().all(|(c, _)| c != "declaration_amount_variation"),
                    "lookbackOk": gate.ok,
                    "amountVariationPct": financial_domain::declaration_post::AMOUNT_VARIATION_PCT,
                    "issues": post_issues.iter().map(|(c, m)| serde_json::json!({"code": c, "message": m})).collect::<Vec<_>>(),
                });
                let mlp_kind = financial_domain::mlp_sec::is_adapter_kind(&declaration_source)
                    || financial_domain::mlp_sec::routes_fetch(
                        &declaration_source,
                        Some(source_url.as_str()),
                    )
                    || financial_domain::mlp_sec::routes_fetch(
                        existing
                            .as_ref()
                            .map(|t| t.declaration_source.as_str())
                            .unwrap_or(""),
                        existing.as_ref().map(|t| t.source_url.as_str()),
                    );
                if mlp_kind {
                    let as_of_mlp = jstr(&json, "asOfDate").unwrap_or_else(today_iso);
                    let outcome = if misses.iter().any(|m| {
                        jstr(m, "code").as_deref() == Some(financial_domain::mlp_sec::CODE_SEC_403)
                            || jstr(m, "reason")
                                .unwrap_or_default()
                                .contains("sec_403")
                    }) {
                        "sec_403"
                    } else if declarations.iter().any(|d| {
                        jstr(d, "source").as_deref()
                            == Some(financial_domain::mlp_sec::SOURCE_SEC_8K)
                            && d.get("amountPerShareMinor")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0)
                                > 0
                    }) || leftover_src.iter().any(|d| {
                        d.get("amountPerShareMinor")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0)
                            > 0
                    }) {
                        "page"
                    } else {
                        "empty"
                    };
                    let (kind_ok, stamp, persist_url) = persist_mlp_sec_last_run(
                        canonical,
                        security_id,
                        &as_of_mlp,
                        outcome,
                        ran_at.clone(),
                        hash.clone(),
                    )
                    .await;
                    source_url = persist_url;
                    message = stamp;
                    ok = kind_ok;
                    if message.ends_with(":need_owner") {
                        let stored_now = canonical
                            .issuer_declaration_list(security_id)
                            .await
                            .unwrap_or_default();
                        let paid_refs: Vec<&str> = stored_now
                            .iter()
                            .filter(|d| d.amount_per_share_minor.unwrap_or(0) > 0)
                            .map(|d| d.payment_period.as_str())
                            .collect();
                        let needed = financial_domain::schedule::derive_quarterly_template_pay_ons(
                            &as_of_mlp,
                            &paid_refs,
                        );
                        if let Some(pay) = needed.first() {
                            let _ = work_ticket_raise_or_bump(
                                canonical,
                                security_id,
                                &symbol,
                                financial_domain::mlp_sec::CODE_OWNER_AMOUNT,
                                &financial_domain::mlp_sec::owner_amount_reason(pay),
                                &source_url,
                            )
                            .await;
                        }
                    }
                }
                if !mlp_kind && (apply_lookback || recorded > 0 || !post_issues.is_empty() || seed_blocked) {
                    if message.is_empty() {
                        message = if ok {
                            "collector retrieve recorded".into()
                        } else {
                            "collector retrieve miss".into()
                        };
                    }
                    let _ = canonical
                        .retrieval_template_touch_run(
                            security_id,
                            ok,
                            message.clone(),
                            ran_at.clone(),
                            hash.clone(),
                            &source_url,
                        )
                        .await;
                }

                let attempted = recorded + skipped + unchanged;
                let run_id = Uuid::new_v4();
                let mut payload = json.clone();
                payload["postChecks"] = post_checks;
                let payload_json = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into());
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
                if ok {
                    work_ticket_auto_file_declaration(canonical, security_id, run_id).await;
                }
                close_stale_expected_4_remaining_year(
                    canonical,
                    security_id,
                    &symbol,
                    &jstr(&json, "asOfDate").unwrap_or_else(today_iso),
                )
                .await;
                if !is_unchanged {
                    if !ok && !code.is_empty() {
                        let ticket_url = source_url.as_str();
                        let _ = work_ticket_raise_or_bump(
                            canonical,
                            security_id,
                            &symbol,
                            &code,
                            &message,
                            ticket_url,
                        )
                        .await;
                    }
                    let ticket_url = source_url.as_str();
                    for (c, m) in &post_issues {
                        if financial_domain::work_ticket::is_retrieve_failure_code(c) && ok {
                            continue;
                        }
                        if !ok && *c == code {
                            continue;
                        }
                        let _ = work_ticket_raise_or_bump(
                            canonical,
                            security_id,
                            &symbol,
                            c,
                            m,
                            ticket_url,
                        )
                        .await;
                    }
                }
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
