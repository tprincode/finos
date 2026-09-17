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

#[tokio::test]
async fn home_schwab_graph_keeps_live_positions_and_typed_week() {
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
    let nine = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "9", "kind": "ira"}),
    )
    .await;
    let income_id = income["accountId"].as_str().unwrap();
    let nine_id = nine["accountId"].as_str().unwrap();
    open_priced_lot(
        &platform,
        income_id,
        "HAKY",
        "Core",
        10,
        2000,
        "2026-01-15",
    )
    .await;
    open_priced_lot(
        &platform,
        nine_id,
        "SWVXX",
        "Foundation",
        10_000,
        100,
        "2026-09-14",
    )
    .await;
    open_priced_lot(
        &platform,
        nine_id,
        "QDVO",
        "Core",
        10,
        2934,
        "2026-09-14",
    )
    .await;
    must_ok(
        &platform,
        "TrendsWeekSave",
        serde_json::json!({
            "periodStart": "2026-08-15",
            "periodEnd": "2026-08-21",
            "capturedAt": "2026-08-21T18:00:00Z",
            "profitMinor": 1,
            "monthlyDivsMinor": 1,
            "fidelityTotalMinor": 1,
            "schwabTotalMinor": 3_348_165,
            "incomeCashMinor": 1,
            "acct9CashMinor": 1,
            "acct9EtfValueMinor": 1,
            "scale": 2
        }),
    )
    .await;
    must_ok(
        &platform,
        "AccountValueSnapshotRecord",
        serde_json::json!({"asOfDate": "2026-09-14"}),
    )
    .await;

    let home = query(
        &platform,
        "AccountValueHomeGet",
        serde_json::json!({"asOfDate": "2026-09-14"}),
    )
    .await;
    let live_minor = 10_000 * 100 + 10 * 2934;
    assert_eq!(home["schwab"]["currentMinor"], live_minor);
    assert_eq!(home["schwab"]["currentComplete"], true);
    let trends = home["schwab"]["trendsPoints"].as_array().unwrap();
    assert!(
        trends
            .iter()
            .any(|p| p["asOf"] == "2026-08-21" && p["marketValueMinor"] == 3_348_165),
        "typed Schwab week must plot: {trends:?}"
    );
    let live = home["schwab"]["points"].as_array().unwrap();
    assert!(
        live.iter()
            .any(|p| p["asOf"] == "2026-09-14" && p["marketValueMinor"] == live_minor),
        "live qty × last price must plot today: {live:?}"
    );
    assert!(
        live.iter().all(|p| {
            p["asOf"] == "2026-09-14"
                || p["marketValueMinor"].is_null()
                || p["marketValueMinor"] != 1_000_000
        }),
        "cash-only SWVXX is not the Schwab live total: {live:?}"
    );
    let nine_row = home["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["accountName"] == "9")
        .unwrap();
    assert_eq!(nine_row["currentMinor"], live_minor);
    assert!(
        nine_row["trendsPoints"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["asOf"] == "2026-08-21" && p["marketValueMinor"] == 3_348_165),
        "{nine_row}"
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
        golden_harness::repo_root().join("apps/desktop/src/features/graphing/HomeAccountCharts.tsx"),
    )
    .unwrap();
    assert!(ui.contains("aria-label=\"Account value legend\""));
    assert!(ui.contains("home-av-card-head"));
    assert!(ui.contains("aria-label=\"Home graphing period\""));
    assert!(ui.contains("aria-label=\"Risk Profile\""));
    assert!(ui.contains("<strong>Live</strong>"));
    assert!(ui.contains("<strong>Trends</strong>"));
    assert!(ui.contains("<strong>Weekly actuals</strong>"));
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
    assert!(ui.contains("aria-label=\"Live by risk level\""));
    assert!(ui.contains("aria-label=\"Live by risk allocation\""));
    assert!(ui.contains("live-risk-pair"));
    assert!(ui.contains("type: \"pie\""));
    assert!(ui.contains(r#"radius: ["50%", "72%"]"#));
    assert!(ui.contains("Current allocation"));
    assert!(!ui.contains("aria-label=\"Live by risk mix\""));
    assert!(!ui.contains("stack: \"mix\""));
    assert!(!ui.contains("<RiskStackCard"));
    assert!(ui.contains("min: 0"));
    assert!(ui.contains("scale: false"));
    assert!(!ui.contains("stack: \"risk\""));
    assert!(!ui.contains("Live last price · dashed Trends"));
    assert!(!ui.contains("{values.note}"));
    assert!(!ui.contains("<LiveByRiskCharts"));
    let app = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/App.tsx"),
    )
    .unwrap();
    assert!(!app.contains("<LiveByRiskCharts"));
    assert!(app.contains("risk={accountValues?.risk ?? null}"));
    assert!(ui.contains("isTrendsPlacedAccount"));
    assert!(app.contains("accountValues={accountValues}"));
    let trends = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/features/graphing/TrendsCharts.tsx"),
    )
    .unwrap();
    assert!(trends.contains("aria-label=\"Trends account charts\""));
    assert!(trends.contains("isTrendsPlacedAccount"));
    assert!(trends.contains("<LiveByRiskCharts"));
    assert!(trends.contains("period={period}"));
    assert!(!ui.contains("Live by risk graphing period"));
    assert!(app.contains("home-top-row"));
    assert!(app.contains("<HomeDividendPlan"));
    assert!(app.contains("DividendPlanHomeGet"));
    let css = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/App.css"),
    )
    .unwrap();
    assert!(css.contains("home-top-row"));
    assert!(css.contains("--home-left-col: 580px"));
    assert!(css.contains("repeat(3, minmax(0, 1fr))"));
    let home_charts = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/features/graphing/HomeAccountCharts.tsx"),
    )
    .unwrap();
    assert!(home_charts.contains("home-av-toolbar"));
    assert!(home_charts.contains("showMaxLabel: true"));
    assert!(home_charts.contains("right: 36"));
    assert!(home_charts.contains(r#"chrome === "trends" && (hasTrends || hasIncome)"#));
    assert!(!home_charts.contains("top: showLegend ? 28 : 10"));
    let plan_ui = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/features/home/HomeDividendPlan.tsx"),
    )
    .unwrap();
    assert!(plan_ui.contains("aria-label=\"Dividend Plan\""));
    assert!(plan_ui.contains("Annual dividend"));
    assert!(plan_ui.contains("Monthly Medical"));
    assert!(!plan_ui.contains("Monthly Reinvest"));
    assert!(plan_ui.contains("Effective annual return"));
}

