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
    }
}

fn qry(name: &str) -> QueryRequest {
    QueryRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        query_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: None,
    }
}

async fn must_ok(platform: &LocalPlatform, name: &str, body: serde_json::Value) -> serde_json::Value {
    let result = execute_command_on(platform, platform, cmd(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

async fn query_json(platform: &LocalPlatform, name: &str) -> serde_json::Value {
    let result = execute_query_on(platform, platform, qry(name)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

#[tokio::test]
async fn plan_approve_does_not_rewrite_dividend_actuals() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Taxable Brokerage", "kind": "taxable"}),
    )
    .await;
    let account_id = account["accountId"].as_str().unwrap();
    must_ok(
        &platform,
        "DividendActualRecord",
        serde_json::json!({
            "accountId": account_id,
            "occurredOn": "2026-06-15",
            "amountMinor": 50_000,
            "scale": 2,
            "idempotencyKey": "plan-div-1"
        }),
    )
    .await;
    let before = query_json(&platform, "DividendGet").await;
    assert_eq!(before["actualTotalMinor"].as_i64().unwrap(), 50_000);

    let plan = must_ok(
        &platform,
        "PlanApprove",
        serde_json::json!({
            "remainingMinor": 3_000_000,
            "scale": 2,
            "approvedOn": "2026-08-18"
        }),
    )
    .await;
    assert_eq!(plan["remainingMinor"].as_i64().unwrap(), 3_000_000);
    assert_eq!(plan["version"].as_u64().unwrap(), 1);

    let after = query_json(&platform, "DividendGet").await;
    assert_eq!(after["actualTotalMinor"].as_i64().unwrap(), 50_000);

    let plan_get = query_json(&platform, "PlanGet").await;
    assert_eq!(plan_get["remainingMinor"].as_i64().unwrap(), 3_000_000);
    assert_eq!(plan_get["version"].as_u64().unwrap(), 1);

    let burn = query_json(&platform, "BurndownGet").await;
    assert_eq!(burn["obligationMinor"].as_i64().unwrap(), 3_000_000);
    assert_eq!(burn["cashMinor"].as_i64().unwrap(), 50_000);
    assert_eq!(burn["surplusMinor"].as_i64().unwrap(), 50_000 - 3_000_000);
    assert_eq!(burn["sufficient"].as_bool().unwrap(), false);

    must_ok(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": account_id,
            "activityType": "deposit",
            "amountMinor": 4_000_000,
            "scale": 2,
            "occurredOn": "2026-08-18",
            "idempotencyKey": "plan-deposit-1"
        }),
    )
    .await;
    let burn_after = query_json(&platform, "BurndownGet").await;
    assert_eq!(burn_after["cashMinor"].as_i64().unwrap(), 4_050_000);
    assert_eq!(burn_after["obligationMinor"].as_i64().unwrap(), 3_000_000);
    assert_eq!(burn_after["surplusMinor"].as_i64().unwrap(), 1_050_000);
    assert_eq!(burn_after["sufficient"].as_bool().unwrap(), true);

    let still = query_json(&platform, "DividendGet").await;
    assert_eq!(still["actualTotalMinor"].as_i64().unwrap(), 50_000);
}
