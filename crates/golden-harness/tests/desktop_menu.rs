use application_core::contracts::{
    CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::queries::{execute_command_on, execute_query_on};
use golden_harness::{profile_a_app_dir, repo_root};
use storage_sqlite::LocalPlatform;
use uuid::Uuid;

fn qry(name: &str, body: serde_json::Value) -> QueryRequest {
    QueryRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        query_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: Some(body.to_string()),
    }
}

fn cmd(name: &str, body: serde_json::Value) -> CommandRequest {
    CommandRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        command_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: Some(body.to_string()),
        expected_version: None,
    }
}

/// Every FinanceClient query the desktop nav actually issues.
const MENU_QUERIES: &[&str] = &[
    "HealthGet",
    "ConfigGet",
    "HandoffStatusGet",
    "IncomePlanWeekGet",
    "DashboardBurndownGet",
    "HoldingsGet",
    "ExceptionList",
    "DataSummaryGet",
    "CalculatorGet",
    "AccountList",
    "SecurityList",
    "PositionDetailsGet",
    "PositionMasterGet",
];

#[test]
fn app_execute_query_names_are_registered() {
    let app = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx")).unwrap();
    let queries = std::fs::read_to_string(
        repo_root().join("crates/application-core/src/queries.rs"),
    )
    .unwrap();
    let on = queries
        .split("pub async fn execute_query_on")
        .nth(1)
        .expect("execute_query_on");
    for name in MENU_QUERIES {
        assert!(
            app.contains(&format!("executeQuery(\"{name}\"")),
            "App.tsx must call {name}"
        );
        assert!(
            on.contains(&format!("\"{name}\"")),
            "execute_query_on must register {name}"
        );
    }
}

#[tokio::test]
async fn menu_queries_succeed_on_data_sqlite() {
    let dir = profile_a_app_dir();
    let db = dir.join("local.sqlite");
    assert!(
        db.is_file(),
        "data file missing at {}; run npm run data-seed",
        db.display()
    );
    let platform = LocalPlatform::open(&dir).await.expect("open data sqlite");
    let mut failures = Vec::new();
    for name in MENU_QUERIES {
        let body = if matches!(*name, "IncomePlanWeekGet" | "DashboardBurndownGet") {
            serde_json::json!({"asOfDate": "2026-07-31"})
        } else {
            serde_json::json!({})
        };
        let result = execute_query_on(&platform, &platform, qry(name, body)).await;
        if !result.ok {
            failures.push(format!(
                "{name}: {}",
                result.error_code.unwrap_or_else(|| "error".into())
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "menu queries failed: {}",
        failures.join("; ")
    );

    let summary = execute_query_on(
        &platform,
        &platform,
        qry("DataSummaryGet", serde_json::json!({})),
    )
    .await;
    let body: serde_json::Value =
        serde_json::from_str(summary.body_json.as_deref().unwrap_or("{}")).unwrap();
    assert!(
        body["planCount"].as_u64().unwrap_or(0) >= 40,
        "Calculator plans not loaded: {body}"
    );
    let calc = execute_query_on(&platform, &platform, qry("CalculatorGet", serde_json::json!({})))
        .await;
    let calc_body: serde_json::Value =
        serde_json::from_str(calc.body_json.as_deref().unwrap_or("{}")).unwrap();
    assert!(
        calc_body["rows"].as_array().map(|a| a.len()).unwrap_or(0) >= 40,
        "CalculatorGet rows missing: {calc_body}"
    );
}

#[tokio::test]
async fn settings_config_set_is_registered() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let settings = execute_command_on(
        &platform,
        &platform,
        cmd("ConfigSet", serde_json::json!({"deviceName": "menu-test"})),
    )
    .await;
    assert!(
        settings.ok,
        "Settings ConfigSet failed: {:?}",
        settings.error_code
    );
}
