//! Collector standing order, retrieve_run ledger, and on-demand retrieve.

use application_core::contracts::{
    CommandRequest, QueryRequest, WorkTicketRecord, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::ports::canonical::Canonical;
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
    golden_harness::complete_collector_for_first_lot(platform, security_id, symbol)
        .await
        .expect("complete collector");
}

const MONTHLY_12: &[&str] = &[
    "2025-09-30",
    "2025-10-31",
    "2025-11-28",
    "2025-12-31",
    "2026-01-30",
    "2026-02-27",
    "2026-03-31",
    "2026-04-30",
    "2026-05-29",
    "2026-06-30",
    "2026-07-31",
    "2026-08-31",
];

fn monthly_paid_candidates(
    security_id: &str,
    source: &str,
    amount: i64,
    scale: u8,
) -> Vec<serde_json::Value> {
    MONTHLY_12
        .iter()
        .map(|pay| {
            serde_json::json!({
                "securityId": security_id,
                "amountPerShareMinor": amount,
                "amountScale": scale,
                "paymentPeriod": pay,
                "source": source,
                "contentHash": format!("hash-{source}")
            })
        })
        .collect()
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
            "candidates": monthly_paid_candidates(&security_id, "amplify", 38826, 5),
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
    assert_eq!(payload["postChecks"]["c5Match"], true);
    assert_eq!(payload["postChecks"]["previousAligned"], true);
    assert_eq!(payload["postChecks"]["cadenceOk"], true);
    assert_eq!(payload["postChecks"]["amountVariationOk"], true);
    assert_eq!(payload["postChecks"]["lookbackOk"], true);
    assert_eq!(payload["postChecks"]["amountVariationPct"], 30);

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
            golden_harness::complete_collector_for_first_lot(&platform, sec, sym)
                .await
                .expect("complete collector");
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
                "replaceCadence": true,
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
    let summary = query_json(&platform, "DataSummaryGet", serde_json::json!({})).await;
    assert_eq!(summary["symbolCount"].as_u64(), Some(2));
    assert_eq!(
        summary["declarationCollectorCount"].as_u64(),
        Some(1),
        "daily retrieve expects one enabled collector, not all open-lot symbols: {summary}"
    );
    assert_eq!(
        summary["declarationCount"].as_u64(),
        Some(0),
        "no retrieve yet today: {summary}"
    );
    assert_eq!(
        summary["declarationAsOf"].as_str().map(|s| s.len()),
        Some(10)
    );
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
        .contains("money-market 7-day yield"));

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
async fn cash_seven_day_yield_sets_monthly_plan_not_actuals() {
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
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "SPAXX",
            "declarationSource": "fidelity",
            "divType": "CASH",
            "candidates": [{
                "kind": "cash_rate",
                "planOnly": true,
                "amountPerShareMinor": 2783,
                "amountScale": 6,
                "annualYieldBps": 334,
                "sevenDayYield": "3.34",
                "paymentPeriod": "2026-09-07",
                "source": "fidelity"
            }]
        }),
    )
    .await;
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-09-08" }),
    )
    .await;
    assert_eq!(inv["planKnown"], true, "{inv}");
    assert_eq!(inv["planPerShareMinor"], 2783);
    assert_eq!(inv["planScale"], 6);
    let reason = inv["planReason"].as_str().unwrap_or("");
    assert!(reason.contains("3.34"), "{reason}");
    let div = query_json(&platform, "DividendGet", serde_json::json!({})).await;
    let actuals = div["actuals"].as_array().cloned().unwrap_or_default();
    assert!(
        actuals.iter().all(|a| a["occurredOn"] != "2026-09-07"),
        "7-day yield must not post a fake actual: {div}"
    );
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
            "candidates": monthly_paid_candidates(&security_id, "ellington", 13, 2),
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
                "candidates": monthly_paid_candidates(&security_id, source, 100, 2),
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


/// forceRefresh false + last run ok today must not duplicate declaration periods
/// or append another identical Yahoo quote for the same as-of/source.
#[tokio::test]
async fn collector_retrieve_fresh_today_does_not_duplicate_decls_or_quotes() {
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
        serde_json::json!({"symbol": "HAKY", "name": "HAKY"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap().to_string();
    research_template(&platform, &security_id, "HAKY").await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account["accountId"],
            "securityId": security_id,
            "openedOn": "2026-01-02",
            "quantityMinor": 10,
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
            "underlying": "HACK",
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
            "collectorEnabled": true,
            "sourceUrl": "https://amplifyetfs.com/haky/#distributions"
        }),
    )
    .await;

    let candidates = monthly_paid_candidates(&security_id, "amplify", 12, 2);
    let quote = serde_json::json!({
        "securityId": security_id,
        "priceMinor": 2550,
        "scale": 2,
        "source": "yahoo",
        "asOfAt": "2026-08-30"
    });

    let first = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "HAKY",
            "declarationSource": "amplify",
            "candidates": candidates.clone(),
            "quote": quote.clone(),
            "contentHash": "hash-haky-fresh",
            "forceRefresh": false
        }),
    )
    .await;
    assert_eq!(first["ok"], true);

    // Second run: forceRefresh false + unchanged (fresh today) — no duplicate periods / quotes.
    let second = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "HAKY",
            "declarationSource": "amplify",
            "candidates": candidates.clone(),
            "quote": quote.clone(),
            "contentHash": "hash-haky-fresh",
            "unchanged": true,
            "forceRefresh": false,
            "lastRunOk": true,
            "lastRunAt": "2026-08-30T12:00:00Z",
            "lastContentHash": "hash-haky-fresh"
        }),
    )
    .await;
    assert!(
        second["unchanged"].as_u64().unwrap_or(0) >= 1,
        "expected unchanged: {second}"
    );

    // Identical LastPriceRefresh rows must not stack for same as-of/source.
    for _ in 0..3 {
        must_ok(
            &platform,
            "LastPriceRefresh",
            serde_json::json!({
                "quotes": [quote.clone()]
            }),
        )
        .await;
    }

    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-08-30" }),
    )
    .await;
    assert_eq!(inv["declarationCount"], 12, "periods must not duplicate: {inv}");

    let quotes = query_json(
        &platform,
        "PriceQuoteList",
        serde_json::json!({ "securityId": security_id }),
    )
    .await;
    let empty: Vec<serde_json::Value> = Vec::new();
    let quote_rows = quotes.as_array().unwrap_or(&empty);
    let yahoo_same: Vec<_> = quote_rows
        .iter()
        .filter(|q| {
            q["source"] == "yahoo"
                && q["asOfAt"].as_str().unwrap_or("").starts_with("2026-08-30")
                && q["validationStatus"] == "accepted"
        })
        .collect();
    assert_eq!(
        yahoo_same.len(),
        1,
        "identical yahoo as-of/source must not stack: {quotes}"
    );
}

async fn seed_div1_monthly(
    platform: &LocalPlatform,
    symbol: &str,
    source: &str,
) -> String {
    let account = must_ok(
        platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let security = must_ok(
        platform,
        "SecurityRegister",
        serde_json::json!({"symbol": symbol, "name": symbol}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap().to_string();
    research_template(platform, &security_id, symbol).await;
    must_ok(
        platform,
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
        platform,
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
        platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "declarationSource": source,
            "sourceSymbol": symbol,
            "sourceUrl": format!("https://example.test/{}/distributions", symbol.to_ascii_lowercase()),
            "calendarPolicy": "issuer_calendar",
            "collectorEnabled": true
        }),
    )
    .await;
    security_id
}

async fn seed_div1_monthly_incomplete(
    platform: &LocalPlatform,
    symbol: &str,
    source: &str,
) -> String {
    let account = must_ok(
        platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let security = must_ok(
        platform,
        "SecurityRegister",
        serde_json::json!({"symbol": symbol, "name": symbol}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap().to_string();
    let _account = account;
    must_ok(
        platform,
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
        platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "declarationSource": source,
            "sourceSymbol": symbol,
            "sourceUrl": format!("https://example.test/{}/distributions", symbol.to_ascii_lowercase()),
            "calendarPolicy": "issuer_calendar",
            "collectorEnabled": true
        }),
    )
    .await;
    security_id
}

#[tokio::test]
async fn collector_post_checks_amount_variation_over_30_is_loud_miss() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "HAKY", "amplify").await;
    let first = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "HAKY",
            "declarationSource": "amplify",
            "candidates": monthly_paid_candidates(&security_id, "amplify", 1000, 2),
            "pagePaid": monthly_paid_candidates(&security_id, "amplify", 1000, 2)
        }),
    )
    .await;
    assert_eq!(first["ok"], true, "{first}");
    golden_harness::complete_collector_for_first_lot(&platform, &security_id, "HAKY")
        .await
        .expect("complete before 30% rule");
    let mut page = monthly_paid_candidates(&security_id, "amplify", 1000, 2);
    page.push(serde_json::json!({
        "securityId": security_id,
        "amountPerShareMinor": 1400,
        "amountScale": 2,
        "paymentPeriod": "2026-09-30",
        "source": "amplify"
    }));
    let second = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "HAKY",
            "declarationSource": "amplify",
            "candidates": [{
                "securityId": security_id,
                "amountPerShareMinor": 1400,
                "amountScale": 2,
                "paymentPeriod": "2026-09-30",
                "source": "amplify"
            }],
            "pagePaid": page
        }),
    )
    .await;
    assert_eq!(second["ok"], true, "{second}");
    assert_eq!(second["code"], "declaration_amount_variation");
    let payload: serde_json::Value =
        serde_json::from_str(second["payloadJson"].as_str().unwrap_or("{}")).unwrap();
    assert_eq!(payload["postChecks"]["amountVariationOk"], false);
    assert_eq!(payload["postChecks"]["amountVariationPct"], 30);
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-09-30" }),
    )
    .await;
    assert!(
        inv["declarations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["paymentPeriod"] == "2026-09-30"),
        "variation still stores the row: {inv}"
    );
    let tickets = query_json(&platform, "WorkTicketList", serde_json::json!({})).await;
    assert_eq!(tickets["openCount"], 1, "{tickets}");
    assert_eq!(tickets["items"][0]["code"], "declaration_amount_variation");
    assert_eq!(tickets["items"][0]["tool"], "amount_confirm");
    let ticket_id = tickets["items"][0]["ticketId"].as_str().unwrap();
    let excepted = must_ok(
        &platform,
        "WorkTicketResolve",
        serde_json::json!({
            "ticketId": ticket_id,
            "tool": "amount_confirm",
            "action": "except"
        }),
    )
    .await;
    assert_eq!(excepted["ok"], true, "{excepted}");
    let after = query_json(&platform, "WorkTicketList", serde_json::json!({})).await;
    assert_eq!(after["openCount"], 0, "except files the ticket: {after}");
}