#[test]
fn home_graphing_period_includes_one_and_two_months() {
    let period = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/features/graphing/graphPeriod.ts"),
    )
    .unwrap();
    assert!(period.contains("\"1m\""));
    assert!(period.contains("2 months"));
    assert!(period.contains("2026-08-11"));
    assert!(period.contains("2026-07-11"));
    assert!(
        period.contains(r#"{ asOf: "2026-09-11", period: "1m" as const, startOn: "2026-08-11" }"#),
        "1m window must stay calendar month"
    );
    assert!(
        period.contains(r#"{ asOf: "2026-09-11", period: "2m" as const, startOn: "2026-07-11" }"#),
        "2m window must stay calendar month"
    );
    assert!(
        period.contains(r#"export const DEFAULT_GRAPH_PERIOD: GraphPeriod = "6m""#)
            && period.contains(r#"{ asOf: "2026-09-11", period: "6m" as const, startOn: "2026-03-11" }"#),
        "Home and Trends default graphing period is 6 months"
    );
    let home = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/features/graphing/HomeAccountCharts.tsx"),
    )
    .unwrap();
    let trends = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/features/graphing/TrendsCharts.tsx"),
    )
    .unwrap();
    let app = std::fs::read_to_string(golden_harness::repo_root().join("apps/desktop/src/App.tsx"))
        .unwrap();
    assert!(
        home.contains("useState<GraphPeriod>(DEFAULT_GRAPH_PERIOD)")
            && trends.contains("useState<GraphPeriod>(DEFAULT_GRAPH_PERIOD)")
            && app.contains("useRef<GraphPeriod>(DEFAULT_GRAPH_PERIOD)"),
        "Home and Trends must open on DEFAULT_GRAPH_PERIOD"
    );
}

#[test]
fn risk_mix_share_matches_dollar_share_within_one_point() {
    let helper = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/features/graphing/riskChart.ts"),
    )
    .unwrap();
    assert!(helper.contains("export function riskDayShares"));
    assert!(helper.contains("maxPts = 1"));
    let foundation = 16_147_952_i64;
    let core = 10_135_312_i64;
    let risk_on = 9_552_360_i64;
    let total = 35_862_244_i64;
    let share = |part: i64| part as f64 / total as f64 * 100.0;
    assert!((share(foundation) - 45.03).abs() < 1.0);
    assert!((share(core) - 28.26).abs() < 1.0);
    assert!((share(risk_on) - 26.64).abs() < 1.0);
    assert!((share(foundation) + share(core) + share(risk_on) - 100.0).abs() < 0.1);
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

#[tokio::test]
async fn home_charts_plot_closed_week_actuals_on_friday() {
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
    let health = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Health", "kind": "taxable"}),
    )
    .await;
    let nine = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "9", "kind": "taxable"}),
    )
    .await;
    let spec = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Speculation", "kind": "taxable"}),
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
            "idempotencyKey": "home-income-vti"
        }),
    )
    .await;
    must_ok(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": health["accountId"],
            "securityId": security["securityId"],
            "activityType": "dividend",
            "amountMinor": 3_400,
            "scale": 2,
            "occurredOn": "2026-08-19",
            "idempotencyKey": "home-health-vti"
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
            "occurredOn": "2026-09-08",
            "idempotencyKey": "home-income-open-week"
        }),
    )
    .await;
    must_ok(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": nine["accountId"],
            "securityId": security["securityId"],
            "activityType": "dividend",
            "amountMinor": 2_159,
            "scale": 2,
            "occurredOn": "2026-08-17",
            "idempotencyKey": "home-nine-vti"
        }),
    )
    .await;
    must_ok(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": spec["accountId"],
            "securityId": security["securityId"],
            "activityType": "dividend",
            "amountMinor": 3,
            "scale": 2,
            "occurredOn": "2026-08-17",
            "idempotencyKey": "home-spec-vti"
        }),
    )
    .await;

    let home = query(
        &platform,
        "AccountValueHomeGet",
        serde_json::json!({"asOfDate": "2026-09-09"}),
    )
    .await;
    let income_row = home["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["accountName"] == "Income")
        .unwrap();
    let health_row = home["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["accountName"] == "Health")
        .unwrap();
    let income_pts = income_row["incomePoints"].as_array().unwrap();
    assert!(
        income_pts
            .iter()
            .any(|p| p["asOf"] == "2026-08-21" && p["incomeMinor"] == 12_500),
        "closed-week actual sits on Friday: {income_pts:?}"
    );
    assert!(
        income_pts.iter().all(|p| p["asOf"] != "2026-09-11"),
        "in-progress week stays unknown: {income_pts:?}"
    );
    let health_pts = health_row["incomePoints"].as_array().unwrap();
    assert!(
        health_pts
            .iter()
            .any(|p| p["asOf"] == "2026-08-21" && p["incomeMinor"] == 3_400),
        "{health_pts:?}"
    );
    let fid = home["fidelity"]["incomePoints"].as_array().unwrap();
    assert!(
        fid.iter()
            .any(|p| p["asOf"] == "2026-08-21" && p["incomeMinor"] == 15_903),
        "Fidelity total sums account actuals including Speculation: {fid:?}"
    );
    let nine_row = home["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["accountName"] == "9")
        .unwrap();
    let nine_pts = nine_row["incomePoints"].as_array().unwrap();
    assert!(
        nine_pts
            .iter()
            .any(|p| p["asOf"] == "2026-08-21" && p["incomeMinor"] == 2_159),
        "Account 9 keeps weekly actuals: {nine_pts:?}"
    );
    let spec_row = home["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["accountName"] == "Speculation")
        .unwrap();
    assert!(
        spec_row["incomePoints"].as_array().unwrap().is_empty(),
        "Speculation has no weekly actuals line: {}",
        spec_row["incomePoints"]
    );
}

#[tokio::test]
async fn dividend_plan_home_rolls_annual_and_buckets_monthly() {
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
    let health = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Health", "kind": "taxable"}),
    )
    .await;
    must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Speculation", "kind": "taxable"}),
    )
    .await;
    let income_id = income["accountId"].as_str().unwrap();
    let health_id = health["accountId"].as_str().unwrap();
    let amdw = open_priced_lot(&platform, income_id, "AMDW", "Core", 10, 2000, "2026-09-11").await;
    let haky = open_priced_lot(&platform, health_id, "HAKY", "Foundation", 10, 1500, "2026-09-11").await;
    must_ok(
        &platform,
        "ProductionSeedLoad",
        serde_json::json!({
            "accounts": [],
            "securities": [],
            "lots": [],
            "yieldBatches": [],
            "disbursements": [],
            "plans": [
                {
                    "symbol": "AMDW",
                    "amountPerShareMinor": 100,
                    "amountScale": 2,
                    "planningPeriodsPerYear": 12,
                    "effectiveFrom": "2026-01-01",
                    "decisionReason": "test"
                },
                {
                    "symbol": "HAKY",
                    "amountPerShareMinor": 100,
                    "amountScale": 2,
                    "planningPeriodsPerYear": 12,
                    "effectiveFrom": "2026-01-01",
                    "decisionReason": "test"
                }
            ],
            "characteristics": [
                {
                    "symbol": "AMDW",
                    "paymentFrequency": "Monthly",
                    "riskTier": "Core",
                    "provider": "Test",
                    "underlying": "AMD",
                    "divType": "DIV-1",
                    "needsRocResearch": false,
                    "notes": ""
                },
                {
                    "symbol": "HAKY",
                    "paymentFrequency": "Monthly",
                    "riskTier": "Foundation",
                    "provider": "Test",
                    "underlying": "HAKY",
                    "divType": "DIV-1",
                    "needsRocResearch": false,
                    "notes": ""
                }
            ]
        }),
    )
    .await;

    let plan = query(
        &platform,
        "DividendPlanHomeGet",
        serde_json::json!({"asOfDate": "2026-09-11"}),
    )
    .await;
    let rows = plan["rows"].as_array().unwrap();
    let names: Vec<&str> = rows
        .iter()
        .map(|r| r["accountName"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "Speculation",
            "Robinhood",
            "Income",
            "Health",
            "FI Roth",
            "Energy",
            "Car",
            "9"
        ]
    );
    let income_row = rows.iter().find(|r| r["accountName"] == "Income").unwrap();
    assert_eq!(income_row["annualDividendMinor"], 12_000);
    assert_eq!(income_row["monthlyIncomeMinor"], 1_000);
    assert!(income_row["monthlyReinvestMinor"].is_null());
    assert!(income_row["monthlyMedicalMinor"].is_null());
    assert_eq!(income_row["weeklyMinor"], 230);
    assert_eq!(income_row["marketValueMinor"], 20_000);
    assert_eq!(income_row["effectiveAnnualBps"], 6_000);
    let health_row = rows.iter().find(|r| r["accountName"] == "Health").unwrap();
    assert_eq!(health_row["annualDividendMinor"], 12_000);
    assert!(health_row["monthlyIncomeMinor"].is_null());
    assert_eq!(health_row["monthlyMedicalMinor"], 1_000);
    assert_eq!(health_row["weeklyMinor"], 230);
    assert_eq!(health_row["marketValueMinor"], 15_000);
    let spec = rows
        .iter()
        .find(|r| r["accountName"] == "Speculation")
        .unwrap();
    assert!(
        spec["annualDividendMinor"].is_null(),
        "no plan stays unknown, not $0: {spec}"
    );
    assert!(spec["monthlyReinvestMinor"].is_null());
    let rh = rows.iter().find(|r| r["accountName"] == "Robinhood").unwrap();
    assert_eq!(rh["annualDividendMinor"], 0);
    assert_eq!(rh["monthlyIncomeMinor"], 0);
    assert_eq!(rh["weeklyMinor"], 0);
    assert_eq!(rh["effectiveAnnualBps"], 0);
    let energy = rows.iter().find(|r| r["accountName"] == "Energy").unwrap();
    assert_eq!(energy["annualDividendMinor"], 0);
    assert_eq!(energy["effectiveAnnualBps"], 0);
    let total = &plan["total"];
    assert_eq!(total["accountName"], "Grand total");
    assert_eq!(total["annualDividendMinor"], 24_000);
    assert_eq!(total["monthlyIncomeMinor"], 1_000);
    assert_eq!(total["monthlyMedicalMinor"], 1_000);
    assert!(total["monthlyReinvestMinor"].is_null());
    assert_eq!(total["weeklyMinor"], 460);

    must_ok(
        &platform,
        "DividendActualRecord",
        serde_json::json!({
            "accountId": income_id,
            "securityId": amdw,
            "occurredOn": "2026-08-15",
            "amountMinor": 24_000,
            "scale": 2,
            "idempotencyKey": "income-in-window"
        }),
    )
    .await;
    must_ok(
        &platform,
        "DividendActualRecord",
        serde_json::json!({
            "accountId": health_id,
            "securityId": haky,
            "occurredOn": "2026-08-15",
            "amountMinor": 12_000,
            "scale": 2,
            "idempotencyKey": "health-excluded"
        }),
    )
    .await;
    must_ok(
        &platform,
        "DividendActualRecord",
        serde_json::json!({
            "accountId": income_id,
            "securityId": amdw,
            "occurredOn": "2026-09-05",
            "amountMinor": 12_000,
            "scale": 2,
            "idempotencyKey": "income-current-month"
        }),
    )
    .await;
    must_ok(
        &platform,
        "DividendActualRecord",
        serde_json::json!({
            "accountId": income_id,
            "securityId": amdw,
            "occurredOn": "2025-08-15",
            "amountMinor": 12_000,
            "scale": 2,
            "idempotencyKey": "income-before-window"
        }),
    )
    .await;

    let summary = query(
        &platform,
        "DataSummaryGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    assert_eq!(
        summary["avgMonthlyPlanIncomeMinor"],
        1_000,
        "Income AMDW annual 12000 ÷ 12; Health excluded: {summary}"
    );
    assert_eq!(
        summary["avgMonthlyActualIncomeMinor"],
        2_000,
        "in-window Income 24000 ÷ 12; Health and out-of-window excluded: {summary}"
    );
}
