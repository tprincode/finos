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
async fn cart_item_does_not_post_dividend_or_lot_facts() {
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
            "idempotencyKey": "cart-div-1"
        }),
    )
    .await;
    let before = query_json(&platform, "DividendGet").await;
    assert_eq!(before["actualTotalMinor"].as_i64().unwrap(), 50_000);
    let lots_before = query_json(&platform, "BasisGet").await;
    assert_eq!(lots_before["lots"].as_array().unwrap().len(), 0);

    let added = must_ok(
        &platform,
        "CartItemAdd",
        serde_json::json!({
            "symbol": "VXUS",
            "quantityMinor": 10_000,
            "quantityScale": 2
        }),
    )
    .await;
    assert_eq!(added["items"].as_array().unwrap().len(), 1);
    assert_eq!(added["items"][0]["symbol"].as_str().unwrap(), "VXUS");
    assert_eq!(added["items"][0]["quantityMinor"].as_i64().unwrap(), 10_000);

    let got = query_json(&platform, "CartGet").await;
    assert_eq!(got["items"][0]["symbol"].as_str().unwrap(), "VXUS");
    let item_id = got["items"][0]["itemId"].as_str().unwrap();

    let after_add = query_json(&platform, "DividendGet").await;
    assert_eq!(after_add["actualTotalMinor"].as_i64().unwrap(), 50_000);
    let lots_after_add = query_json(&platform, "BasisGet").await;
    assert_eq!(lots_after_add["lots"].as_array().unwrap().len(), 0);

    let removed = must_ok(
        &platform,
        "CartItemRemove",
        serde_json::json!({"itemId": item_id}),
    )
    .await;
    assert_eq!(removed["items"].as_array().unwrap().len(), 0);

    let empty = query_json(&platform, "CartGet").await;
    assert_eq!(empty["items"].as_array().unwrap().len(), 0);
    let after_remove = query_json(&platform, "DividendGet").await;
    assert_eq!(after_remove["actualTotalMinor"].as_i64().unwrap(), 50_000);
}
