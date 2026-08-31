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

#[tokio::test]
async fn calculator_plan_sets_income_plan_week_not_actuals() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let income = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "AMDW", "name": "AMD"}),
    )
    .await;
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security["securityId"],
            "priceSource": "public",
            "sourceSymbol": "AMDW",
            "declarationSource": "issuer",
            "sourceUrl": "https://example.test/amdw/distributions",
            "calendarPolicy": "derived_walk",
            "collectorEnabled": true,
            "lookbackCount": 12
        }),
    )
    .await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": income["accountId"],
            "securityId": security["securityId"],
            "openedOn": "2026-01-15",
            "origin": "purchase",
            "quantityMinor": 69,
            "quantityScale": 0,
            "performanceBasisMinor": 401200,
            "taxBasisMinor": 401200,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "ProductionSeedLoad",
        serde_json::json!({
            "accounts": [],
            "securities": [],
            "lots": [],
            "yieldBatches": [],
            "disbursements": [],
            "plans": [{
                "symbol": "AMDW",
                "amountPerShareMinor": 55,
                "amountScale": 2,
                "planningPeriodsPerYear": 52,
                "effectiveFrom": "2026-08-12",
                "decisionReason": "test"
            }],
            "characteristics": [{
                "symbol": "AMDW",
                "paymentFrequency": "Weekly",
                "riskTier": "HighRisk",
                "provider": "Roundhill",
                "underlying": "AMD",
                "divType": "DIV-1",
                "needsRocResearch": false,
                "notes": ""
            }]
        }),
    )
    .await;

    let calc = query_json(&platform, "CalculatorGet", serde_json::json!({})).await;
    assert_eq!(calc["planCount"].as_u64().unwrap(), 1);
    let row = calc["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["symbol"] == "AMDW")
        .unwrap();
    assert_eq!(row["planKnown"], true);
    assert_eq!(row["planPaymentMinor"].as_i64().unwrap(), 3_795);

    let week = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({"asOfDate": "2026-08-25"}),
    )
    .await;
    let income_line = week["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["accountName"] == "Income")
        .unwrap();
    assert_eq!(income_line["planKnown"], true);
    assert_eq!(income_line["plannedMinor"].as_i64().unwrap(), 3_795);
    assert_eq!(income_line["actualMinor"].as_i64().unwrap(), 0);

    must_ok(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": income["accountId"],
            "securityId": security["securityId"],
            "activityType": "dividend",
            "amountMinor": 100,
            "scale": 2,
            "occurredOn": "2026-08-25",
            "idempotencyKey": "cash-is-not-plan"
        }),
    )
    .await;
    let after = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({"asOfDate": "2026-08-25"}),
    )
    .await;
    let after_line = after["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["accountName"] == "Income")
        .unwrap();
    assert_eq!(after_line["actualMinor"].as_i64().unwrap(), 100);
    assert_eq!(after_line["plannedMinor"].as_i64().unwrap(), 3_795);
}
