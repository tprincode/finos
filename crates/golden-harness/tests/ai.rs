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

fn qry_body(name: &str, body: serde_json::Value) -> QueryRequest {
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

async fn query_json(platform: &LocalPlatform, name: &str) -> serde_json::Value {
    let result = execute_query_on(platform, platform, qry(name)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

#[tokio::test]
async fn ai_analyze_does_not_post_dividend_facts() {
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
            "idempotencyKey": "ai-div-1"
        }),
    )
    .await;
    let before = query_json(&platform, "DividendGet").await;
    assert_eq!(before["actualTotalMinor"].as_i64().unwrap(), 50_000);

    let body = must_ok(
        &platform,
        "AiAnalyze",
        serde_json::json!({"prompt": "Should I rebalance VXUS? XAI_API_KEY=should-not-store"}),
    )
    .await;
    assert_eq!(body["status"].as_str().unwrap(), "completed");
    assert_eq!(body["provider"].as_str().unwrap(), "stub");
    assert_eq!(
        body["recommendation"].as_str().unwrap(),
        "advisory-only; does not post facts"
    );
    let prompt = body["prompt"].as_str().unwrap();
    assert!(!prompt.contains("should-not-store"));
    assert!(prompt.contains("[redacted]"));
    let run_id = body["runId"].as_str().unwrap();

    let got = execute_query_on(
        &platform,
        &platform,
        qry_body("AiRunGet", serde_json::json!({"runId": run_id})),
    )
    .await;
    assert!(got.ok, "AiRunGet failed: {:?}", got.error_code);
    let got_json: serde_json::Value =
        serde_json::from_str(got.body_json.as_deref().unwrap_or("{}")).unwrap();
    assert_eq!(got_json["runId"].as_str().unwrap(), run_id);

    let listed = query_json(&platform, "AnalysisRunList").await;
    assert_eq!(listed["runs"].as_array().unwrap().len(), 1);

    let after = query_json(&platform, "DividendGet").await;
    assert_eq!(after["actualTotalMinor"].as_i64().unwrap(), 50_000);
}
