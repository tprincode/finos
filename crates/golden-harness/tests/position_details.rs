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
async fn position_details_rollup_does_not_post_dividend_or_change_basis_totals() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let taxable = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Taxable Brokerage", "kind": "taxable"}),
    )
    .await;
    let account_id = taxable["accountId"].as_str().unwrap();
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "AAPL", "name": "Apple"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();

    must_ok(
        &platform,
        "DividendActualRecord",
        serde_json::json!({
            "accountId": account_id,
            "occurredOn": "2026-06-15",
            "amountMinor": 50_000,
            "scale": 2,
            "idempotencyKey": "pos-div-1"
        }),
    )
    .await;

    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "openedOn": "2026-01-05",
            "origin": "purchase",
            "quantityMinor": 10,
            "quantityScale": 0,
            "performanceBasisMinor": 100_000,
            "taxBasisMinor": 80_000,
            "scale": 2
        }),
    )
    .await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "openedOn": "2026-02-05",
            "origin": "purchase",
            "quantityMinor": 10,
            "quantityScale": 0,
            "performanceBasisMinor": 40_000,
            "taxBasisMinor": 40_000,
            "scale": 2
        }),
    )
    .await;

    let dividend_before = query_json(&platform, "DividendGet").await;
    let basis_before = query_json(&platform, "BasisGet").await;
    assert_eq!(dividend_before["actualTotalMinor"].as_i64().unwrap(), 50_000);
    assert_eq!(basis_before["openPerformanceMinor"].as_i64().unwrap(), 140_000);
    assert_eq!(basis_before["lots"].as_array().unwrap().len(), 2);

    let details = query_json(&platform, "PositionDetailsGet").await;
    let positions = details["positions"].as_array().unwrap();
    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0]["symbol"].as_str().unwrap(), "AAPL");
    assert_eq!(positions[0]["accountName"].as_str().unwrap(), "Taxable Brokerage");
    assert_eq!(positions[0]["remainingQuantityMinor"].as_i64().unwrap(), 20);
    assert_eq!(positions[0]["remainingPerformanceMinor"].as_i64().unwrap(), 140_000);
    assert_eq!(positions[0]["remainingTaxMinor"].as_i64().unwrap(), 120_000);
    assert_eq!(positions[0]["lotCount"].as_u64().unwrap(), 2);
    assert_eq!(details["openPerformanceMinor"].as_i64().unwrap(), 140_000);
    assert_eq!(details["openTaxMinor"].as_i64().unwrap(), 120_000);

    let dividend_after = query_json(&platform, "DividendGet").await;
    let basis_after = query_json(&platform, "BasisGet").await;
    assert_eq!(
        dividend_after["actualTotalMinor"],
        dividend_before["actualTotalMinor"]
    );
    assert_eq!(
        basis_after["openPerformanceMinor"],
        basis_before["openPerformanceMinor"]
    );
    assert_eq!(basis_after["lots"].as_array().unwrap().len(), 2);
}
