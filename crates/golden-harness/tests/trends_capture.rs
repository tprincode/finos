//! Trends weekly capture / close / IAL profit suggestion harness.

use application_core::contracts::{
    CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::queries::{execute_command_on, execute_query_on};
use golden_harness::{load_production_seed_via_commands, repo_root};
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

fn qry(name: &str, body: Option<&str>) -> QueryRequest {
    QueryRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        query_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: body.map(|s| s.to_string()),
    }
}

async fn must_cmd(platform: &LocalPlatform, name: &str, body: serde_json::Value) -> serde_json::Value {
    let result = execute_command_on(platform, platform, cmd(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

async fn query_json(platform: &LocalPlatform, name: &str, body: Option<&str>) -> serde_json::Value {
    let result = execute_query_on(platform, platform, qry(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

#[tokio::test]
async fn trends_week_save_updates_charts_and_is_idempotent() {
    let root = repo_root();
    let production = root.join("database/seed/production");
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    load_production_seed_via_commands(&platform, &production)
        .await
        .expect("seed");

    let body = serde_json::json!({
        "periodStart": "2026-08-22",
        "periodEnd": "2026-08-28",
        "capturedAt": "2026-08-29T12:00:00Z",
        "profitMinor": 150000,
        "monthlyDivsMinor": 490000,
        "fidelityTotalMinor": 31000000,
        "schwabTotalMinor": 3200000,
        "incomeCashMinor": 700000,
        "acct9CashMinor": 1000000,
        "acct9EtfValueMinor": 1100000,
        "carBalanceMinor": 4500000,
        "incomeBalanceMinor": 21200000,
        "healthBalanceMinor": 1900000,
        "rothBalanceMinor": 880000,
        "speculationBalanceMinor": 3000000,
        "scale": 2
    });
    must_cmd(&platform, "TrendsWeekSave", body.clone()).await;
    must_cmd(&platform, "TrendsWeekSave", body).await;

    let trends = query_json(&platform, "TrendsGet", Some(r#"{"asOfDate":"2026-08-28"}"#)).await;
    let weeks = trends["weeks"].as_array().unwrap();
    assert!(weeks.iter().any(|w| w["periodEnd"] == "2026-08-28"));
    assert!(trends["overview"].is_object());
    assert!(trends["distributions"].is_object());
    assert!(trends["taxMonitor"].is_object());
    assert!(trends["taxMonitor"]["acaThresholdMinor"].as_i64().unwrap() > 0);
}

#[tokio::test]
async fn trends_week_close_blocks_save_allows_correct() {
    let root = repo_root();
    let production = root.join("database/seed/production");
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    load_production_seed_via_commands(&platform, &production)
        .await
        .expect("seed");

    let body = serde_json::json!({
        "periodStart": "2026-08-22",
        "periodEnd": "2026-08-28",
        "capturedAt": "2026-08-29T12:00:00Z",
        "profitMinor": 100000,
        "monthlyDivsMinor": 480000,
        "fidelityTotalMinor": 30000000,
        "schwabTotalMinor": 3000000,
        "incomeCashMinor": 1,
        "acct9CashMinor": 1,
        "acct9EtfValueMinor": 1,
        "scale": 2
    });
    must_cmd(&platform, "TrendsWeekSave", body.clone()).await;
    must_cmd(
        &platform,
        "TrendsWeekClose",
        serde_json::json!({"periodEnd":"2026-08-28"}),
    )
    .await;

    let blocked = execute_command_on(&platform, &platform, cmd("TrendsWeekSave", body.clone())).await;
    assert!(!blocked.ok);
    assert_eq!(blocked.error_code.as_deref(), Some("trends_week_closed"));

    let mut corrected = body.clone();
    corrected["profitMinor"] = serde_json::json!(111100);
    must_cmd(&platform, "TrendsWeekCorrect", corrected).await;
    let open = query_json(
        &platform,
        "TrendsWeekGet",
        Some(r#"{"asOfDate":"2026-08-28"}"#),
    )
    .await;
    assert_eq!(open["current"]["profitMinor"], 111100);
    assert_eq!(open["closed"], true);
}

#[tokio::test]
async fn trends_suggested_profit_from_dividend_activity() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    must_cmd(
        &platform,
        "AccountRegister",
        serde_json::json!({"name":"Income","kind":"ira"}),
    )
    .await;
    must_cmd(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol":"XDTE","name":"XDTE"}),
    )
    .await;
    let accounts = query_json(&platform, "AccountList", None).await;
    let account_id = accounts[0]["accountId"].as_str().unwrap();
    let securities = query_json(&platform, "SecurityList", None).await;
    let security_id = securities[0]["securityId"].as_str().unwrap();
    must_cmd(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "activityType": "dividend",
            "amountMinor": 25000,
            "scale": 2,
            "occurredOn": "2026-08-25"
        }),
    )
    .await;
    let capture = query_json(
        &platform,
        "TrendsWeekGet",
        Some(r#"{"asOfDate":"2026-08-28"}"#),
    )
    .await;
    assert_eq!(capture["periodStart"], "2026-08-22");
    assert_eq!(capture["periodEnd"], "2026-08-28");
    assert_eq!(capture["suggestedProfitMinor"], 25000);
    assert_eq!(
        capture["suggestedMonthlyDivsMinor"], 25000,
        "week-aligned income is the paid dividend in that Sat–Fri week"
    );
}

#[tokio::test]
async fn trends_week_get_defaults_to_first_unpopulated() {
    let root = repo_root();
    let production = root.join("database/seed/production");
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    load_production_seed_via_commands(&platform, &production)
        .await
        .expect("seed");

    let capture = query_json(&platform, "TrendsWeekGet", None).await;
    assert_eq!(capture["firstUnpopulatedStart"], "2026-08-22");
    assert_eq!(capture["periodStart"], "2026-08-22");
    let chooser = capture["chooserSaturdays"].as_array().expect("chooser");
    assert_eq!(chooser[0], "2026-08-22");
    assert!(chooser.iter().any(|s| s == "2026-09-12"));
}

#[tokio::test]
async fn trends_week_save_cash_and_derived_totals_hit_charts() {
    let root = repo_root();
    let production = root.join("database/seed/production");
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    load_production_seed_via_commands(&platform, &production)
        .await
        .expect("seed");

    let body = serde_json::json!({
        "periodStart": "2026-08-22",
        "periodEnd": "2026-08-28",
        "capturedAt": "2026-08-29T12:00:00Z",
        "incomeBalanceMinor": 20000000,
        "rothBalanceMinor": 1000000,
        "speculationBalanceMinor": 2000000,
        "healthBalanceMinor": 1500000,
        "carBalanceMinor": 4000000,
        "acct9BalanceMinor": 3500000,
        "incomeCashMinor": 500000,
        "rothCashMinor": 100000,
        "speculationCashMinor": 50000,
        "healthCashMinor": 25000,
        "carCashMinor": 75000,
        "acct9CashMinor": 200000,
        "acct9EtfValueMinor": 0,
        "scale": 2
    });
    let saved = must_cmd(&platform, "TrendsWeekSave", body).await;
    assert_eq!(saved["current"]["fidelityTotalMinor"], 28_500_000);
    assert_eq!(saved["current"]["schwabTotalMinor"], 3_500_000);

    let trends = query_json(&platform, "TrendsGet", Some(r#"{"asOfDate":"2026-08-28"}"#)).await;
    let week = trends["weeks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["periodEnd"] == "2026-08-28")
        .expect("saved week");
    assert_eq!(week["fidelityTotalMinor"], 28_500_000);
    assert_eq!(week["schwabTotalMinor"], 3_500_000);
    assert!(week["totalCashMinor"].as_i64().unwrap() >= 950_000);
    assert_eq!(week["healthBalanceMinor"], 1_500_000);
}

#[tokio::test]
async fn trends_acct9_etf_uses_last_price_not_tax_basis() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    must_cmd(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "9", "kind": "ira"}),
    )
    .await;
    let empty = query_json(
        &platform,
        "TrendsWeekGet",
        Some(r#"{"asOfDate":"2026-08-28"}"#),
    )
    .await;
    assert_eq!(empty["suggestedAcct9EtfProxyMinor"], 0);

    let root = repo_root();
    let production = root.join("database/seed/production");
    let seeded = tempfile::tempdir().unwrap();
    let seeded_platform = LocalPlatform::open(seeded.path().join("app-data"))
        .await
        .unwrap();
    load_production_seed_via_commands(&seeded_platform, &production)
        .await
        .expect("seed");
    let capture = query_json(
        &seeded_platform,
        "TrendsWeekGet",
        Some(r#"{"asOfDate":"2026-08-28"}"#),
    )
    .await;
    let proxy = capture["suggestedAcct9EtfProxyMinor"].as_i64();
    assert!(
        proxy.is_some_and(|v| v > 0) || capture["suggestedAcct9EtfProxyMinor"].is_null(),
        "last-price 70% is a positive value or unknown, never invented $0: {}",
        capture["suggestedAcct9EtfProxyMinor"]
    );
    if let Some(v) = proxy {
        assert_ne!(v, 0, "Account 9 open lots have last prices");
    }
}
