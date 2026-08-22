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
    let summary_before = query_json(&platform, "HouseholdSummaryGet", serde_json::json!({})).await;
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
    let summary_after = query_json(&platform, "HouseholdSummaryGet", serde_json::json!({})).await;
    assert_eq!(
        summary_before["planCount"], summary_after["planCount"],
        "Add Lot must not create PlanHistory"
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
