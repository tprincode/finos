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
async fn recovery_drill_restore_returns_posted_dividend_to_pre_mutation_state() {
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

    let created = must_ok(&platform, "SnapshotCreate", serde_json::json!({})).await;
    let snapshot_id = created["snapshotId"].as_str().unwrap();
    let created_hash = created["databaseHash"].as_str().unwrap();
    assert_eq!(created_hash.len(), 64);
    assert_eq!(query_json(&platform, "DividendGet").await["actualTotalMinor"].as_i64().unwrap(), 0);

    must_ok(
        &platform,
        "DividendActualRecord",
        serde_json::json!({
            "accountId": account_id,
            "occurredOn": "2026-06-15",
            "amountMinor": 50_000,
            "scale": 2,
            "idempotencyKey": "recovery-div-1"
        }),
    )
    .await;
    assert_eq!(
        query_json(&platform, "DividendGet").await["actualTotalMinor"]
            .as_i64()
            .unwrap(),
        50_000
    );

    let restored = must_ok(
        &platform,
        "SnapshotRestore",
        serde_json::json!({"snapshotId": snapshot_id}),
    )
    .await;
    assert_eq!(restored["databaseHash"].as_str().unwrap(), created_hash);
    assert_eq!(restored["restoreTestStatus"].as_str().unwrap(), "passed");
    assert_eq!(
        query_json(&platform, "DividendGet").await["actualTotalMinor"]
            .as_i64()
            .unwrap(),
        0
    );
}
