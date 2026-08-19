//! Snapshot import SQLite → PostgreSQL (AC-ARCH-08). Desktop stays the SQLite writer.

use application_core::contracts::{
    CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::ports::canonical::Canonical;
use application_core::ports::platform::Platform;
use application_core::queries::{execute_command_on, execute_query_on};
use storage_postgres::PostgresPlatform;
use storage_sqlite::LocalPlatform;
use uuid::Uuid;

fn require_postgres_url() -> String {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "DATABASE_URL is required for postgres_snapshot_import; do not substitute SQLite. \
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

async fn must_ok<P: application_core::ports::platform::Platform + Canonical>(
    platform: &P,
    name: &str,
    body: serde_json::Value,
) -> serde_json::Value {
    let result = execute_command_on(platform, platform, cmd(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

#[tokio::test]
async fn sqlite_snapshot_imports_into_postgres_with_reconciled_cents() {
    let dir = tempfile::tempdir().unwrap();
    let sqlite = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let account = must_ok(
        &sqlite,
        "AccountRegister",
        serde_json::json!({"name": "Taxable Brokerage", "kind": "taxable"}),
    )
    .await;
    let account_id = account["accountId"].as_str().unwrap();
    let security = must_ok(
        &sqlite,
        "SecurityRegister",
        serde_json::json!({"symbol": "AAPL", "name": "Apple"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    must_ok(
        &sqlite,
        "DividendActualRecord",
        serde_json::json!({
            "accountId": account_id,
            "occurredOn": "2026-06-15",
            "amountMinor": 50_000,
            "scale": 2,
            "idempotencyKey": "imp-div-1"
        }),
    )
    .await;
    must_ok(
        &sqlite,
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
        &sqlite,
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
    must_ok(
        &sqlite,
        "MagiRuleSet",
        serde_json::json!({
            "thresholdMinor": 8_460_000,
            "safetyReserveMinor": 500_000,
            "scale": 2
        }),
    )
    .await;
    must_ok(
        &sqlite,
        "MagiFactRecord",
        serde_json::json!({
            "sourceId": "G01-DIV-1",
            "treatment": "include",
            "amountMinor": 4_000_000,
            "scale": 2,
            "category": "taxable_dividend"
        }),
    )
    .await;
    must_ok(
        &sqlite,
        "MagiCoverageSet",
        serde_json::json!({
            "completeness": "complete",
            "remainingMinor": 1_000_000,
            "withholdingMinor": 0,
            "formTotalMinor": 0,
            "warnings": []
        }),
    )
    .await;

    let sqlite_counts = sqlite.reconcile_counts().await.unwrap();
    let sqlite_div = execute_query_on(&sqlite, &sqlite, qry("DividendGet")).await;
    let sqlite_pos = execute_query_on(&sqlite, &sqlite, qry("PositionDetailsGet")).await;
    let sqlite_magi = execute_query_on(&sqlite, &sqlite, qry("MagiProjectionGet")).await;
    assert!(sqlite_div.ok && sqlite_pos.ok && sqlite_magi.ok);

    let snap = sqlite.snapshot_create().await.expect("snapshot");
    let sqlite_path = dir
        .path()
        .join("app-data")
        .join("snapshot-catalog")
        .join("snapshots")
        .join(snap.snapshot_id.to_string())
        .join("database.sqlite");
    assert!(sqlite_path.exists(), "{}", sqlite_path.display());

    let url = require_postgres_url();
    let postgres = PostgresPlatform::connect(&url)
        .await
        .unwrap_or_else(|e| panic!("PostgreSQL connect failed (do not use SQLite): {e}"));
    let imported = must_ok(
        &postgres,
        "SnapshotImport",
        serde_json::json!({"sqlitePath": sqlite_path.to_string_lossy()}),
    )
    .await;
    assert_eq!(imported["accounts"].as_u64().unwrap(), sqlite_counts.accounts);
    assert_eq!(imported["securities"].as_u64().unwrap(), sqlite_counts.securities);
    assert_eq!(
        imported["postedActivities"].as_u64().unwrap(),
        sqlite_counts.posted_activities
    );
    assert_eq!(
        imported["amountMinorSum"].as_i64().unwrap(),
        sqlite_counts.amount_minor_sum
    );
    assert_eq!(
        imported["auditRecords"].as_u64().unwrap(),
        sqlite_counts.audit_records
    );

    let pg_div = execute_query_on(&postgres, &postgres, qry("DividendGet")).await;
    let pg_pos = execute_query_on(&postgres, &postgres, qry("PositionDetailsGet")).await;
    let pg_magi = execute_query_on(&postgres, &postgres, qry("MagiProjectionGet")).await;
    assert!(pg_div.ok && pg_pos.ok && pg_magi.ok);
    let d_sql: serde_json::Value =
        serde_json::from_str(sqlite_div.body_json.as_deref().unwrap_or("{}")).unwrap();
    let d_pg: serde_json::Value =
        serde_json::from_str(pg_div.body_json.as_deref().unwrap_or("{}")).unwrap();
    let p_sql: serde_json::Value =
        serde_json::from_str(sqlite_pos.body_json.as_deref().unwrap_or("{}")).unwrap();
    let p_pg: serde_json::Value =
        serde_json::from_str(pg_pos.body_json.as_deref().unwrap_or("{}")).unwrap();
    let m_sql: serde_json::Value =
        serde_json::from_str(sqlite_magi.body_json.as_deref().unwrap_or("{}")).unwrap();
    let m_pg: serde_json::Value =
        serde_json::from_str(pg_magi.body_json.as_deref().unwrap_or("{}")).unwrap();
    assert_eq!(d_sql["actualTotalMinor"], d_pg["actualTotalMinor"]);
    assert_eq!(p_sql["openPerformanceMinor"], p_pg["openPerformanceMinor"]);
    assert_eq!(p_sql["openTaxMinor"], p_pg["openTaxMinor"]);
    assert_eq!(m_sql["decisionState"], m_pg["decisionState"]);
    assert_eq!(
        m_sql["actualIncludedYtd"]["amountMinor"],
        m_pg["actualIncludedYtd"]["amountMinor"]
    );
}