#[tokio::test]
async fn stored_history_tickets_amount_variation_even_if_incomplete() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly_incomplete(&platform, "HAKY", "amplify").await;
    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "HAKY",
            "declarationSource": "amplify",
            "candidates": monthly_paid_candidates(&security_id, "amplify", 1000, 2),
            "pagePaid": monthly_paid_candidates(&security_id, "amplify", 1000, 2)
        }),
    )
    .await;
    let mut page = monthly_paid_candidates(&security_id, "amplify", 1000, 2);
    page.push(serde_json::json!({
        "securityId": security_id,
        "amountPerShareMinor": 1400,
        "amountScale": 2,
        "paymentPeriod": "2026-09-30",
        "source": "amplify"
    }));
    let second = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "HAKY",
            "declarationSource": "amplify",
            "candidates": [{
                "securityId": security_id,
                "amountPerShareMinor": 1400,
                "amountScale": 2,
                "paymentPeriod": "2026-09-30",
                "source": "amplify"
            }],
            "pagePaid": page
        }),
    )
    .await;
    assert_eq!(second["ok"], true, "{second}");
    assert_eq!(second["code"], "declaration_amount_variation", "{second}");
    let tickets = query_json(&platform, "WorkTicketList", serde_json::json!({})).await;
    let open_var = tickets["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["code"] == "declaration_amount_variation" && t["status"] == "open");
    assert!(open_var, "stored pays are facts; ticket the change: {tickets}");
}

#[tokio::test]
async fn establish_does_not_replace_stored_pays() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly_incomplete(&platform, "HAKY", "amplify").await;
    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "HAKY",
            "declarationSource": "amplify",
            "candidates": monthly_paid_candidates(&security_id, "import", 100, 2),
            "pagePaid": monthly_paid_candidates(&security_id, "import", 100, 2)
        }),
    )
    .await;
    let issuer = monthly_paid_candidates(&security_id, "amplify", 250, 2);
    let established = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "HAKY",
            "declarationSource": "amplify",
            "establish": true,
            "forceRefresh": true,
            "candidates": issuer,
            "pagePaid": issuer
        }),
    )
    .await;
    assert_eq!(established["ok"], true, "{established}");
    let sid = Uuid::parse_str(&security_id).unwrap();
    let decls = platform.issuer_declaration_list(sid).await.unwrap();
    let feb = decls
        .iter()
        .find(|d| d.payment_period == "2026-02-27" && d.amount_per_share_minor.unwrap_or(0) > 0)
        .expect("feb pay");
    assert_eq!(
        feb.amount_per_share_minor,
        Some(100),
        "establish must keep stored pays"
    );
}

#[tokio::test]
async fn collector_post_checks_weekly_amount_swing_raises_variation_not_miss() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "AMDW", "roundhill").await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Weekly",
            "provider": "Roundhill",
            "divType": "DIV-1",
            "isActive": true,
            "replaceCadence": true
        }),
    )
    .await;
    let weeks = [
        "2026-05-26",
        "2026-06-02",
        "2026-06-09",
        "2026-06-16",
        "2026-06-23",
        "2026-06-30",
        "2026-07-07",
        "2026-07-14",
        "2026-07-21",
        "2026-07-28",
        "2026-08-04",
        "2026-08-11",
    ];
    let page: Vec<serde_json::Value> = weeks
        .iter()
        .enumerate()
        .map(|(i, p)| {
            serde_json::json!({
                "securityId": security_id,
                "amountPerShareMinor": if i == 11 { 1400 } else { 1000 },
                "amountScale": 2,
                "paymentPeriod": p,
                "source": "roundhill"
            })
        })
        .collect();
    let baseline: Vec<serde_json::Value> = weeks
        .iter()
        .map(|p| {
            serde_json::json!({
                "securityId": security_id,
                "amountPerShareMinor": 1000,
                "amountScale": 2,
                "paymentPeriod": p,
                "source": "roundhill"
            })
        })
        .collect();
    let seed = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "AMDW",
            "declarationSource": "roundhill",
            "candidates": baseline,
            "pagePaid": baseline
        }),
    )
    .await;
    assert_eq!(seed["ok"], true, "{seed}");
    golden_harness::complete_collector_for_first_lot_as(
        &platform,
        &security_id,
        "AMDW",
        "Weekly",
        true,
    )
    .await
    .expect("complete before 30% rule");
    let first = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "AMDW",
            "declarationSource": "roundhill",
            "candidates": page,
            "pagePaid": page
        }),
    )
    .await;
    assert_eq!(first["ok"], true, "weekly swing is not a retrieve miss: {first}");
    assert_eq!(first["code"], "declaration_amount_variation", "{first}");
    let tickets = query_json(&platform, "WorkTicketList", serde_json::json!({})).await;
    assert_eq!(tickets["openCount"], 1, "{tickets}");
    assert_eq!(tickets["items"][0]["code"], "declaration_amount_variation");
    assert_eq!(tickets["items"][0]["tool"], "amount_confirm");
}

#[tokio::test]
async fn collector_post_checks_cadence_mismatch_is_loud_miss() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "NVDY", "yieldmax").await;
    let weeks = [
        "2026-04-03",
        "2026-04-10",
        "2026-04-17",
        "2026-04-24",
        "2026-05-01",
        "2026-05-08",
        "2026-05-15",
        "2026-05-22",
        "2026-05-29",
        "2026-06-05",
        "2026-06-12",
        "2026-06-19",
    ];
    let weekly: Vec<serde_json::Value> = weeks
        .iter()
        .map(|pay| {
            serde_json::json!({
                "securityId": security_id,
                "amountPerShareMinor": 100,
                "amountScale": 2,
                "paymentPeriod": pay,
                "source": "yieldmax"
            })
        })
        .collect();
    let retrieved = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "NVDY",
            "declarationSource": "yieldmax",
            "candidates": weekly.clone(),
            "pagePaid": weekly
        }),
    )
    .await;
    assert_eq!(retrieved["ok"], false);
    assert_eq!(retrieved["code"], "declaration_cadence_mismatch");
}

#[tokio::test]
async fn collector_post_checks_dropped_history_is_loud_miss() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "PAY1", "amplify").await;
    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "PAY1",
            "declarationSource": "amplify",
            "candidates": monthly_paid_candidates(&security_id, "amplify", 100, 2),
            "pagePaid": monthly_paid_candidates(&security_id, "amplify", 100, 2)
        }),
    )
    .await;
    let mut page = monthly_paid_candidates(&security_id, "amplify", 100, 2);
    page.remove(5);
    let dropped = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "PAY1",
            "declarationSource": "amplify",
            "candidates": [],
            "pagePaid": page
        }),
    )
    .await;
    assert_eq!(dropped["ok"], true, "short increment keeps last_run ok: {dropped}");
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-09-07" }),
    )
    .await;
    let n = inv["declarations"].as_array().map(|a| a.len()).unwrap_or(0);
    assert_eq!(n, 12, "stored pays stay: {inv}");
}

#[tokio::test]
async fn collector_post_checks_c5_amount_mismatch_is_loud_miss() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "PAY1", "amplify").await;
    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "PAY1",
            "declarationSource": "amplify",
            "candidates": monthly_paid_candidates(&security_id, "amplify", 100, 2),
            "pagePaid": monthly_paid_candidates(&security_id, "amplify", 100, 2)
        }),
    )
    .await;
    let mut page = monthly_paid_candidates(&security_id, "amplify", 100, 2);
    page[11]["amountPerShareMinor"] = serde_json::json!(999);
    let mismatch = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "PAY1",
            "declarationSource": "amplify",
            "candidates": [],
            "pagePaid": page
        }),
    )
    .await;
    assert_eq!(mismatch["ok"], true, "overlap amount change is not last_run fail: {mismatch}");
    assert_eq!(mismatch["code"], "declaration_amount_variation");
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-09-07" }),
    )
    .await;
    let decls = inv["declarations"].as_array().cloned().unwrap_or_default();
    assert_eq!(decls.len(), 12);
    let last = decls.iter().find(|d| d["paymentPeriod"] == "2026-08-31");
    assert_eq!(last.unwrap()["amountPerShareMinor"], 100);
}

#[tokio::test]
async fn collector_lookback_applies_when_adapter_registered_even_if_div_type_blank() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "HAKY", "name": "HAKY"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap().to_string();
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "provider": "Amplify",
            "divType": "",
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
            "symbol": "HAKY",
            "declarationSource": "amplify",
            "candidates": [{
                "securityId": security_id,
                "amountPerShareMinor": 100,
                "amountScale": 2,
                "paymentPeriod": "2026-08-31",
                "source": "amplify"
            }]
        }),
    )
    .await;
    assert_eq!(retrieved["ok"], false);
    assert_eq!(retrieved["code"], "declaration_lookback_short");
}

#[tokio::test]
async fn roc_retrieve_does_not_change_declaration_last_run_ok() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "HAKY", "amplify").await;
    let first = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "HAKY",
            "declarationSource": "amplify",
            "candidates": monthly_paid_candidates(&security_id, "amplify", 100, 2),
            "pagePaid": monthly_paid_candidates(&security_id, "amplify", 100, 2)
        }),
    )
    .await;
    assert_eq!(first["ok"], true);
    must_ok(
        &platform,
        "RocResearchRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "HAKY",
            "declarationSource": "amplify",
            "asOfDate": "2026-09-02",
            "candidates": [{
                "rocPctMinor": 10000,
                "scale": 2,
                "taxYear": "2026",
                "source": "19a-1",
                "sourceUrl": "https://amplifyetfs.com/wp-content/uploads/files/19a-1_Notice_05-29-26_HAKY.pdf",
                "method": "19a-1-current-year",
                "asOf": "2026-05-29",
                "kind": "estimate",
                "ownerOverride": false
            }]
        }),
    )
    .await;
    let set = query_json(&platform, "CollectorSetGet", serde_json::json!({})).await;
    let item = set["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["symbol"] == "HAKY")
        .expect("HAKY");
    assert_eq!(item["lastRunOk"], true);
}

