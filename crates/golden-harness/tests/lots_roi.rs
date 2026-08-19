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

fn qry(name: &str, body: Option<serde_json::Value>) -> QueryRequest {
    QueryRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        query_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: body.map(|v| v.to_string()),
    }
}

async fn must_ok(platform: &LocalPlatform, name: &str, body: serde_json::Value) -> serde_json::Value {
    let result = execute_command_on(platform, platform, cmd(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

async fn must_err(platform: &LocalPlatform, name: &str, body: serde_json::Value, code: &str) {
    let result = execute_command_on(platform, platform, cmd(name, body)).await;
    assert!(!result.ok, "{name} should fail");
    assert_eq!(result.error_code.as_deref(), Some(code));
}

async fn query_json(
    platform: &LocalPlatform,
    name: &str,
    body: Option<serde_json::Value>,
) -> serde_json::Value {
    let result = execute_query_on(platform, platform, qry(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

#[tokio::test]
async fn explicit_lots_dual_basis_no_fifo_and_broker_recon() {
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
        "ActivityPost",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "activityType": "buy",
            "amountMinor": 100_000,
            "scale": 2,
            "occurredOn": "2026-01-05"
        }),
    )
    .await;
    let expensive = must_ok(
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
        "ActivityPost",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "activityType": "buy",
            "amountMinor": 40_000,
            "scale": 2,
            "occurredOn": "2026-02-05"
        }),
    )
    .await;
    let cheap = must_ok(
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
    let recommend = query_json(
        &platform,
        "LotRecommendGet",
        Some(serde_json::json!({
            "accountId": account_id,
            "securityId": security_id
        })),
    )
    .await;
    assert_eq!(
        recommend["lotIds"][0].as_str().unwrap(),
        cheap["lotId"].as_str().unwrap()
    );

    let sell = must_ok(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "activityType": "sell",
            "amountMinor": 60_000,
            "scale": 2,
            "occurredOn": "2026-03-01"
        }),
    )
    .await;
    let before = query_json(&platform, "BrokerLotReconcileGet", None).await;
    assert_eq!(before["unmatchedSells"].as_u64().unwrap(), 1);
    assert_eq!(before["matched"].as_bool().unwrap(), false);

    must_ok(
        &platform,
        "LotAssign",
        serde_json::json!({
            "lotId": expensive["lotId"],
            "activityId": sell["activityId"],
            "quantityMinor": 10,
            "quantityScale": 0
        }),
    )
    .await;
    let after = query_json(&platform, "BrokerLotReconcileGet", None).await;
    assert_eq!(after["unmatchedSells"].as_u64().unwrap(), 0);
    assert_eq!(after["matched"].as_bool().unwrap(), true);
    assert_eq!(after["assignedQuantityMinor"].as_i64().unwrap(), 10);

    let roi = query_json(&platform, "RoiGet", None).await;
    assert_eq!(roi["proceedsMinor"].as_i64().unwrap(), 60_000);
    assert_eq!(roi["performanceCostMinor"].as_i64().unwrap(), 100_000);
    assert_eq!(roi["taxCostMinor"].as_i64().unwrap(), 80_000);
    assert_eq!(roi["performanceGainMinor"].as_i64().unwrap(), -40_000);
    assert_eq!(roi["taxGainMinor"].as_i64().unwrap(), -20_000);
    assert_ne!(
        roi["performanceGainMinor"].as_i64().unwrap(),
        roi["taxGainMinor"].as_i64().unwrap()
    );

    let basis = query_json(&platform, "BasisGet", None).await;
    assert_eq!(basis["openPerformanceMinor"].as_i64().unwrap(), 40_000);
    assert_eq!(basis["openTaxMinor"].as_i64().unwrap(), 40_000);
    let remaining_expensive = basis["lots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["lotId"] == expensive["lotId"])
        .unwrap();
    assert_eq!(remaining_expensive["remainingQuantityMinor"].as_i64().unwrap(), 0);
}

#[tokio::test]
async fn crf_policy_and_fi_roth_auto_capture() {
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
    let crf = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "CRF", "kind": "crf"}),
    )
    .await;
    must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "FI Roth", "kind": "fi_roth"}),
    )
    .await;
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "VTI", "name": "Vanguard Total"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();

    must_err(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": taxable["accountId"],
            "securityId": security_id,
            "openedOn": "2026-01-17",
            "origin": "drip",
            "quantityMinor": 1,
            "quantityScale": 0,
            "performanceBasisMinor": 0,
            "taxBasisMinor": 0,
            "scale": 2
        }),
        "zero_cost_drip_not_crf",
    )
    .await;

    let crf_lot = must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": crf["accountId"],
            "securityId": security_id,
            "openedOn": "2026-01-17",
            "origin": "drip",
            "quantityMinor": 3,
            "quantityScale": 0,
            "performanceBasisMinor": 0,
            "taxBasisMinor": 0,
            "scale": 2
        }),
    )
    .await;
    assert_eq!(crf_lot["crfZeroCost"].as_bool().unwrap(), true);

    let staged = must_ok(
        &platform,
        "ImportStage",
        serde_json::json!({
            "sourceId": "fi-roth-drip",
            "filename": "drip.txt",
            "content": "drip",
            "accountName": "FI Roth",
            "candidates": [{
                "accountName": "FI Roth",
                "symbol": "VTI",
                "activityType": "drip",
                "amountMinor": 0,
                "scale": 2,
                "occurredOn": "2026-01-18"
            }]
        }),
    )
    .await;
    let batch_id = staged["batchId"].as_str().unwrap();
    for name in ["ImportValidate", "ImportApprove", "ImportPost"] {
        must_ok(&platform, name, serde_json::json!({"batchId": batch_id})).await;
    }
    let basis = query_json(&platform, "BasisGet", None).await;
    let auto = basis["lots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["crfZeroCost"].as_bool() == Some(true) && l["quantityMinor"] == 1)
        .expect("FI Roth auto drip lot");
    assert_eq!(auto["origin"].as_str().unwrap(), "drip");

    let bad = must_ok(
        &platform,
        "ImportStage",
        serde_json::json!({
            "sourceId": "taxable-drip",
            "filename": "drip2.txt",
            "content": "drip2",
            "accountName": "Taxable Brokerage",
            "candidates": [{
                "accountName": "Taxable Brokerage",
                "symbol": "VTI",
                "activityType": "drip",
                "amountMinor": 0,
                "scale": 2,
                "occurredOn": "2026-01-19"
            }]
        }),
    )
    .await;
    let bad_id = bad["batchId"].as_str().unwrap();
    for name in ["ImportValidate", "ImportApprove", "ImportPost"] {
        must_ok(&platform, name, serde_json::json!({"batchId": bad_id})).await;
    }
    let exceptions = query_json(&platform, "ExceptionList", None).await;
    let hit = exceptions
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["code"].as_str() == Some("zero_cost_drip_not_crf"));
    assert!(hit);
}

