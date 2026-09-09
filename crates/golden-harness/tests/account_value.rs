//! Home account-value series: live last price plus stored Trends on the same chart.

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

async fn must_ok(platform: &LocalPlatform, name: &str, body: serde_json::Value) -> serde_json::Value {
    let result = execute_command_on(platform, platform, cmd(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

async fn query(platform: &LocalPlatform, name: &str, body: serde_json::Value) -> serde_json::Value {
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

#[tokio::test]
async fn home_charts_combine_live_last_price_and_stored_trends() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .expect("open sqlite");
    must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "ENERGYX", "kind": "taxable"}),
    )
    .await;
    must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Robinhood", "kind": "taxable"}),
    )
    .await;

    must_ok(
        &platform,
        "TrendsWeekSave",
        serde_json::json!({
            "periodStart": "2026-08-29",
            "periodEnd": "2026-09-04",
            "capturedAt": "2026-09-04T18:00:00Z",
            "profitMinor": 1,
            "monthlyDivsMinor": 1,
            "fidelityTotalMinor": 31000000,
            "schwabTotalMinor": 3200000,
            "incomeCashMinor": 1,
            "acct9CashMinor": 1,
            "acct9EtfValueMinor": 1,
            "incomeBalanceMinor": 21200000,
            "scale": 2
        }),
    )
    .await;
    must_ok(
        &platform,
        "AccountValueSnapshotRecord",
        serde_json::json!({"asOfDate": "2026-09-09"}),
    )
    .await;

    let home = query(
        &platform,
        "AccountValueHomeGet",
        serde_json::json!({"asOfDate": "2026-09-09"}),
    )
    .await;
    assert_eq!(home["fidelity"]["accountName"], "Fidelity Total");
    assert_eq!(home["fidelity"]["currentMinor"], 0);
    assert_eq!(home["schwab"]["currentMinor"], 0);
    let trends = home["fidelity"]["trendsPoints"].as_array().unwrap();
    assert!(
        trends
            .iter()
            .any(|p| p["asOf"] == "2026-09-04" && p["marketValueMinor"] == 31000000),
        "{trends:?}"
    );
    let live = home["fidelity"]["points"].as_array().unwrap();
    assert!(
        live.iter()
            .any(|p| p["asOf"] == "2026-09-09" && p["marketValueMinor"] == 0),
        "{live:?}"
    );

    let names: Vec<&str> = home["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["accountName"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"Income"), "{names:?}");
    assert!(names.contains(&"ENERGYX"), "{names:?}");
    let energy = home["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["accountName"] == "ENERGYX")
        .unwrap();
    assert_eq!(energy["custodian"], "Direct", "{energy}");
    let income = home["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["accountName"] == "Income")
        .unwrap();
    assert_eq!(income["custodian"], "Fidelity");
    assert_eq!(income["currentMinor"], 0);
    assert!(
        income["trendsPoints"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["marketValueMinor"] == 21200000),
        "{income}"
    );
}