#[tokio::test]
async fn empty_retrieve_raises_one_work_ticket() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "HAKY", "amplify").await;
    let missed = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "HAKY",
            "declarationSource": "amplify",
            "candidates": [],
            "misses": [{
                "securityId": security_id,
                "symbol": "HAKY",
                "code": "declaration_retrieve_miss",
                "reason": "Issuer page empty.",
                "declarationSource": "amplify"
            }]
        }),
    )
    .await;
    assert_eq!(missed["ok"], false);
    let tickets = query_json(&platform, "WorkTicketList", serde_json::json!({})).await;
    assert_eq!(tickets["openCount"], 1, "{tickets}");
    let items = tickets["items"].as_array().unwrap();
    assert_eq!(items[0]["code"], "declaration_retrieve_miss");
    assert_eq!(items[0]["tool"], "retry_retrieve");
    assert_eq!(items[0]["symbol"], "HAKY");
    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "HAKY",
            "declarationSource": "amplify",
            "candidates": [],
            "misses": [{
                "securityId": security_id,
                "symbol": "HAKY",
                "code": "declaration_retrieve_miss",
                "reason": "Issuer page empty.",
            }]
        }),
    )
    .await;
    let tickets2 = query_json(&platform, "WorkTicketList", serde_json::json!({})).await;
    assert_eq!(tickets2["openCount"], 1, "dedup: {tickets2}");
}

#[tokio::test]
async fn amplify_retry_refuses_roundhill_url_and_keeps_adapter() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "HAKY", "amplify").await;
    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "HAKY",
            "declarationSource": "amplify",
            "candidates": [],
            "misses": [{
                "securityId": security_id,
                "symbol": "HAKY",
                "code": "declaration_retrieve_miss",
                "reason": "Issuer page empty."
            }]
        }),
    )
    .await;
    let tickets = query_json(&platform, "WorkTicketList", serde_json::json!({})).await;
    let ticket_id = tickets["items"][0]["ticketId"].as_str().unwrap();
    let refused = must_ok(
        &platform,
        "WorkTicketResolve",
        serde_json::json!({
            "ticketId": ticket_id,
            "tool": "retry_retrieve",
            "sourceUrl": "https://www.roundhillinvestments.com/etf/haky/"
        }),
    )
    .await;
    assert_eq!(refused["ok"], false);
    assert_eq!(refused["code"], "adapter_url_mismatch");
    let set = query_json(&platform, "CollectorSetGet", serde_json::json!({})).await;
    let item = set["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["symbol"] == "HAKY")
        .expect("HAKY");
    assert_eq!(item["declarationSource"], "amplify");
    assert_ne!(
        item["sourceUrl"].as_str().unwrap_or(""),
        "https://www.roundhillinvestments.com/etf/haky/"
    );
    let still = query_json(&platform, "WorkTicketList", serde_json::json!({})).await;
    assert_eq!(still["openCount"], 1);
    assert_eq!(still["items"][0]["status"], "open");
}

#[tokio::test]
async fn amplify_retry_accepts_amplify_url_then_retrieve_files_ticket() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "HAKY", "amplify").await;
    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "HAKY",
            "declarationSource": "amplify",
            "candidates": [],
            "misses": [{
                "securityId": security_id,
                "symbol": "HAKY",
                "code": "declaration_retrieve_miss",
                "reason": "Issuer page empty."
            }]
        }),
    )
    .await;
    let tickets = query_json(&platform, "WorkTicketList", serde_json::json!({})).await;
    let ticket_id = tickets["items"][0]["ticketId"].as_str().unwrap();
    let saved = must_ok(
        &platform,
        "WorkTicketResolve",
        serde_json::json!({
            "ticketId": ticket_id,
            "tool": "retry_retrieve",
            "sourceUrl": "https://amplifyetfs.com/haky/#distributions"
        }),
    )
    .await;
    assert_eq!(saved["ok"], true, "{saved}");
    let set = query_json(&platform, "CollectorSetGet", serde_json::json!({})).await;
    let item = set["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["symbol"] == "HAKY")
        .expect("HAKY");
    assert_eq!(item["declarationSource"], "amplify");
    assert_eq!(
        item["sourceUrl"],
        "https://amplifyetfs.com/haky/#distributions"
    );
    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "HAKY",
            "declarationSource": "amplify",
            "candidates": monthly_paid_candidates(&security_id, "amplify", 100, 2),
            "pagePaid": monthly_paid_candidates(&security_id, "amplify", 100, 2)
        }),
    )
    .await;
    let after = query_json(
        &platform,
        "WorkTicketList",
        serde_json::json!({ "status": "open" }),
    )
    .await;
    assert_eq!(after["openCount"], 0, "auto-file: {after}");
}

#[tokio::test]
async fn identity_save_not_blocked_by_open_ticket() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "HAKY", "amplify").await;
    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "HAKY",
            "declarationSource": "amplify",
            "candidates": [],
            "misses": [{
                "securityId": security_id,
                "symbol": "HAKY",
                "code": "declaration_retrieve_miss",
                "reason": "Issuer page empty."
            }]
        }),
    )
    .await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "divType": "DIV-1",
            "underlying": "HACK",
            "isActive": true
        }),
    )
    .await;
    let tickets = query_json(&platform, "WorkTicketList", serde_json::json!({})).await;
    assert_eq!(tickets["openCount"], 1);
}

#[tokio::test]
async fn sync_misses_tickets_latest_retrieve_miss_when_last_run_ok() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "AMDY", "yieldmax").await;
    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "AMDY",
            "declarationSource": "yieldmax",
            "unchanged": true,
            "candidates": monthly_paid_candidates(&security_id, "yieldmax", 4843, 4),
            "pagePaid": monthly_paid_candidates(&security_id, "yieldmax", 4843, 4)
        }),
    )
    .await;
    let before = query_json(&platform, "WorkTicketList", serde_json::json!({})).await;
    assert_eq!(before["openCount"], 0, "unchanged must not ticket: {before}");
    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "AMDY",
            "declarationSource": "yieldmax",
            "candidates": [],
            "misses": [{
                "securityId": security_id,
                "symbol": "AMDY",
                "code": "declaration_retrieve_miss",
                "reason": "Issuer page changed; unparseable."
            }]
        }),
    )
    .await;
    let after_miss = query_json(&platform, "WorkTicketList", serde_json::json!({})).await;
    assert_eq!(after_miss["openCount"], 1, "live miss tickets: {after_miss}");
    let ticket_id = after_miss["items"][0]["ticketId"].as_str().unwrap();
    must_ok(
        &platform,
        "WorkTicketFile",
        serde_json::json!({
            "ticketId": ticket_id,
            "note": "filed before last_run overwrite"
        }),
    )
    .await;
    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "AMDY",
            "declarationSource": "yieldmax",
            "unchanged": true,
            "candidates": monthly_paid_candidates(&security_id, "yieldmax", 4843, 4),
            "pagePaid": monthly_paid_candidates(&security_id, "yieldmax", 4843, 4)
        }),
    )
    .await;
    let filed = query_json(&platform, "WorkTicketList", serde_json::json!({})).await;
    assert_eq!(filed["openCount"], 0, "unchanged must not auto-reopen: {filed}");
    let synced = must_ok(&platform, "WorkTicketSyncMisses", serde_json::json!({})).await;
    let after_sync = query_json(&platform, "WorkTicketList", serde_json::json!({})).await;
    // Latest run is unchanged ok, so sync must not reopen owner-filed or invent a ticket.
    assert_eq!(synced["raised"], 0, "{synced}");
    assert_eq!(after_sync["openCount"], 0, "{after_sync}");
}

#[tokio::test]
async fn sync_misses_tickets_unticketed_declaration_refresh_miss() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "CONY", "yieldmax").await;
    must_ok(
        &platform,
        "DeclarationRefresh",
        serde_json::json!({
            "misses": [{
                "securityId": security_id,
                "symbol": "CONY",
                "code": "declaration_retrieve_miss",
                "reason": "Issuer page changed; unparseable."
            }]
        }),
    )
    .await;
    let before = query_json(&platform, "WorkTicketList", serde_json::json!({})).await;
    assert_eq!(
        before["openCount"], 0,
        "DeclarationRefresh miss must not have ticketed yet: {before}"
    );
    let synced = must_ok(&platform, "WorkTicketSyncMisses", serde_json::json!({})).await;
    assert_eq!(synced["raised"], 1, "{synced}");
    let after = query_json(&platform, "WorkTicketList", serde_json::json!({})).await;
    assert_eq!(after["openCount"], 1, "{after}");
    assert_eq!(after["items"][0]["symbol"], "CONY");
    assert_eq!(after["items"][0]["code"], "declaration_retrieve_miss");
    let again = must_ok(&platform, "WorkTicketSyncMisses", serde_json::json!({})).await;
    assert_eq!(again["raised"], 0, "idempotent: {again}");
    assert_eq!(
        query_json(&platform, "WorkTicketList", serde_json::json!({})).await["openCount"],
        1
    );
}

#[test]
fn tickets_nav_opens_all_symbol_queue() {
    let root = golden_harness::repo_root();
    let app = std::fs::read_to_string(root.join("apps/desktop/src/App.tsx")).unwrap();
    let ui = std::fs::read_to_string(root.join("packages/ui-components/src/index.tsx")).unwrap();
    assert!(
        app.contains("navButton(\"tickets\", \"Tickets\")"),
        "main menu must include Tickets"
    );
    assert!(
        app.contains("screen === \"tickets\""),
        "Tickets screen must render the full queue"
    );
    assert!(
        app.contains("executeCommand(\"WorkTicketSyncMisses\""),
        "desktop must ticket missed collectors that have no open ticket"
    );
    assert!(
        app.contains("formatCollectorClock") && app.contains("As of {formatCollectorClock"),
        "declaration status must show date and time"
    );
    assert!(
        !app.contains("DIV-1 compliance") && !app.contains("Required paid"),
        "Collectors must not keep the DIV-1 compliance date dump"
    );
    assert!(
        app.contains("aria-label=\"Missing collector URLs\"") && app.contains("Apply URLs"),
        "failing DIV-1/CASH names without a seed URL must get a fill-in grid"
    );
    assert!(
        ui.contains("Collector to compare")
            && ui.contains("roc_pct_2024_actual")
            && ui.contains("roc_pct_2026_estimate")
            && ui.contains("declared_per_share")
            && ui.contains("paymentPeriod"),
        "collector proof must be one symbol with Positions ROC columns and paymentPeriod/declared amount rows"
    );
}

