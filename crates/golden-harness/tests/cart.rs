use application_core::contracts::{
    CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::queries::{execute_command_on, execute_query_on};
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

fn qry(name: &str) -> QueryRequest {
    QueryRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        query_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: None,
    }
}

fn qry_body(name: &str, body: serde_json::Value) -> QueryRequest {
    QueryRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        query_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: Some(body.to_string()),
    }
}

async fn query_body(platform: &LocalPlatform, name: &str, body: serde_json::Value) -> serde_json::Value {
    let result = execute_query_on(platform, platform, qry_body(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

async fn research_and_open(
    platform: &LocalPlatform,
    account_id: &str,
    symbol: &str,
    qty_minor: i64,
    qty_scale: u8,
    basis_minor: i64,
) -> (String, String) {
    let security = must_ok(
        platform,
        "SecurityRegister",
        serde_json::json!({"symbol": symbol, "name": symbol}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap().to_string();
    must_ok(
        platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "priceSource": "public",
            "sourceSymbol": symbol,
            "declarationSource": "issuer",
            "sourceUrl": format!("https://example.test/{}/distributions", symbol.to_ascii_lowercase()),
            "calendarPolicy": "derived_walk",
            "collectorEnabled": true,
            "lookbackCount": 12
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot(platform, &security_id, symbol)
        .await
        .expect("complete collector");
    let lot = must_ok(
        platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "openedOn": "2026-09-01",
            "origin": "purchase",
            "quantityMinor": qty_minor,
            "quantityScale": qty_scale,
            "performanceBasisMinor": basis_minor,
            "taxBasisMinor": basis_minor,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;
    (security_id, lot["lotId"].as_str().unwrap().to_string())
}

async fn must_ok(platform: &LocalPlatform, name: &str, body: serde_json::Value) -> serde_json::Value {
    let result = execute_command_on(platform, platform, cmd(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

async fn query_json(platform: &LocalPlatform, name: &str) -> serde_json::Value {
    let result = execute_query_on(platform, platform, qry(name)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

#[tokio::test]
async fn cart_item_does_not_post_dividend_or_lot_facts() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Taxable Brokerage", "kind": "taxable"}),
    )
    .await;
    let account_id = account["accountId"].as_str().unwrap();
    must_ok(
        &platform,
        "DividendActualRecord",
        serde_json::json!({
            "accountId": account_id,
            "occurredOn": "2026-06-15",
            "amountMinor": 50_000,
            "scale": 2,
            "idempotencyKey": "cart-div-1"
        }),
    )
    .await;
    let before = query_json(&platform, "DividendGet").await;
    assert_eq!(before["actualTotalMinor"].as_i64().unwrap(), 50_000);
    let lots_before = query_json(&platform, "BasisGet").await;
    assert_eq!(lots_before["lots"].as_array().unwrap().len(), 0);

    let added = must_ok(
        &platform,
        "CartItemAdd",
        serde_json::json!({
            "symbol": "VXUS",
            "quantityMinor": 10_000,
            "quantityScale": 2
        }),
    )
    .await;
    assert_eq!(added["items"].as_array().unwrap().len(), 1);
    assert_eq!(added["items"][0]["symbol"].as_str().unwrap(), "VXUS");
    assert_eq!(added["items"][0]["quantityMinor"].as_i64().unwrap(), 10_000);

    let got = query_json(&platform, "CartGet").await;
    assert_eq!(got["items"][0]["symbol"].as_str().unwrap(), "VXUS");
    let item_id = got["items"][0]["itemId"].as_str().unwrap();

    let after_add = query_json(&platform, "DividendGet").await;
    assert_eq!(after_add["actualTotalMinor"].as_i64().unwrap(), 50_000);
    let lots_after_add = query_json(&platform, "BasisGet").await;
    assert_eq!(lots_after_add["lots"].as_array().unwrap().len(), 0);

    let removed = must_ok(
        &platform,
        "CartItemRemove",
        serde_json::json!({"itemId": item_id}),
    )
    .await;
    assert_eq!(removed["items"].as_array().unwrap().len(), 0);

    let empty = query_json(&platform, "CartGet").await;
    assert_eq!(empty["items"].as_array().unwrap().len(), 0);
    let after_remove = query_json(&platform, "DividendGet").await;
    assert_eq!(after_remove["actualTotalMinor"].as_i64().unwrap(), 50_000);
}

#[tokio::test]
async fn leftover_keeps_yield_and_draft_does_not_change_books() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "FI Roth", "kind": "roth"}),
    )
    .await;
    let account_id = account["accountId"].as_str().unwrap();
    must_ok(
        &platform,
        "DividendActualRecord",
        serde_json::json!({
            "accountId": account_id,
            "occurredOn": "2026-06-15",
            "amountMinor": 50_000,
            "scale": 2,
            "idempotencyKey": "cart-swap-div"
        }),
    )
    .await;
    let (_spaxx_id, lot_id) =
        research_and_open(&platform, account_id, "SPAXX", 6_577, 2, 6_577).await;
    let haky = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "HAKY", "name": "HAKY"}),
    )
    .await;
    let haky_id = haky["securityId"].as_str().unwrap();
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": haky_id,
            "priceSource": "public",
            "sourceSymbol": "HAKY",
            "declarationSource": "issuer",
            "sourceUrl": "https://example.test/haky/distributions",
            "calendarPolicy": "derived_walk",
            "collectorEnabled": true,
            "lookbackCount": 12
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot(&platform, haky_id, "HAKY")
        .await
        .expect("complete collector");

    let div_before = query_json(&platform, "DividendGet").await;
    let plan_before = query_json(&platform, "IncomePlanGet").await;
    let basis_before = query_json(&platform, "BasisGet").await;
    assert_eq!(basis_before["lots"].as_array().unwrap().len(), 1);

    let scene = must_ok(
        &platform,
        "CartScenarioCreate",
        serde_json::json!({
            "accountId": account_id,
            "asOf": "2026-09-10",
            "cashYieldBps": 334
        }),
    )
    .await;
    let scenario_id = scene["scenarioId"].as_str().unwrap();
    must_ok(
        &platform,
        "CartSellLineAdd",
        serde_json::json!({
            "scenarioId": scenario_id,
            "lotId": lot_id,
            "qtyMinor": 5_990,
            "unitMinor": 100,
            "isCash": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "CartBuyLineAdd",
        serde_json::json!({
            "scenarioId": scenario_id,
            "securityId": haky_id,
            "qtyWhole": 2,
            "lastMinor": 2_995,
            "planAnnualMinor": 912
        }),
    )
    .await;

    let evaluated = query_body(
        &platform,
        "CartScenarioEvaluate",
        serde_json::json!({ "scenarioId": scenario_id }),
    )
    .await;
    let ev = &evaluated["eval"];
    assert_eq!(ev["remainingMinor"].as_i64().unwrap(), 6_577);
    assert_eq!(ev["spendMinor"].as_i64().unwrap(), 5_990);
    assert_eq!(ev["leftoverMinor"].as_i64().unwrap(), 587);
    assert_eq!(ev["surrenderedAnnualMinor"].as_i64().unwrap(), 200);
    assert_eq!(ev["leftoverAnnualMinor"].as_i64().unwrap(), 19);
    assert_eq!(ev["netAnnualMinor"].as_i64().unwrap(), 712);
    assert_eq!(ev["insufficientLotQty"].as_bool().unwrap(), false);

    let div_after = query_json(&platform, "DividendGet").await;
    assert_eq!(
        div_after["actualTotalMinor"].as_i64().unwrap(),
        div_before["actualTotalMinor"].as_i64().unwrap()
    );
    let plan_after = query_json(&platform, "IncomePlanGet").await;
    assert_eq!(
        plan_after["plannedMinor"].as_i64().unwrap(),
        plan_before["plannedMinor"].as_i64().unwrap()
    );
    assert_eq!(
        plan_after["actualMinor"].as_i64().unwrap(),
        plan_before["actualMinor"].as_i64().unwrap()
    );
    let basis_after = query_json(&platform, "BasisGet").await;
    assert_eq!(basis_after["lots"].as_array().unwrap().len(), 1);
    assert_eq!(
        basis_after["lots"][0]["remainingQuantityMinor"].as_i64().unwrap(),
        6_577
    );

    let agreed = must_ok(
        &platform,
        "CartScenarioAgree",
        serde_json::json!({ "scenarioId": scenario_id }),
    )
    .await;
    assert_eq!(agreed["status"].as_str().unwrap(), "agreed");
    must_ok(
        &platform,
        "CartExecuteSell",
        serde_json::json!({
            "scenarioId": scenario_id,
            "occurredOn": "2026-09-10"
        }),
    )
    .await;
    let after_sell = query_json(&platform, "BasisGet").await;
    assert_eq!(after_sell["lots"].as_array().unwrap().len(), 1);
    assert_eq!(
        after_sell["lots"][0]["remainingQuantityMinor"].as_i64().unwrap(),
        587
    );
}

#[tokio::test]
async fn over_remaining_evaluate_shows_intent_agree_blocked() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "FI Roth", "kind": "roth"}),
    )
    .await;
    let account_id = account["accountId"].as_str().unwrap();
    let (_spaxx_id, lot_id) =
        research_and_open(&platform, account_id, "SPAXX", 6_577, 2, 6_577).await;
    let haky = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "HAKY", "name": "HAKY"}),
    )
    .await;
    let haky_id = haky["securityId"].as_str().unwrap();
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": haky_id,
            "priceSource": "public",
            "sourceSymbol": "HAKY",
            "declarationSource": "issuer",
            "sourceUrl": "https://example.test/haky/distributions",
            "calendarPolicy": "derived_walk",
            "collectorEnabled": true,
            "lookbackCount": 12
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot(&platform, haky_id, "HAKY")
        .await
        .expect("complete collector");

    let scene = must_ok(
        &platform,
        "CartScenarioCreate",
        serde_json::json!({
            "accountId": account_id,
            "asOf": "2026-09-10",
            "cashYieldBps": 334
        }),
    )
    .await;
    let scenario_id = scene["scenarioId"].as_str().unwrap();
    must_ok(
        &platform,
        "CartSellLineAdd",
        serde_json::json!({
            "scenarioId": scenario_id,
            "lotId": lot_id,
            "qtyMinor": 6_577,
            "unitMinor": 100,
            "isCash": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "CartBuyLineAdd",
        serde_json::json!({
            "scenarioId": scenario_id,
            "securityId": haky_id,
            "qtyWhole": 11,
            "lastMinor": 2_995,
            "planAnnualMinor": 5_016
        }),
    )
    .await;
    let evaluated = query_body(
        &platform,
        "CartScenarioEvaluate",
        serde_json::json!({ "scenarioId": scenario_id }),
    )
    .await;
    assert_eq!(evaluated["eval"]["insufficientLotQty"].as_bool().unwrap(), true);
    assert_eq!(evaluated["eval"]["spendMinor"].as_i64().unwrap(), 32_945);
    assert_eq!(evaluated["eval"]["remainingMinor"].as_i64().unwrap(), 6_577);
    let blocked = execute_command_on(
        &platform,
        &platform,
        cmd(
            "CartScenarioAgree",
            serde_json::json!({ "scenarioId": scenario_id }),
        ),
    )
    .await;
    assert!(!blocked.ok);
    assert_eq!(blocked.error_code.as_deref(), Some("insufficient_lot_qty"));
    let lots = query_json(&platform, "BasisGet").await;
    assert_eq!(lots["lots"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn draft_discard_does_not_change_lots() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "FI Roth", "kind": "roth"}),
    )
    .await;
    let scene = must_ok(
        &platform,
        "CartScenarioCreate",
        serde_json::json!({
            "accountId": account["accountId"],
            "asOf": "2026-09-10",
            "cashYieldBps": 334
        }),
    )
    .await;
    let scenario_id = scene["scenarioId"].as_str().unwrap();
    must_ok(
        &platform,
        "CartScenarioDiscard",
        serde_json::json!({ "scenarioId": scenario_id }),
    )
    .await;
    let missing = execute_query_on(
        &platform,
        &platform,
        qry_body(
            "CartScenarioGet",
            serde_json::json!({ "scenarioId": scenario_id }),
        ),
    )
    .await;
    assert!(!missing.ok);
    assert_eq!(missing.error_code.as_deref(), Some("missing_scenario"));
    let lots = query_json(&platform, "BasisGet").await;
    assert_eq!(lots["lots"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn sell_line_snapshots_tax_and_performance_pnl() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Car", "kind": "taxable"}),
    )
    .await;
    let account_id = account["accountId"].as_str().unwrap();
    let (security_id, lot_id) =
        research_and_open(&platform, account_id, "LOSS1", 10, 0, 20_000).await;
    // Re-open is not needed; overwrite tax via a second lot is hard. Use this lot's
    // performance=20000 as both bases (research_and_open sets tax=perf). Sell at $18.
    let _ = security_id;
    let scene = must_ok(
        &platform,
        "CartScenarioCreate",
        serde_json::json!({
            "accountId": account_id,
            "asOf": "2026-09-11",
            "cashYieldBps": 0,
            "name": "Tax loss"
        }),
    )
    .await;
    let added = must_ok(
        &platform,
        "CartSellLineAdd",
        serde_json::json!({
            "scenarioId": scene["scenarioId"],
            "lotId": lot_id,
            "qtyMinor": 10,
            "unitMinor": 1_800,
            "isCash": false
        }),
    )
    .await;
    assert_eq!(added["name"].as_str().unwrap(), "Tax loss");
    let line = &added["sellLines"][0];
    assert_eq!(line["proceedsMinor"].as_i64().unwrap(), 18_000);
    assert_eq!(line["performanceCostMinor"].as_i64().unwrap(), 20_000);
    assert_eq!(line["taxCostMinor"].as_i64().unwrap(), 20_000);
    assert_eq!(line["performanceGainMinor"].as_i64().unwrap(), -2_000);
    assert_eq!(line["taxGainMinor"].as_i64().unwrap(), -2_000);
    let books = query_json(&platform, "BasisGet").await;
    assert_eq!(books["lots"][0]["remainingQuantityMinor"].as_i64().unwrap(), 10);
}

#[tokio::test]
async fn duplicate_draft_lists_for_monthly_compare() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "FI Roth", "kind": "roth"}),
    )
    .await;
    let account_id = account["accountId"].as_str().unwrap();
    let scene = must_ok(
        &platform,
        "CartScenarioCreate",
        serde_json::json!({
            "accountId": account_id,
            "asOf": "2026-09-11",
            "cashYieldBps": 334,
            "name": "2 HAKY"
        }),
    )
    .await;
    let copy = must_ok(
        &platform,
        "CartScenarioDuplicate",
        serde_json::json!({ "scenarioId": scene["scenarioId"] }),
    )
    .await;
    assert_eq!(copy["name"].as_str().unwrap(), "2 HAKY (copy)");
    assert_eq!(copy["status"].as_str().unwrap(), "draft");
    assert_ne!(
        copy["scenarioId"].as_str().unwrap(),
        scene["scenarioId"].as_str().unwrap()
    );
    let listed = query_body(
        &platform,
        "CartScenarioList",
        serde_json::json!({ "accountId": account_id }),
    )
    .await;
    assert_eq!(listed["items"].as_array().unwrap().len(), 2);
    let lots = query_json(&platform, "BasisGet").await;
    assert_eq!(lots["lots"].as_array().unwrap().len(), 0);
}

