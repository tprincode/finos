use std::path::PathBuf;
use std::sync::Arc;

use ai_gateway::GrokAdvisory;
use application_core::contracts::{
    CommandRequest, CommandResult, QueryRequest, QueryResult, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::ports::advisory::{Advisory, MissingKeyAdvisory};
use application_core::queries::{execute_command_on, execute_query_on};
use storage_sqlite::LocalPlatform;
use tauri::menu::{MenuBuilder, SubmenuBuilder};
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

#[tauri::command]
async fn finance_query(
    platform: State<'_, Arc<LocalPlatform>>,
    request: QueryRequest,
) -> Result<QueryResult, String> {
    Ok(execute_query_on(platform.inner().as_ref(), platform.inner().as_ref(), request).await)
}

#[tauri::command]
async fn finance_command(
    platform: State<'_, Arc<LocalPlatform>>,
    mut request: CommandRequest,
) -> Result<CommandResult, String> {
    if matches!(
        request.command_name.as_str(),
        "PriceQuoteRetrieve" | "DeclarationRetrieve" | "MarketRetrieve" | "PeriodSeriesRetrieve" | "RocResearchRetrieve"
    ) {
        fill_declaration_source_from_template(platform.inner().as_ref(), &mut request).await;
        let name = request.command_name.clone();
        let mut body: serde_json::Value = request
            .body_json
            .as_deref()
            .and_then(|raw| serde_json::from_str(raw).ok())
            .unwrap_or_else(|| serde_json::json!({}));
        let filled = tauri::async_runtime::spawn_blocking(move || {
            import_engine::enrich_retrieve_body(&name, &mut body);
            body
        })
        .await
        .map_err(|e| e.to_string())?;
        request.body_json = Some(filled.to_string());
    }
    if request.command_name == "LastPriceRefresh" {
        fill_last_price_refresh(platform.inner().as_ref(), &mut request).await;
    }
    if request.command_name == "DeclarationRefresh" {
        fill_declaration_refresh(platform.inner().as_ref(), &mut request).await;
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
            targets.push(import_engine::LastPriceTarget {
                security_id: id.to_string(),
                symbol: item
                    .get("symbol")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
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
        "MarketRetrieve" | "DeclarationRetrieve"
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
    if body
        .get("declarationSource")
        .and_then(|s| s.as_str())
        .map(|s| !s.is_empty())
        .unwrap_or(false)
    {
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
                        if !src.is_empty() {
                            body["declarationSource"] = serde_json::Value::String(src.to_string());
                        }
                    }
                    if let Some(src) = item.get("sourceSymbol").and_then(|s| s.as_str()) {
                        if !src.is_empty() && body.get("sourceSymbol").and_then(|s| s.as_str()).unwrap_or("").is_empty()
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
                    return;
                }
            }
        }
    }
    let inv = execute_query_on(
        platform,
        platform,
        qry("InvestmentGet", serde_json::json!({ "symbol": symbol })),
    )
    .await;
    if inv.ok {
        let val: serde_json::Value =
            serde_json::from_str(inv.body_json.as_deref().unwrap_or("{}")).unwrap_or_default();
        if let Some(src) = val
            .pointer("/template/declarationSource")
            .and_then(|s| s.as_str())
        {
            if !src.is_empty() {
                body["declarationSource"] = serde_json::Value::String(src.to_string());
            }
        }
        if let Some(src) = val.pointer("/template/sourceSymbol").and_then(|s| s.as_str()) {
            if !src.is_empty() {
                body["sourceSymbol"] = serde_json::Value::String(src.to_string());
            }
        }
    }
    request.body_json = Some(body.to_string());
}

async fn declaration_entered_today(platform: &LocalPlatform, security_id: &str, source: &str, today: &str) -> bool {
    let inv = execute_query_on(
        platform,
        platform,
        qry("InvestmentGet", serde_json::json!({ "securityId": security_id })),
    )
    .await;
    if !inv.ok {
        return false;
    }
    let val: serde_json::Value =
        serde_json::from_str(inv.body_json.as_deref().unwrap_or("{}")).unwrap_or_default();
    val.get("declarations")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter().any(|d| {
                let src = d.get("source").and_then(|s| s.as_str()).unwrap_or("");
                let entered = d.get("enteredAt").and_then(|s| s.as_str()).unwrap_or("");
                src.eq_ignore_ascii_case(source) && entered.starts_with(today)
            })
        })
        .unwrap_or(false)
}

async fn fill_declaration_refresh(platform: &LocalPlatform, request: &mut CommandRequest) {
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
    let set = execute_query_on(platform, platform, qry("PriceRetrievalSetGet", serde_json::json!({}))).await;
    if !set.ok {
        body["declarations"] = serde_json::json!([]);
        request.body_json = Some(body.to_string());
        return;
    }
    let set_val: serde_json::Value =
        serde_json::from_str(set.body_json.as_deref().unwrap_or("{}")).unwrap_or_default();
    let today = chrono::Utc::now().date_naive().to_string();
    let mut targets = Vec::new();
    let items = set_val
        .get("items")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    for item in items {
        let Some(id) = item.get("securityId").and_then(|v| v.as_str()) else {
            continue;
        };
        let source = item
            .get("declarationSource")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let skip = source.is_empty()
            || source.eq_ignore_ascii_case("unassigned")
            || source.eq_ignore_ascii_case("public")
            || source.eq_ignore_ascii_case("import")
            || source.eq_ignore_ascii_case("sec-edgar");
        if skip {
            continue;
        }
        if declaration_entered_today(platform, id, source, &today).await {
            continue;
        }
        targets.push(import_engine::DeclarationTarget {
            security_id: id.to_string(),
            symbol: item
                .get("symbol")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            declaration_source: source.to_string(),
            source_symbol: item
                .get("sourceSymbol")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            source_url: item
                .get("sourceUrl")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            last_content_hash: item
                .get("lastContentHash")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        });
    }
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        import_engine::collect_declarations_for(targets)
    })
    .await
    .unwrap_or_default();
    body["declarations"] = serde_json::Value::Array(outcome.candidates);
    body["payDates"] = serde_json::Value::Array(outcome.pay_dates);
    body["misses"] = serde_json::Value::Array(outcome.misses);
    body["unchanged"] = serde_json::Value::Array(outcome.unchanged);
    request.body_json = Some(body.to_string());
}

#[tauri::command]
fn app_exit(app: AppHandle) {
    app.exit(0);
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
                .quit_with_text("Exit")
                .build()?;
            let menu = MenuBuilder::new(app).item(&file_menu).build()?;
            app.set_menu(menu)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            finance_query,
            finance_command,
            app_exit
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