#[test]
fn work_ticket_recreate_adapter_opens_add_position() {
    let root = golden_harness::repo_root();
    let ui = std::fs::read_to_string(root.join("packages/ui-components/src/index.tsx")).unwrap();
    let app = std::fs::read_to_string(root.join("apps/desktop/src/App.tsx")).unwrap();
    assert!(
        ui.contains("Recreate adapter"),
        "ticket queue must offer Recreate adapter"
    );
    assert!(
        ui.contains("onRecreateAdapter"),
        "WorkTicketQueue must take onRecreateAdapter"
    );
    assert!(
        ui.contains("aria-label={`Recreate adapter ${symbol}`}"),
        "Recreate adapter must have an accessible name with the ticker"
    );
    assert!(
        app.contains("openRecreateAdapter"),
        "desktop must open Add Position from the ticket"
    );
    assert_eq!(
        app.matches("openRecreateAdapter(t as WorkTicketRecord)").count(),
        3,
        "Collectors, Position Details, and Tickets must wire Recreate adapter"
    );
    let fn_start = app
        .find("const openRecreateAdapter")
        .expect("openRecreateAdapter");
    let fn_body = &app[fn_start..fn_start + 1800];
    assert!(
        fn_body.contains("setScreen(\"new-investment\")"),
        "openRecreateAdapter must switch to Add Position: {fn_body}"
    );
    assert!(
        fn_body.contains("setWizSymbol(symbol)"),
        "openRecreateAdapter must prefill Symbol: {fn_body}"
    );
    assert!(
        fn_body.contains("setWizSourceUrl(\"\")"),
        "openRecreateAdapter must leave Distribution URL empty so the owner pastes it: {fn_body}"
    );
    assert!(
        ui.contains("aria-label={`Retry ${symbol}`}"),
        "Retry must be a button that uses the stored URL, not a URL field"
    );
    assert!(
        ui.contains("Retrying {retryingSymbol"),
        "Retry must announce progress with the ticker"
    );
    assert!(
        ui.contains("<progress aria-label={`Retrying ${retryingSymbol || \"symbol\"}`} />")
            || ui.contains("aria-label={`Retrying ${retryingSymbol"),
        "Retry must show a progress bar"
    );
    assert!(
        ui.contains("Retrying…"),
        "Retry button must switch to Retrying while the retrieve runs"
    );
    assert!(
        ui.contains("retryingTicketId"),
        "WorkTicketQueue must take retryingTicketId"
    );
    assert!(
        !ui.contains("Retry URL (same adapter)"),
        "ticket queue must not ask for a Retry URL"
    );
    assert!(
        ui.contains("Except") && ui.contains("Reject"),
        "amount variation must offer Except and Reject"
    );
    assert!(
        app.contains("resolveAmountConfirm"),
        "desktop must Except or Reject variation tickets"
    );
}

#[tokio::test]
#[ignore]
async fn live_amdw_collector_retrieve_and_close() {
    let app_dir = std::path::PathBuf::from(
        std::env::var("LOCALAPPDATA").expect("LOCALAPPDATA"),
    )
    .join("com.finos.desktop");
    let platform = LocalPlatform::open(&app_dir).await.expect("open live sqlite");
    let set = query_json(&platform, "CollectorSetGet", serde_json::json!({})).await;
    let row = set["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["symbol"] == "AMDW")
        .expect("AMDW in collector set");
    let security_id = row["securityId"].as_str().unwrap().to_string();
    let target = import_engine::DeclarationTarget {
        security_id: security_id.clone(),
        symbol: "AMDW".into(),
        declaration_source: row["declarationSource"].as_str().unwrap_or("roundhill").into(),
        source_symbol: row["sourceSymbol"]
            .as_str()
            .or_else(|| row["symbol"].as_str())
            .unwrap_or("AMDW")
            .into(),
        source_url: row["sourceUrl"].as_str().unwrap_or("").into(),
        last_content_hash: String::new(),
        div_type: row["divType"].as_str().unwrap_or("DIV-1").into(),
        force_refresh: true,
        last_run_ok: false,
        last_run_at: String::new(),
        inception_on: row["inceptionOn"].as_str().unwrap_or("").into(),
        payment_frequency: row["paymentFrequency"].as_str().unwrap_or("Weekly").into(),
        paid_count: 0,
        known_payment_periods: Vec::new(),
        known_declaration_amounts: Vec::new(),
    };
    eprintln!("AMDW collect via {} {}", target.declaration_source, target.source_url);
    let outcome = import_engine::collect_declarations_for(vec![target]);
    eprintln!(
        "AMDW collect: paid={} upcoming={} misses={} unchanged={}",
        outcome.candidates.len(),
        outcome.pay_dates.len(),
        outcome.misses.len(),
        outcome.unchanged.len()
    );
    for c in &outcome.candidates {
        eprintln!(
            "  paid {} {} / {}",
            c.get("paymentPeriod").and_then(|v| v.as_str()).unwrap_or("?"),
            c.get("amountPerShareMinor").and_then(|v| v.as_i64()).unwrap_or(0),
            c.get("amountScale").and_then(|v| v.as_u64()).unwrap_or(0)
        );
    }
    for m in &outcome.misses {
        eprintln!("  miss {m}");
    }
    assert!(
        outcome.misses.is_empty(),
        "AMDW collector miss: {:?}",
        outcome.misses
    );
    assert!(
        !outcome.candidates.is_empty(),
        "AMDW collector returned no paid rows"
    );
    let retrieved = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "AMDW",
            "declarationSource": row["declarationSource"],
            "candidates": outcome.candidates,
            "pagePaid": if outcome.page_paid.is_empty() {
                outcome.candidates.clone()
            } else {
                outcome.page_paid
            },
            "upcomingPays": outcome.pay_dates,
            "forceRefresh": true
        }),
    )
    .await;
    eprintln!("AMDW retrieve: {retrieved}");
    assert_eq!(retrieved["ok"], true, "AMDW retrieve failed: {retrieved}");
    let tickets = query_json(
        &platform,
        "WorkTicketList",
        serde_json::json!({ "securityId": security_id, "status": "open" }),
    )
    .await;
    eprintln!("AMDW tickets after retrieve: {tickets}");
    for item in tickets["items"].as_array().cloned().unwrap_or_default() {
        if item["symbol"] != "AMDW" {
            continue;
        }
        if item["status"] != "open" {
            continue;
        }
        let code = item["code"].as_str().unwrap_or("");
        let ticket_id = item["ticketId"].as_str().unwrap();
        if code == "declaration_amount_variation" {
            let resolved = must_ok(
                &platform,
                "WorkTicketResolve",
                serde_json::json!({
                    "ticketId": ticket_id,
                    "tool": "amount_confirm",
                    "action": "except",
                    "note": "excepted: Roundhill paid 2026-06-02 $1.818867 vs 2026-05-27 $0.884774 is the vendor page"
                }),
            )
            .await;
            eprintln!("except variation: {resolved}");
        } else {
            let filed = must_ok(
                &platform,
                "WorkTicketFile",
                serde_json::json!({
                    "ticketId": ticket_id,
                    "note": "AMDW live retrieve ok — leftover ticket closed"
                }),
            )
            .await;
            eprintln!("file leftover {code}: {filed}");
        }
    }
    let after = query_json(
        &platform,
        "WorkTicketList",
        serde_json::json!({ "securityId": security_id, "status": "open" }),
    )
    .await;
    let still_open: Vec<_> = after["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|t| t["symbol"] == "AMDW" && t["status"] == "open")
        .collect();
    assert!(
        still_open.is_empty(),
        "AMDW still has open tickets: {still_open:?}"
    );
}

/// Daily fleet is open lots only. Saved-but-incomplete (template, no lots) stays out.
#[tokio::test]
async fn collector_set_excludes_saved_incomplete_without_lots() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let seed = must_ok(
        &platform,
        "PositionResearchSeed",
        serde_json::json!({
            "symbol": "NEW1",
            "sourceUrl": "https://example.test/new1/distributions"
        }),
    )
    .await;
    assert!(seed["securityId"].as_str().is_some());
    let set = query_json(&platform, "CollectorSetGet", serde_json::json!({})).await;
    assert!(
        set["items"].as_array().unwrap().is_empty(),
        "saved-but-incomplete must stay out of daily fleet: {set}"
    );
}

/// Routine collect applies one new vendor payable and keeps the rest of the year.
#[tokio::test]
async fn collector_retrieve_applies_unoccurred_payable_without_replacing_year() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "PAY1", "issuer").await;
    must_ok(
        &platform,
        "IssuerPayDateReplace",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-09-05",
            "dates": [
                {"payOn": "2026-09-30", "source": "derived_walk"},
                {"payOn": "2026-10-31", "source": "derived_walk"},
                {"payOn": "2026-11-30", "source": "derived_walk"},
                {"payOn": "2026-12-31", "source": "derived_walk"}
            ]
        }),
    )
    .await;
    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "PAY1",
            "declarationSource": "issuer",
            "asOfDate": "2026-09-05",
            "candidates": monthly_paid_candidates(&security_id, "issuer", 12, 2),
            "upcomingPays": [{"payOn": "2026-09-28", "source": "issuer"}]
        }),
    )
    .await;
    let remaining = query_json(
        &platform,
        "RemainingYearIncomeGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-09-05" }),
    )
    .await;
    let dates: Vec<_> = remaining["payments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["payOn"].as_str().unwrap().to_string())
        .collect();
    assert!(dates.contains(&"2026-09-28".to_string()), "{dates:?}");
    assert!(dates.contains(&"2026-10-31".to_string()), "{dates:?}");
    assert!(dates.contains(&"2026-11-30".to_string()), "{dates:?}");
    assert!(dates.contains(&"2026-12-31".to_string()), "{dates:?}");
    assert!(!dates.contains(&"2026-09-30".to_string()), "{dates:?}");
}

