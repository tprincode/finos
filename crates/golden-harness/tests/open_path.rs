//! Home first-paint query list and TaxPlanning compose. MAGI oracles unchanged.

use application_core::contracts::{
    QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::queries::execute_query_on;
use golden_harness::repo_root;
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

/// Queries the desktop issues on Home open. Grid / performance-all / CM tail stay off this list.
const HOME_OPEN_QUERIES: &[&str] = &[
    "HomeOpenGet",
    "DividendGet",
    "HoldingsGet",
    "ExceptionList",
    "AccountList",
    "SecurityList",
    "WorkTicketList",
];

#[test]
fn home_open_queries_are_registered_and_not_the_deferred_tail() {
    let app = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx")).unwrap();
    let queries = std::fs::read_to_string(
        repo_root().join("crates/application-core/src/queries.rs"),
    )
    .unwrap();
    let on = queries
        .split("pub async fn execute_query_on")
        .nth(1)
        .expect("execute_query_on");
    for name in HOME_OPEN_QUERIES {
        assert!(
            app.contains(&format!("executeQuery(\"{name}\"")),
            "App.tsx must call {name}"
        );
        assert!(
            on.contains(&format!("\"{name}\"")),
            "execute_query_on must register {name}"
        );
    }
    let home_fn = app
        .split("const refreshHomeData")
        .nth(1)
        .and_then(|s| s.split("const loadIncomePack").next())
        .expect("refreshHomeData");
    for deferred in [
        "IncomePlanGridGet",
        "DividendPerformanceGet",
        "TaxPlanningGet",
        "CarRocPlanGet",
        "CashYtdGet",
        "CashRegisterGet",
        "CashCoverageGet",
        "WeekAheadGet",
    ] {
        assert!(
            !home_fn.contains(&format!("executeQuery(\"{deferred}\"")),
            "Home open must not call {deferred}"
        );
    }
}

#[tokio::test]
async fn home_open_and_tax_planning_compose() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let home = execute_query_on(
        &platform,
        &platform,
        qry("HomeOpenGet", serde_json::json!({"asOfDate": "2026-09-19"})),
    )
    .await;
    assert!(home.ok, "HomeOpenGet {}", home.error_code.unwrap_or_default());
    let body: serde_json::Value =
        serde_json::from_str(home.body_json.as_deref().unwrap_or("{}")).unwrap();
    assert!(body.get("summary").is_some());
    assert!(body.get("accountValue").is_some());
    assert!(body.get("dividendPlan").is_some());
    assert!(
        body["accountValue"]["weeks"].is_array(),
        "HomeOpenGet must return the week cash series Home paints"
    );

    let tax = execute_query_on(
        &platform,
        &platform,
        qry("TaxPlanningGet", serde_json::json!({"asOfDate": "2026-09-19"})),
    )
    .await;
    assert!(tax.ok, "TaxPlanningGet {}", tax.error_code.unwrap_or_default());
    let tax_body: serde_json::Value =
        serde_json::from_str(tax.body_json.as_deref().unwrap_or("{}")).unwrap();
    assert!(tax_body.get("car").is_some(), "TaxPlanningGet must embed Car");
    assert!(tax_body.get("ytd").is_some(), "TaxPlanningGet must embed YTD");
}
