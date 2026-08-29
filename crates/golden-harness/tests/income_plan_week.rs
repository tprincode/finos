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
async fn week_dividend_actuals_match_dividend_get_by_account() {
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
    must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Car", "kind": "taxable"}),
    )
    .await;
    must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Account 9", "kind": "taxable"}),
    )
    .await;
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "VTI", "name": "Vanguard Total"}),
    )
    .await;
    must_ok(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": income["accountId"],
            "securityId": security["securityId"],
            "activityType": "dividend",
            "amountMinor": 12_500,
            "scale": 2,
            "occurredOn": "2026-08-17",
            "idempotencyKey": "week-income-vti"
        }),
    )
    .await;
    let car = query_json(&platform, "AccountList", serde_json::json!({})).await;
    let car_id = car
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["name"] == "Car")
        .unwrap()["accountId"]
        .clone();
    must_ok(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": car_id,
            "securityId": security["securityId"],
            "activityType": "dividend",
            "amountMinor": 4_000,
            "scale": 2,
            "occurredOn": "2026-08-19",
            "idempotencyKey": "week-car-vti"
        }),
    )
    .await;
    must_ok(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": income["accountId"],
            "securityId": security["securityId"],
            "activityType": "dividend",
            "amountMinor": 99_000,
            "scale": 2,
            "occurredOn": "2026-08-10",
            "idempotencyKey": "prior-week"
        }),
    )
    .await;

    let dividend = query_json(&platform, "DividendGet", serde_json::json!({})).await;
    let week_actuals: i64 = dividend["actuals"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| {
            let on = row["occurredOn"].as_str().unwrap_or("");
            on >= "2026-08-15" && on <= "2026-08-21"
        })
        .map(|row| row["amountMinor"].as_i64().unwrap_or(0))
        .sum();

    let week = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({"asOfDate": "2026-08-18"}),
    )
    .await;
    assert_eq!(week["start"], "2026-08-15");
    assert_eq!(week["end"], "2026-08-21");
    assert_eq!(week["status"], "Open");
    let income_line = week["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["accountName"] == "Income")
        .unwrap();
    assert_eq!(income_line["actualMinor"].as_i64().unwrap(), 12_500);
    assert_eq!(income_line["planKnown"], false);
    let car_line = week["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["accountName"] == "Car")
        .unwrap();
    assert_eq!(car_line["actualMinor"].as_i64().unwrap(), 4_000);
    let nine = week["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["accountName"] == "Account 9")
        .unwrap();
    assert_eq!(nine["actualMinor"].as_i64().unwrap(), 0);
    let grid_total: i64 = week["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["actualMinor"].as_i64().unwrap_or(0))
        .sum();
    assert_eq!(grid_total, week_actuals);

    let burn = query_json(
        &platform,
        "DashboardBurndownGet",
        serde_json::json!({"asOfDate": "2026-08-18"}),
    )
    .await;
    let burn_nine = burn["lines"]
        .as_array()
        .unwrap()
        .iter()
        .any(|l| l["accountName"] == "Account 9");
    assert!(!burn_nine, "Account 9 must not appear on burndown");
    let burn_income = burn["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["accountName"] == "Income")
        .unwrap();
    assert_eq!(burn_income["inflowMinor"].as_i64().unwrap(), 12_500);
    assert_eq!(burn_income["floorKnown"], false);

    let latest = query_json(&platform, "IncomePlanWeekGet", serde_json::json!({})).await;
    assert_eq!(latest["latestActualOn"], "2026-08-19");
    assert_eq!(latest["yieldCount"].as_u64().unwrap(), 3);
    let latest_income = latest["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["accountName"] == "Income")
        .unwrap();
    assert_eq!(latest_income["actualMinor"].as_i64().unwrap(), 12_500);

    let summary = query_json(&platform, "DataSummaryGet", serde_json::json!({})).await;
    assert_eq!(summary["accountCount"].as_u64().unwrap(), 3);
    assert_eq!(summary["yieldCount"].as_u64().unwrap(), 3);
    assert_eq!(summary["latestYieldOn"], "2026-08-19");
}