/// Future placeholder with a copied amount must move to the derived/listing pay date.
#[tokio::test]
async fn collector_retrieve_moves_future_placeholder_pay_date() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "PAY1", "issuer").await;
    must_ok(
        &platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 70497,
            "amountScale": 5,
            "paymentPeriod": "2026-08-05",
            "source": "issuer",
            "enteredAt": "2026-08-05"
        }),
    )
    .await;
    must_ok(
        &platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 70497,
            "amountScale": 5,
            "paymentPeriod": "2026-10-01",
            "source": "import",
            "enteredAt": "2026-09-01"
        }),
    )
    .await;
    must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 1200,
            "amountScale": 4,
            "planningPeriodsPerYear": 12,
            "effectiveFrom": "2026-08-01",
            "decisionReason": "owner",
            "incompleteAnalysisReason": "Fewer than 6 observations"
        }),
    )
    .await;
    must_ok(
        &platform,
        "IssuerPayDateReplace",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-09-01",
            "dates": [
                {"payOn": "2026-10-01", "source": "import"},
                {"payOn": "2026-11-03", "source": "derived_walk"}
            ]
        }),
    )
    .await;
    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "PAY1",
            "declarationSource": "issuer",
            "asOfDate": "2026-09-07",
            "candidates": monthly_paid_candidates(&security_id, "issuer", 12, 2),
            "upcomingPays": [{"payOn": "2026-10-03", "source": "derived_walk"}]
        }),
    )
    .await;
    let tickets = query_json(
        &platform,
        "WorkTicketList",
        serde_json::json!({ "securityId": security_id, "status": "open" }),
    )
    .await;
    assert!(
        tickets["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["code"] != "paid_payable_supersede"),
        "{tickets}"
    );
    let remaining = query_json(
        &platform,
        "RemainingYearIncomeGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-09-07" }),
    )
    .await;
    let dates: Vec<_> = remaining["payments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["payOn"].as_str().unwrap().to_string())
        .collect();
    assert!(dates.contains(&"2026-10-03".to_string()), "{dates:?}");
    assert!(!dates.contains(&"2026-10-01".to_string()), "{dates:?}");
    let sid = Uuid::parse_str(&security_id).unwrap();
    let decls = platform.issuer_declaration_list(sid).await.unwrap();
    assert!(
        decls.iter().any(|d| d.payment_period == "2026-08-05"),
        "occurred paid history stays: {decls:?}"
    );
    assert!(
        decls
            .iter()
            .all(|d| d.payment_period != "2026-10-01" && d.payment_period != "2026-10-03"),
        "future copy must not stay as a declaration; Plan $ fills the date: {decls:?}"
    );
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-09-07" }),
    )
    .await;
    assert_eq!(inv["planPerShareMinor"], 1200);
    assert_eq!(inv["planScale"], 4);
}

/// Already-paid payable: ticket, do not silent-supersede. Plan $ unchanged.
#[tokio::test]
async fn collector_retrieve_tickets_paid_payable_and_keeps_plan() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "PAY1", "issuer").await;
    must_ok(
        &platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 12,
            "amountScale": 2,
            "paymentPeriod": "2026-08-31",
            "source": "issuer",
            "enteredAt": "2026-08-31"
        }),
    )
    .await;
    must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 1200,
            "amountScale": 4,
            "planningPeriodsPerYear": 12,
            "effectiveFrom": "2026-08-01",
            "decisionReason": "owner",
            "incompleteAnalysisReason": "Fewer than 6 observations"
        }),
    )
    .await;
    must_ok(
        &platform,
        "IssuerPayDateReplace",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-08-01",
            "dates": [
                {"payOn": "2026-08-31", "source": "derived_walk"},
                {"payOn": "2026-09-30", "source": "derived_walk"}
            ]
        }),
    )
    .await;
    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "PAY1",
            "declarationSource": "issuer",
            "asOfDate": "2026-09-05",
            "candidates": monthly_paid_candidates(&security_id, "issuer", 12, 2),
            "upcomingPays": [{"payOn": "2026-08-28", "source": "issuer"}]
        }),
    )
    .await;
    let tickets = query_json(
        &platform,
        "WorkTicketList",
        serde_json::json!({ "securityId": security_id, "status": "open" }),
    )
    .await;
    assert!(
        tickets["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["code"] == "paid_payable_supersede"),
        "{tickets}"
    );
    let remaining = query_json(
        &platform,
        "RemainingYearIncomeGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-08-22" }),
    )
    .await;
    let dates: Vec<_> = remaining["payments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["payOn"].as_str().unwrap().to_string())
        .collect();
    assert!(dates.contains(&"2026-08-31".to_string()), "{dates:?}");
    assert!(!dates.contains(&"2026-08-28".to_string()), "{dates:?}");
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-09-05" }),
    )
    .await;
    assert_eq!(inv["planPerShareMinor"], 1200);
    assert_eq!(inv["planScale"], 4);
}

/// CollectorRetrieve does not write Yahoo quotes. Price is LastPriceRefresh.
#[tokio::test]
async fn collector_retrieve_does_not_write_yahoo_quote() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "PAY1", "issuer").await;
    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "PAY1",
            "declarationSource": "issuer",
            "candidates": monthly_paid_candidates(&security_id, "issuer", 12, 2),
            "quote": {
                "securityId": security_id,
                "priceMinor": 2550,
                "scale": 2,
                "source": "yahoo",
                "asOfAt": "2026-09-05"
            }
        }),
    )
    .await;
    let quotes = query_json(
        &platform,
        "PriceQuoteList",
        serde_json::json!({ "securityId": security_id }),
    )
    .await;
    let yahoo = quotes
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|q| q["source"] == "yahoo")
        .count();
    assert_eq!(yahoo, 0, "retrieve must not write yahoo: {quotes}");
    must_ok(
        &platform,
        "LastPriceRefresh",
        serde_json::json!({
            "quotes": [{
                "securityId": security_id,
                "priceMinor": 2550,
                "scale": 2,
                "source": "yahoo",
                "asOfAt": "2026-09-05"
            }]
        }),
    )
    .await;
    let after = query_json(
        &platform,
        "PriceQuoteList",
        serde_json::json!({ "securityId": security_id }),
    )
    .await;
    let yahoo_after = after
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|q| q["source"] == "yahoo")
        .count();
    assert_eq!(yahoo_after, 1, "{after}");
}

/// Fill research gaps: underlying-only miss is not a hole. Missing ROC estimate is.
#[tokio::test]
async fn research_gaps_are_provider_frequency_div1_roc_not_underlying() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "PAY1", "issuer").await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "provider": "Issuer",
            "underlying": "",
            "divType": "DIV-1",
            "rocPct2026EstimateMinor": 5000,
            "needsRocResearch": false,
            "isActive": true
        }),
    )
    .await;
    let gaps = query_json(&platform, "ResearchGapsGet", serde_json::json!({})).await;
    assert!(
        !gaps["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["symbol"] == "PAY1"),
        "underlying is not a fill-gaps hole: {gaps}"
    );
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "provider": "Issuer",
            "underlying": "",
            "divType": "DIV-1",
            "rocPct2026EstimateMinor": null,
            "isActive": true
        }),
    )
    .await;
    let holes = query_json(&platform, "ResearchGapsGet", serde_json::json!({})).await;
    let row = holes["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["symbol"] == "PAY1")
        .expect("ROC estimate hole");
    assert_eq!(row["rocEstimateBlank"], true);
    assert_eq!(row["underlyingBlank"], true);
}

/// Planned remaining count must match remaining periods through 31 Dec (cap 4/12/52).
#[tokio::test]
async fn collector_retrieve_tickets_remaining_year_count_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "PAY1", "issuer").await;
    must_ok(
        &platform,
        "IssuerPayDateReplace",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-09-05",
            "dates": [{"payOn": "2026-09-30", "source": "derived_walk"}]
        }),
    )
    .await;
    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "PAY1",
            "declarationSource": "issuer",
            "asOfDate": "2026-09-05",
            "upcomingPays": [{"payOn": "2026-10-15", "source": "issuer"}]
        }),
    )
    .await;
    let tickets = query_json(
        &platform,
        "WorkTicketList",
        serde_json::json!({ "securityId": security_id, "status": "open" }),
    )
    .await;
    assert!(
        tickets["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["code"] == "remaining_year"),
        "{tickets}"
    );
}

/// Fill-gaps: blank DIV-1 is a hole. CASH has div type filled. Underlying is not a hole.
#[tokio::test]
async fn fill_gaps_blank_div1_is_hole_cash_is_filled() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let pay1 = seed_div1_monthly(&platform, "PAY1", "issuer").await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": pay1,
            "paymentFrequency": "Monthly",
            "provider": "Issuer",
            "divType": "",
            "rocPct2026EstimateMinor": 5000,
            "isActive": true
        }),
    )
    .await;
    let holes = query_json(&platform, "ResearchGapsGet", serde_json::json!({})).await;
    let row = holes["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["symbol"] == "PAY1")
        .expect("PAY1 blank div_type is a fill-gaps hole");
    assert_eq!(row["divTypeBlank"], true);

    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Cash", "kind": "taxable"}),
    )
    .await;
    let cash = must_ok(
        &platform,
        "PositionResearchSeed",
        serde_json::json!({
            "symbol": "CASH1",
            "sourceUrl": "https://example.test/cash1",
            "divType": "CASH",
            "skipRefresh": true
        }),
    )
    .await;
    let cash_id = cash["securityId"].as_str().unwrap();
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": cash_id,
            "divType": "CASH",
            "provider": "Broker",
            "paymentFrequency": "Monthly",
            "isActive": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account["accountId"],
            "securityId": cash_id,
            "openedOn": "2026-01-02",
            "quantityMinor": 100,
            "quantityScale": 0,
            "performanceCostMinor": 10000,
            "taxCostMinor": 10000,
            "scale": 2
        }),
    )
    .await;
    let gaps = query_json(&platform, "ResearchGapsGet", serde_json::json!({})).await;
    let cash_row = gaps["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["symbol"] == "CASH1");
    if let Some(row) = cash_row {
        assert_eq!(row["divTypeBlank"], false, "{gaps}");
        assert_eq!(row["rocEstimateBlank"], false, "{gaps}");
    }
}

/// Standing template copy: adapter, declaration URL, ROC URL, content hash.
#[tokio::test]
async fn collector_set_exposes_standing_template_copy() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "PAY1", "issuer").await;
    let roc_url = "https://issuer.example/files/19a-1_Notice_05-29-26_PAY1.pdf";
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "declarationSource": "issuer",
            "sourceSymbol": "PAY1",
            "sourceUrl": "https://example.test/pay1/distributions",
            "calendarPolicy": "derived_walk",
            "collectorEnabled": true,
            "lookbackCount": 12,
            "rocSourceUrl": roc_url,
            "lastContentHash": "hash-pay1"
        }),
    )
    .await;
    let set = query_json(&platform, "CollectorSetGet", serde_json::json!({})).await;
    let row = set["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["symbol"] == "PAY1")
        .expect("PAY1 in collector set");
    assert_eq!(row["declarationSource"], "issuer");
    assert_eq!(row["sourceUrl"], "https://example.test/pay1/distributions");
    assert_eq!(row["rocSourceUrl"], roc_url);
    assert_eq!(row["lastContentHash"], "hash-pay1");
}

