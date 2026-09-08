//! Optimistic concurrency on Postgres via Axum (AC-ARCH-09). Requests must be authenticated.

use application_core::contracts::{CommandRequest, CommandResult, FINANCE_CLIENT_CONTRACT_VERSION};
use finos_server::{http_command, mint_test_token, test_router};
use serde_json::Value;
use storage_postgres::PostgresPlatform;
use uuid::Uuid;

fn require_postgres_url() -> String {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "DATABASE_URL is required for remote_concurrency; do not substitute SQLite. \
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

fn cmd(name: &str, body: Value, expected_version: Option<i64>) -> CommandRequest {
    CommandRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        command_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: Some(body.to_string()),
        expected_version,
    }
}

#[tokio::test]
async fn concurrent_account_update_returns_concurrency_conflict() {
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

    let registered: CommandResult = http_command(
        app.clone(),
        cmd(
            "AccountRegister",
            serde_json::json!({"name": "Concurrency", "kind": "taxable"}),
            None,
        ),
        &token,
    )
    .await
    .unwrap();
    assert!(registered.ok, "{:?}", registered.error_code);
    let body: Value = serde_json::from_str(registered.body_json.as_deref().unwrap_or("{}")).unwrap();
    let account_id = body["accountId"].as_str().unwrap().to_string();
    let version = body["rowVersion"].as_i64().unwrap_or(1);

    let a = http_command(
        app.clone(),
        cmd(
            "AccountUpdate",
            serde_json::json!({"accountId": account_id, "name": "First"}),
            Some(version),
        ),
        &token,
    );
    let b = http_command(
        app,
        cmd(
            "AccountUpdate",
            serde_json::json!({"accountId": account_id, "name": "Second"}),
            Some(version),
        ),
        &token,
    );
    let (first, second) = tokio::join!(a, b);
    let first = first.expect("http a");
    let second = second.expect("http b");
    let codes = [
        first.ok,
        second.ok,
        first.error_code.as_deref() == Some("concurrency_conflict"),
        second.error_code.as_deref() == Some("concurrency_conflict"),
    ];
    assert!(
        (first.ok && second.error_code.as_deref() == Some("concurrency_conflict"))
            || (second.ok && first.error_code.as_deref() == Some("concurrency_conflict")),
        "expected one success and one concurrency_conflict, got {first:?} {second:?} {codes:?}"
    );
}
