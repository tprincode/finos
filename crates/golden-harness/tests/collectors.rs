//! Collector standing order, retrieve_run ledger, and on-demand retrieve.

use application_core::contracts::{
    CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::queries::{execute_command_on, execute_query_on};
use storage_sqlite::LocalPlatform;
use uuid::Uuid;

fn cmd(name: &str, body: serde_json::Value) -> CommandRequest {
    CommandRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        command_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: Some(body.to_string()),
        expected_version: None,
    }
}

fn qry(name: &str, body: serde_json::Value) -> QueryRequest {
    QueryRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        query_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: Some(body.to_string()),
    }
}

async fn must_ok(platform: &LocalPlatform, name: &str, body: serde_json::Value) -> serde_json::Value {
    let result = execute_command_on(platform, platform, cmd(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

async fn query_json(platform: &LocalPlatform, name: &str, body: serde_json::Value) -> serde_json::Value {
    let result = execute_query_on(platform, platform, qry(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

async fn research_template(platform: &LocalPlatform, security_id: &str, symbol: &str) {
    must_ok(
        platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "priceSource": "public",
            "sourceSymbol": symbol,
            "declarationSource": "issuer",
            "sourceUrl": format!("https://example.test/{}/distributions", symbol.to_ascii_lowercase()),
            "calendarPolicy": "derived_walk",
            "collectorEnabled": true,
            "lookbackCount": 12
        }),
    )
    .await;
}

#[tokio::test]
async fn collector_retrieve_persists_run_and_posts_declaration() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .expect("open sqlite");
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "QDVO", "name": "QDVO"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap().to_string();
    research_template(&platform, &security_id, "QDVO").await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account["accountId"],
            "securityId": security_id,
            "openedOn": "2026-01-02",
            "quantityMinor": 100,
            "quantityScale": 0,
            "performanceCostMinor": 10000,
            "taxCostMinor": 10000,
            "scale": 2
        }),
    )
    .await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "provider": "Amplify",
            "divType": "DIV-1",
            "isActive": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "declarationSource": "amplify",
            "calendarPolicy": "issuer_calendar",
            "collectorEnabled": true
        }),
    )
    .await;

    let retrieved = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "QDVO",
            "declarationSource": "amplify",
            "candidates": [{
                "securityId": security_id,
                "amountPerShareMinor": 38826,
                "amountScale": 5,
                "paymentPeriod": "2026-07-31",
                "source": "amplify",
                "contentHash": "hash-qdvo-1"
            }],
            "upcomingPays": [{
                "securityId": security_id,
                "payOn": "2026-08-29",
                "source": "amplify",
                "contentHash": "hash-qdvo-1"
            }],
            "sourceAnalytics": {
                "htmlReturned": true,
                "fundPage": true,
                "tableOnGet": true,
                "tableRowCount": 3,
                "jsLikely": false
            },
            "researchAttempts": [{
                "vendor": "amplify",
                "url": "https://amplifyetfs.com/qdvo/",
                "found": true,
                "note": "fixture"
            }]
        }),
    )
    .await;
    assert!(retrieved["ok"].as_bool().unwrap_or(false));
    assert!(retrieved["recorded"].as_u64().unwrap_or(0) >= 1);
    assert!(!retrieved["runId"].as_str().unwrap_or("").is_empty());
    let payload: serde_json::Value =
        serde_json::from_str(retrieved["payloadJson"].as_str().unwrap_or("{}")).unwrap();
    assert_eq!(payload["sourceAnalytics"]["tableRowCount"], 3);
    assert_eq!(payload["researchAttempts"][0]["vendor"], "amplify");

    let runs = query_json(
        &platform,
        "RetrieveRunList",
        serde_json::json!({ "securityId": security_id, "limit": 10 }),
    )
    .await;
    assert_eq!(runs["runs"].as_array().unwrap().len(), 1);
    assert_eq!(runs["runs"][0]["kind"], "declaration");
    assert_eq!(runs["runs"][0]["ok"], true);

    let set = query_json(&platform, "CollectorSetGet", serde_json::json!({})).await;
    let item = set["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["symbol"] == "QDVO")
        .expect("QDVO in collector set");
    assert_eq!(item["collectorEnabled"], true);
    assert_eq!(item["declarationSource"], "amplify");
    assert!(item["lastRunAt"].as_str().unwrap_or("").len() >= 10);

    let last_run = item["lastRunAt"].as_str().unwrap_or("");
    assert!(last_run.len() >= 10);
    let as_of = &last_run[..10];
    let stats = query_json(
        &platform,
        "CollectorStatsGet",
        serde_json::json!({ "asOfDate": as_of }),
    )
    .await;
    assert!(stats["assigned"].as_u64().unwrap_or(0) >= 1);
    assert!(stats["enabled"].as_u64().unwrap_or(0) >= 1);
    assert!(stats["ranToday"].as_u64().unwrap_or(0) >= 1);
}