/// Footer grid: complete flag, div_type, URLs, paid count, and tickets match domain.
#[tokio::test]
async fn fleet_row_matches_complete_gate_and_template() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "PAY1", "issuer").await;
    let roc_url = "https://issuer.example/files/19a-1_Notice_05-29-26_PAY1.pdf";
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "declarationSource": "issuer",
            "sourceSymbol": "PAY1",
            "sourceUrl": "https://example.test/pay1/distributions",
            "calendarPolicy": "derived_walk",
            "collectorEnabled": true,
            "lookbackCount": 12,
            "rocSourceUrl": roc_url
        }),
    )
    .await;
    let set = query_json(
        &platform,
        "CollectorSetGet",
        serde_json::json!({ "asOfDate": "2026-09-05" }),
    )
    .await;
    let row = set["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["symbol"] == "PAY1")
        .expect("PAY1 fleet row");
    assert_eq!(row["complete"], true, "{row}");
    assert_eq!(row["divType"], "DIV-1");
    assert_eq!(row["declarationSource"], "issuer");
    assert_eq!(row["sourceUrl"], "https://example.test/pay1/distributions");
    assert_eq!(row["rocSourceUrl"], roc_url);
    assert_eq!(row["fillGapsDivTypeBlank"], false);
    assert_eq!(row["fillGapsRocBlank"], false);
    assert!(row["paidDeclarationCount"].as_u64().is_some(), "{row}");

    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "provider": "Issuer",
            "divType": "",
            "isActive": true
        }),
    )
    .await;
    let holes = query_json(
        &platform,
        "CollectorSetGet",
        serde_json::json!({ "asOfDate": "2026-09-05" }),
    )
    .await;
    let incomplete = holes["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["symbol"] == "PAY1")
        .expect("PAY1 still in fleet");
    assert_eq!(incomplete["complete"], false);
    assert!(
        incomplete["gaps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g == "DIV-1"),
        "{incomplete}"
    );
    assert_eq!(incomplete["fillGapsDivTypeBlank"], true);
    assert!(incomplete["openTicketCount"].as_u64().unwrap() >= 1);
}

/// CASH row: div type filled, no ROC hole. Incomplete DIV-1 shows Complete=N.
#[tokio::test]
async fn fleet_cash_has_no_roc_hole_and_incomplete_div1_shows_gaps() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Cash", "kind": "taxable"}),
    )
    .await;
    let cash = must_ok(
        &platform,
        "PositionResearchSeed",
        serde_json::json!({
            "symbol": "CASH1",
            "sourceUrl": "https://example.test/cash1",
            "divType": "CASH",
            "skipRefresh": true
        }),
    )
    .await;
    let cash_id = cash["securityId"].as_str().unwrap();
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account["accountId"],
            "securityId": cash_id,
            "openedOn": "2026-01-02",
            "quantityMinor": 100,
            "quantityScale": 0,
            "performanceCostMinor": 10000,
            "taxCostMinor": 10000,
            "scale": 2
        }),
    )
    .await;
    let set = query_json(
        &platform,
        "CollectorSetGet",
        serde_json::json!({ "asOfDate": "2026-09-05" }),
    )
    .await;
    let row = set["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["symbol"] == "CASH1")
        .expect("CASH1 fleet row");
    assert_eq!(row["divType"], "CASH");
    assert_eq!(row["fillGapsDivTypeBlank"], false);
    assert_eq!(row["fillGapsRocBlank"], false);
    assert_eq!(row["complete"], true, "{row}");
}

/// Run misses / unchanged collect must not increment paid count by cloning months.
#[tokio::test]
async fn fleet_paid_count_unchanged_when_history_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "PAY1", "issuer").await;
    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "PAY1",
            "declarationSource": "issuer",
            "asOfDate": "2026-09-05",
            "candidates": monthly_paid_candidates(&security_id, "issuer", 12, 2)
        }),
    )
    .await;
    let before = query_json(
        &platform,
        "CollectorSetGet",
        serde_json::json!({ "asOfDate": "2026-09-05" }),
    )
    .await;
    let paid_before = before["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["symbol"] == "PAY1")
        .unwrap()["paidDeclarationCount"]
        .as_u64()
        .unwrap();
    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "PAY1",
            "declarationSource": "issuer",
            "asOfDate": "2026-09-05",
            "candidates": monthly_paid_candidates(&security_id, "issuer", 12, 2)
        }),
    )
    .await;
    let after = query_json(
        &platform,
        "CollectorSetGet",
        serde_json::json!({ "asOfDate": "2026-09-05" }),
    )
    .await;
    let paid_after = after["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["symbol"] == "PAY1")
        .unwrap()["paidDeclarationCount"]
        .as_u64()
        .unwrap();
    assert_eq!(paid_after, paid_before, "unchanged history must not clone months");
}

#[test]
fn collectors_footer_grid_is_the_operator_console() {
    let root = golden_harness::repo_root();
    let app = std::fs::read_to_string(root.join("apps/desktop/src/App.tsx")).unwrap();
    assert!(app.contains("aria-label=\"Collector footer grid\""));
    assert!(app.contains("aria-label={`Open ${row.symbol} position`}"));
    assert!(app.contains("aria-label={`Open tickets for ${row.symbol}`}"));
    assert!(app.contains("setScreen(\"position-details\")"));
    assert!(app.contains("setTicketFocusSymbol(row.symbol)"));
    assert!(app.contains("Collect fresh distribution data for this symbol"));
    assert!(app.contains("Run misses only"));
    assert!(
        !app.contains("DIV-1 compliance") && !app.contains("Required paid"),
        "footer grid is the operator console; compliance date dump must be gone"
    );
    assert!(
        !app.contains("futurePayDates.join"),
        "do not dump every remaining pay date on Collectors"
    );
    assert!(app.contains("aria-label=\"Missing collector URLs\""));
    assert!(app.contains("Apply URLs"));
    assert!(app.contains("collectorNeedsOwnerUrl"));
}

/// Failing DIV-1 with empty seed URL tickets and blocks probe-only retrieve.
#[tokio::test]
async fn pay1_failing_empty_url_requires_owner_seed() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "PAY1", "issuer").await;
    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "PAY1",
            "declarationSource": "issuer",
            "asOfDate": "2026-09-05",
            "misses": [{
                "securityId": security_id,
                "symbol": "PAY1",
                "code": "declaration_retrieve_miss",
                "reason": "issuer page empty"
            }]
        }),
    )
    .await;
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "declarationSource": "issuer",
            "sourceSymbol": "PAY1",
            "sourceUrl": "",
            "calendarPolicy": "derived_walk",
            "collectorEnabled": true,
            "lookbackCount": 12
        }),
    )
    .await;
    let blocked = execute_command_on(
        &platform,
        &platform,
        cmd(
            "CollectorRetrieve",
            serde_json::json!({
                "securityId": security_id,
                "symbol": "PAY1",
                "declarationSource": "issuer",
                "asOfDate": "2026-09-05",
                "candidates": monthly_paid_candidates(&security_id, "issuer", 12, 2)
            }),
        ),
    )
    .await;
    assert!(blocked.ok, "{blocked:?}");
    let body: serde_json::Value =
        serde_json::from_str(blocked.body_json.as_deref().unwrap_or("{}")).unwrap();
    assert_eq!(body["ok"], false, "{body}");
    assert_eq!(body["code"], "missing_seed_url", "{body}");
    let set = query_json(
        &platform,
        "CollectorSetGet",
        serde_json::json!({ "asOfDate": "2026-09-05" }),
    )
    .await;
    let row = set["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["symbol"] == "PAY1")
        .expect("PAY1 fleet row");
    let gaps = row["gaps"].as_array().cloned().unwrap_or_default();
    assert!(
        gaps.iter().any(|g| g.as_str() == Some("seed URL")),
        "{row}"
    );
    assert!(row["openTicketCount"].as_u64().unwrap() >= 1, "{row}");

    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "declarationSource": "issuer",
            "sourceSymbol": "PAY1",
            "sourceUrl": "https://example.test/pay1/distributions",
            "calendarPolicy": "derived_walk",
            "collectorEnabled": true,
            "lookbackCount": 12
        }),
    )
    .await;
    let after = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "PAY1",
            "declarationSource": "issuer",
            "asOfDate": "2026-09-05",
            "candidates": monthly_paid_candidates(&security_id, "issuer", 12, 2)
        }),
    )
    .await;
    assert_eq!(after["ok"], true, "{after}");
}

/// Stored seed that later misses is not a missing_seed_url demand.
#[tokio::test]
async fn pay1_stored_seed_miss_is_not_missing_seed_url() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "PAY1", "issuer").await;
    let missed = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "PAY1",
            "declarationSource": "issuer",
            "asOfDate": "2026-09-05",
            "sourceUrl": "https://example.test/pay1/distributions",
            "misses": [{
                "securityId": security_id,
                "symbol": "PAY1",
                "code": "declaration_retrieve_miss",
                "reason": "issuer page empty"
            }]
        }),
    )
    .await;
    assert_ne!(missed["code"], "missing_seed_url", "{missed}");
}

/// Blank div_type non-payer is not a seed-URL sheet row.
#[tokio::test]
async fn new1_blank_div_type_is_not_on_url_sheet() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Taxable", "kind": "taxable"}),
    )
    .await;
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "NEW1", "name": "NEW1"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "declarationSource": "issuer",
            "sourceSymbol": "NEW1",
            "sourceUrl": "",
            "calendarPolicy": "derived_walk",
            "collectorEnabled": true,
            "lookbackCount": 12
        }),
    )
    .await;
    let _ = execute_command_on(
        &platform,
        &platform,
        cmd(
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
        ),
    )
    .await;
    let _ = execute_command_on(
        &platform,
        &platform,
        cmd(
            "CollectorRetrieve",
            serde_json::json!({
                "securityId": security_id,
                "symbol": "NEW1",
                "declarationSource": "issuer",
                "asOfDate": "2026-09-05",
                "misses": [{
                    "securityId": security_id,
                    "symbol": "NEW1",
                    "code": "declaration_retrieve_miss",
                    "reason": "issuer page empty"
                }]
            }),
        ),
    )
    .await;
    let set = query_json(
        &platform,
        "CollectorSetGet",
        serde_json::json!({ "asOfDate": "2026-09-05" }),
    )
    .await;
    let row = set["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["symbol"] == "NEW1");
    if let Some(row) = row {
        let gaps = row["gaps"].as_array().cloned().unwrap_or_default();
        assert!(
            !gaps.iter().any(|g| g.as_str() == Some("seed URL")),
            "blank div_type must not be on the URL sheet: {row}"
        );
    }
}

