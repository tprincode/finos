use std::path::PathBuf;
use std::sync::Arc;

use ai_gateway::GrokAdvisory;
use application_core::contracts::{
    CommandRequest, CommandResult, QueryRequest, QueryResult, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::ports::advisory::{Advisory, MissingKeyAdvisory};
use application_core::ports::canonical::Canonical;
use application_core::queries::{execute_command_on, execute_query_on};
use storage_sqlite::LocalPlatform;
use tauri::menu::{MenuBuilder, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

const POSITION_RESEARCH_PROGRESS_EVENT: &str = "position-research-progress";
const DECLARATION_REFRESH_PROGRESS_EVENT: &str = "declaration-refresh-progress";
const POSITION_RESEARCH_TOTAL_STEPS: u32 = 4;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PositionResearchProgressPayload {
    step: u32,
    total: u32,
    label: String,
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DeclarationRefreshProgressPayload {
    current: u32,
    total: u32,
    symbol: String,
}

fn emit_declaration_refresh_progress(app: &AppHandle, current: u32, total: u32, symbol: &str) {
    let _ = app.emit(
        DECLARATION_REFRESH_PROGRESS_EVENT,
        DeclarationRefreshProgressPayload {
            current,
            total,
            symbol: symbol.to_string(),
        },
    );
}

fn merge_declaration_collect(
    into: &mut import_engine::DeclarationCollectOutcome,
    add: import_engine::DeclarationCollectOutcome,
) {
    into.candidates.extend(add.candidates);
    into.pay_dates.extend(add.pay_dates);
    into.misses.extend(add.misses);
    into.unchanged.extend(add.unchanged);
    if !add.fetched_source_url.is_empty() {
        into.fetched_source_url = add.fetched_source_url;
    }
}

fn emit_position_research_progress(app: &AppHandle, step: u32, label: &str) {
    let _ = app.emit(
        POSITION_RESEARCH_PROGRESS_EVENT,
        PositionResearchProgressPayload {
            step,
            total: POSITION_RESEARCH_TOTAL_STEPS,
            label: label.to_string(),
        },
    );
}

#[tauri::command]
async fn finance_query(
    platform: State<'_, Arc<LocalPlatform>>,
    request: QueryRequest,
) -> Result<QueryResult, String> {
    Ok(execute_query_on(platform.inner().as_ref(), platform.inner().as_ref(), request).await)
}

#[tauri::command]
async fn finance_command(
    app: AppHandle,
    platform: State<'_, Arc<LocalPlatform>>,
    mut request: CommandRequest,
) -> Result<CommandResult, String> {
    if matches!(
        request.command_name.as_str(),
        "PriceQuoteRetrieve"
            | "DeclarationRetrieve"
            | "MarketRetrieve"
            | "PeriodSeriesRetrieve"
            | "RocResearchRetrieve"
            | "CollectorRetrieve"
    ) {
        fill_declaration_source_from_template(platform.inner().as_ref(), &mut request).await;
        let name = request.command_name.clone();
        let mut body: serde_json::Value = request
            .body_json
            .as_deref()
            .and_then(|raw| serde_json::from_str(raw).ok())
            .unwrap_or_else(|| serde_json::json!({}));
        if name == "CollectorRetrieve" {
            fill_collector_retrieve(platform.inner().as_ref(), &mut body).await;
            request.body_json = Some(body.to_string());
        } else {
            let filled = tauri::async_runtime::spawn_blocking(move || {
                import_engine::enrich_retrieve_body(&name, &mut body);
                body
            })
            .await
            .map_err(|e| e.to_string())?;
            request.body_json = Some(filled.to_string());
        }
    }
    if request.command_name == "LastPriceRefresh" {
        fill_last_price_refresh(platform.inner().as_ref(), &mut request).await;
    }
    if request.command_name == "DeclarationRefresh" {
        fill_declaration_refresh(&app, platform.inner().as_ref(), &mut request).await;
    }
    if request.command_name == "PositionResearchRefresh" {
        return Ok(run_position_research_refresh(&app, platform.inner().as_ref(), request).await);
    }
    if request.command_name == "PositionResearchSeed" {
        return Ok(run_position_research_seed(&app, platform.inner().as_ref(), request).await);
    }
    Ok(execute_command_on(platform.inner().as_ref(), platform.inner().as_ref(), request).await)
}

fn qry(name: &str, body: serde_json::Value) -> QueryRequest {
    QueryRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        query_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: Some(body.to_string()),
    }
}

fn cmd(name: &str, body: serde_json::Value) -> CommandRequest {
    CommandRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        command_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: Some(body.to_string()),
        expected_version: None,
    }
}

/// Process A desktop path: seed identity+URL template, then live PositionResearchRefresh.
/// Injected candidates/misses/quotes (tests) skip live fill and use nested refresh.
/// Emits `position-research-progress` for the Research activity indicator (steps 1–3 of 4).
async fn run_position_research_seed(
    app: &AppHandle,
    platform: &LocalPlatform,
    mut request: CommandRequest,
) -> CommandResult {
    let body: serde_json::Value = request
        .body_json
        .as_deref()
        .and_then(|raw| serde_json::from_str(raw).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    let injected = body.get("candidates").is_some()
        || body.get("declarations").is_some()
        || body.get("misses").is_some()
        || body.get("payDates").is_some()
        || body.get("upcomingPays").is_some()
        || body.get("quote").is_some()
        || body.get("quotes").is_some()
        || body.get("rocCandidates").is_some();
    let mut merged = body;
    // Live issuer page → name / provider / underlying / lookthrough before seed persists them.
    if !injected {
        let symbol = merged
            .get("symbol")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let source_url = merged
            .get("sourceUrl")
            .or_else(|| merged.get("distributionUrl"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let declaration_source = merged
            .get("declarationSource")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if !symbol.is_empty() && !source_url.is_empty() {
            let profile = tauri::async_runtime::spawn_blocking({
                let symbol = symbol.clone();
                let source_url = source_url.clone();
                let declaration_source = declaration_source.clone();
                move || {
                    import_engine::live_research_identity(
                        &symbol,
                        &source_url,
                        &declaration_source,
                    )
                }
            })
            .await
            .unwrap_or_else(|_| serde_json::json!({}));
            merge_research_identity_into(&mut merged, &profile);
        }
    }
    if injected {
        request.body_json = Some(merged.to_string());
        return execute_command_on(platform, platform, request).await;
    }

    // Live path: write template only, then shared PositionResearchRefresh with enrichers.
    let mut refresh_carry = serde_json::json!({});
    for key in ["name", "provider", "suggestedProvider", "underlying", "lookthrough"] {
        if let Some(v) = merged.get(key) {
            refresh_carry[key] = v.clone();
        }
    }
    merged["skipRefresh"] = serde_json::json!(true);
    request.body_json = Some(merged.to_string());
    emit_position_research_progress(app, 1, "retrieving declarations");
    let seed_result = execute_command_on(platform, platform, request).await;
    if !seed_result.ok {
        return seed_result;
    }
    let seed: serde_json::Value =
        serde_json::from_str(seed_result.body_json.as_deref().unwrap_or("{}")).unwrap_or_default();
    let security_id = seed
        .get("securityId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let symbol = seed
        .get("symbol")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if security_id.is_empty() || symbol.is_empty() {
        return seed_result;
    }
    let mut refresh_body = refresh_carry;
    refresh_body["securityId"] = serde_json::json!(security_id);
    refresh_body["symbol"] = serde_json::json!(symbol);
    refresh_body["declarationSource"] = seed
        .get("declarationSource")
        .cloned()
        .unwrap_or(serde_json::json!(""));
    refresh_body["sourceUrl"] = seed
        .get("sourceUrl")
        .cloned()
        .unwrap_or(serde_json::json!(""));
    let refresh = run_position_research_refresh(
        app,
        platform,
        cmd("PositionResearchRefresh", refresh_body),
    )
    .await;
    if !refresh.ok {
        return refresh;
    }
    let refreshed: serde_json::Value =
        serde_json::from_str(refresh.body_json.as_deref().unwrap_or("{}")).unwrap_or_default();
    let mut out = seed;
    for key in [
        "paymentFrequency",
        "retrieveOk",
        "retrieveCode",
        "retrieveMessage",
        "rocPctMinor",
        "rocScale",
        "rocSourceUrl",
        "rocMethod",
        "rocKind",
        "rocAsOf",
        "rocEstablishedHow",
        "rocComplete",
        "rocProbes",
    ] {
        if let Some(v) = refreshed.get(key) {
            out[key] = v.clone();
        }
    }
    out["rocComplete"] = serde_json::json!(false);
    CommandResult {
        body_json: Some(out.to_string()),
        ..seed_result
    }
}

fn merge_research_identity_into(merged: &mut serde_json::Value, profile: &serde_json::Value) {
    for key in [
        "name",
        "suggestedProvider",
        "provider",
        "underlying",
        "lookthrough",
    ] {
        if let Some(v) = profile.get(key) {
            if key == "suggestedProvider" {
                if merged
                    .get("provider")
                    .and_then(|p| p.as_str())
                    .unwrap_or("")
                    .is_empty()
                    && v.as_str().map(|s| !s.is_empty()).unwrap_or(false)
                {
                    merged["provider"] = v.clone();
                }
            } else if merged.get(key).is_none()
                || merged
                    .get(key)
                    .and_then(|p| p.as_str())
                    .map(|s| s.is_empty())
                    .unwrap_or(false)
            {
                merged[key] = v.clone();
            }
        }
    }
    if merged
        .get("provider")
        .and_then(|p| p.as_str())
        .unwrap_or("")
        .is_empty()
    {
        if let Some(v) = profile.get("suggestedProvider") {
            merged["provider"] = v.clone();
        }
    }
}

/// Shared live research hole-fill (Complete research / Fill research gaps / Process A after seed).
async fn run_position_research_refresh(
    app: &AppHandle,
    platform: &LocalPlatform,
    mut request: CommandRequest,
) -> CommandResult {
    let body: serde_json::Value = request
        .body_json
        .as_deref()
        .and_then(|raw| serde_json::from_str(raw).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    let injected = body.get("candidates").is_some()
        || body.get("declarations").is_some()
        || body.get("misses").is_some()
        || body.get("payDates").is_some()
        || body.get("upcomingPays").is_some()
        || body.get("quote").is_some()
        || body.get("quotes").is_some()
        || body.get("rocCandidates").is_some();
    if injected {
        return execute_command_on(platform, platform, request).await;
    }

    let mut work = body.clone();
    // Resolve template URL / source when only securityId/symbol supplied.
    fill_research_refresh_from_template(platform, &mut work).await;

    let symbol = work
        .get("symbol")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let source_url = work
        .get("sourceUrl")
        .or_else(|| work.get("distributionUrl"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let declaration_source = work
        .get("declarationSource")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let security_id = work
        .get("securityId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if !symbol.is_empty() && !source_url.is_empty() {
        let profile = tauri::async_runtime::spawn_blocking({
            let symbol = symbol.clone();
            let source_url = source_url.clone();
            let declaration_source = declaration_source.clone();
            move || {
                import_engine::live_research_identity(&symbol, &source_url, &declaration_source)
            }
        })
        .await
        .unwrap_or_else(|_| serde_json::json!({}));
        merge_research_identity_into(&mut work, &profile);
    }

    emit_position_research_progress(app, 1, "retrieving declarations");
    let mut retrieve_body = work.clone();
    fill_collector_retrieve(platform, &mut retrieve_body).await;
    for key in [
        "candidates",
        "declarations",
        "misses",
        "upcomingPays",
        "payDates",
        "quote",
        "contentHash",
        "unchanged",
        "missExplanation",
        "suggestedFrequency",
        "fetchedSourceUrl",
        "fetchedPaymentCalendarUrl",
        "sourceAnalytics",
        "researchAttempts",
    ] {
        if let Some(v) = retrieve_body.get(key) {
            work[key] = v.clone();
        }
    }

    emit_position_research_progress(app, 2, "retrieving 19a-1");
    {
        let name = "RocResearchRetrieve".to_string();
        let filled = tauri::async_runtime::spawn_blocking({
            let mut roc_body = serde_json::json!({
                "securityId": security_id,
                "symbol": symbol,
                "declarationSource": declaration_source,
                "sourceUrl": source_url,
                "asOfDate": chrono::Utc::now().date_naive().to_string(),
            });
            move || {
                import_engine::enrich_retrieve_body(&name, &mut roc_body);
                roc_body
            }
        })
        .await
        .unwrap_or_else(|_| {
            serde_json::json!({
                "securityId": security_id,
                "symbol": symbol,
                "declarationSource": declaration_source,
                "sourceUrl": source_url,
            })
        });
        if let Some(c) = filled.get("candidates") {
            work["rocCandidates"] = c.clone();
        }
        if let Some(p) = filled.get("rocProbes") {
            work["rocProbes"] = p.clone();
        }
    }

    emit_position_research_progress(app, 3, "retrieving price");
    if work.get("quote").is_none()
        && work
            .get("unchanged")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
            == false
        && !security_id.is_empty()
        && !symbol.is_empty()
    {
        let targets = vec![import_engine::LastPriceTarget {
            security_id: security_id.clone(),
            symbol: symbol.clone(),
            price_source: "public".into(),
            source_symbol: symbol.clone(),
        }];
        let quotes = tauri::async_runtime::spawn_blocking(move || {
            import_engine::collect_last_price_quotes_for(targets)
        })
        .await
        .unwrap_or_default();
        if let Some(q) = quotes.first() {
            work["quote"] = q.clone();
        } else if let Some(q) = work.get("quote").cloned() {
            work["quote"] = q;
        }
    }

    emit_position_research_progress(app, 4, "drafting overview");
    request.body_json = Some(work.to_string());
    execute_command_on(platform, platform, request).await
}

async fn fill_research_refresh_from_template(
    platform: &LocalPlatform,
    body: &mut serde_json::Value,
) {
    let has_url = body
        .get("sourceUrl")
        .and_then(|s| s.as_str())
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    let has_source = body
        .get("declarationSource")
        .and_then(|s| s.as_str())
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    if has_url && has_source && body.get("securityId").is_some() {
        return;
    }
    let inv_body = if let Some(sid) = body.get("securityId").and_then(|s| s.as_str()) {
        serde_json::json!({ "securityId": sid })
    } else if let Some(symbol) = body.get("symbol").and_then(|s| s.as_str()) {
        serde_json::json!({ "symbol": symbol })
    } else {
        return;
    };
    let inv = execute_query_on(platform, platform, qry("InvestmentGet", inv_body)).await;
    if !inv.ok {
        return;
    }
    let inv_val: serde_json::Value =
        serde_json::from_str(inv.body_json.as_deref().unwrap_or("{}")).unwrap_or_default();
    if body.get("securityId").is_none() {
        if let Some(id) = inv_val.get("securityId") {
            body["securityId"] = id.clone();
        }
    }
    if body
        .get("symbol")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .is_empty()
    {
        if let Some(s) = inv_val.get("symbol") {
            body["symbol"] = s.clone();
        }
    }
    if let Some(t) = inv_val.get("template") {
        if !has_source {
            if let Some(s) = t.get("declarationSource") {
                body["declarationSource"] = s.clone();
            }
        }
        if !has_url {
            if let Some(u) = t.get("sourceUrl") {
                body["sourceUrl"] = u.clone();
            }
        }
        if body.get("sourceSymbol").is_none() {
            if let Some(s) = t.get("sourceSymbol") {
                body["sourceSymbol"] = s.clone();
            }
        }
        if body.get("lastContentHash").is_none() {
            if let Some(h) = t.get("lastContentHash") {
                body["lastContentHash"] = h.clone();
            }
        }
        if body.get("lastRunOk").is_none() {
            if let Some(ok) = t.get("lastRunOk") {
                body["lastRunOk"] = ok.clone();
            }
        }
        if body.get("lastRunAt").is_none() {
            if let Some(at) = t.get("lastRunAt") {
                body["lastRunAt"] = at.clone();
            }
        }
        if body.get("inceptionOn").is_none() {
            if let Some(i) = t.get("inceptionOn") {
                body["inceptionOn"] = i.clone();
            }
        }
    }
    if body
        .get("paymentFrequency")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .is_empty()
    {
        if let Some(f) = inv_val.get("paymentFrequency") {
            body["paymentFrequency"] = f.clone();
        }
    }
    if body
        .get("divType")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .is_empty()
    {
        if let Some(d) = inv_val.get("divType") {
            body["divType"] = d.clone();
        }
    }
}

async fn last_price_is_current(platform: &LocalPlatform, security_id: &str, today: &str) -> bool {
    let current = execute_query_on(
        platform,
        platform,
        qry(
            "CurrentPriceGet",
            serde_json::json!({ "securityId": security_id, "asOfDate": today }),
        ),
    )
    .await;
    if !current.ok {
        return false;
    }
    serde_json::from_str::<serde_json::Value>(current.body_json.as_deref().unwrap_or("{}"))
        .ok()
        .and_then(|price| {
            price
                .get("freshness")
                .and_then(|f| f.as_str())
                .map(|f| f == "current")
        })
        .unwrap_or(false)
}

/// Fill LastPriceRefresh quotes from the open-lot set. Injected quotes win (tests).
/// Misses stay unknown — never write $0. Today's current quote is not fetched again.
async fn fill_last_price_refresh(platform: &LocalPlatform, request: &mut CommandRequest) {
    let mut body: serde_json::Value = request
        .body_json
        .as_deref()
        .and_then(|raw| serde_json::from_str(raw).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    if body
        .get("quotes")
        .and_then(|q| q.as_array())
        .is_some()
    {
        return;
    }
    let _ = application_core::production_seed::ensure_btc_usd_split(platform).await;
    let set = execute_query_on(platform, platform, qry("PriceRetrievalSetGet", serde_json::json!({}))).await;
    let secs = execute_query_on(platform, platform, qry("SecurityList", serde_json::json!({}))).await;
    if !set.ok || !secs.ok {
        body["quotes"] = serde_json::json!([]);
        request.body_json = Some(body.to_string());
        return;
    }
    let set_val: serde_json::Value =
        serde_json::from_str(set.body_json.as_deref().unwrap_or("{}")).unwrap_or_default();
    let secs_val: serde_json::Value =
        serde_json::from_str(secs.body_json.as_deref().unwrap_or("[]")).unwrap_or_default();
    let mut symbol_by_id = std::collections::HashMap::<String, String>::new();
    if let Some(arr) = secs_val.as_array() {
        for row in arr {
            if let (Some(id), Some(symbol)) = (
                row.get("securityId").and_then(|v| v.as_str()),
                row.get("symbol").and_then(|v| v.as_str()),
            ) {
                symbol_by_id.insert(id.to_string(), symbol.to_string());
            }
        }
    }
    let today = chrono::Utc::now().date_naive().to_string();
    let mut targets = Vec::new();
    let items = set_val
        .get("items")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    if !items.is_empty() {
        for item in items {
            let Some(id) = item.get("securityId").and_then(|v| v.as_str()) else {
                continue;
            };
            if last_price_is_current(platform, id, &today).await {
                continue;
            }
            let symbol = item
                .get("symbol")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let div_type = item
                .get("divType")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            // Defense in depth: CASH / SPAXX / FDRXX / SWVXX never hit Yahoo.
            // PriceRetrievalSetGet already excludes them.
            if div_type.eq_ignore_ascii_case("CASH")
                || matches!(
                    symbol.to_ascii_uppercase().as_str(),
                    "SPAXX" | "FDRXX" | "SWVXX"
                )
            {
                continue;
            }
            targets.push(import_engine::LastPriceTarget {
                security_id: id.to_string(),
                symbol,
                price_source: item
                    .get("priceSource")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                source_symbol: item
                    .get("sourceSymbol")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            });
        }
    } else {
        let ids = set_val
            .get("securityIds")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        for id_val in ids {
            let Some(id) = id_val.as_str() else {
                continue;
            };
            if last_price_is_current(platform, id, &today).await {
                continue;
            }
            if let Some(symbol) = symbol_by_id.get(id) {
                if matches!(
                    symbol.to_ascii_uppercase().as_str(),
                    "SPAXX" | "FDRXX" | "SWVXX"
                ) {
                    continue;
                }
                targets.push(import_engine::LastPriceTarget {
                    security_id: id.to_string(),
                    symbol: symbol.clone(),
                    price_source: String::new(),
                    source_symbol: String::new(),
                });
            }
        }
    }
    let quotes = tauri::async_runtime::spawn_blocking({
        let targets = targets.clone();
        move || import_engine::collect_last_price_quotes_for(targets)
    })
    .await
    .unwrap_or_default();
    let mut misses = Vec::new();
    for target in &targets {
        let found = quotes.iter().any(|q| {
            q.get("securityId").and_then(|v| v.as_str()) == Some(target.security_id.as_str())
        });
        if !found {
            misses.push(serde_json::json!({
                "securityId": target.security_id,
                "symbol": target.symbol,
                "code": "price_retrieve_miss",
                "reason": format!(
                    "Last price miss for {}. Stored price still displays; never $0.",
                    target.symbol
                )
            }));
        }
    }
    body["quotes"] = serde_json::Value::Array(quotes);
    body["misses"] = serde_json::Value::Array(misses);
    request.body_json = Some(body.to_string());
}

async fn fill_declaration_source_from_template(platform: &LocalPlatform, request: &mut CommandRequest) {
    if !matches!(
        request.command_name.as_str(),
        "MarketRetrieve" | "DeclarationRetrieve" | "RocResearchRetrieve"
    ) {
        return;
    }
    let mut body: serde_json::Value = request
        .body_json
        .as_deref()
        .and_then(|raw| serde_json::from_str(raw).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    if body
        .get("candidates")
        .and_then(|c| c.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false)
    {
        return;
    }
    let has_source = body
        .get("declarationSource")
        .and_then(|s| s.as_str())
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    let has_url = body
        .get("sourceUrl")
        .and_then(|s| s.as_str())
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    // Roc validate needs stored issuer URL even when declarationSource is already set.
    if has_source && has_url && request.command_name != "RocResearchRetrieve" {
        return;
    }
    if has_source && request.command_name != "RocResearchRetrieve" {
        return;
    }
    let symbol = body
        .get("symbol")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    if symbol.is_empty() {
        return;
    }
    let set = execute_query_on(platform, platform, qry("PriceRetrievalSetGet", serde_json::json!({}))).await;
    if set.ok {
        let set_val: serde_json::Value =
            serde_json::from_str(set.body_json.as_deref().unwrap_or("{}")).unwrap_or_default();
        if let Some(items) = set_val.get("items").and_then(|v| v.as_array()) {
            for item in items {
                let item_symbol = item.get("symbol").and_then(|s| s.as_str()).unwrap_or("");
                if item_symbol.eq_ignore_ascii_case(&symbol) {
                    if let Some(src) = item.get("declarationSource").and_then(|s| s.as_str()) {
                        if !src.is_empty()
                            && body
                                .get("declarationSource")
                                .and_then(|s| s.as_str())
                                .unwrap_or("")
                                .is_empty()
                        {
                            body["declarationSource"] = serde_json::Value::String(src.to_string());
                        }
                    }
                    if let Some(src) = item.get("sourceSymbol").and_then(|s| s.as_str()) {
                        if !src.is_empty()
                            && body.get("sourceSymbol").and_then(|s| s.as_str()).unwrap_or("").is_empty()
                        {
                            body["sourceSymbol"] = serde_json::Value::String(src.to_string());
                        }
                    }
                    if let Some(url) = item.get("sourceUrl").and_then(|s| s.as_str()) {
                        if !url.is_empty()
                            && body.get("sourceUrl").and_then(|s| s.as_str()).unwrap_or("").is_empty()
                        {
                            body["sourceUrl"] = serde_json::Value::String(url.to_string());
                        }
                    }
                    request.body_json = Some(body.to_string());
                    if request.command_name != "RocResearchRetrieve" {
                        return;
                    }
                    break;
                }
            }
        }
    }
    let inv_body = if let Some(sid) = body.get("securityId").and_then(|s| s.as_str()) {
        serde_json::json!({ "securityId": sid })
    } else {
        serde_json::json!({ "symbol": symbol })
    };
    let inv = execute_query_on(platform, platform, qry("InvestmentGet", inv_body)).await;
    if inv.ok {
        let val: serde_json::Value =
            serde_json::from_str(inv.body_json.as_deref().unwrap_or("{}")).unwrap_or_default();
        if body
            .get("declarationSource")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .is_empty()
        {
            if let Some(src) = val
                .pointer("/template/declarationSource")
                .and_then(|s| s.as_str())
            {
                if !src.is_empty() {
                    body["declarationSource"] = serde_json::Value::String(src.to_string());
                }
            }
        }
        if body
            .get("sourceSymbol")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .is_empty()
        {
            if let Some(src) = val.pointer("/template/sourceSymbol").and_then(|s| s.as_str()) {
                if !src.is_empty() {
                    body["sourceSymbol"] = serde_json::Value::String(src.to_string());
                }
            }
        }
        if body
            .get("sourceUrl")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .is_empty()
        {
            if let Some(url) = val.pointer("/template/sourceUrl").and_then(|s| s.as_str()) {
                if !url.is_empty() {
                    body["sourceUrl"] = serde_json::Value::String(url.to_string());
                }
            }
        }
    }
    request.body_json = Some(body.to_string());
}

async fn paid_declaration_context(
    platform: &LocalPlatform,
    security_id: &str,
) -> (u8, Vec<String>, Vec<(String, i64, u8)>) {
    let inv = execute_query_on(
        platform,
        platform,
        qry(
            "InvestmentGet",
            serde_json::json!({ "securityId": security_id }),
        ),
    )
    .await;
    if !inv.ok {
        return (0, Vec::new(), Vec::new());
    }
    let val: serde_json::Value =
        serde_json::from_str(inv.body_json.as_deref().unwrap_or("{}")).unwrap_or_default();
    let mut paid = 0u8;
    let mut periods = Vec::new();
    let mut amounts = Vec::new();
    if let Some(arr) = val.get("declarations").and_then(|d| d.as_array()) {
        for row in arr {
            let amount = row
                .get("amountPerShareMinor")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            if amount <= 0 {
                continue;
            }
            paid = paid.saturating_add(1);
            if let Some(p) = row.get("paymentPeriod").and_then(|v| v.as_str()) {
                let p = p.trim();
                if !p.is_empty() {
                    periods.push(p.to_string());
                    let scale = row
                        .get("amountScale")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(2) as u8;
                    amounts.push((p.to_string(), amount, scale));
                }
            }
        }
    }
    (paid, periods, amounts)
}

fn declaration_target_from_item(
    item: &application_core::contracts::CollectorSetItem,
    paid_count: u8,
    known_payment_periods: Vec<String>,
    known_declaration_amounts: Vec<(String, i64, u8)>,
) -> import_engine::DeclarationTarget {
    import_engine::DeclarationTarget {
        security_id: item.security_id.to_string(),
        symbol: item.symbol.clone(),
        declaration_source: item.declaration_source.clone(),
        source_symbol: item.source_symbol.clone(),
        source_url: item.source_url.clone(),
        last_content_hash: item.last_content_hash.clone(),
        div_type: item.div_type.clone(),
        force_refresh: false,
        last_run_ok: item.last_run_ok.unwrap_or(false),
        last_run_at: item.last_run_at.clone(),
        inception_on: item.inception_on.clone(),
        payment_frequency: item.payment_frequency.clone(),
        paid_count,
        known_payment_periods,
        known_declaration_amounts,
    }
}

async fn fill_collector_retrieve(platform: &LocalPlatform, body: &mut serde_json::Value) {
    if body.get("candidates").is_some()
        || body.get("declarations").is_some()
        || body.get("misses").is_some()
        || body.get("payDates").is_some()
    {
        return;
    }
    let security_id = body
        .get("securityId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let symbol = body
        .get("symbol")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let declaration_source = body
        .get("declarationSource")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let source_symbol = body
        .get("sourceSymbol")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let source_url = body
        .get("sourceUrl")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let last_content_hash = body
        .get("lastContentHash")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let div_type = body
        .get("divType")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let force_refresh = body
        .get("forceRefresh")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let last_run_ok = body
        .get("lastRunOk")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let last_run_at = body
        .get("lastRunAt")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    body["symbol"] = serde_json::Value::String(symbol.clone());
    body["declarationSource"] = serde_json::Value::String(declaration_source.clone());
    body["divType"] = serde_json::Value::String(div_type.clone());
    let (paid_count, known_periods, known_declaration_amounts) =
        paid_declaration_context(platform, &security_id).await;
    let known_payment_periods = body
        .get("knownPaymentPeriods")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|p| p.as_str().map(|s| s.trim().to_string()))
                .filter(|p| !p.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|v| !v.is_empty())
        .unwrap_or(known_periods);
    let target = import_engine::DeclarationTarget {
        security_id: security_id.clone(),
        symbol: symbol.clone(),
        declaration_source: declaration_source.clone(),
        source_symbol,
        source_url: source_url.clone(),
        last_content_hash,
        div_type: div_type.clone(),
        force_refresh,
        last_run_ok,
        last_run_at,
        inception_on: body
            .get("inceptionOn")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        payment_frequency: body
            .get("paymentFrequency")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        paid_count,
        known_payment_periods,
        known_declaration_amounts,
    };
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        import_engine::collect_declarations_for(vec![target])
    })
    .await
    .unwrap_or_default();
    body["candidates"] = serde_json::Value::Array(outcome.candidates.clone());
    body["declarations"] = serde_json::Value::Array(outcome.candidates);
    body["payDates"] = serde_json::Value::Array(outcome.pay_dates);
    body["upcomingPays"] = body["payDates"].clone();
    body["misses"] = serde_json::Value::Array(outcome.misses);
    body["unchanged"] = serde_json::json!(!outcome.unchanged.is_empty());
    if !outcome.fetched_source_url.is_empty() {
        body["fetchedSourceUrl"] = serde_json::Value::String(outcome.fetched_source_url);
    }
    if !outcome.fetched_payment_calendar_url.is_empty() {
        body["fetchedPaymentCalendarUrl"] =
            serde_json::Value::String(outcome.fetched_payment_calendar_url);
    }
    if !outcome.fetched_page.is_empty() {
        body["fetchedPage"] = serde_json::Value::String(outcome.fetched_page);
    }
    if !outcome.page_paid.is_empty() {
        body["pagePaid"] = serde_json::Value::Array(outcome.page_paid);
    }
    let is_cash = div_type.eq_ignore_ascii_case("CASH")
        || matches!(
            symbol.to_ascii_uppercase().as_str(),
            "SPAXX" | "FDRXX" | "SWVXX"
        );
    if is_cash
        && body
            .get("candidates")
            .and_then(|c| c.as_array())
            .map(|a| a.is_empty())
            .unwrap_or(true)
    {
        body["missExplanation"] = serde_json::Value::String(
            "CASH 7-day yield page empty.".into(),
        );
    }
    let _ = tauri::async_runtime::spawn_blocking({
        let mut quote_body = body.clone();
        move || {
            import_engine::enrich_collector_quote_only(&mut quote_body);
            quote_body
        }
    })
    .await
    .map(|quote_body| {
        if let Some(quote) = quote_body.get("quote") {
            body["quote"] = quote.clone();
        }
    });
    if financial_domain::mlp_sec::routes_fetch(&declaration_source, Some(&source_url)) {
        body["fetchedSourceUrl"] =
            serde_json::Value::String(financial_domain::mlp_sec::atom_url());
    }
    if body
        .get("candidates")
        .and_then(|c| c.as_array())
        .map(|a| a.is_empty())
        .unwrap_or(true)
        && !is_cash
    {
        body["missExplanation"] = serde_json::Value::String(
            if financial_domain::mlp_sec::routes_fetch(&declaration_source, Some(&source_url)) {
                financial_domain::mlp_sec::last_run_stamp("empty")
            } else {
                "Issuer page empty.".into()
            },
        );
    }
    if let Some(hash) = outcome
        .unchanged
        .first()
        .and_then(|u| u.get("contentHash"))
        .cloned()
        .or_else(|| {
            body.get("candidates")
                .and_then(|c| c.as_array())
                .and_then(|a| a.first())
                .and_then(|c| c.get("contentHash"))
                .cloned()
        })
    {
        body["contentHash"] = hash;
    }
}

async fn fill_declaration_refresh(
    app: &AppHandle,
    platform: &LocalPlatform,
    request: &mut CommandRequest,
) {
    let mut body: serde_json::Value = request
        .body_json
        .as_deref()
        .and_then(|raw| serde_json::from_str(raw).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    if body.get("declarations").is_some()
        || body.get("misses").is_some()
        || body.get("payDates").is_some()
    {
        return;
    }
    let set = match platform.collector_set().await {
        Ok(set) => set,
        Err(_) => {
            body["declarations"] = serde_json::json!([]);
            request.body_json = Some(body.to_string());
            return;
        }
    };
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let mut targets = Vec::new();
    let mut disabled_misses = Vec::new();
    let mut already_current = Vec::new();
    let mut queued: std::collections::HashSet<String> = std::collections::HashSet::new();
    for item in &set.items {
        if !item.open_lots {
            continue;
        }
        let id = item.security_id.to_string();
        let source = item.declaration_source.as_str();
        let registered = import_engine::is_registered_declaration_source(source);

        if registered && item.collector_enabled {
            if financial_domain::collector::declaration_daily_retrieve_current(
                item.last_run_ok,
                &item.last_run_at,
                &today,
            ) {
                already_current.push(serde_json::json!({
                    "securityId": id,
                    "symbol": item.symbol,
                }));
                queued.insert(id);
                continue;
            }
            let (paid_count, known_payment_periods, known_declaration_amounts) =
                paid_declaration_context(platform, &id).await;
            targets.push(declaration_target_from_item(
                item,
                paid_count,
                known_payment_periods,
                known_declaration_amounts,
            ));
            queued.insert(id);
            continue;
        }

        // DIV-1 must be assigned and enabled; otherwise raise a loud miss (never Yahoo).
        if import_engine::div1_adapter_missing(&item.div_type, source) {
            targets.push(import_engine::DeclarationTarget {
                security_id: id.clone(),
                symbol: item.symbol.clone(),
                declaration_source: item.declaration_source.clone(),
                source_symbol: item.source_symbol.clone(),
                source_url: String::new(),
                last_content_hash: String::new(),
                div_type: item.div_type.clone(),
                force_refresh: false,
                last_run_ok: false,
                last_run_at: String::new(),
                inception_on: String::new(),
                payment_frequency: item.payment_frequency.clone(),
                paid_count: 0,
                known_payment_periods: Vec::new(),
                known_declaration_amounts: Vec::new(),
            });
            queued.insert(id);
            continue;
        }
        if import_engine::is_div1(&item.div_type) && registered && !item.collector_enabled {
            disabled_misses.push(serde_json::json!({
                "securityId": id,
                "symbol": item.symbol,
                "declarationSource": source,
                "reason": "DIV-1 adapter assigned but collector disabled.",
                "code": "div1_adapter_missing",
            }));
            queued.insert(id);
        }
    }
    for item in &set.items {
        if !item.open_lots || !item.collector_enabled {
            continue;
        }
        if !import_engine::is_registered_declaration_source(&item.declaration_source) {
            continue;
        }
        let id = item.security_id.to_string();
        if queued.contains(&id) {
            continue;
        }
        let (paid_count, known_payment_periods, known_declaration_amounts) =
            paid_declaration_context(platform, &id).await;
        targets.push(declaration_target_from_item(
            item,
            paid_count,
            known_payment_periods,
            known_declaration_amounts,
        ));
    }
    let total = (already_current.len() + targets.len()) as u32;
    emit_declaration_refresh_progress(app, 0, total, "");
    let mut done = 0u32;
    for row in &already_current {
        done = done.saturating_add(1);
        let symbol = row.get("symbol").and_then(|v| v.as_str()).unwrap_or("");
        emit_declaration_refresh_progress(app, done, total, symbol);
    }
    let mut outcome = import_engine::DeclarationCollectOutcome::default();
    for target in targets {
        done = done.saturating_add(1);
        emit_declaration_refresh_progress(app, done, total, &target.symbol);
        let stamp_id = target.security_id.clone();
        let stamp_symbol = target.symbol.clone();
        let stamp_hash = target.last_content_hash.clone();
        let mut one = tauri::async_runtime::spawn_blocking(move || {
            import_engine::collect_declarations_for(vec![target])
        })
        .await
        .unwrap_or_default();
        let quiet = one.misses.is_empty() && one.unchanged.is_empty();
        if quiet {
            one.unchanged.push(serde_json::json!({
                "securityId": stamp_id,
                "symbol": stamp_symbol,
                "contentHash": stamp_hash,
            }));
        }
        merge_declaration_collect(&mut outcome, one);
    }
    outcome.misses.extend(disabled_misses);
    outcome.unchanged.extend(already_current);
    body["declarations"] = serde_json::Value::Array(outcome.candidates);
    body["payDates"] = serde_json::Value::Array(outcome.pay_dates);
    body["misses"] = serde_json::Value::Array(outcome.misses);
    body["unchanged"] = serde_json::Value::Array(outcome.unchanged);
    request.body_json = Some(body.to_string());
}

#[tauri::command]
fn open_exception_log(
    app: AppHandle,
    exceptions: Vec<serde_json::Value>,
    process_lines: Option<Vec<String>>,
) -> Result<String, String> {
    let preferred = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?;
    let dir = resolve_app_data_dir(preferred).join("logs");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let today = chrono::Utc::now().date_naive().to_string();
    let path = dir.join(format!("capture-{today}.log"));
    let mut lines = Vec::new();
    lines.push(format!("# finos capture log {today}"));
    lines.push(format!("# written_at {}", chrono::Utc::now().to_rfc3339()));
    if let Some(process) = process_lines.as_ref().filter(|p| !p.is_empty()) {
        lines.push("# process".into());
        lines.extend(process.iter().cloned());
        lines.push(String::new());
    }
    lines.push("# exceptions".into());
    for row in &exceptions {
        let created = row
            .get("createdAt")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let code = row.get("code").and_then(|v| v.as_str()).unwrap_or("");
        let message = row
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let ack = row
            .get("acknowledged")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        lines.push(format!(
            "{created}\t{code}\t{}\t{message}",
            if ack { "ack" } else { "open" }
        ));
    }
    if exceptions.is_empty() {
        lines.push("# (no exceptions in current ExceptionList)".into());
    }
    let body = lines.join("\n") + "\n";
    std::fs::write(&path, body).map_err(|e| e.to_string())?;
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path.to_string_lossy()])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .or_else(|_| std::process::Command::new("open").arg(&path).spawn())
            .map_err(|e| e.to_string())?;
    }
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
fn save_local_bytes(default_file_name: String, bytes: Vec<u8>) -> Result<String, String> {
    let local = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let dir = resolve_app_data_dir(local.join("finos-exports"));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(default_file_name);
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
fn app_exit(app: AppHandle) {
    // Destroy WebView windows before process exit. Chromium on Windows otherwise
    // races UnregisterClass(Chrome_WidgetWin_0) and prints Error 1412
    // (ERROR_CLASS_DOES_NOT_EXIST) — see tauri#7606 / Chromium 40720563.
    for (_label, window) in app.webview_windows() {
        let _ = window.destroy();
    }
    app.exit(0);
}

/// Coding launch (`tauri dev` / `start-finos-dev.bat`) serves the UI from Vite on
/// localhost:1420. `AppHandle::restart` only relaunches this exe. The Tauri CLI
/// treats that as the app quitting and kills Vite, so the new window shows
/// Chrome's "can't reach this page" / connection refused. Household release
/// bundles the UI and can use `app.restart()`.
fn dev_stack_bat() -> Option<PathBuf> {
    let compile_time =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("start-finos-dev.bat");
    if compile_time.is_file() {
        return Some(compile_time);
    }
    let Ok(exe) = std::env::current_exe() else {
        return None;
    };
    for dir in exe.ancestors().take(8) {
        let nested = dir.join("apps").join("desktop").join("start-finos-dev.bat");
        if nested.is_file() {
            return Some(nested);
        }
        let beside = dir.join("start-finos-dev.bat");
        if beside.is_file() {
            return Some(beside);
        }
    }
    None
}

fn spawn_dev_stack(bat: &std::path::Path) -> Result<(), String> {
    let bat = std::fs::canonicalize(bat).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        // New console, same as starting from a DOS prompt. Delay so this process
        // and the Tauri CLI release SQLite and port 1420 before the bat starts.
        let script = format!(
            "timeout /t 2 /nobreak >nul & call \"{}\"",
            bat.display()
        );
        std::process::Command::new("cmd")
            .args(["/C", "start", "", "cmd", "/C", &script])
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(not(windows))]
    {
        let _ = bat;
        Err("dev stack restart is Windows-only".into())
    }
}

#[tauri::command]
async fn app_restart(
    app: AppHandle,
    platform: State<'_, Arc<LocalPlatform>>,
) -> Result<(), String> {
    platform.close_for_shutdown().await;
    for (_label, window) in app.webview_windows() {
        let _ = window.destroy();
    }
    if cfg!(debug_assertions) {
        if let Some(bat) = dev_stack_bat() {
            spawn_dev_stack(&bat)?;
            app.exit(0);
            return Ok(());
        }
    }
    app.restart();
}

fn looks_like_sync_folder(path: &std::path::Path) -> bool {
    let s = path.to_string_lossy().to_lowercase();
    s.contains("onedrive") || s.contains("dropbox") || s.contains("icloud")
}

fn resolve_app_data_dir(preferred: PathBuf) -> PathBuf {
    if looks_like_sync_folder(&preferred) {
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            return PathBuf::from(local).join("finos");
        }
    }
    preferred
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let preferred = app.path().app_local_data_dir()?;
            let dir = resolve_app_data_dir(preferred);
            std::fs::create_dir_all(&dir)?;
            let _ = dotenvy::from_filename(dir.join(".env"));
            let _ = dotenvy::dotenv();
            let advisory: Arc<dyn Advisory> = match GrokAdvisory::from_env() {
                Some(grok) => Arc::new(grok),
                None => Arc::new(MissingKeyAdvisory),
            };
            let platform = tauri::async_runtime::block_on(LocalPlatform::open_with_advisory(
                dir, advisory,
            ))
            .map_err(|e| e.to_string())?;
            app.manage(Arc::new(platform));
            let file_menu = SubmenuBuilder::new(app, "File")
                .text("home", "Home")
                .text("data-snapshot", "Save data snapshot")
                .text("app-restart", "Restart Application")
                .text("app-exit", "Exit")
                .build()?;
            let income_menu = SubmenuBuilder::new(app, "Income Plan")
                .text("income-plan", "Income Plan")
                .build()?;
            let plan_menu = SubmenuBuilder::new(app, "Plan")
                .text("calculator", "Calculator")
                .text("dashboard", "Dashboard")
                .text("trends", "Trends")
                .build()?;
            let positions_menu = SubmenuBuilder::new(app, "Positions")
                .text("position-details", "Position Details")
                .text("holdings", "Holdings")
                .text("new-investment", "Add Position")
                .text("add-lot", "Add Lot")
                .build()?;
            let data_menu = SubmenuBuilder::new(app, "Data")
                .text("import", "Import")
                .text("collectors", "Collectors")
                .text("tickets", "Tickets")
                .build()?;
            let tools_menu = SubmenuBuilder::new(app, "Tools")
                .text("collector-establish", "Reevaluate collector")
                .text("settings", "Settings")
                .build()?;
            let menu = MenuBuilder::new(app)
                .item(&file_menu)
                .item(&income_menu)
                .item(&plan_menu)
                .item(&positions_menu)
                .item(&data_menu)
                .item(&tools_menu)
                .build()?;
            app.set_menu(menu)?;
            Ok(())
        })
        .on_menu_event(|app, event| {
            if event.id() == "app-exit" {
                app_exit(app.clone());
                return;
            }
            if event.id() == "data-snapshot" {
                let _ = app.emit("finos-data-snapshot", "export");
                return;
            }
            if event.id() == "app-restart" {
                let _ = app.emit("finos-app-restart", "restart");
                return;
            }
            let id = event.id().as_ref();
            if matches!(
                id,
                "home"
                    | "income-plan"
                    | "calculator"
                    | "dashboard"
                    | "trends"
                    | "position-details"
                    | "holdings"
                    | "new-investment"
                    | "add-lot"
                    | "import"
                    | "collectors"
                    | "tickets"
                    | "collector-establish"
                    | "settings"
            ) {
                let _ = app.emit("finos-navigate", id);
            }
        })
        .invoke_handler(tauri::generate_handler![
            finance_query,
            finance_command,
            open_exception_log,
            save_local_bytes,
            app_exit,
            app_restart
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
