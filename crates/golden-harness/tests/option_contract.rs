//! Option contracts persist in SQLite. Money is cents (scale 2). OCC parse fixtures lock.

use application_core::contracts::{
    CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::option_contract::parse_occ;
use application_core::queries::{execute_command_on, execute_query_on};
use golden_harness::repo_root;
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

fn qry(name: &str, body: serde_json::Value) -> QueryRequest {
    QueryRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        query_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: Some(body.to_string()),
    }
}

async fn must_cmd(platform: &LocalPlatform, name: &str, body: serde_json::Value) -> serde_json::Value {
    let result = execute_command_on(platform, platform, cmd(name, body)).await;
    assert!(
        result.ok,
        "{name} failed: {:?}",
        result.error_code
    );
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

async fn must_qry(platform: &LocalPlatform, name: &str, body: serde_json::Value) -> serde_json::Value {
    let result = execute_query_on(platform, platform, qry(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

fn format_usd_cents(minor: i64) -> String {
    let sign = if minor < 0 { "-" } else { "" };
    let abs = minor.unsigned_abs();
    format!("{sign}${}.{:02}", abs / 100, abs % 100)
}

#[test]
fn parse_occ_fixtures_lock_strike_minor() {
    let a = parse_occ(".TSLL1270115C20.7").expect("dot tsll");
    assert_eq!(a.underlying, "TSLL");
    assert_eq!(a.expiry_on, "2027-01-15");
    assert_eq!(a.put_call, "C");
    assert_eq!(a.strike_minor, 2070);

    let b = parse_occ("TSLL270115C20.7").expect("tsll");
    assert_eq!(b.expiry_on, "2027-01-15");
    assert_eq!(b.strike_minor, 2070);

    let c = parse_occ(".AAPL251219P250").expect("aapl put");
    assert_eq!(c.underlying, "AAPL");
    assert_eq!(c.expiry_on, "2025-12-19");
    assert_eq!(c.put_call, "P");
    assert_eq!(c.strike_minor, 25000);
}

#[test]
fn display_lock_cents_not_dollars_as_minor() {
    assert_eq!(format_usd_cents(2070), "$20.70");
    assert_eq!(format_usd_cents(115), "$1.15");
    assert_eq!(format_usd_cents(248_580), "$2485.80");
}

#[tokio::test]
async fn create_survives_reopen_and_stores_cents() {
    let dir = tempfile::tempdir().unwrap();
    let app_data = dir.path().join("app-data");
    let platform = LocalPlatform::open(&app_data).await.unwrap();
    let created = must_cmd(
        &platform,
        "ContractCreate",
        serde_json::json!({
            "occSymbol": ".TSLL1270115C20.7",
            "side": "short",
            "quantity": 1,
            "openPremiumMinor": 115,
            "openOn": "2026-09-30"
        }),
    )
    .await;
    assert_eq!(created["strikeMinor"], 2070);
    assert_eq!(created["openPremiumMinor"], 115);
    assert_ne!(created["strikeMinor"], 207_000);
    assert_eq!(created["status"], "open");

    drop(platform);
    let platform2 = LocalPlatform::open(&app_data).await.unwrap();
    let list = must_qry(&platform2, "ContractList", serde_json::json!({})).await;
    let items = list["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["occSymbol"], "TSLL1270115C20.7");
    assert_eq!(items[0]["strikeMinor"], 2070);
    assert_eq!(items[0]["openPremiumMinor"], 115);
    assert_eq!(format_usd_cents(2070), "$20.70");
    assert_eq!(format_usd_cents(115), "$1.15");
}

#[tokio::test]
async fn roll_links_old_to_new() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let created = must_cmd(
        &platform,
        "ContractCreate",
        serde_json::json!({
            "occSymbol": "TSLL270115C20.7",
            "side": "short",
            "quantity": 1,
            "openPremiumMinor": 115,
            "openOn": "2026-09-01"
        }),
    )
    .await;
    let id = created["contractId"].as_str().unwrap().to_string();
    let list = must_cmd(
        &platform,
        "ContractRoll",
        serde_json::json!({
            "contractId": id,
            "newOccSymbol": "TSLL270219C22",
            "closePremiumMinor": 40,
            "newOpenPremiumMinor": 90,
            "newOpenOn": "2026-09-30"
        }),
    )
    .await;
    let items = list["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    let old = items.iter().find(|r| r["status"] == "rolled").unwrap();
    let new = items.iter().find(|r| r["status"] == "open").unwrap();
    assert_eq!(old["rollToContractId"], new["contractId"]);
    assert_eq!(old["closePremiumMinor"], 40);
    assert_eq!(new["strikeMinor"], 2200);
}

#[tokio::test]
async fn bad_symbol_does_not_insert() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let result = execute_command_on(
        &platform,
        &platform,
        cmd(
            "ContractCreate",
            serde_json::json!({
                "occSymbol": "NOTANOCC",
                "side": "short",
                "quantity": 1,
                "openPremiumMinor": 115,
                "openOn": "2026-09-30"
            }),
        ),
    )
    .await;
    assert!(!result.ok);
    assert_eq!(
        result.error_code.as_deref(),
        Some("Not an OCC symbol.")
    );
    let list = must_qry(&platform, "ContractList", serde_json::json!({})).await;
    assert!(list["items"].as_array().unwrap().is_empty());
}

#[test]
fn app_tsx_is_thin_mount_only() {
    let app = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx")).unwrap();
    assert!(app.contains("<ContractPositions client={client}"));
    assert!(!app.contains("parseOcc("));
    assert!(!app.contains("option_contract"));
    assert!(!app.contains("ContractCreate"));
}

#[test]
fn interest_rate_calculator_has_no_occ_rows() {
    let screen = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/interest-rate/InterestRateCalculator.tsx"),
    )
    .unwrap();
    assert!(!screen.contains("parseOcc"));
    assert!(!screen.contains("ContractPositions"));
    assert!(!screen.contains("OCC"));
}