/// CASH failing without a seed URL still requires an owner declaration URL.
#[tokio::test]
async fn cash1_failing_empty_url_is_on_url_sheet() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Cash", "kind": "taxable"}),
    )
    .await;
    let cash = must_ok(
        &platform,
        "PositionResearchSeed",
        serde_json::json!({
            "symbol": "CASH1",
            "sourceUrl": "https://example.test/cash1",
            "divType": "CASH",
            "skipRefresh": true
        }),
    )
    .await;
    let cash_id = cash["securityId"].as_str().unwrap();
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account["accountId"],
            "securityId": cash_id,
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
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": cash_id,
            "symbol": "CASH1",
            "declarationSource": "fidelity",
            "divType": "CASH",
            "asOfDate": "2026-09-05",
            "misses": [{
                "securityId": cash_id,
                "symbol": "CASH1",
                "code": "declaration_retrieve_miss",
                "reason": "rate page empty"
            }]
        }),
    )
    .await;
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": cash_id,
            "declarationSource": "fidelity",
            "sourceSymbol": "CASH1",
            "sourceUrl": "",
            "calendarPolicy": "none",
            "collectorEnabled": true,
            "lookbackCount": 12
        }),
    )
    .await;
    let set = query_json(
        &platform,
        "CollectorSetGet",
        serde_json::json!({ "asOfDate": "2026-09-05" }),
    )
    .await;
    let row = set["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["symbol"] == "CASH1")
        .expect("CASH1 fleet row");
    let gaps = row["gaps"].as_array().cloned().unwrap_or_default();
    assert!(
        gaps.iter().any(|g| g.as_str() == Some("seed URL")),
        "{row}"
    );
    assert_eq!(row["remainingPlanned"], serde_json::Value::Null, "{row}");
    assert_eq!(row["remainingExpected"], serde_json::Value::Null, "{row}");
}

#[tokio::test]
async fn collector_retrieve_drops_third_party_declaration_urls() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "PAY1", "issuer").await;
    let retrieved = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "PAY1",
            "declarationSource": "issuer",
            "candidates": [
                {
                    "securityId": security_id,
                    "amountPerShareMinor": 10,
                    "amountScale": 2,
                    "paymentPeriod": "2026-08-31",
                    "source": "yahoo",
                    "sourceUrl": "https://finance.yahoo.com/quote/PAY1"
                },
                {
                    "securityId": security_id,
                    "amountPerShareMinor": 11,
                    "amountScale": 2,
                    "paymentPeriod": "2026-07-31",
                    "source": "nasdaq",
                    "sourceUrl": "https://api.nasdaq.com/api/quote/PAY1/dividends?assetclass=etf"
                },
                {
                    "securityId": security_id,
                    "amountPerShareMinor": 12,
                    "amountScale": 2,
                    "paymentPeriod": "2026-06-30",
                    "source": "dividendinvestor",
                    "sourceUrl": "https://www.dividendinvestor.com/dividend-history/?symbol=PAY1"
                }
            ]
        }),
    )
    .await;
    assert_eq!(
        retrieved["recorded"].as_u64().unwrap_or(99),
        0,
        "third-party declaration URLs must not persist: {retrieved}"
    );
}

#[tokio::test]
async fn pay1_three_remaining_without_december_is_not_a_gap() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "PAY1", "issuer").await;
    must_ok(
        &platform,
        "IssuerPayDateReplace",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-09-06",
            "dates": [
                {"payOn": "2026-10-15", "source": "issuer"},
                {"payOn": "2026-11-13", "source": "issuer"}
            ]
        }),
    )
    .await;
    let set = query_json(
        &platform,
        "CollectorSetGet",
        serde_json::json!({ "asOfDate": "2026-09-06" }),
    )
    .await;
    let row = set["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["symbol"] == "PAY1")
        .expect("PAY1 fleet row");
    let gaps = row["gaps"].as_array().cloned().unwrap_or_default();
    assert!(
        !gaps.iter().any(|g| g.as_str() == Some("remaining_year")),
        "Oct/Nov without December is not a remaining-year gap: {row}"
    );
    let tickets = query_json(
        &platform,
        "WorkTicketList",
        serde_json::json!({ "securityId": security_id, "status": "open" }),
    )
    .await;
    let open = tickets
        .get("items")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        !open.iter().any(|t| t["code"].as_str() == Some("remaining_year")),
        "calendar leftover must not ticket remaining_year: {tickets}"
    );
}

#[tokio::test]
async fn pay1_short_increment_lookback_complete_last_run_ok() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "PAY1", "amplify").await;
    let first = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "PAY1",
            "declarationSource": "amplify",
            "candidates": monthly_paid_candidates(&security_id, "amplify", 100, 2),
            "pagePaid": monthly_paid_candidates(&security_id, "amplify", 100, 2)
        }),
    )
    .await;
    assert_eq!(first["ok"], true, "{first}");
    let page = monthly_paid_candidates(&security_id, "amplify", 100, 2)
        .into_iter()
        .rev()
        .take(3)
        .collect::<Vec<_>>();
    let second = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "PAY1",
            "declarationSource": "amplify",
            "candidates": [],
            "pagePaid": page
        }),
    )
    .await;
    assert_eq!(second["ok"], true, "stored lookback complete: {second}");
    assert_ne!(second["code"], "declaration_lookback_short", "{second}");
    let payload: serde_json::Value =
        serde_json::from_str(second["payloadJson"].as_str().unwrap_or("{}")).unwrap();
    assert_eq!(payload["postChecks"]["lookbackOk"], true, "{payload}");
    let sid = Uuid::parse_str(&security_id).unwrap();
    let n = platform
        .issuer_declaration_list(sid)
        .await
        .unwrap()
        .iter()
        .filter(|d| d.amount_per_share_minor.unwrap_or(0) > 0)
        .count();
    assert_eq!(n, 12);
}

#[tokio::test]
async fn pay1_overlap_amount_change_tickets_keeps_stored() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "PAY1", "amplify").await;
    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "PAY1",
            "declarationSource": "amplify",
            "candidates": monthly_paid_candidates(&security_id, "amplify", 100, 2),
            "pagePaid": monthly_paid_candidates(&security_id, "amplify", 100, 2)
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot(&platform, &security_id, "PAY1")
        .await
        .expect("complete before 30% rule");
    let mut page = monthly_paid_candidates(&security_id, "amplify", 100, 2)
        .into_iter()
        .rev()
        .take(2)
        .collect::<Vec<_>>();
    page[0]["amountPerShareMinor"] = serde_json::json!(999);
    let second = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "PAY1",
            "declarationSource": "amplify",
            "candidates": [],
            "pagePaid": page
        }),
    )
    .await;
    assert_eq!(second["ok"], true, "amount change is not last_run fail: {second}");
    assert_eq!(second["code"], "declaration_amount_variation", "{second}");
    let sid = Uuid::parse_str(&security_id).unwrap();
    let decls = platform.issuer_declaration_list(sid).await.unwrap();
    let paid: Vec<_> = decls
        .iter()
        .filter(|d| d.amount_per_share_minor.unwrap_or(0) > 0)
        .collect();
    assert_eq!(paid.len(), 12);
    let last = paid.iter().find(|d| d.payment_period == "2026-08-31").unwrap();
    assert_eq!(last.amount_per_share_minor, Some(100));
    let tickets = query_json(&platform, "WorkTicketList", serde_json::json!({})).await;
    assert!(
        tickets["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["code"] == "declaration_amount_variation" && t["status"] == "open"),
        "{tickets}"
    );
}

#[tokio::test]
async fn pay1_stale_expected_4_remaining_year_closes_without_december() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1_monthly(&platform, "PAY1", "amplify").await;
    must_ok(
        &platform,
        "IssuerPayDateReplace",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-09-06",
            "dates": [
                {"payOn": "2026-10-15", "source": "issuer"},
                {"payOn": "2026-11-13", "source": "issuer"}
            ]
        }),
    )
    .await;
    let sid = Uuid::parse_str(&security_id).unwrap();
    platform
        .work_ticket_raise(WorkTicketRecord {
            ticket_id: Uuid::new_v4(),
            security_id: sid,
            symbol: "PAY1".into(),
            field: "remaining_year".into(),
            code: "remaining_year".into(),
            tool: "fix_remaining_year".into(),
            reason: "Remaining-year count is 3, expected 4 periods through 31 Dec.".into(),
            urls_tried: "[]".into(),
            opened_on: "2026-09-01".into(),
            last_seen_on: "2026-09-01".into(),
            status: "open".into(),
            filed_on: String::new(),
            completed_how: String::new(),
            owner_note: String::new(),
            retrieve_run_id: String::new(),
        })
        .await
        .expect("raise leftover expected-4 ticket");
    must_ok(&platform, "WorkTicketSyncMisses", serde_json::json!({})).await;
    let tickets = query_json(
        &platform,
        "WorkTicketList",
        serde_json::json!({ "securityId": security_id }),
    )
    .await;
    let leftover = tickets["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["code"] == "remaining_year")
        .expect("remaining_year ticket");
    assert_eq!(leftover["status"], "done", "{tickets}");
    assert_eq!(leftover["completedHow"], "auto_resolved", "{tickets}");
    let pays = platform.issuer_pay_date_list(sid).await.unwrap();
    assert!(
        !pays.iter().any(|p| p.pay_on.starts_with("2026-12")),
        "must not insert December: {pays:?}"
    );
    assert!(pays.iter().any(|p| p.pay_on == "2026-10-15"), "{pays:?}");
    assert!(pays.iter().any(|p| p.pay_on == "2026-11-13"), "{pays:?}");
}

const MLP1_AUG_8K: &str =
    include_str!("../../import-engine/tests/fixtures/energytransfer_et_8k.html");
const MLP1_NOV_8K: &str = include_str!("../../import-engine/tests/fixtures/mlp1_8k_nov21.html");
const MLP1_PREF_8K: &str =
    include_str!("../../import-engine/tests/fixtures/mlp1_8k_preferred_only.html");

