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

async fn income_call_book(platform: &LocalPlatform, shares: i64) -> String {
    let account = must_cmd(
        platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "ira"}),
    )
    .await;
    let account_id = account["accountId"].as_str().unwrap().to_string();
    let security = must_cmd(
        platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "TSLL", "name": "TSLA 2x"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    must_cmd(
        platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "priceSource": "public",
            "sourceSymbol": "TSLL",
            "declarationSource": "issuer",
            "sourceUrl": "https://example.test/tsll/distributions",
            "calendarPolicy": "derived_walk",
            "collectorEnabled": true,
            "lookbackCount": 12
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot(platform, security_id, "TSLL")
        .await
        .expect("complete collector");
    if shares > 0 {
        must_cmd(
            platform,
            "LotOpen",
            serde_json::json!({
                "accountId": account_id,
                "securityId": security_id,
                "openedOn": "2024-01-05",
                "origin": "purchase",
                "quantityMinor": shares,
                "quantityScale": 0,
                "performanceBasisMinor": 1424 * shares,
                "taxBasisMinor": 1424 * shares,
                "scale": 2,
                "isOpen": true
            }),
        )
        .await;
    }
    account_id
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
    let account_id = income_call_book(&platform, 100).await;
    let created = must_cmd(
        &platform,
        "ContractCreate",
        serde_json::json!({
            "occSymbol": ".TSLL1270115C20.7",
            "side": "short",
            "quantity": 1,
            "accountId": account_id,
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
    let account_id = income_call_book(&platform, 100).await;
    let created = must_cmd(
        &platform,
        "ContractCreate",
        serde_json::json!({
            "occSymbol": "TSLL270115C20.7",
            "side": "short",
            "quantity": 1,
            "accountId": account_id,
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

#[tokio::test]
async fn ira_accounts_are_accepted_and_others_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let income = income_call_book(&platform, 100).await;
    let created = must_cmd(
        &platform,
        "ContractCreate",
        serde_json::json!({
            "occSymbol": "TSLL270115C20.7",
            "side": "short",
            "quantity": 1,
            "accountId": income,
            "openPremiumMinor": 11500,
            "priorBalanceMinor": 26400,
            "openOn": "2026-10-06"
        }),
    )
    .await;
    assert_eq!(created["status"], "open");
    let posts = must_qry(&platform, "ContractPostList", serde_json::json!({})).await;
    let items = posts["items"].as_array().unwrap();
    let open = items.iter().find(|row| row["reason"] == "open").unwrap();
    assert_eq!(open["weekStart"], "2026-10-03");
    assert_eq!(open["amountMinor"], 11500);
    assert_eq!(open["category"], "Covered Call");
    let prior = items.iter().find(|row| row["reason"] == "prior_balance").unwrap();
    assert_eq!(prior["weekStart"], "");
    assert_eq!(prior["amountMinor"], 26400);
    let roi = must_qry(&platform, "RoiGet", serde_json::json!({})).await;
    assert_eq!(roi["optionPremiumMinor"], 11500 + 26400);
    assert_eq!(roi["dividendActualMinor"], 0);

    for name in ["Car", "Health", "Robinhood", "FI Roth"] {
        let account = must_cmd(
            &platform,
            "AccountRegister",
            serde_json::json!({"name": name, "kind": "taxable"}),
        )
        .await;
        let refused = execute_command_on(
            &platform,
            &platform,
            cmd(
                "ContractCreate",
                serde_json::json!({
                    "occSymbol": "TSLL270115C20.7",
                    "side": "short",
                    "quantity": 1,
                    "accountId": account["accountId"],
                    "openPremiumMinor": 100,
                    "openOn": "2026-10-06"
                }),
            ),
        )
        .await;
        assert!(!refused.ok, "{name} must be refused");
        assert_eq!(refused.error_code.as_deref(), Some("account_refused"));
    }
}

#[tokio::test]
async fn short_put_and_uncovered_call_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let account_id = income_call_book(&platform, 40).await;
    let put = execute_command_on(
        &platform,
        &platform,
        cmd(
            "ContractCreate",
            serde_json::json!({
                "occSymbol": "TSLL270115P20.7",
                "side": "short",
                "quantity": 1,
                "accountId": account_id,
                "openPremiumMinor": 100,
                "openOn": "2026-10-06"
            }),
        ),
    )
    .await;
    assert_eq!(put.error_code.as_deref(), Some("short_put_refused"));
    let uncovered = execute_command_on(
        &platform,
        &platform,
        cmd(
            "ContractCreate",
            serde_json::json!({
                "occSymbol": "TSLL270115C20.7",
                "side": "short",
                "quantity": 1,
                "accountId": account_id,
                "openPremiumMinor": 100,
                "openOn": "2026-10-06"
            }),
        ),
    )
    .await;
    assert_eq!(uncovered.error_code.as_deref(), Some("uncovered_refused"));
    let posts = must_qry(&platform, "ContractPostList", serde_json::json!({})).await;
    assert!(posts["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn close_posts_in_the_close_week() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let account_id = income_call_book(&platform, 100).await;
    let created = must_cmd(
        &platform,
        "ContractCreate",
        serde_json::json!({
            "occSymbol": "TSLL270115C20.7",
            "side": "short",
            "quantity": 1,
            "accountId": account_id,
            "openPremiumMinor": 11500,
            "openOn": "2026-10-06"
        }),
    )
    .await;
    must_cmd(
        &platform,
        "ContractClose",
        serde_json::json!({
            "contractId": created["contractId"],
            "closePremiumMinor": 4000,
            "closedOn": "2026-10-09",
            "how": "closed"
        }),
    )
    .await;
    let posts = must_qry(&platform, "ContractPostList", serde_json::json!({})).await;
    let close = posts["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["reason"] == "close")
        .unwrap();
    assert_eq!(close["weekStart"], "2026-10-03");
    assert_eq!(close["amountMinor"], 7500);
}

#[tokio::test]
async fn assignment_sells_the_reserved_shares() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let account_id = income_call_book(&platform, 100).await;
    let created = must_cmd(
        &platform,
        "ContractCreate",
        serde_json::json!({
            "occSymbol": "TSLL270115C20.7",
            "side": "short",
            "quantity": 1,
            "accountId": account_id,
            "openPremiumMinor": 11500,
            "openOn": "2026-10-06"
        }),
    )
    .await;
    must_cmd(
        &platform,
        "ContractClose",
        serde_json::json!({
            "contractId": created["contractId"],
            "closedOn": "2026-10-09",
            "how": "assigned"
        }),
    )
    .await;
    let basis = must_qry(&platform, "BasisGet", serde_json::json!({})).await;
    let lot = basis["lots"].as_array().unwrap()[0].clone();
    assert_eq!(lot["remainingQuantityMinor"], 0);
    assert_eq!(lot["remainingPerformanceMinor"], 0);
}

fn remaining_qty(basis: &serde_json::Value) -> i64 {
    basis["lots"].as_array().unwrap()[0]["remainingQuantityMinor"]
        .as_i64()
        .unwrap()
}

#[tokio::test]
async fn promised_shares_are_refused_by_cart_and_lot_assign() {
    let open_premium = 11_500_i64;

    let held = tempfile::tempdir().unwrap();
    let held_platform = LocalPlatform::open(held.path().join("app-data"))
        .await
        .unwrap();
    let held_account = income_call_book(&held_platform, 100).await;
    let held_contract = must_cmd(
        &held_platform,
        "ContractCreate",
        serde_json::json!({
            "occSymbol": "TSLL270115C20.7",
            "side": "short",
            "quantity": 1,
            "accountId": held_account,
            "openPremiumMinor": open_premium,
            "openOn": "2026-10-06"
        }),
    )
    .await;
    let held_basis = must_qry(&held_platform, "BasisGet", serde_json::json!({})).await;
    assert_eq!(remaining_qty(&held_basis), 100);
    must_cmd(
        &held_platform,
        "ContractClose",
        serde_json::json!({
            "contractId": held_contract["contractId"],
            "closePremiumMinor": 4_000,
            "closedOn": "2026-10-09",
            "how": "closed"
        }),
    )
    .await;
    let after_close = must_qry(&held_platform, "BasisGet", serde_json::json!({})).await;
    assert_eq!(remaining_qty(&after_close), 100);
    let reconcile = must_qry(&held_platform, "BrokerLotReconcileGet", serde_json::json!({})).await;
    assert_eq!(reconcile["unmatchedSells"].as_u64().unwrap(), 0);

    let sold = tempfile::tempdir().unwrap();
    let sold_platform = LocalPlatform::open(sold.path().join("app-data"))
        .await
        .unwrap();
    let sold_account = income_call_book(&sold_platform, 100).await;
    must_cmd(
        &sold_platform,
        "ContractCreate",
        serde_json::json!({
            "occSymbol": "TSLL270115C20.7",
            "side": "short",
            "quantity": 1,
            "accountId": sold_account,
            "openPremiumMinor": open_premium,
            "openOn": "2026-10-06"
        }),
    )
    .await;
    let book = must_qry(&sold_platform, "BasisGet", serde_json::json!({})).await;
    let lot = &book["lots"].as_array().unwrap()[0];
    let scene = must_cmd(
        &sold_platform,
        "CartScenarioCreate",
        serde_json::json!({
            "accountId": sold_account,
            "asOf": "2026-10-08",
            "name": "Covered",
            "fundingSource": "sellLots"
        }),
    )
    .await;
    let cart_sell = execute_command_on(
        &sold_platform,
        &sold_platform,
        cmd(
            "CartSellLineAdd",
            serde_json::json!({
                "scenarioId": scene["scenarioId"],
                "lotId": lot["lotId"],
                "qtyMinor": 100,
                "unitMinor": 2000,
                "unitScale": 2
            }),
        ),
    )
    .await;
    assert_eq!(cart_sell.error_code.as_deref(), Some("shares_promised"));
    let sale = must_cmd(
        &sold_platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": sold_account,
            "securityId": lot["securityId"],
            "activityType": "sell",
            "amountMinor": 200_000,
            "scale": 2,
            "occurredOn": "2026-10-08"
        }),
    )
    .await;
    let assign = execute_command_on(
        &sold_platform,
        &sold_platform,
        cmd(
            "LotAssign",
            serde_json::json!({
                "lotId": lot["lotId"],
                "activityId": sale["activityId"],
                "quantityMinor": 100,
                "quantityScale": 0
            }),
        ),
    )
    .await;
    assert_eq!(assign.error_code.as_deref(), Some("shares_promised"));
    let still = must_qry(&sold_platform, "BasisGet", serde_json::json!({})).await;
    assert_eq!(remaining_qty(&still), 100);
    let sold_recon = must_qry(&sold_platform, "BrokerLotReconcileGet", serde_json::json!({})).await;
    assert_eq!(sold_recon["unmatchedSells"].as_u64().unwrap(), 1);
    let roi = must_qry(&sold_platform, "RoiGet", serde_json::json!({})).await;
    assert_eq!(roi["performanceGainMinor"].as_i64().unwrap(), 0);
    assert_eq!(roi["dividendActualMinor"].as_i64().unwrap(), 0);
    assert_eq!(roi["optionPremiumMinor"].as_i64().unwrap(), open_premium);

    let assigned = tempfile::tempdir().unwrap();
    let assigned_platform = LocalPlatform::open(assigned.path().join("app-data"))
        .await
        .unwrap();
    let assigned_account = income_call_book(&assigned_platform, 100).await;
    let assigned_contract = must_cmd(
        &assigned_platform,
        "ContractCreate",
        serde_json::json!({
            "occSymbol": "TSLL270115C20.7",
            "side": "short",
            "quantity": 1,
            "accountId": assigned_account,
            "openPremiumMinor": open_premium,
            "openOn": "2026-10-06"
        }),
    )
    .await;
    must_cmd(
        &assigned_platform,
        "ContractClose",
        serde_json::json!({
            "contractId": assigned_contract["contractId"],
            "closedOn": "2026-10-09",
            "how": "assigned"
        }),
    )
    .await;
    let assigned_basis = must_qry(&assigned_platform, "BasisGet", serde_json::json!({})).await;
    assert_eq!(remaining_qty(&assigned_basis), 0);
    let assigned_recon = must_qry(
        &assigned_platform,
        "BrokerLotReconcileGet",
        serde_json::json!({}),
    )
    .await;
    assert_eq!(assigned_recon["assignedQuantityMinor"].as_i64().unwrap(), 0);
    assert_eq!(assigned_recon["unmatchedSells"].as_u64().unwrap(), 0);
}

#[test]
fn contract_fields_and_roadmap_are_registered() {
    let root = repo_root();
    let fields = std::fs::read_to_string(
        root.join("apps/desktop/src/features/field-intent/contractFields.ts"),
    )
    .unwrap();
    for name in [
        "OCC symbol",
        "side",
        "call or put",
        "quantity",
        "account",
        "fragment list",
        "block average",
        "open premium",
        "Previously realized P/L $",
        "live underlying",
        "open date",
        "strike",
        "DTE",
        "ITM",
        "assign risk",
        "period %",
        "annual",
        "week category",
        "option result",
        "stock result",
    ] {
        assert!(fields.contains(&format!("\"{name}\"")), "missing {name}");
    }
    assert!(fields.contains("pageId"));
    assert!(!fields.contains("underlying at open"));
    assert!(fields.contains("contract-positions"));
    let intent = std::fs::read_to_string(
        root.join("apps/desktop/src/features/field-intent/FieldIntentScreen.tsx"),
    )
    .unwrap();
    assert!(intent.contains("aria-label=\"Field intent page\""));
    let page_col = intent.find("<th>Page</th>").expect("Page column");
    let name_col = intent.find("<th>Column</th>").expect("Column");
    assert!(page_col < name_col, "Page is the first column");
    let form = std::fs::read_to_string(
        root.join("apps/desktop/src/features/contracts/ContractCreateForm.tsx"),
    )
    .unwrap();
    assert!(form.contains("Short Cover"));
    assert!(form.contains("Previously realized P/L $"));
    assert!(!form.contains("Underlying at open"));
    assert!(!form.contains("Prior balance"));
    assert!(fields.contains("contract-create"));
    assert!(!fields.contains("Option mid"));
    let registry = std::fs::read_to_string(
        root.join("apps/desktop/src/features/components/ComponentRegistry.tsx"),
    )
    .unwrap();
    assert!(registry.contains("fieldNamesFor"));
    assert!(registry.contains("<dt>Fields</dt>"));
    let app = std::fs::read_to_string(root.join("apps/desktop/src/App.tsx")).unwrap();
    assert!(app.contains("navButton(\"roadmap\", \"Roadmap\")"));
    assert!(app.contains("<RoadmapScreen />"));
    assert!(!app.contains("cash-covered short puts"));
    let roadmap = std::fs::read_to_string(
        root.join("apps/desktop/src/features/roadmap/RoadmapScreen.tsx"),
    )
    .unwrap();
    assert!(roadmap.contains("cash-covered short puts"));
    assert!(!roadmap.contains("SWVXX"));
    for rel in [
        "crates/financial-domain/src/magi.rs",
        "crates/financial-domain/src/roc.rs",
        "crates/storage-sqlite/src/magi.rs",
    ] {
        let text = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(!text.contains("option_premium_post"), "{rel}");
    }
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
