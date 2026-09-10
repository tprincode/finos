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
    assert!(
        live.iter().any(|p| p["asOf"] == "2026-09-08"),
        "yesterday stays on the axis: {live:?}"
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

async fn open_priced_lot(
    platform: &LocalPlatform,
    account_id: &str,
    symbol: &str,
    risk_tier: &str,
    qty: i64,
    price_minor: i64,
    as_of: &str,
) -> String {
    let security = must_ok(
        platform,
        "SecurityRegister",
        serde_json::json!({"symbol": symbol, "name": symbol}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap().to_string();
    must_ok(
        platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "priceSource": "public",
            "sourceSymbol": symbol,
            "declarationSource": "issuer",
            "sourceUrl": format!("https://example.test/{symbol}/distributions"),
            "calendarPolicy": "derived_walk",
            "collectorEnabled": true,
            "lookbackCount": 12
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot(platform, &security_id, symbol)
        .await
        .expect("complete collector");
    must_ok(
        platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "openedOn": "2026-01-05",
            "origin": "purchase",
            "quantityMinor": qty,
            "quantityScale": 0,
            "performanceBasisMinor": qty * price_minor,
            "taxBasisMinor": qty * price_minor,
            "scale": 2
        }),
    )
    .await;
    must_ok(
        platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "riskTier": risk_tier,
            "provider": "Test"
        }),
    )
    .await;
    must_ok(
        platform,
        "LastPriceRefresh",
        serde_json::json!({
            "quotes": [{
                "securityId": security_id,
                "priceMinor": price_minor,
                "scale": 2,
                "asOfAt": as_of,
                "source": "test"
            }]
        }),
    )
    .await;
    security_id
}

