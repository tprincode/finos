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

async fn query_json(
    platform: &LocalPlatform,
    name: &str,
    body: Option<serde_json::Value>,
) -> serde_json::Value {
    let result = execute_query_on(platform, platform, qry(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

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

fn missing_open(exceptions: &serde_json::Value) -> Vec<&serde_json::Value> {
    exceptions
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| {
            e["code"].as_str() == Some("missing_cash_dividend") && e["acknowledged"] == false
        })
        .collect()
}

#[tokio::test]
async fn open_spaxx_without_august_broker_cash_raises_missing_cash_dividend() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let income = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let acct9 = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Account 9", "kind": "taxable"}),
    )
    .await;
    let spaxx = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "SPAXX", "name": "SPAXX"}),
    )
    .await;
    let swvxx = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "SWVXX", "name": "SWVXX"}),
    )
    .await;
    let spaxx_id = spaxx["securityId"].as_str().unwrap();
    let swvxx_id = swvxx["securityId"].as_str().unwrap();
    research_template(&platform, spaxx_id, "SPAXX").await;
    research_template(&platform, swvxx_id, "SWVXX").await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": income["accountId"],
            "securityId": spaxx_id,
            "openedOn": "2026-01-01",
            "origin": "purchase",
            "quantityMinor": 100,
            "quantityScale": 0,
            "performanceBasisMinor": 10_000,
            "taxBasisMinor": 10_000,
            "scale": 2
        }),
    )
    .await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": acct9["accountId"],
            "securityId": swvxx_id,
            "openedOn": "2026-01-01",
            "origin": "purchase",
            "quantityMinor": 50,
            "quantityScale": 0,
            "performanceBasisMinor": 5_000,
            "taxBasisMinor": 5_000,
            "scale": 2
        }),
    )
    .await;

    let cover = query_json(
        &platform,
        "CashDividendCoverageGet",
        Some(serde_json::json!({"asOfDate": "2026-08-15"})),
    )
    .await;
    assert_eq!(cover["month"], "2026-08");
    assert_eq!(cover["missingCount"].as_u64(), Some(2));
    let exceptions = query_json(&platform, "ExceptionList", None).await;
    let open = missing_open(&exceptions);
    assert_eq!(open.len(), 2, "{exceptions}");
    assert!(open.iter().any(|e| e["message"]
        .as_str()
        .unwrap()
        .contains("Income SPAXX 2026-08")));
    assert!(open.iter().any(|e| e["message"]
        .as_str()
        .unwrap()
        .contains("Account 9 SWVXX 2026-08")));

    let again = query_json(
        &platform,
        "CashDividendCoverageGet",
        Some(serde_json::json!({"asOfDate": "2026-08-15"})),
    )
    .await;
    assert_eq!(again["raisedCount"].as_u64(), Some(0));
    assert_eq!(missing_open(&query_json(&platform, "ExceptionList", None).await).len(), 2);

    let csv = "History: All Accounts\n\
From: 08/01/2026\n\
\n\
\"Date\",\"Account\",\"Symbol\",\"Description\",\"Quantity\",\"Price\",\"Amount\",\"Commission\",\"Fees\",\"Type\"\n\
\"08/01/2026\",\"Income (00000)\",\"SPAXX\",\"DIVIDEND RECEIVED\",\"0\",\"$0.00\",\"$21.27\",\"$0.00\",\"$0.00\",\"Cash\"\n";
    let staged = must_ok(
        &platform,
        "ImportStage",
        serde_json::json!({
            "sourceId": "spaxx-aug",
            "filename": "spaxx.csv",
            "content": csv,
        }),
    )
    .await;
    let batch_id = staged["batchId"].as_str().unwrap();
    for name in ["ImportValidate", "ImportApprove", "ImportPost"] {
        must_ok(&platform, name, serde_json::json!({"batchId": batch_id})).await;
    }

    let after = query_json(
        &platform,
        "CashDividendCoverageGet",
        Some(serde_json::json!({"asOfDate": "2026-08-15"})),
    )
    .await;
    let spaxx_row = after["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "SPAXX")
        .unwrap();
    assert_eq!(spaxx_row["present"], true);
    let income_id = income["accountId"].clone();
    let dup = must_ok(
        &platform,
        "DividendActualRecord",
        serde_json::json!({
            "accountId": income_id,
            "securityId": spaxx_id,
            "occurredOn": "2026-08-01",
            "amountMinor": 2_127,
            "scale": 2,
            "idempotencyKey": "manual-after-csv-spaxx"
        }),
    )
    .await;
    assert_eq!(dup["alreadyPosted"].as_bool(), Some(true));
    let dividends = query_json(&platform, "DividendGet", None).await;
    let spaxx_actuals = dividends["actuals"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|a| a["occurredOn"] == "2026-08-01" && a["amountMinor"] == 2_127)
        .count();
    assert_eq!(spaxx_actuals, 1);
    let swvxx_row = after["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "SWVXX")
        .unwrap();
    assert_eq!(swvxx_row["present"], false);
    let exceptions_after = query_json(&platform, "ExceptionList", None).await;
    let open_after = missing_open(&exceptions_after);
    assert_eq!(open_after.len(), 1, "{open_after:?}");
    assert!(open_after[0]["message"]
        .as_str()
        .unwrap()
        .contains("SWVXX"));

    must_ok(
        &platform,
        "DividendActualRecord",
        serde_json::json!({
            "accountId": acct9["accountId"],
            "securityId": swvxx_id,
            "occurredOn": "2026-08-31",
            "amountMinor": 12,
            "scale": 2,
            "idempotencyKey": "manual-acct9-swvxx-2026-08-31-12"
        }),
    )
    .await;
    let covered = query_json(
        &platform,
        "CashDividendCoverageGet",
        Some(serde_json::json!({"asOfDate": "2026-08-31"})),
    )
    .await;
    let swvxx_done = covered["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "SWVXX")
        .unwrap();
    assert_eq!(swvxx_done["present"], true);
    let exceptions_done = query_json(&platform, "ExceptionList", None).await;
    let open_done = missing_open(&exceptions_done);
    assert!(
        open_done
            .iter()
            .all(|e| !e["message"].as_str().unwrap_or("").contains("SWVXX")),
        "{open_done:?}"
    );
}
