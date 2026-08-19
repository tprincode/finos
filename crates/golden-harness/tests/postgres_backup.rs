//! Postgres logical dump/restore (M9 B3). MAGI oracles are never written.

use application_core::contracts::{
    CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::ports::canonical::Canonical;
use application_core::ports::platform::Platform;
use application_core::queries::{execute_command_on, execute_query_on};
use golden_harness::repo_root;
use storage_postgres::PostgresPlatform;
use uuid::Uuid;

fn require_postgres_url() -> String {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "DATABASE_URL is required for postgres_backup; do not substitute SQLite. \
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

fn with_database(url: &str, db: &str) -> String {
    let (base, query) = match url.split_once('?') {
        Some((b, q)) => (b, Some(q)),
        None => (url, None),
    };
    let trimmed = base.trim_end_matches('/');
    let replaced = match trimmed.rfind('/') {
        Some(i) => format!("{}/{}", &trimmed[..i], db),
        None => panic!("DATABASE_URL has no database path: {url}"),
    };
    match query {
        Some(q) => format!("{replaced}?{q}"),
        None => replaced,
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

fn compose(args: &[&str]) -> Result<(), String> {
    let output = std::process::Command::new("docker")
        .arg("compose")
        .args(args)
        .current_dir(repo_root())
        .output()
        .map_err(|e| format!("docker compose: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "docker compose {args:?} failed (status {:?}): {} {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

async fn seed_contract_and_g01(platform: &PostgresPlatform) {
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
            "idempotencyKey": "backup-div-1"
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
    must_ok(
        platform,
        "MagiRuleSet",
        serde_json::json!({
            "thresholdMinor": 8_460_000,
            "safetyReserveMinor": 500_000,
            "scale": 2
        }),
    )
    .await;
    must_ok(
        platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": account_id,
            "activityType": "dividend",
            "amountMinor": 2_000_000,
            "scale": 2,
            "occurredOn": "2026-03-15",
            "idempotencyKey": "G01-DIV-1"
        }),
    )
    .await;
    must_ok(
        platform,
        "MagiFactRecord",
        serde_json::json!({
            "sourceId": "G01-DIV-1",
            "treatment": "include",
            "amountMinor": 2_000_000,
            "scale": 2,
            "category": "taxable_dividend"
        }),
    )
    .await;
    must_ok(
        platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": account_id,
            "activityType": "dividend",
            "amountMinor": 2_000_000,
            "scale": 2,
            "occurredOn": "2026-09-15",
            "idempotencyKey": "G01-DIV-2"
        }),
    )
    .await;
    must_ok(
        platform,
        "MagiFactRecord",
        serde_json::json!({
            "sourceId": "G01-DIV-2",
            "treatment": "include",
            "amountMinor": 2_000_000,
            "scale": 2,
            "category": "taxable_dividend"
        }),
    )
    .await;
    must_ok(
        platform,
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
}

fn assert_cents_and_g01(
    dividend: &serde_json::Value,
    details: &serde_json::Value,
    magi: &serde_json::Value,
) {
    assert_eq!(details["openPerformanceMinor"].as_i64().unwrap(), 140_000);
    assert_eq!(details["openTaxMinor"].as_i64().unwrap(), 120_000);
    assert!(
        dividend["actualTotalMinor"].as_i64().unwrap() >= 50_000,
        "dividend actuals missing: {dividend}"
    );
    assert_eq!(magi["decisionState"], "SAFE");
    assert_eq!(magi["actualIncludedYtd"]["amountMinor"].as_i64().unwrap(), 4_000_000);
    assert_eq!(magi["protectedHeadroom"]["amountMinor"].as_i64().unwrap(), 2_960_000);
}

#[tokio::test]
async fn postgres_logical_dump_restore_keeps_magi_and_contract_cents() {
    let url = require_postgres_url();
    let platform = PostgresPlatform::connect(&url)
        .await
        .unwrap_or_else(|e| panic!("PostgreSQL connect failed (do not use SQLite): {e}"));
    platform
        .reset_contract_tables()
        .await
        .unwrap_or_else(|e| panic!("PostgreSQL reset failed: {e}"));
    seed_contract_and_g01(&platform).await;
    let before_div = query_json(&platform, "DividendGet").await;
    let before_details = query_json(&platform, "PositionDetailsGet").await;
    let before_magi = query_json(&platform, "MagiProjectionGet").await;
    assert_cents_and_g01(&before_div, &before_details, &before_magi);

    compose(&[
        "exec",
        "-T",
        "postgres",
        "pg_dump",
        "-U",
        "finos",
        "-d",
        "finos",
        "-Fc",
        "--no-owner",
        "-f",
        "/tmp/finos.dump",
    ])
    .unwrap_or_else(|e| panic!("{e}"));
    compose(&[
        "exec",
        "-T",
        "postgres",
        "dropdb",
        "-U",
        "finos",
        "--if-exists",
        "finos_restore",
    ])
    .unwrap_or_else(|e| panic!("{e}"));
    compose(&[
        "exec",
        "-T",
        "postgres",
        "createdb",
        "-U",
        "finos",
        "finos_restore",
    ])
    .unwrap_or_else(|e| panic!("{e}"));
    compose(&[
        "exec",
        "-T",
        "postgres",
        "pg_restore",
        "-U",
        "finos",
        "--no-owner",
        "-d",
        "finos_restore",
        "/tmp/finos.dump",
    ])
    .unwrap_or_else(|e| panic!("{e}"));

    let restore_url = with_database(&url, "finos_restore");
    let restored = PostgresPlatform::connect(&restore_url)
        .await
        .unwrap_or_else(|e| panic!("restore DB connect failed (do not use SQLite): {e}"));
    let after_div = query_json(&restored, "DividendGet").await;
    let after_details = query_json(&restored, "PositionDetailsGet").await;
    let after_magi = query_json(&restored, "MagiProjectionGet").await;
    assert_cents_and_g01(&after_div, &after_details, &after_magi);
    assert_eq!(before_div["actualTotalMinor"], after_div["actualTotalMinor"]);
    assert_eq!(
        before_details["openPerformanceMinor"],
        after_details["openPerformanceMinor"]
    );
    assert_eq!(before_details["openTaxMinor"], after_details["openTaxMinor"]);
    assert_eq!(before_magi["decisionState"], after_magi["decisionState"]);
    assert_eq!(
        before_magi["actualIncludedYtd"]["amountMinor"],
        after_magi["actualIncludedYtd"]["amountMinor"]
    );
    assert_eq!(
        before_magi["protectedHeadroom"]["amountMinor"],
        after_magi["protectedHeadroom"]["amountMinor"]
    );
}
