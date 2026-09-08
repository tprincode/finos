use application_core::contracts::{
    CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::queries::{execute_command_on, execute_query_on};
use golden_harness::repo_root;
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

async fn must_ok(platform: &LocalPlatform, name: &str, body: serde_json::Value) -> serde_json::Value {
    let result = execute_command_on(platform, platform, cmd(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

async fn query_json(platform: &LocalPlatform, name: &str) -> serde_json::Value {
    query_json_body(platform, name, serde_json::json!({})).await
}

async fn query_json_body(
    platform: &LocalPlatform,
    name: &str,
    body: serde_json::Value,
) -> serde_json::Value {
    let result = execute_query_on(
        platform,
        platform,
        QueryRequest {
            contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
            query_name: name.to_string(),
            correlation_id: Uuid::new_v4(),
            body_json: Some(body.to_string()),
        },
    )
    .await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

async fn register_taxable(platform: &LocalPlatform) {
    must_ok(
        platform,
        "AccountRegister",
        serde_json::json!({"name": "Taxable Brokerage", "kind": "taxable"}),
    )
    .await;
    must_ok(
        platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "CASH", "name": "USD Cash"}),
    )
    .await;
}

async fn stage_and_post(platform: &LocalPlatform, source_id: &str, filename: &str, content: &str) {
    let staged = must_ok(
        platform,
        "ImportStage",
        serde_json::json!({
            "sourceId": source_id,
            "filename": filename,
            "content": content,
            "accountName": "Taxable Brokerage"
        }),
    )
    .await;
    let batch_id = staged["batchId"].as_str().unwrap();
    for name in ["ImportValidate", "ImportApprove", "ImportPost"] {
        must_ok(platform, name, serde_json::json!({"batchId": batch_id})).await;
    }
}

async fn assert_views_agree(platform: &LocalPlatform, expected_minor: i64, expected_count: usize) {
    let dividend = query_json(platform, "DividendGet").await;
    let income = query_json(platform, "IncomePlanGet").await;
    let dashboard = query_json(platform, "DashboardGet").await;
    let trends = query_json(platform, "TrendsGet").await;
    let actual = dividend["actualTotalMinor"].as_i64().unwrap();
    assert_eq!(actual, expected_minor);
    assert_eq!(income["actualMinor"].as_i64().unwrap(), actual);
    assert_eq!(dashboard["actualDividendMinor"].as_i64().unwrap(), actual);
    assert_eq!(trends["totalMinor"].as_i64().unwrap(), actual);
    assert_eq!(dividend["actuals"].as_array().unwrap().len(), expected_count);
}

#[tokio::test]
async fn fidelity_reimport_posts_one_actual_and_all_views_agree() {
    let root = repo_root();
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    register_taxable(&platform).await;
    let fidelity = std::fs::read_to_string(root.join("tests/golden/fixtures/fidelity-dividend.csv"))
        .unwrap();

    stage_and_post(&platform, "fidelity-div-2026", "fidelity-dividend.csv", &fidelity).await;
    stage_and_post(&platform, "fidelity-div-2026", "fidelity-dividend.csv", &fidelity).await;

    assert_views_agree(&platform, 50_000, 1).await;

    must_ok(
        &platform,
        "DividendDeclare",
        serde_json::json!({
            "securitySymbol": "CASH",
            "declaredOn": "2026-02-01",
            "amountMinor": 999999,
            "scale": 2
        }),
    )
    .await;
    assert_views_agree(&platform, 50_000, 1).await;
    let after_declare = query_json(&platform, "DividendGet").await;
    assert_eq!(after_declare["declarations"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn schwab_capture_matches_fidelity_amount_on_the_same_views() {
    let root = repo_root();
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    register_taxable(&platform).await;
    let schwab = std::fs::read_to_string(root.join("tests/golden/fixtures/schwab-dividend.csv"))
        .unwrap();
    stage_and_post(&platform, "schwab-div-2026", "schwab-dividend.csv", &schwab).await;
    assert_views_agree(&platform, 50_000, 1).await;
}

#[tokio::test]
async fn import_batch_get_lists_rows_and_flags_duplicate_without_posting_again() {
    let root = repo_root();
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    register_taxable(&platform).await;
    let fidelity = std::fs::read_to_string(root.join("tests/golden/fixtures/fidelity-dividend.csv"))
        .unwrap();
    let staged = must_ok(
        &platform,
        "ImportStage",
        serde_json::json!({
            "sourceId": "wiz-1",
            "filename": "fidelity-dividend.csv",
            "content": fidelity,
            "accountName": "Taxable Brokerage"
        }),
    )
    .await;
    let batch_id = staged["batchId"].as_str().unwrap();
    must_ok(
        &platform,
        "ImportValidate",
        serde_json::json!({"batchId": batch_id}),
    )
    .await;
    let preview = query_json_body(
        &platform,
        "ImportBatchGet",
        serde_json::json!({"batchId": batch_id}),
    )
    .await;
    assert_eq!(preview["status"], "validated");
    let rows = preview["candidates"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["accountName"], "Taxable Brokerage");
    assert_eq!(rows[0]["symbol"], "CASH");
    assert_eq!(rows[0]["validation"], "ready");
    let pending = query_json(&platform, "ImportPendingGet").await;
    assert_eq!(pending["batch"]["batchId"], batch_id);

    must_ok(
        &platform,
        "ImportApprove",
        serde_json::json!({"batchId": batch_id}),
    )
    .await;
    must_ok(
        &platform,
        "ImportPost",
        serde_json::json!({"batchId": batch_id}),
    )
    .await;
    let staged2 = must_ok(
        &platform,
        "ImportStage",
        serde_json::json!({
            "sourceId": "wiz-2",
            "filename": "fidelity-dividend.csv",
            "content": fidelity,
            "accountName": "Taxable Brokerage"
        }),
    )
    .await;
    let batch2 = staged2["batchId"].as_str().unwrap();
    must_ok(
        &platform,
        "ImportValidate",
        serde_json::json!({"batchId": batch2}),
    )
    .await;
    let dup = query_json_body(
        &platform,
        "ImportBatchGet",
        serde_json::json!({"batchId": batch2}),
    )
    .await;
    assert_eq!(dup["candidates"][0]["validation"], "duplicate");
    assert_eq!(dup["status"], "validated");
    let still = query_json(&platform, "DividendGet").await;
    assert_eq!(still["actuals"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn import_batch_get_blocks_unregistered_account() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let staged = must_ok(
        &platform,
        "ImportStage",
        serde_json::json!({
            "sourceId": "wiz-blocked",
            "filename": "ghost.csv",
            "content": "x",
            "candidates": [{
                "accountName": "Not An Account",
                "symbol": "CASH",
                "activityType": "dividend",
                "amountMinor": 100,
                "scale": 2,
                "occurredOn": "2026-08-01"
            }]
        }),
    )
    .await;
    let batch_id = staged["batchId"].as_str().unwrap();
    let preview = query_json_body(
        &platform,
        "ImportBatchGet",
        serde_json::json!({"batchId": batch_id}),
    )
    .await;
    assert_eq!(preview["candidates"][0]["validation"], "blocked");
    assert!(preview["candidates"][0]["issue"]
        .as_str()
        .unwrap()
        .contains("account not registered"));
    let pending = query_json(&platform, "ImportPendingGet").await;
    assert_eq!(pending["batch"]["status"], "staged");
}