#[tokio::test]
async fn declaration_refresh_writes_retrieve_run_on_miss() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .expect("open sqlite");
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "TOPW", "name": "TOPW"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap().to_string();
    research_template(&platform, &security_id, "TOPW").await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account["accountId"],
            "securityId": security_id,
            "openedOn": "2026-01-02",
            "quantityMinor": 10,
            "quantityScale": 0,
            "performanceCostMinor": 1000,
            "taxCostMinor": 1000,
            "scale": 2
        }),
    )
    .await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Weekly",
            "provider": "Roundhill",
            "divType": "DIV-1",
            "isActive": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "DeclarationRefresh",
        serde_json::json!({
            "misses": [{
                "securityId": security_id,
                "symbol": "TOPW",
                "declarationSource": "unassigned",
                "reason": "DIV-1 has no issuer adapter assigned.",
                "code": "div1_adapter_missing"
            }]
        }),
    )
    .await;
    let runs = query_json(
        &platform,
        "RetrieveRunList",
        serde_json::json!({ "securityId": security_id }),
    )
    .await;
    assert_eq!(runs["runs"].as_array().unwrap().len(), 1);
    assert_eq!(runs["runs"][0]["ok"], false);
    assert_eq!(runs["runs"][0]["code"], "div1_adapter_missing");
}

#[tokio::test]
async fn cash_par_price_and_excluded_from_yahoo_set() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .expect("open sqlite");
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Brokerage", "kind": "taxable"}),
    )
    .await;
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "SPAXX", "name": "Fidelity Government Money Market"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap().to_string();
    research_template(&platform, &security_id, "SPAXX").await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account["accountId"],
            "securityId": security_id,
            "openedOn": "2026-01-02",
            "quantityMinor": 10000,
            "quantityScale": 0,
            "performanceCostMinor": 1000000,
            "taxCostMinor": 1000000,
            "scale": 2
        }),
    )
    .await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "provider": "Fidelity",
            "divType": "CASH",
            "isActive": true
        }),
    )
    .await;

    let price = query_json(
        &platform,
        "CurrentPriceGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-08-26" }),
    )
    .await;
    assert_eq!(price["priceMinor"], 100);
    assert_eq!(price["scale"], 2);
    assert_eq!(price["freshness"], "current");
    assert_eq!(price["priceDerivedValid"], true);

    let set = query_json(&platform, "PriceRetrievalSetGet", serde_json::json!({})).await;
    let items = set["items"].as_array().cloned().unwrap_or_default();
    assert!(
        !items.iter().any(|i| i["symbol"] == "SPAXX"),
        "SPAXX must not be in Yahoo last-price set: {set}"
    );
    let ids = set["securityIds"].as_array().cloned().unwrap_or_default();
    assert!(
        !ids.iter().any(|id| id.as_str() == Some(security_id.as_str())),
        "SPAXX security id must not be in price retrieval ids"
    );
}

#[tokio::test]
async fn collector_set_excludes_non_paying_symbols() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .expect("open sqlite");
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Brokerage", "kind": "taxable"}),
    )
    .await;
    let payer = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "QDVO", "name": "QDVO"}),
    )
    .await;
    let non_payer = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "TSLA", "name": "Tesla"}),
    )
    .await;
    for (sec, freq, div) in [
        (payer["securityId"].as_str().unwrap(), "Monthly", "DIV-1"),
        (non_payer["securityId"].as_str().unwrap(), "None", ""),
    ] {
        let sym = if sec == payer["securityId"].as_str().unwrap() {
            "QDVO"
        } else {
            "TSLA"
        };
        if div == "DIV-1" {
            research_template(&platform, sec, sym).await;
        } else {
            // Process B needs a template to open a lot; keep collector fleet off for non-payers.
            must_ok(
                &platform,
                "RetrievalTemplateSet",
                serde_json::json!({
                    "securityId": sec,
                    "sourceSymbol": sym,
                    "declarationSource": "issuer",
                    "collectorEnabled": false,
                    "lookbackCount": 12
                }),
            )
            .await;
        }
        must_ok(
            &platform,
            "LotOpen",
            serde_json::json!({
                "accountId": account["accountId"],
                "securityId": sec,
                "openedOn": "2026-01-02",
                "quantityMinor": 10,
                "quantityScale": 0,
                "performanceCostMinor": 1000,
                "taxCostMinor": 1000,
                "scale": 2
            }),
        )
        .await;
        must_ok(
            &platform,
            "PositionCharacteristicUpsert",
            serde_json::json!({
                "securityId": sec,
                "paymentFrequency": freq,
                "provider": if div == "DIV-1" { "Amplify" } else { "" },
                "divType": div,
                "isActive": true
            }),
        )
        .await;
    }
    let set = query_json(&platform, "CollectorSetGet", serde_json::json!({})).await;
    let symbols: Vec<_> = set["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|i| i["symbol"].as_str())
        .collect();
    assert!(symbols.contains(&"QDVO"), "payer missing: {set}");
    assert!(!symbols.contains(&"TSLA"), "non-payer must be excluded: {set}");
}

