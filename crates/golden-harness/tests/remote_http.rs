//! Remote HTTP FinanceClient path (AC-ARCH-01 / AC-ARCH-17). Desktop stays SQLite.
//! Missing DATABASE_URL must fail — never fall back to SQLite. Never writes oracles.

use application_core::contracts::{
    CommandRequest, CommandResult, QueryRequest, QueryResult, FINANCE_CLIENT_CONTRACT_VERSION,
};
use finos_server::{http_command, http_query, mint_test_token, test_router};
use serde_json::Value;
use storage_postgres::PostgresPlatform;
use uuid::Uuid;

fn require_postgres_url() -> String {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "DATABASE_URL is required for remote_http; do not substitute SQLite. \
             Start docker compose (postgres:16) and set \
             DATABASE_URL=postgres://finos:finos@localhost:5432/finos"
        )
    });
    assert!(
        url.starts_with("postgres://") || url.starts_with("postgresql://"),
        "DATABASE_URL must be PostgreSQL, not SQLite: {url}"
    );
    url
}

fn cmd(name: &str, body: Value) -> CommandRequest {
    CommandRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        command_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: Some(body.to_string()),
        expected_version: None,
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

async fn must_cmd(app: axum::Router, name: &str, body: Value, bearer: &str) -> Value {
    let result: CommandResult = http_command(app, cmd(name, body), bearer)
        .await
        .unwrap_or_else(|e| panic!("{name} http: {e}"));
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

async fn must_qry(app: axum::Router, name: &str, bearer: &str) -> Value {
    let result: QueryResult = http_query(app, qry(name), bearer)
        .await
        .unwrap_or_else(|e| panic!("{name} http: {e}"));
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

#[tokio::test]
async fn remote_http_contract_cents_and_g01_magi() {
    let url = require_postgres_url();
    let platform = PostgresPlatform::connect(&url)
        .await
        .unwrap_or_else(|e| panic!("PostgreSQL connect failed (do not use SQLite): {e}"));
    platform
        .reset_contract_tables()
        .await
        .unwrap_or_else(|e| panic!("PostgreSQL reset failed: {e}"));
    let token = mint_test_token("user-a", "device-1");
    let app = test_router(platform);

    let health = must_qry(app.clone(), "HealthGet", &token).await;
    assert_eq!(health["status"], "ok");

    let account = must_cmd(
        app.clone(),
        "AccountRegister",
        serde_json::json!({"name": "Taxable Brokerage", "kind": "taxable"}),
        &token,
    )
    .await;
    let account_id = account["accountId"].as_str().unwrap();
    let security = must_cmd(
        app.clone(),
        "SecurityRegister",
        serde_json::json!({"symbol": "AAPL", "name": "Apple"}),
        &token,
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();

    must_cmd(
        app.clone(),
        "DividendActualRecord",
        serde_json::json!({
            "accountId": account_id,
            "occurredOn": "2026-06-15",
            "amountMinor": 50_000,
            "scale": 2,
            "idempotencyKey": "http-div-1"
        }),
        &token,
    )
    .await;
    must_cmd(
        app.clone(),
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
        &token,
    )
    .await;
    must_cmd(
        app.clone(),
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
        &token,
    )
    .await;

    let dividend = must_qry(app.clone(), "DividendGet", &token).await;
    let details = must_qry(app.clone(), "PositionDetailsGet", &token).await;
    assert_eq!(dividend["actualTotalMinor"].as_i64().unwrap(), 50_000);
    assert_eq!(details["openPerformanceMinor"].as_i64().unwrap(), 140_000);
    assert_eq!(details["openTaxMinor"].as_i64().unwrap(), 120_000);

    must_cmd(
        app.clone(),
        "MagiRuleSet",
        serde_json::json!({
            "thresholdMinor": 8_460_000,
            "safetyReserveMinor": 500_000,
            "scale": 2
        }),
        &token,
    )
    .await;
    must_cmd(
        app.clone(),
        "ActivityPost",
        serde_json::json!({
            "accountId": account_id,
            "activityType": "dividend",
            "amountMinor": 2_000_000,
            "scale": 2,
            "occurredOn": "2026-03-15",
            "idempotencyKey": "G01-DIV-1"
        }),
        &token,
    )
    .await;
    must_cmd(
        app.clone(),
        "MagiFactRecord",
        serde_json::json!({
            "sourceId": "G01-DIV-1",
            "treatment": "include",
            "amountMinor": 2_000_000,
            "scale": 2,
            "category": "taxable_dividend"
        }),
        &token,
    )
    .await;
    must_cmd(
        app.clone(),
        "ActivityPost",
        serde_json::json!({
            "accountId": account_id,
            "activityType": "dividend",
            "amountMinor": 2_000_000,
            "scale": 2,
            "occurredOn": "2026-09-15",
            "idempotencyKey": "G01-DIV-2"
        }),
        &token,
    )
    .await;
    must_cmd(
        app.clone(),
        "MagiFactRecord",
        serde_json::json!({
            "sourceId": "G01-DIV-2",
            "treatment": "include",
            "amountMinor": 2_000_000,
            "scale": 2,
            "category": "taxable_dividend"
        }),
        &token,
    )
    .await;
    must_cmd(
        app.clone(),
        "MagiCoverageSet",
        serde_json::json!({
            "completeness": "complete",
            "remainingMinor": 1_000_000,
            "withholdingMinor": 0,
            "formTotalMinor": 0,
            "warnings": []
        }),
        &token,
    )
    .await;

    let magi = must_qry(app, "MagiProjectionGet", &token).await;
    assert_eq!(magi["decisionState"], "SAFE");
    assert_eq!(magi["actualIncludedYtd"]["amountMinor"].as_i64().unwrap(), 4_000_000);
    assert_eq!(magi["knownRemaining"]["amountMinor"].as_i64().unwrap(), 1_000_000);
    assert_eq!(magi["protectedHeadroom"]["amountMinor"].as_i64().unwrap(), 2_960_000);
}