#[test]
fn shopping_cart_feature_folder_not_app_or_queries_math() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for rel in [
        "apps/desktop/src/features/shopping-cart/ShoppingCartScreen.tsx",
        "apps/desktop/src/features/shopping-cart/CartStartWizard.tsx",
        "apps/desktop/src/features/shopping-cart/AccountCashPlan.tsx",
        "crates/application-core/src/cash_pile.rs",
        "apps/desktop/src/features/shopping-cart/StepRail.tsx",
        "apps/desktop/src/features/shopping-cart/AffordStrip.tsx",
        "apps/desktop/src/features/shopping-cart/IncomeCompare.tsx",
        "apps/desktop/src/features/shopping-cart/MixBars.tsx",
        "apps/desktop/src/features/shopping-cart/TradeoffCallout.tsx",
        "apps/desktop/src/features/shopping-cart/ExecutePanel.tsx",
        "apps/desktop/src/features/shopping-cart/ComparePlans.tsx",
        "apps/desktop/src/features/shopping-cart/CartBlendTable.tsx",
        "apps/desktop/src/features/shopping-cart/cartPlan.ts",
        "apps/desktop/src/features/shared/pickers/AccountSelect.tsx",
        "apps/desktop/src/features/shared/pickers/LotSelect.tsx",
        "apps/desktop/src/features/shared/pickers/LotCostTable.tsx",
        "apps/desktop/src/features/shared/pickers/ResearchedSymbolCombobox.tsx",
        "crates/application-core/src/cart.rs",
    ] {
        assert!(
            root.join(rel).is_file(),
            "missing {rel}"
        );
    }
    let app = std::fs::read_to_string(root.join("apps/desktop/src/App.tsx")).unwrap();
    assert!(app.contains("setScreen(\"shopping-cart\")") || app.contains("\"shopping-cart\""));
    assert!(
        !app.contains("AffordStrip") && !app.contains("cart-eval-snapshot"),
        "App.tsx must not host cart grid or evaluate markup"
    );
    let queries = std::fs::read_to_string(root.join("crates/application-core/src/queries.rs")).unwrap();
    assert!(queries.contains("CartScenarioEvaluate"));
    assert!(
        !queries.contains("evaluate_swap") && !queries.contains("INSERT INTO cart_scenario"),
        "queries.rs is dispatch only"
    );
    let tradeoff = std::fs::read_to_string(
        root.join("apps/desktop/src/features/shopping-cart/TradeoffCallout.tsx"),
    )
    .unwrap();
    assert!(
        tradeoff.contains("Income would rise") && tradeoff.contains("Mix would worsen"),
        "BR-SC-013 both sentences"
    );
    let table = std::fs::read_to_string(
        root.join("apps/desktop/src/features/shared/pickers/LotCostTable.tsx"),
    )
    .unwrap();
    assert!(table.contains("lowest-cost") && table.contains("largest-tax-loss"));
    let compare = std::fs::read_to_string(
        root.join("apps/desktop/src/features/shopping-cart/ComparePlans.tsx"),
    )
    .unwrap();
    assert!(compare.contains("Keep cash") && compare.contains("Monthly"));
    let screen = std::fs::read_to_string(
        root.join("apps/desktop/src/features/shopping-cart/ShoppingCartScreen.tsx"),
    )
    .unwrap();
    assert!(
        screen.contains("Cart cash yield from collector"),
        "yield is collector-owned"
    );
    assert!(
        !screen.contains("Cart cash yield bps") && !screen.contains("Cash yield (bps)"),
        "owner must not type a fictitious cash yield"
    );
    assert!(
        screen.contains("CartStartWizard")
            && screen.contains("Cart funding next step")
            && screen.contains("AccountCashPlan"),
        "start is a wizard; parked funding does not open the lot picker"
    );
    assert!(
        !screen.contains("Create swap draft") && !screen.contains("Cart draft name"),
        "one-screen create chrome is gone"
    );
    let wizard = std::fs::read_to_string(
        root.join("apps/desktop/src/features/shopping-cart/CartStartWizard.tsx"),
    )
    .unwrap();
    assert!(
        wizard.contains("prompt === \"account\"")
            && wizard.contains("prompt === \"planName\"")
            && wizard.contains("prompt === \"funding\""),
        "one prompt at a time"
    );
    assert!(
        wizard.contains("Next cart step") && wizard.contains("Previous cart step"),
        "wizard next/back"
    );
    let rail = std::fs::read_to_string(
        root.join("apps/desktop/src/features/shopping-cart/StepRail.tsx"),
    )
    .unwrap();
    assert!(rail.contains("Account") && rail.contains("Plan name") && rail.contains("How funded"));
}