#[tokio::test]
async fn moneymarket_retrieve_updates_plan_and_actuals() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .expect("open sqlite");
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "FDRXX", "name": "Fidelity Government Cash Reserves"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap().to_string();
    research_template(&platform, &security_id, "FDRXX").await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account["accountId"],
            "securityId": security_id,
            "openedOn": "2026-01-02",
            "quantityMinor": 10000,
            "quantityScale": 0,
            "performanceCostMinor": 1000000,
            "taxCostMinor": 1000000,
            "scale": 2
        }),
    )
    .await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "provider": "Fidelity",
            "divType": "CASH",
            "isActive": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "declarationSource": "fidelity",
            "calendarPolicy": "issuer_calendar",
            "collectorEnabled": true
        }),
    )
    .await;

    // Broker yield already present for June — collector must not duplicate it.
    must_ok(
        &platform,
        "DividendActualRecord",
        serde_json::json!({
            "accountId": account["accountId"],
            "securityId": security_id,
            "occurredOn": "2026-06-30",
            "amountMinor": 999,
            "scale": 2,
            "idempotencyKey": "broker-june"
        }),
    )
    .await;

    let retrieved = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "FDRXX",
            "declarationSource": "fidelity",
            "candidates": [{
                "securityId": security_id,
                "amountPerShareMinor": 286,
                "amountScale": 5,
                "paymentPeriod": "2026-07-31",
                "source": "fidelity",
                "contentHash": "hash-fdrxx-1"
            }, {
                "securityId": security_id,
                "amountPerShareMinor": 276,
                "amountScale": 5,
                "paymentPeriod": "2026-06-30",
                "source": "fidelity",
                "contentHash": "hash-fdrxx-1"
            }],
            "upcomingPays": [{
                "securityId": security_id,
                "payOn": "2026-08-31",
                "source": "fidelity",
                "contentHash": "hash-fdrxx-1"
            }],
            "quote": { "priceMinor": 100, "scale": 2, "source": "par", "asOfAt": "2026-08-26" }
        }),
    )
    .await;
    assert_eq!(retrieved["ok"], true);
    assert!(retrieved["recorded"].as_u64().unwrap() >= 2);

    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-08-26" }),
    )
    .await;
    assert_eq!(inv["planKnown"], true, "expected CASH plan from MM rate: {inv}");
    assert_eq!(inv["planPerShareMinor"], 286);
    assert_eq!(inv["planScale"], 5);
    assert!(inv["planReason"]
        .as_str()
        .unwrap_or("")
        .contains("money-market rate"));

    let div = query_json(&platform, "DividendGet", serde_json::json!({})).await;
    let actuals = div["actuals"].as_array().cloned().unwrap_or_default();
    assert!(
        actuals.iter().any(|a| a["occurredOn"] == "2026-07-31"),
        "expected MM actual for July: {div}"
    );
    let june: Vec<_> = actuals
        .iter()
        .filter(|a| a["occurredOn"] == "2026-06-30")
        .collect();
    assert_eq!(june.len(), 1, "broker June must not be duplicated: {div}");
    assert_eq!(june[0]["amountMinor"], 999);
}

#[tokio::test]
async fn moneymarket_rate_change_writes_new_plan_version() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .expect("open sqlite");
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "SPAXX", "name": "SPAXX"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap().to_string();
    research_template(&platform, &security_id, "SPAXX").await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account["accountId"],
            "securityId": security_id,
            "openedOn": "2026-01-02",
            "quantityMinor": 1000,
            "quantityScale": 0,
            "performanceCostMinor": 100000,
            "taxCostMinor": 100000,
            "scale": 2
        }),
    )
    .await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "provider": "Fidelity",
            "divType": "CASH",
            "isActive": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "SPAXX",
            "declarationSource": "fidelity",
            "candidates": [{
                "amountPerShareMinor": 300,
                "amountScale": 5,
                "paymentPeriod": "2026-06-30",
                "source": "fidelity"
            }]
        }),
    )
    .await;
    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "SPAXX",
            "declarationSource": "fidelity",
            "candidates": [{
                "amountPerShareMinor": 280,
                "amountScale": 5,
                "paymentPeriod": "2026-07-31",
                "source": "fidelity"
            }]
        }),
    )
    .await;
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-08-26" }),
    )
    .await;
    assert_eq!(inv["planPerShareMinor"], 280, "rate change should rewrite Plan: {inv}");
}

