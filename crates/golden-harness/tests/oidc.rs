//! OIDC Bearer JWT + command audit on Postgres via Axum (AC-ARCH-09, AC-ARCH-10).
//! Unauthenticated requests fail closed. Desktop stays SQLite.

use application_core::contracts::{
    CommandRequest, CommandResult, QueryRequest, QueryResult, FINANCE_CLIENT_CONTRACT_VERSION,
};
use axum::http::StatusCode;
use finos_server::{http_command, http_post_status, http_query, mint_test_token, test_router};
use serde_json::Value;
use storage_postgres::PostgresPlatform;
use uuid::Uuid;

fn require_postgres_url() -> String {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "DATABASE_URL is required for oidc; do not substitute SQLite. \
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

fn qry(name: &str) -> QueryRequest {
    QueryRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        query_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: None,
    }
}

#[tokio::test]
async fn unauthenticated_commands_and_queries_fail_closed() {
    let url = require_postgres_url();
    let platform = PostgresPlatform::connect(&url)
        .await
        .unwrap_or_else(|e| panic!("PostgreSQL connect failed (do not use SQLite): {e}"));
    platform
        .reset_contract_tables()
        .await
        .unwrap_or_else(|e| panic!("PostgreSQL reset failed: {e}"));
    let app = test_router(platform);

    let status = http_post_status(
        app.clone(),
        "/v1/queries",
        &qry("HealthGet"),
        None,
    )
    .await
    .expect("dispatch");
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let status = http_post_status(
        app.clone(),
        "/v1/commands",
        &cmd(
            "AccountRegister",
            serde_json::json!({"name": "NoAuth", "kind": "taxable"}),
            None,
        ),
        None,
    )
    .await
    .expect("dispatch");
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let status = http_post_status(
        app,
        "/v1/queries",
        &qry("HealthGet"),
        Some("not-a-jwt"),
    )
    .await
    .expect("dispatch");
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn two_authenticated_clients_and_concurrency_conflict() {
    let url = require_postgres_url();
    let platform = PostgresPlatform::connect(&url)
        .await
        .unwrap_or_else(|e| panic!("PostgreSQL connect failed (do not use SQLite): {e}"));
    platform
        .reset_contract_tables()
        .await
        .unwrap_or_else(|e| panic!("PostgreSQL reset failed: {e}"));
    let client_a = mint_test_token("user-a", "device-1");
    let client_b = mint_test_token("user-b", "device-2");
    let app = test_router(platform.clone());

    let health: QueryResult = http_query(app.clone(), qry("HealthGet"), &client_a)
        .await
        .expect("client a health");
    assert!(health.ok, "{:?}", health.error_code);

    let health_b: QueryResult = http_query(app.clone(), qry("HealthGet"), &client_b)
        .await
        .expect("client b health");
    assert!(health_b.ok, "{:?}", health_b.error_code);

    let registered: CommandResult = http_command(
        app.clone(),
        cmd(
            "AccountRegister",
            serde_json::json!({"name": "Oidc Concurrent", "kind": "taxable"}),
            None,
        ),
        &client_a,
    )
    .await
    .expect("register");
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
        &client_a,
    );
    let b = http_command(
        app,
        cmd(
            "AccountUpdate",
            serde_json::json!({"accountId": account_id, "name": "Second"}),
            Some(version),
        ),
        &client_b,
    );
    let (first, second) = tokio::join!(a, b);
    let first = first.expect("http a");
    let second = second.expect("http b");
    assert!(
        (first.ok && second.error_code.as_deref() == Some("concurrency_conflict"))
            || (second.ok && first.error_code.as_deref() == Some("concurrency_conflict")),
        "expected one success and one concurrency_conflict, got {first:?} {second:?}"
    );

    let audits = platform
        .list_command_audits()
        .await
        .unwrap_or_else(|e| panic!("command_audit: {e}"));
    assert!(
        audits.iter().any(|row| {
            row.command_name == "AccountRegister"
                && row.user_sub == "user-a"
                && row.device_id == "device-1"
                && row.ok
                && row.correlation_id == registered.correlation_id.to_string()
        }),
        "AccountRegister must be audited with user/device/correlation: {audits:?}"
    );
    let updates: Vec<_> = audits
        .iter()
        .filter(|row| row.command_name == "AccountUpdate")
        .collect();
    assert_eq!(updates.len(), 2, "both AccountUpdate attempts must be audited: {audits:?}");
    assert!(
        updates.iter().any(|row| row.user_sub == "user-a" && row.device_id == "device-1"),
        "client A update missing: {audits:?}"
    );
    assert!(
        updates.iter().any(|row| row.user_sub == "user-b" && row.device_id == "device-2"),
        "client B update missing: {audits:?}"
    );
    assert!(
        updates.iter().any(|row| row.ok)
            && updates
                .iter()
                .any(|row| !row.ok && row.error_code.as_deref() == Some("concurrency_conflict")),
        "audit must record one success and one concurrency_conflict: {audits:?}"
    );
}