#[tokio::test]
async fn option_close_requires_explicit_lot() {
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
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "AAPL", "name": "Apple"}),
    )
    .await;
    let lot = must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account["accountId"],
            "securityId": security["securityId"],
            "openedOn": "2026-01-02",
            "origin": "option",
            "quantityMinor": 1,
            "quantityScale": 0,
            "performanceBasisMinor": 5_000,
            "taxBasisMinor": 5_000,
            "scale": 2
        }),
    )
    .await;
    let close = must_ok(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": account["accountId"],
            "securityId": security["securityId"],
            "activityType": "option_close",
            "amountMinor": 7_000,
            "scale": 2,
            "occurredOn": "2026-04-01"
        }),
    )
    .await;
    let before = query_json(&platform, "BrokerLotReconcileGet", None).await;
    assert_eq!(before["matched"].as_bool().unwrap(), false);
    must_ok(
        &platform,
        "LotAssign",
        serde_json::json!({
            "lotId": lot["lotId"],
            "activityId": close["activityId"],
            "quantityMinor": 1,
            "quantityScale": 0
        }),
    )
    .await;
    let after = query_json(&platform, "BrokerLotReconcileGet", None).await;
    assert_eq!(after["matched"].as_bool().unwrap(), true);
    let roi = query_json(&platform, "RoiGet", None).await;
    assert_eq!(roi["performanceGainMinor"].as_i64().unwrap(), 2_000);
}
