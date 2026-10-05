//! Individual-workbook snapshot under raw-data/<date>/.

use application_core::contracts::{
    CommandRequest, FINANCE_CLIENT_CONTRACT_VERSION, SCHEMA_VERSION,
};
use application_core::queries::execute_command_on;
use import_engine::parse_production_templates;
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

async fn must_ok(platform: &LocalPlatform, name: &str, body: serde_json::Value) -> serde_json::Value {
    let result = execute_command_on(platform, platform, cmd(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

#[tokio::test]
async fn data_snapshot_writes_importable_individual_workbooks() {
    let dir = tempfile::tempdir().unwrap();
    let app_dir = dir.path().join("app-data");
    let platform = LocalPlatform::open(&app_dir).await.expect("open sqlite");
    must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "HAKY", "name": "HAKY"}),
    )
    .await;

    let snap = must_ok(
        &platform,
        "DataSnapshotExport",
        serde_json::json!({"asOfDate": "2026-09-09"}),
    )
    .await;
    assert_eq!(snap["asOf"], "2026-09-09");
    assert_eq!(snap["accountCount"], 1);
    let folder = snap["folder"].as_str().expect("folder");
    let expected = app_dir.join("raw-data").join("2026-09-09");
    assert_eq!(std::path::Path::new(folder), expected.as_path());
    for name in [
        "Template_Accounts.xlsx",
        "Template_Positions.xlsx",
        "Template_Lots.xlsx",
        "Template_Transactions_Yield.xlsx",
        "Template_Transactions_Disbursement.xlsx",
        "Template_Trends_Weekly.xlsx",
        "Template_Account_Week_Cash.xlsx",
        "Template_Cash_Elements.xlsx",
        "Template_Planned_Occurrences.xlsx",
        "Template_PlanHistory.xlsx",
        "Template_PaymentPatterns.xlsx",
        "Template_PayDateOverrides.xlsx",
        "Template_RocObservations.xlsx",
        "Template_RetrieveRuns.xlsx",
        "Template_WorkTickets.xlsx",
        "Template_Exceptions.xlsx",
        "Template_HoldingQtyEvents.xlsx",
        "Template_LotAssignments.xlsx",
        "Template_TaxProfiles.xlsx",
        "Template_MarketValueDaily.xlsx",
        "Template_DividendActuals.xlsx",
        "Template_Declarations.xlsx",
        "Template_PayDates.xlsx",
        "Template_LastPrices.xlsx",
        "Template_RetrievalTemplates.xlsx",
        "Template_UiModules.xlsx",
        "calculator-plan-seed.yaml",
        "local.sqlite",
        "MANIFEST.md",
    ] {
        assert!(expected.join(name).is_file(), "missing {name}");
    }
    assert!(
        expected.join("local.sqlite").metadata().unwrap().len() > 0,
        "copied local.sqlite is empty"
    );

    let doc = parse_production_templates(&expected).expect("parse snapshot folder");
    assert!(
        doc.accounts.iter().any(|a| a.name == "Income" && a.kind == "taxable"),
        "Income account missing: {:?}",
        doc.accounts
    );
    assert!(
        doc.securities.iter().any(|s| s.symbol == "HAKY"),
        "HAKY missing: {:?}",
        doc.securities
    );
}

/// The stamp on a new snapshot is the highest SQLite migration number, not the August "28".
#[tokio::test]
async fn snapshot_stamp_matches_the_latest_sqlite_migration() {
    let latest = latest_sqlite_migration_number();
    assert_eq!(
        SCHEMA_VERSION,
        latest.to_string(),
        "SCHEMA_VERSION must be the highest crates/storage-sqlite/migrations number"
    );
    assert_ne!(SCHEMA_VERSION, "28");
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .expect("open sqlite");
    let created = must_ok(&platform, "SnapshotCreate", serde_json::json!({})).await;
    assert_eq!(created["schemaVersion"].as_str().unwrap(), SCHEMA_VERSION);
}

fn latest_sqlite_migration_number() -> u32 {
    let dir = golden_harness::repo_root().join("crates/storage-sqlite/migrations");
    let mut highest = 0_u32;
    for entry in std::fs::read_dir(&dir).expect("migrations dir") {
        let entry = entry.expect("migration entry");
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.ends_with(".sql") {
            continue;
        }
        let digits: String = name.chars().take_while(|c| c.is_ascii_digit()).collect();
        let number: u32 = digits
            .parse()
            .unwrap_or_else(|_| panic!("migration file has no number: {name}"));
        highest = highest.max(number);
    }
    assert!(highest > 0, "no sqlite migrations");
    highest
}