async fn seed_mlp1_quarterly(platform: &LocalPlatform) -> String {
    let account = must_ok(
        platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let security = must_ok(
        platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "MLP1", "name": "MLP1"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap().to_string();
    must_ok(
        platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "priceSource": "public",
            "sourceSymbol": "MLP1",
            "declarationSource": "mlp_sec_8k",
            "sourceUrl": "https://ir.energytransfer.com/distribution-history-et",
            "calendarPolicy": "derived_walk",
            "collectorEnabled": true,
            "lookbackCount": 12
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot(platform, &security_id, "MLP1")
        .await
        .expect("complete collector");
    must_ok(
        platform,
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
        platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Quarterly",
            "provider": "Energy Transfer",
            "divType": "DIV-1",
            "isActive": true
        }),
    )
    .await;
    security_id
}

fn mlp1_collect(html: &str) -> import_engine::DeclarationCollectOutcome {
    import_engine::collect_from_fetched_page(
        &import_engine::DeclarationTarget {
            security_id: "mlp1".into(),
            symbol: "MLP1".into(),
            declaration_source: "mlp_sec_8k".into(),
            source_symbol: "MLP1".into(),
            payment_frequency: "Quarterly".into(),
            paid_count: 1,
            ..Default::default()
        },
        "mlp_sec_8k",
        Some(html),
    )
}

#[tokio::test]
async fn mlp_sec_8k_fixture_declares_aug_and_derives_nov() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .expect("open sqlite");
    let security_id = seed_mlp1_quarterly(&platform).await;
    let out = mlp1_collect(MLP1_AUG_8K);
    assert!(out.misses.is_empty(), "{:?}", out.misses);
    assert_eq!(out.candidates[0]["source"], "sec_8k");
    assert_eq!(out.candidates[0]["paymentPeriod"], "2026-08-19");
    assert_eq!(out.candidates[0]["amountPerShareMinor"], 3400);
    assert_eq!(out.pay_dates.len(), 1);
    assert_eq!(out.pay_dates[0]["payOn"], "2026-11-19");
    assert_eq!(out.pay_dates[0]["source"], "derived_template");
    assert!(out.pay_dates[0]["amountPerShareMinor"].is_null());

    let atom = financial_domain::mlp_sec::atom_url();
    let retrieved = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "MLP1",
            "declarationSource": "mlp_sec_8k",
            "asOfDate": "2026-09-08",
            "candidates": out.candidates,
            "pagePaid": out.page_paid,
            "upcomingPays": out.pay_dates,
            "fetchedSourceUrl": atom,
            "contentHash": "mlp1-aug-8k"
        }),
    )
    .await;
    assert_eq!(retrieved["ok"], true, "{retrieved}");
    assert_eq!(retrieved["message"], "mlp_sec_8k:page", "{retrieved}");
    assert!(
        !retrieved["message"].as_str().unwrap_or("").contains("Issuer page empty"),
        "{retrieved}"
    );
    let sid = Uuid::parse_str(&security_id).unwrap();
    let decls = platform.issuer_declaration_list(sid).await.unwrap();
    let sec8k: Vec<_> = decls
        .iter()
        .filter(|d| d.source.eq_ignore_ascii_case("sec_8k"))
        .collect();
    assert_eq!(sec8k.len(), 1, "{decls:?}");
    assert_eq!(sec8k[0].payment_period, "2026-08-19");
    assert_eq!(sec8k[0].amount_per_share_minor, Some(3400));
    let pays = platform.issuer_pay_date_list(sid).await.unwrap();
    let derived: Vec<_> = pays
        .iter()
        .filter(|p| p.source.eq_ignore_ascii_case("derived_template"))
        .collect();
    assert_eq!(derived.len(), 1, "{pays:?}");
    assert_eq!(derived[0].pay_on, "2026-11-19");
    let set = query_json(&platform, "CollectorSetGet", serde_json::json!({})).await;
    let item = set["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["symbol"] == "MLP1")
        .expect("MLP1");
    assert_eq!(item["lastRunOk"], true, "{item}");
    assert_eq!(item["lastRunMessage"], "mlp_sec_8k:page", "{item}");
    assert!(
        item["sourceUrl"].as_str().unwrap_or("").contains("sec.gov"),
        "{item}"
    );
    assert!(!item["sourceUrl"]
        .as_str()
        .unwrap_or("")
        .contains("energytransfer.com"));
}

#[tokio::test]
async fn mlp_sec_8k_nov_8k_moves_derived_date() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .expect("open sqlite");
    let security_id = seed_mlp1_quarterly(&platform).await;
    let first = mlp1_collect(MLP1_AUG_8K);
    let atom = financial_domain::mlp_sec::atom_url();
    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "MLP1",
            "declarationSource": "mlp_sec_8k",
            "asOfDate": "2026-09-08",
            "candidates": first.candidates,
            "pagePaid": first.page_paid,
            "upcomingPays": first.pay_dates,
            "fetchedSourceUrl": atom,
            "contentHash": "mlp1-aug-8k"
        }),
    )
    .await;
    let second = mlp1_collect(MLP1_NOV_8K);
    let retrieved = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "MLP1",
            "declarationSource": "mlp_sec_8k",
            "asOfDate": "2026-09-08",
            "candidates": second.candidates,
            "pagePaid": second.page_paid,
            "upcomingPays": second.pay_dates,
            "fetchedSourceUrl": atom,
            "contentHash": "mlp1-nov-8k"
        }),
    )
    .await;
    assert_eq!(retrieved["ok"], true, "{retrieved}");
    assert_eq!(retrieved["message"], "mlp_sec_8k:page", "{retrieved}");
    let tickets = query_json(
        &platform,
        "WorkTicketList",
        serde_json::json!({ "securityId": security_id }),
    )
    .await;
    assert!(
        tickets["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["code"] != "payable_date_moved"),
        "no payable_date_moved inside 3 business days: {tickets}"
    );
    let sid = Uuid::parse_str(&security_id).unwrap();
    let decls = platform.issuer_declaration_list(sid).await.unwrap();
    assert!(
        decls.iter().any(|d| d.payment_period == "2026-11-21"
            && d.source.eq_ignore_ascii_case("sec_8k")
            && d.amount_per_share_minor == Some(3400)),
        "{decls:?}"
    );
}

#[tokio::test]
async fn mlp_sec_8k_preferred_only_misses_common_quarter() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .expect("open sqlite");
    let security_id = seed_mlp1_quarterly(&platform).await;
    let out = mlp1_collect(MLP1_PREF_8K);
    assert!(out.candidates.is_empty(), "{:?}", out.candidates);
    assert!(out
        .candidates
        .iter()
        .chain(out.page_paid.iter())
        .all(|c| c["amountPerShareMinor"] != 2111));
    let retrieved = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "MLP1",
            "declarationSource": "mlp_sec_8k",
            "asOfDate": "2026-09-08",
            "candidates": out.candidates,
            "misses": out.misses,
            "upcomingPays": out.pay_dates,
            "fetchedSourceUrl": financial_domain::mlp_sec::atom_url()
        }),
    )
    .await;
    assert_eq!(retrieved["ok"], true, "{retrieved}");
    assert_eq!(retrieved["message"], "mlp_sec_8k:waiting", "{retrieved}");
    assert!(
        !retrieved["message"].as_str().unwrap_or("").contains("Issuer page empty"),
        "{retrieved}"
    );
    let sid = Uuid::parse_str(&security_id).unwrap();
    let decls = platform.issuer_declaration_list(sid).await.unwrap();
    assert!(
        !decls.iter().any(|d| d.amount_per_share_minor == Some(2111)),
        "{decls:?}"
    );
}

#[tokio::test]
async fn mlp_sec_8k_empty_sec_is_not_issuer_page_empty() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .expect("open sqlite");
    let security_id = seed_mlp1_quarterly(&platform).await;
    let out = import_engine::collect_from_fetched_page(
        &import_engine::DeclarationTarget {
            security_id: security_id.clone(),
            symbol: "MLP1".into(),
            declaration_source: "mlp_sec_8k".into(),
            source_symbol: "MLP1".into(),
            payment_frequency: "Quarterly".into(),
            known_payment_periods: vec!["2026-08-19".into()],
            ..Default::default()
        },
        "mlp_sec_8k",
        None,
    );
    assert!(
        out.misses.is_empty(),
        "no miss before Nov 7 ask window: {:?}",
        out.misses
    );
    let retrieved = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "MLP1",
            "declarationSource": "mlp_sec_8k",
            "asOfDate": "2026-09-08",
            "candidates": [],
            "misses": out.misses,
            "upcomingPays": out.pay_dates
        }),
    )
    .await;
    assert_eq!(retrieved["ok"], true, "{retrieved}");
    assert_eq!(retrieved["message"], "mlp_sec_8k:waiting", "{retrieved}");
    assert!(
        !retrieved["message"].as_str().unwrap_or("").contains("Issuer page empty"),
        "{retrieved}"
    );
    let set = query_json(&platform, "CollectorSetGet", serde_json::json!({})).await;
    let item = set["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["symbol"] == "MLP1")
        .expect("MLP1");
    assert!(
        item["sourceUrl"].as_str().unwrap_or("").contains("sec.gov"),
        "{item}"
    );
    assert!(!item["sourceUrl"]
        .as_str()
        .unwrap_or("")
        .contains("energytransfer.com"));
    assert_eq!(item["lastRunOk"], true, "{item}");
    assert_eq!(item["lastRunMessage"], "mlp_sec_8k:waiting", "{item}");
}

#[tokio::test]
async fn mlp_sec_8k_asks_owner_amount_on_nov_7() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .expect("open sqlite");
    let security_id = seed_mlp1_quarterly(&platform).await;
    let first = mlp1_collect(MLP1_AUG_8K);
    let atom = financial_domain::mlp_sec::atom_url();
    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-09-08",
            "declarationSource": "mlp_sec_8k",
            "candidates": first.candidates,
            "upcomingPays": first.pay_dates,
            "fetchedSourceUrl": atom
        }),
    )
    .await;
    let retrieved = must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-11-07",
            "declarationSource": "mlp_sec_8k",
            "candidates": [],
            "upcomingPays": [{
                "payOn": "2026-11-19",
                "amountPerShareMinor": null,
                "source": "derived_template"
            }]
        }),
    )
    .await;
    assert_eq!(retrieved["ok"], false, "{retrieved}");
    assert_eq!(retrieved["message"], "mlp_sec_8k:need_owner", "{retrieved}");
    let tickets = query_json(
        &platform,
        "WorkTicketList",
        serde_json::json!({ "securityId": security_id }),
    )
    .await;
    let ask = tickets["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["code"] == "mlp_sec_owner_amount")
        .expect("owner amount ticket");
    assert_eq!(ask["status"], "open", "{tickets}");
    must_ok(
        &platform,
        "WorkTicketResolve",
        serde_json::json!({
            "ticketId": ask["ticketId"],
            "tool": "enter_declared_amount",
            "amount": "0.3425"
        }),
    )
    .await;
    let sid = Uuid::parse_str(&security_id).unwrap();
    let decls = platform.issuer_declaration_list(sid).await.unwrap();
    assert!(
        decls.iter().any(|d| d.payment_period == "2026-11-19"
            && d.source.eq_ignore_ascii_case("owner")
            && d.amount_per_share_minor == Some(3425)),
        "{decls:?}"
    );
}