#[tokio::test]
async fn leftover_yield_comes_from_collector_plan_not_typed_bps() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "FI Roth", "kind": "roth"}),
    )
    .await;
    let account_id = account["accountId"].as_str().unwrap();
    let (spaxx_id, lot_id) =
        research_and_open(&platform, account_id, "SPAXX", 6_577, 2, 6_577).await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": spaxx_id,
            "paymentFrequency": "Monthly",
            "replaceCadence": true,
            "divType": "CASH",
            "riskTier": "Foundation"
        }),
    )
    .await;
    must_ok(
        &platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": spaxx_id,
            "amountPerShareMinor": 2_783,
            "amountScale": 6,
            "paymentPeriod": "2026-09-01",
            "source": "collector",
            "enteredAt": "2026-09-01"
        }),
    )
    .await;
    must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": spaxx_id,
            "amountPerShareMinor": 2_783,
            "amountScale": 6,
            "planningPeriodsPerYear": 12,
            "effectiveFrom": "2026-09-01",
            "decisionReason": "collector",
            "incompleteAnalysisReason": "Fewer than 6 observations"
        }),
    )
    .await;
    let haky = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "HAKY", "name": "HAKY"}),
    )
    .await;
    let haky_id = haky["securityId"].as_str().unwrap();
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": haky_id,
            "priceSource": "public",
            "sourceSymbol": "HAKY",
            "declarationSource": "issuer",
            "sourceUrl": "https://example.test/haky/distributions",
            "calendarPolicy": "derived_walk",
            "collectorEnabled": true,
            "lookbackCount": 12
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot(&platform, haky_id, "HAKY")
        .await
        .expect("complete collector");

    let scene = must_ok(
        &platform,
        "CartScenarioCreate",
        serde_json::json!({
            "accountId": account_id,
            "asOf": "2026-09-11",
            "cashYieldBps": 999
        }),
    )
    .await;
    assert_eq!(
        scene["cashYieldBps"].as_i64().unwrap(),
        334,
        "typed 9.99% must not override the SPAXX collector plan"
    );
    let scenario_id = scene["scenarioId"].as_str().unwrap();
    must_ok(
        &platform,
        "CartSellLineAdd",
        serde_json::json!({
            "scenarioId": scenario_id,
            "lotId": lot_id,
            "qtyMinor": 5_990,
            "unitMinor": 100,
            "isCash": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "CartBuyLineAdd",
        serde_json::json!({
            "scenarioId": scenario_id,
            "securityId": haky_id,
            "qtyWhole": 2,
            "lastMinor": 2_995,
            "planAnnualMinor": 912
        }),
    )
    .await;
    let evaluated = query_body(
        &platform,
        "CartScenarioEvaluate",
        serde_json::json!({ "scenarioId": scenario_id }),
    )
    .await;
    let ev = &evaluated["eval"];
    assert_eq!(ev["leftoverMinor"].as_i64().unwrap(), 587);
    assert_eq!(ev["surrenderedAnnualMinor"].as_i64().unwrap(), 200);
    assert_eq!(ev["leftoverAnnualMinor"].as_i64().unwrap(), 19);
    assert_eq!(ev["netAnnualMinor"].as_i64().unwrap(), 712);
}

#[tokio::test]
async fn missing_cash_plan_leaves_leftover_yield_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "FI Roth", "kind": "roth"}),
    )
    .await;
    let account_id = account["accountId"].as_str().unwrap();
    let (_spaxx_id, lot_id) =
        research_and_open(&platform, account_id, "SPAXX", 6_577, 2, 6_577).await;
    let haky = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "HAKY", "name": "HAKY"}),
    )
    .await;
    let haky_id = haky["securityId"].as_str().unwrap();
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": haky_id,
            "priceSource": "public",
            "sourceSymbol": "HAKY",
            "declarationSource": "issuer",
            "sourceUrl": "https://example.test/haky/distributions",
            "calendarPolicy": "derived_walk",
            "collectorEnabled": true,
            "lookbackCount": 12
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot(&platform, haky_id, "HAKY")
        .await
        .expect("complete collector");

    let scene = must_ok(
        &platform,
        "CartScenarioCreate",
        serde_json::json!({
            "accountId": account_id,
            "asOf": "2026-09-11"
        }),
    )
    .await;
    assert_eq!(scene["cashYieldBps"].as_i64().unwrap(), 0);
    let scenario_id = scene["scenarioId"].as_str().unwrap();
    must_ok(
        &platform,
        "CartSellLineAdd",
        serde_json::json!({
            "scenarioId": scenario_id,
            "lotId": lot_id,
            "qtyMinor": 5_990,
            "unitMinor": 100,
            "isCash": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "CartBuyLineAdd",
        serde_json::json!({
            "scenarioId": scenario_id,
            "securityId": haky_id,
            "qtyWhole": 2,
            "lastMinor": 2_995,
            "planAnnualMinor": 912
        }),
    )
    .await;
    let evaluated = query_body(
        &platform,
        "CartScenarioEvaluate",
        serde_json::json!({ "scenarioId": scenario_id }),
    )
    .await;
    let ev = &evaluated["eval"];
    assert_eq!(ev["leftoverMinor"].as_i64().unwrap(), 587);
    assert!(ev["surrenderedAnnualMinor"].is_null());
    assert!(ev["leftoverAnnualMinor"].is_null());
    assert!(ev["netAnnualMinor"].is_null());
    assert_eq!(ev["buyAnnualMinor"].as_i64().unwrap(), 912);
}

#[tokio::test]
async fn create_persists_funding_source_and_rejects_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "FI Roth", "kind": "roth"}),
    )
    .await;
    let account_id = account["accountId"].as_str().unwrap();
    let omitted = must_ok(
        &platform,
        "CartScenarioCreate",
        serde_json::json!({
            "accountId": account_id,
            "asOf": "2026-09-11",
            "name": "Default"
        }),
    )
    .await;
    assert_eq!(omitted["fundingSource"].as_str().unwrap(), "sellLots");
    assert_eq!(omitted["kind"].as_str().unwrap(), "Swap");
    let cash = must_ok(
        &platform,
        "CartScenarioCreate",
        serde_json::json!({
            "accountId": account_id,
            "asOf": "2026-09-11",
            "name": "Cash cover",
            "fundingSource": "accountCash"
        }),
    )
    .await;
    assert_eq!(cash["fundingSource"].as_str().unwrap(), "accountCash");
    let deposit = must_ok(
        &platform,
        "CartScenarioCreate",
        serde_json::json!({
            "accountId": account_id,
            "asOf": "2026-09-11",
            "name": "Deposit",
            "fundingSource": "newDeposit"
        }),
    )
    .await;
    assert_eq!(deposit["fundingSource"].as_str().unwrap(), "newDeposit");
    let blocked = execute_command_on(
        &platform,
        &platform,
        cmd(
            "CartScenarioCreate",
            serde_json::json!({
                "accountId": account_id,
                "asOf": "2026-09-11",
                "fundingSource": "fifo"
            }),
        ),
    )
    .await;
    assert!(!blocked.ok);
    assert_eq!(blocked.error_code.as_deref(), Some("invalid_funding_source"));
    let lots = query_json(&platform, "BasisGet").await;
    assert_eq!(lots["lots"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn account_cash_approve_checks_live_qty_fill_deducts() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let energy = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Energy", "kind": "taxable"}),
    )
    .await;
    let blocked_energy = execute_command_on(
        &platform,
        &platform,
        cmd(
            "CartScenarioCreate",
            serde_json::json!({
                "accountId": energy["accountId"],
                "asOf": "2026-09-11",
                "fundingSource": "accountCash"
            }),
        ),
    )
    .await;
    assert!(!blocked_energy.ok);
    assert_eq!(
        blocked_energy.error_code.as_deref(),
        Some("account_cash_not_offered")
    );

    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "FI Roth", "kind": "roth"}),
    )
    .await;
    let account_id = account["accountId"].as_str().unwrap();
    let (_spaxx_id, _lot_id) =
        research_and_open(&platform, account_id, "SPAXX", 5_000, 2, 5_000).await;
    let haky = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "HAKY", "name": "HAKY"}),
    )
    .await;
    let haky_id = haky["securityId"].as_str().unwrap();
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": haky_id,
            "priceSource": "public",
            "sourceSymbol": "HAKY",
            "declarationSource": "issuer",
            "sourceUrl": "https://example.test/haky/distributions",
            "calendarPolicy": "derived_walk",
            "collectorEnabled": true,
            "lookbackCount": 12
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot(&platform, haky_id, "HAKY")
        .await
        .expect("complete collector");

    let pile = query_body(
        &platform,
        "CashPileGet",
        serde_json::json!({ "accountId": account_id }),
    )
    .await;
    assert_eq!(pile["found"], true);
    assert_eq!(pile["symbol"], "SPAXX");
    assert_eq!(pile["dollarsMinor"].as_i64().unwrap(), 5_000);

    let scene = must_ok(
        &platform,
        "CartScenarioCreate",
        serde_json::json!({
            "accountId": account_id,
            "asOf": "2026-09-11",
            "name": "2 HAKY cash",
            "fundingSource": "accountCash"
        }),
    )
    .await;
    let scenario_id = scene["scenarioId"].as_str().unwrap();
    must_ok(
        &platform,
        "CartBuyLineAdd",
        serde_json::json!({
            "scenarioId": scenario_id,
            "securityId": haky_id,
            "qtyWhole": 2,
            "lastMinor": 2_995,
            "planAnnualMinor": 912
        }),
    )
    .await;
    let after_draft = query_json(&platform, "BasisGet").await;
    assert_eq!(
        after_draft["lots"][0]["remainingQuantityMinor"].as_i64().unwrap(),
        5_000,
        "draft must not spend the cash pile"
    );

    let evaluated = query_body(
        &platform,
        "CartScenarioEvaluate",
        serde_json::json!({ "scenarioId": scenario_id }),
    )
    .await;
    assert_eq!(evaluated["eval"]["remainingMinor"].as_i64().unwrap(), 5_000);
    assert_eq!(evaluated["eval"]["spendMinor"].as_i64().unwrap(), 5_990);
    assert_eq!(evaluated["eval"]["insufficientLotQty"], true);

    let agree_short = execute_command_on(
        &platform,
        &platform,
        cmd(
            "CartScenarioAgree",
            serde_json::json!({ "scenarioId": scenario_id }),
        ),
    )
    .await;
    assert!(!agree_short.ok);
    assert_eq!(
        agree_short.error_code.as_deref(),
        Some("insufficient_account_cash")
    );

    must_ok(
        &platform,
        "CashDeposit",
        serde_json::json!({
            "accountId": account_id,
            "amountMinor": 2_000,
            "occurredOn": "2026-09-11"
        }),
    )
    .await;
    let piled = query_body(
        &platform,
        "CashPileGet",
        serde_json::json!({ "accountId": account_id }),
    )
    .await;
    assert_eq!(piled["dollarsMinor"].as_i64().unwrap(), 7_000);
    let ledger = query_body(
        &platform,
        "CashLedgerGet",
        serde_json::json!({ "accountId": account_id }),
    )
    .await;
    assert_eq!(ledger["entries"].as_array().unwrap().len(), 1);
    assert_eq!(ledger["entries"][0]["activityType"], "deposit");

    let enough = query_body(
        &platform,
        "CartScenarioEvaluate",
        serde_json::json!({ "scenarioId": scenario_id }),
    )
    .await;
    assert_eq!(enough["eval"]["remainingMinor"].as_i64().unwrap(), 7_000);
    assert_eq!(enough["eval"]["insufficientLotQty"], false);

    must_ok(
        &platform,
        "CartScenarioAgree",
        serde_json::json!({ "scenarioId": scenario_id }),
    )
    .await;
    let line_id = enough["buyLines"][0]["lineId"].as_str().unwrap();
    must_ok(
        &platform,
        "CartExecuteFill",
        serde_json::json!({
            "scenarioId": scenario_id,
            "occurredOn": "2026-09-11",
            "fills": [{ "lineId": line_id, "fillMinor": 2_995 }]
        }),
    )
    .await;
    let after_fill = query_json(&platform, "BasisGet").await;
    assert_eq!(
        after_fill["lots"][0]["remainingQuantityMinor"].as_i64().unwrap(),
        1_010
    );
    let ledger2 = query_body(
        &platform,
        "CashLedgerGet",
        serde_json::json!({ "accountId": account_id }),
    )
    .await;
    assert_eq!(ledger2["entries"].as_array().unwrap().len(), 2);
    assert!(ledger2["entries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["activityType"] == "withdrawal"));
}

#[tokio::test]
async fn cart_buy_qty_set_scales_plan_and_allows_over_budget() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "FI Roth", "kind": "roth"}),
    )
    .await;
    let account_id = account["accountId"].as_str().unwrap();
    let (_spaxx_id, lot_id) =
        research_and_open(&platform, account_id, "SPAXX", 6_577, 2, 6_577).await;
    let haky = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "HAKY", "name": "HAKY"}),
    )
    .await;
    let haky_id = haky["securityId"].as_str().unwrap();
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": haky_id,
            "priceSource": "public",
            "sourceSymbol": "HAKY",
            "declarationSource": "issuer",
            "sourceUrl": "https://example.test/haky/distributions",
            "calendarPolicy": "derived_walk",
            "collectorEnabled": true,
            "lookbackCount": 12
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot(&platform, haky_id, "HAKY")
        .await
        .expect("complete collector");

    let scene = must_ok(
        &platform,
        "CartScenarioCreate",
        serde_json::json!({
            "accountId": account_id,
            "asOf": "2026-09-11",
            "cashYieldBps": 334
        }),
    )
    .await;
    let scenario_id = scene["scenarioId"].as_str().unwrap();
    must_ok(
        &platform,
        "CartSellLineAdd",
        serde_json::json!({
            "scenarioId": scenario_id,
            "lotId": lot_id,
            "qtyMinor": 6_577,
            "unitMinor": 100,
            "isCash": true
        }),
    )
    .await;
    let added = must_ok(
        &platform,
        "CartBuyLineAdd",
        serde_json::json!({
            "scenarioId": scenario_id,
            "securityId": haky_id,
            "qtyWhole": 2,
            "lastMinor": 2_995,
            "planAnnualMinor": 912
        }),
    )
    .await;
    let line_id = added["buyLines"][0]["lineId"].as_str().unwrap();
    let next = must_ok(
        &platform,
        "CartBuyLineQtySet",
        serde_json::json!({
            "scenarioId": scenario_id,
            "lineId": line_id,
            "qtyWhole": 11
        }),
    )
    .await;
    assert_eq!(next["buyLines"][0]["qtyWhole"].as_i64().unwrap(), 11);
    assert_eq!(next["buyLines"][0]["spendMinor"].as_i64().unwrap(), 32_945);
    assert_eq!(next["buyLines"][0]["planAnnualMinor"].as_i64().unwrap(), 5_016);
    let evaluated = query_body(
        &platform,
        "CartScenarioEvaluate",
        serde_json::json!({ "scenarioId": scenario_id }),
    )
    .await;
    assert_eq!(evaluated["eval"]["insufficientLotQty"].as_bool().unwrap(), true);
    assert_eq!(evaluated["eval"]["spendMinor"].as_i64().unwrap(), 32_945);
}
