//! Dual-adapter contract: SQLite and PostgreSQL return the same cents (AC-ARCH-03).
//! PostgreSQL is required. Missing DATABASE_URL must fail — never fall back to SQLite.

use application_core::contracts::{
    CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::ports::canonical::Canonical;
use application_core::ports::platform::Platform;
use application_core::queries::{execute_command_on, execute_query_on};
use storage_postgres::PostgresPlatform;
use storage_sqlite::LocalPlatform;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ContractCents {
    dividend_actual_minor: i64,
    open_performance_minor: i64,
    open_tax_minor: i64,
    remaining_quantity_minor: i64,
    lot_count: u64,
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

fn qry(name: &str) -> QueryRequest {
    QueryRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        query_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: None,
    }
}

async fn must_ok<P: Platform + Canonical>(
    platform: &P,
    name: &str,
    body: serde_json::Value,
) -> serde_json::Value {
    let result = execute_command_on(platform, platform, cmd(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

async fn query_json<P: Platform + Canonical>(platform: &P, name: &str) -> serde_json::Value {
    let result = execute_query_on(platform, platform, qry(name)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

async fn run_contract<P: Platform + Canonical>(platform: &P) -> ContractCents {
    let account = must_ok(
        platform,
        "AccountRegister",
        serde_json::json!({"name": "Taxable Brokerage", "kind": "taxable"}),
    )
    .await;
    let account_id = account["accountId"].as_str().unwrap();
    let security = must_ok(
        platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "AAPL", "name": "Apple"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();

    must_ok(
        platform,
        "DividendActualRecord",
        serde_json::json!({
            "accountId": account_id,
            "occurredOn": "2026-06-15",
            "amountMinor": 50_000,
            "scale": 2,
            "idempotencyKey": "pg-div-1"
        }),
    )
    .await;

    must_ok(
        platform,
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
    )
    .await;
    must_ok(
        platform,
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
    )
    .await;

    let dividend = query_json(platform, "DividendGet").await;
    let details = query_json(platform, "PositionDetailsGet").await;
    let positions = details["positions"].as_array().unwrap();
    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0]["symbol"].as_str().unwrap(), "AAPL");
    ContractCents {
        dividend_actual_minor: dividend["actualTotalMinor"].as_i64().unwrap(),
        open_performance_minor: details["openPerformanceMinor"].as_i64().unwrap(),
        open_tax_minor: details["openTaxMinor"].as_i64().unwrap(),
        remaining_quantity_minor: positions[0]["remainingQuantityMinor"].as_i64().unwrap(),
        lot_count: positions[0]["lotCount"].as_u64().unwrap(),
    }
}

fn expected_cents() -> ContractCents {
    ContractCents {
        dividend_actual_minor: 50_000,
        open_performance_minor: 140_000,
        open_tax_minor: 120_000,
        remaining_quantity_minor: 20,
        lot_count: 2,
    }
}

#[tokio::test]
async fn postgres_contract_sqlite_and_postgres_match_cents() {
    let dir = tempfile::tempdir().unwrap();
    let sqlite = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let sqlite_cents = run_contract(&sqlite).await;
    assert_eq!(sqlite_cents, expected_cents());

    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "DATABASE_URL is required for postgres_contract; do not substitute SQLite. \
             Start docker compose (postgres:16) and set \
             DATABASE_URL=postgres://finos:finos@localhost:5432/finos"
        )
    });
    assert!(
        url.starts_with("postgres://") || url.starts_with("postgresql://"),
        "DATABASE_URL must be PostgreSQL, not SQLite: {url}"
    );
    let postgres = PostgresPlatform::connect(&url)
        .await
        .unwrap_or_else(|e| panic!("PostgreSQL connect failed (do not use SQLite): {e}"));
    postgres
        .reset_contract_tables()
        .await
        .unwrap_or_else(|e| panic!("PostgreSQL reset failed: {e}"));
    let postgres_cents = run_contract(&postgres).await;
    assert_eq!(postgres_cents, expected_cents());
    assert_eq!(sqlite_cents, postgres_cents);
}