#[tokio::test]
async fn home_charts_stack_live_value_by_risk_tier() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .expect("open sqlite");
    let income = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let account_id = income["accountId"].as_str().unwrap();
    let haky_id =
        open_priced_lot(&platform, account_id, "HAKY", "Core", 10, 2000, "2026-09-08").await;
    let cony_id =
        open_priced_lot(&platform, account_id, "CONY", "Risk On", 5, 3500, "2026-09-08").await;
    open_priced_lot(&platform, account_id, "FDRXX", "Foundation", 100, 100, "2026-09-08").await;
    must_ok(
        &platform,
        "AccountValueSnapshotRecord",
        serde_json::json!({"asOfDate": "2026-09-08"}),
    )
    .await;
    must_ok(
        &platform,
        "LastPriceRefresh",
        serde_json::json!({
            "quotes": [
                {"securityId": haky_id, "priceMinor": 2500, "scale": 2, "asOfAt": "2026-09-09", "source": "test"},
                {"securityId": cony_id, "priceMinor": 4000, "scale": 2, "asOfAt": "2026-09-09", "source": "test"}
            ]
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
    let risk = &home["risk"];
    assert_eq!(risk["currentTotalMinor"], 100 * 100 + 10 * 2500 + 5 * 4000);
    assert_eq!(risk["currentComplete"], true);
    let groups = risk["groups"].as_array().unwrap();
    let foundation = groups.iter().find(|g| g["riskTier"] == "Foundation").unwrap();
    let core = groups.iter().find(|g| g["riskTier"] == "Core").unwrap();
    let risk_on = groups.iter().find(|g| g["riskTier"] == "Risk On").unwrap();
    assert_eq!(foundation["currentMinor"], 10_000);
    assert_eq!(core["currentMinor"], 25_000);
    assert_eq!(risk_on["currentMinor"], 20_000);
    assert_eq!(foundation["symbols"][0]["symbol"], "FDRXX");
    assert_eq!(core["symbols"][0]["symbol"], "HAKY");
    assert_eq!(risk_on["symbols"][0]["symbol"], "CONY");
    let points = risk["points"].as_array().unwrap();
    let today = points.iter().find(|p| p["asOf"] == "2026-09-09").unwrap();
    assert_eq!(today["foundationMinor"], 10_000);
    assert_eq!(today["coreMinor"], 25_000);
    assert_eq!(today["riskOnMinor"], 20_000);
    assert_eq!(today["totalMinor"], 55_000);
    let yesterday = points.iter().find(|p| p["asOf"] == "2026-09-08").unwrap();
    assert_eq!(yesterday["foundationMinor"], 10_000);
    assert_eq!(yesterday["coreMinor"], 20_000);
    assert_eq!(yesterday["riskOnMinor"], 17_500);
    assert_eq!(yesterday["totalMinor"], 47_500);
}

#[test]
fn home_charts_legend_replaces_sentence() {
    let ui = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/HomeAccountCharts.tsx"),
    )
    .unwrap();
    assert!(ui.contains("aria-label=\"Account value legend\""));
    assert!(ui.contains("aria-label=\"Home graphing period\""));
    assert!(ui.contains("aria-label=\"Live value by risk\""));
    assert!(ui.contains("<strong>Live</strong>"));
    assert!(ui.contains("<strong>Trends</strong>"));
    assert!(ui.contains("legend:"));
    assert!(ui.contains("right: 8"));
    assert!(ui.contains("aria-label=\"Symbol totals\""));
    assert!(ui.contains("aria-label=\"Exit symbol totals\""));
    assert!(ui.contains("role=\"dialog\""));
    assert!(ui.contains("dblclick"));
    assert!(ui.contains("home-av-risk-groups"));
    assert!(ui.contains("aria-label=\"Risk symbol groups\""));
    assert!(!ui.contains("<strong>Foundation</strong>"));
    assert!(!ui.contains(r#"name: "Undecided""#));
    assert!(!ui.contains("Undecided:"));
    assert!(ui.contains("GRAPH_PERIOD_OPTIONS"));
    assert!(!ui.contains("Live last price · dashed Trends"));
    assert!(!ui.contains("{values.note}"));
}

#[tokio::test]
async fn home_risk_includes_every_exact_quote_day() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .expect("open sqlite");
    let income = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let account_id = income["accountId"].as_str().unwrap();
    let haky_id =
        open_priced_lot(&platform, account_id, "HAKY", "Core", 10, 2500, "2026-09-09").await;
    must_ok(
        &platform,
        "LastPriceRefresh",
        serde_json::json!({
            "quotes": [
                {"securityId": haky_id, "priceMinor": 1800, "scale": 2, "asOfAt": "2026-09-07", "source": "test"},
                {"securityId": haky_id, "priceMinor": 2000, "scale": 2, "asOfAt": "2026-09-08", "source": "test"}
            ]
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
    let points = home["risk"]["points"].as_array().unwrap();
    let day = |as_of: &str| {
        points
            .iter()
            .find(|p| p["asOf"] == as_of)
            .unwrap_or_else(|| panic!("missing {as_of}: {points:?}"))
    };
    assert_eq!(day("2026-09-07")["coreMinor"], 18_000);
    assert_eq!(day("2026-09-08")["coreMinor"], 20_000);
    assert!(
        points.iter().any(|p| p["asOf"] == "2026-09-09"),
        "today live stays: {points:?}"
    );
}

#[tokio::test]
async fn home_risk_omits_day_without_quotes() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .expect("open sqlite");
    let income = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let account_id = income["accountId"].as_str().unwrap();
    open_priced_lot(&platform, account_id, "HAKY", "Core", 10, 2500, "2026-09-09").await;
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
    let points = home["risk"]["points"].as_array().unwrap();
    assert!(
        points.iter().any(|p| p["asOf"] == "2026-09-09"),
        "today live stays: {points:?}"
    );
    assert!(
        points.iter().all(|p| p["asOf"] != "2026-09-08"),
        "a day with no stored last price is not invented: {points:?}"
    );
}

#[tokio::test]
async fn home_risk_omits_thin_weekend_quote_day() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .expect("open sqlite");
    let income = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let account_id = income["accountId"].as_str().unwrap();
    let haky_id =
        open_priced_lot(&platform, account_id, "HAKY", "Core", 10, 2500, "2026-09-09").await;
    let cony_id =
        open_priced_lot(&platform, account_id, "CONY", "Risk On", 5, 3500, "2026-09-09").await;
    must_ok(
        &platform,
        "LastPriceRefresh",
        serde_json::json!({
            "quotes": [
                {"securityId": haky_id, "priceMinor": 2000, "scale": 2, "asOfAt": "2026-09-05", "source": "test"},
                {"securityId": haky_id, "priceMinor": 2100, "scale": 2, "asOfAt": "2026-09-08", "source": "test"},
                {"securityId": cony_id, "priceMinor": 4000, "scale": 2, "asOfAt": "2026-09-08", "source": "test"}
            ]
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
    let points = home["risk"]["points"].as_array().unwrap();
    assert!(
        points.iter().all(|p| p["asOf"] != "2026-09-05"),
        "a crypto-only / one-ticker weekend is not a session: {points:?}"
    );
    assert!(
        points.iter().any(|p| p["asOf"] == "2026-09-08"),
        "a full last-price session stays: {points:?}"
    );
}
