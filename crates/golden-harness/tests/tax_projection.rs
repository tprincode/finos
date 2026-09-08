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
async fn tax_projection_matches_magi_and_does_not_post_dividend_or_rewrite_oracles() {
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
            "idempotencyKey": "tax-div-1"
        }),
    )
    .await;
    must_ok(
        &platform,
        "MagiRuleSet",
        serde_json::json!({
            "thresholdMinor": 8_460_000,
            "safetyReserveMinor": 500_000,
            "scale": 2
        }),
    )
    .await;
    must_ok(
        &platform,
        "MagiFactRecord",
        serde_json::json!({
            "sourceId": "w2-1",
            "treatment": "include",
            "amountMinor": 300_000,
            "scale": 2,
            "category": "wages"
        }),
    )
    .await;
    must_ok(
        &platform,
        "MagiCoverageSet",
        serde_json::json!({
            "completeness": "complete",
            "remainingMinor": 0,
            "withholdingMinor": 0,
            "formTotalMinor": 300_000,
            "warnings": []
        }),
    )
    .await;

    let dividend_before = query_json(&platform, "DividendGet").await;
    let magi = query_json(&platform, "MagiProjectionGet").await;
    assert_eq!(dividend_before["actualTotalMinor"].as_i64().unwrap(), 50_000);
    assert_eq!(magi["decisionState"].as_str().unwrap(), "SAFE");

    let tax = query_json(&platform, "TaxProjectionGet").await;
    assert_eq!(tax["sourceQuery"].as_str().unwrap(), "MagiProjectionGet");
    assert_eq!(tax["decisionState"], magi["decisionState"]);
    assert_eq!(tax["actualIncludedYtd"], magi["actualIncludedYtd"]);
    assert_eq!(tax["applicableThreshold"], magi["applicableThreshold"]);
    assert_eq!(tax["dataCompleteness"], magi["dataCompleteness"]);

    let magi_after = query_json(&platform, "MagiProjectionGet").await;
    let dividend_after = query_json(&platform, "DividendGet").await;
    assert_eq!(magi_after["decisionState"], magi["decisionState"]);
    assert_eq!(
        dividend_after["actualTotalMinor"],
        dividend_before["actualTotalMinor"]
    );
}