#[tokio::test]
async fn collector_retrieve_persists_winning_source_url() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .expect("open sqlite");
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "EFC", "name": "EFC"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap().to_string();
    research_template(&platform, &security_id, "EFC").await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account["accountId"],
            "securityId": security_id,
            "openedOn": "2026-01-02",
            "quantityMinor": 10,
            "quantityScale": 0,
            "performanceCostMinor": 1000,
            "taxCostMinor": 1000,
            "scale": 2
        }),
    )
    .await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "provider": "Ellington",
            "divType": "DIV-1",
            "isActive": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "declarationSource": "ellington",
            "collectorEnabled": true
        }),
    )
    .await;
    let win_url = "https://www.ellingtonfinancial.com/dividends-common-stock";
    let retrieved = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "EFC",
            "declarationSource": "ellington",
            "fetchedSourceUrl": win_url,
            "candidates": [{
                "securityId": security_id,
                "amountPerShareMinor": 13,
                "amountScale": 2,
                "paymentPeriod": "2026-09-30",
                "source": "ellington",
                "fetchedSourceUrl": win_url,
                "contentHash": "hash-efc-1"
            }]
        }),
    )
    .await;
    assert!(retrieved["ok"].as_bool().unwrap_or(false));
    assert!(retrieved["recorded"].as_u64().unwrap_or(0) >= 1);

    let set = query_json(&platform, "CollectorSetGet", serde_json::json!({})).await;
    let item = set["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["symbol"] == "EFC")
        .expect("EFC in collector set");
    assert_eq!(item["sourceUrl"].as_str().unwrap_or(""), win_url);
    assert_eq!(item["lastRunOk"], true);
}

#[tokio::test]
async fn collector_fleet_zero_misses_after_injected_refresh() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .expect("open sqlite");
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    for (symbol, source) in [("EFC", "ellington"), ("JEPQ", "jpmorgan")] {
        let security = must_ok(
            &platform,
            "SecurityRegister",
            serde_json::json!({"symbol": symbol, "name": symbol}),
        )
        .await;
        let security_id = security["securityId"].as_str().unwrap().to_string();
        research_template(&platform, &security_id, symbol).await;
        must_ok(
            &platform,
            "LotOpen",
            serde_json::json!({
                "accountId": account["accountId"],
                "securityId": security_id,
                "openedOn": "2026-01-02",
                "quantityMinor": 10,
                "quantityScale": 0,
                "performanceCostMinor": 1000,
                "taxCostMinor": 1000,
                "scale": 2
            }),
        )
        .await;
        must_ok(
            &platform,
            "PositionCharacteristicUpsert",
            serde_json::json!({
                "securityId": security_id,
                "paymentFrequency": "Monthly",
                "provider": "Test",
                "divType": "DIV-1",
                "isActive": true
            }),
        )
        .await;
        must_ok(
            &platform,
            "RetrievalTemplateSet",
            serde_json::json!({
                "securityId": security_id,
                "declarationSource": source,
                "collectorEnabled": true
            }),
        )
        .await;
        let retrieved = must_ok(
            &platform,
            "CollectorRetrieve",
            serde_json::json!({
                "securityId": security_id,
                "symbol": symbol,
                "declarationSource": source,
                "candidates": [{
                    "securityId": security_id,
                    "amountPerShareMinor": 100,
                    "amountScale": 2,
                    "paymentPeriod": "2026-08-31",
                    "source": source,
                    "contentHash": format!("hash-{symbol}")
                }]
            }),
        )
        .await;
        assert_eq!(retrieved["ok"], true, "{symbol} retrieve must ok");
        assert!(retrieved["recorded"].as_u64().unwrap_or(0) >= 1);
    }
    let set = query_json(&platform, "CollectorSetGet", serde_json::json!({})).await;
    let enabled: Vec<_> = set["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["collectorEnabled"] == true)
        .collect();
    assert!(
        enabled.iter().all(|r| r["lastRunOk"] == true),
        "fleet gate: every enabled symbol lastRunOk: {set:?}"
    );
}

