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

/// Process B requires a researched identity before LotOpen.
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
async fn process_b_researched_zero_lots_then_explicit_open() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let seed = must_ok(
        &platform,
        "PositionResearchSeed",
        serde_json::json!({
            "symbol": "HAKY",
            "sourceUrl": "https://amplifyetfs.com/haky/#distributions"
        }),
    )
    .await;
    let security_id = seed["securityId"].as_str().unwrap();
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-08-29" }),
    )
    .await;
    assert!(inv["lots"].as_array().unwrap().is_empty());
    assert_eq!(inv["remainingQuantityMinor"], 0);

    let lot = must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account["accountId"],
            "securityId": security_id,
            "openedOn": "2026-08-15",
            "origin": "purchase",
            "quantityMinor": 25,
            "quantityScale": 0,
            "performanceBasisMinor": 50_000,
            "taxBasisMinor": 48_000,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;
    assert_eq!(lot["origin"], "purchase");
    assert_eq!(lot["performanceBasisMinor"], 50_000);
    assert_eq!(lot["taxBasisMinor"], 48_000);
    assert_eq!(lot["openedOn"], "2026-08-15");

    let inv2 = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-08-29" }),
    )
    .await;
    assert_eq!(inv2["lots"].as_array().unwrap().len(), 1);
    assert_eq!(inv2["remainingQuantityMinor"], 25);
}

#[tokio::test]
async fn process_b_lot_open_without_research_fails() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "RAW1", "name": "RAW1"}),
    )
    .await;
    let result = execute_command_on(
        &platform,
        &platform,
        cmd(
            "LotOpen",
            serde_json::json!({
                "accountId": account["accountId"],
                "securityId": security["securityId"],
                "openedOn": "2026-08-01",
                "origin": "purchase",
                "quantityMinor": 1,
                "quantityScale": 0,
                "performanceBasisMinor": 100,
                "taxBasisMinor": 100,
                "scale": 2,
                "isOpen": true
            }),
        ),
    )
    .await;
    assert!(!result.ok);
    assert_eq!(result.error_code.as_deref(), Some("not_researched"));
}

#[tokio::test]
async fn process_b_origin_drip_and_transfer_are_explicit() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "ORIG", "name": "ORIG", "crf": true}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    research_template(&platform, security_id, "ORIG").await;
    let drip = must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account["accountId"],
            "securityId": security_id,
            "openedOn": "2026-08-10",
            "origin": "drip",
            "quantityMinor": 2,
            "quantityScale": 0,
            "performanceBasisMinor": 0,
            "taxBasisMinor": 0,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;
    assert_eq!(drip["origin"], "drip");
    let xfer = must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account["accountId"],
            "securityId": security_id,
            "openedOn": "2026-08-11",
            "origin": "transfer",
            "quantityMinor": 3,
            "quantityScale": 0,
            "performanceBasisMinor": 300,
            "taxBasisMinor": 300,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;
    assert_eq!(xfer["origin"], "transfer");
}

#[tokio::test]
async fn wz_add_lot_existing_increases_income_plan_not_plan_history() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "ADD1", "name": "ADD1"}),
    )
    .await;
    let account_id = account["accountId"].as_str().unwrap();
    let security_id = security["securityId"].as_str().unwrap();
    research_template(&platform, security_id, "ADD1").await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Weekly",
            "riskTier": "Core"
        }),
    )
    .await;
    for i in 0..6 {
        must_ok(
            &platform,
            "IssuerDeclarationRecord",
            serde_json::json!({
                "securityId": security_id,
                "amountPerShareMinor": 10 + i,
                "amountScale": 2,
                "paymentPeriod": format!("w{i}"),
                "source": "provider-site",
                "enteredAt": "2026-08-21"
            }),
        )
        .await;
    }
    must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 100,
            "amountScale": 2,
            "planningPeriodsPerYear": 52,
            "effectiveFrom": "2026-08-21",
            "decisionReason": "owner"
        }),
    )
    .await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "openedOn": "2026-08-01",
            "origin": "purchase",
            "quantityMinor": 10,
            "quantityScale": 0,
            "performanceBasisMinor": 10_000,
            "taxBasisMinor": 10_000,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;
    let before = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({"asOfDate": "2026-08-21"}),
    )
    .await;
    let plan_before = before["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["accountName"] == "Income")
        .unwrap()["plannedMinor"]
        .as_i64()
        .unwrap();
    let remaining_preview = query_json(
        &platform,
        "RemainingYearIncomeGet",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-08-21",
            "thisLotQuantityMinor": 10,
            "thisLotOpenedOn": "2026-08-15"
        }),
    )
    .await;
    assert!(remaining_preview["known"].as_bool().unwrap());
    let this_lot = remaining_preview["thisLotYearToGoMinor"].as_i64().unwrap();
    let after_add = remaining_preview["positionAfterYearToGoMinor"].as_i64().unwrap();
    assert!(this_lot > 0, "Add Lot remaining-year this-lot cash: {remaining_preview}");
    assert!(
        after_add > this_lot,
        "position after add must exceed this lot: {remaining_preview}"
    );
    let summary_before = query_json(&platform, "DataSummaryGet", serde_json::json!({})).await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "openedOn": "2026-08-15",
            "origin": "purchase",
            "quantityMinor": 10,
            "quantityScale": 0,
            "performanceBasisMinor": 10_000,
            "taxBasisMinor": 10_000,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;
    let after = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({"asOfDate": "2026-08-21"}),
    )
    .await;
    let plan_after = after["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["accountName"] == "Income")
        .unwrap()["plannedMinor"]
        .as_i64()
        .unwrap();
    assert!(
        plan_after > plan_before,
        "WZ-add-lot-existing planned amount should rise {plan_before} -> {plan_after}"
    );
    let summary_after = query_json(&platform, "DataSummaryGet", serde_json::json!({})).await;
    assert_eq!(
        summary_before["planCount"], summary_after["planCount"],
        "Add Lot must not create PlanHistory"
    );
    let remaining_stored = query_json(
        &platform,
        "RemainingYearIncomeGet",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-08-21"
        }),
    )
    .await;
    assert_eq!(
        remaining_stored["yearToGoMinor"].as_i64(),
        remaining_preview["positionAfterYearToGoMinor"].as_i64(),
        "stored lots must match the Add Lot position-after-add preview"
    );
}

#[tokio::test]
async fn wz_add_lot_unknown_security_fails() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let result = execute_command_on(
        &platform,
        &platform,
        cmd(
            "LotOpen",
            serde_json::json!({
                "accountId": account["accountId"],
                "securityId": Uuid::new_v4().to_string(),
                "openedOn": "2026-08-01",
                "origin": "purchase",
                "quantityMinor": 1,
                "quantityScale": 0,
                "performanceBasisMinor": 100,
                "taxBasisMinor": 100,
                "scale": 2,
                "isOpen": true
            }),
        ),
    )
    .await;
    assert!(!result.ok, "WZ-add-lot-unknown must fail");
}
