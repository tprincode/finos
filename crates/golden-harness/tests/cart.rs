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
            "priceScale": 2,
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
        6_577
    );
    let opened = must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account_id,
            "securityId": haky_id,
            "openedOn": "2026-09-10",
            "origin": "purchase",
            "quantityMinor": 2,
            "quantityScale": 0,
            "performanceBasisMinor": 5_990,
            "taxBasisMinor": 5_990,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;
    let haky_lot = opened["lotId"].as_str().unwrap();
    let after_buy = must_ok(
        &platform,
        "CartExecuteBuyStep",
        serde_json::json!({
            "scenarioId": scenario_id,
            "lotId": haky_lot
        }),
    )
    .await;
    assert_eq!(after_buy["status"].as_str().unwrap(), "executing");
    let buy_kinds: Vec<&str> = after_buy["executeSteps"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|step| step["kind"].as_str())
        .collect();
    assert!(buy_kinds.contains(&"buy"));
    assert!(
        !buy_kinds.contains(&"cash_align"),
        "buy step records the lot; leftover cash aligns on the next command"
    );
    must_ok(
        &platform,
        "CartExecuteCashAlign",
        serde_json::json!({
            "scenarioId": scenario_id,
            "occurredOn": "2026-09-10"
        }),
    )
    .await;
    let after_align = query_json(&platform, "BasisGet").await;
    assert_eq!(after_align["lots"].as_array().unwrap().len(), 2);
    let spaxx = after_align["lots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|lot| lot["lotId"].as_str() == Some(lot_id.as_str()))
        .expect("spaxx lot");
    assert_eq!(
        spaxx["remainingQuantityMinor"].as_i64().unwrap(),
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
            "priceScale": 2,
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
        app.contains("setScreen(fromCart ? \"shopping-cart\"")
            && app.contains("resumeScenarioId={cartResumeScenarioId}")
            && app.contains("writeCartResume("),
        "LotOpen from cart must return to Shopping Cart and pass resumeScenarioId + writeCartResume"
    );
    let cart_screen = std::fs::read_to_string(
        root.join("apps/desktop/src/features/shopping-cart/ShoppingCartScreen.tsx"),
    )
    .expect("ShoppingCartScreen.tsx");
    let cart_resume = std::fs::read_to_string(
        root.join("apps/desktop/src/features/shopping-cart/cartResume.ts"),
    )
    .expect("cartResume.ts");
    assert!(
        cart_resume.contains("CART_RESUME_KEY") && cart_resume.contains("finos.cart.resume"),
        "cart resume key must persist across Add Lot remount"
    );
    assert!(
        cart_screen.contains("CartScenarioGet")
            && cart_screen.contains("resumingCart")
            && cart_screen.contains("writeCartResume")
            && cart_screen.contains("Returning to in-progress cart"),
        "Shopping Cart remount must CartScenarioGet the in-progress execute scene, not the start wizard"
    );
    assert!(
        cart_screen.contains("confirmedBuySymbols")
            && !cart_screen.contains("index < buyStepsDone"),
        "cart buy Confirmed must match execute-step lot symbol, not buy-line index (MUIB must not mark HAKY)"
    );
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
    assert!(compare.contains("Cash on traded") && compare.contains("Monthly"));
    let screen = std::fs::read_to_string(
        root.join("apps/desktop/src/features/shopping-cart/ShoppingCartScreen.tsx"),
    )
    .unwrap();
    assert!(
        screen.contains("cashYieldBps") && !screen.contains("Cash income uses the"),
        "cash yield stays in the sell table from the collector plan"
    );
    assert!(
        !screen.contains("Cart cash yield bps") && !screen.contains("Cash yield (bps)"),
        "owner must not type a fictitious cash yield"
    );
    assert!(
        screen.contains("CartStartWizard")
            && screen.contains("ariaLabel=\"Sell plan\"")
            && screen.contains("Scenario A")
            && screen.contains("Add scenario B")
            && !screen.contains("Cart funding next step"),
        "start is account and plan name, then the sell table funds the scenarios"
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
            && !wizard.contains("prompt === \"funding\""),
        "account then plan name; funding is the sell table"
    );
    assert!(
        wizard.contains("Next cart step") && wizard.contains("Previous cart step"),
        "wizard next/back"
    );
    let rail = std::fs::read_to_string(
        root.join("apps/desktop/src/features/shopping-cart/StepRail.tsx"),
    )
    .unwrap();
    assert!(
        rail.contains("Account") && rail.contains("Plan name") && !rail.contains("How funded")
    );
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
            "priceScale": 2,
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
            "priceScale": 2,
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
            "priceScale": 2,
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

    let agreed = must_ok(
        &platform,
        "CartScenarioAgree",
        serde_json::json!({ "scenarioId": scenario_id }),
    )
    .await;
    assert_eq!(agreed["status"].as_str(), Some("agreed"));
    let line_id = evaluated["buyLines"][0]["lineId"].as_str().unwrap();
    let fill_short = execute_command_on(
        &platform,
        &platform,
        cmd(
            "CartExecuteFill",
            serde_json::json!({
                "scenarioId": scenario_id,
                "occurredOn": "2026-09-11",
                "fills": [{ "lineId": line_id, "fillMinor": 2_995 }]
            }),
        ),
    )
    .await;
    assert!(!fill_short.ok);
    assert_eq!(
        fill_short.error_code.as_deref(),
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
    let purchase = ledger2["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["activityType"] == "ETF Purchase")
        .expect("account-cash fill posts ETF Purchase");
    assert_eq!(purchase["amountMinor"], 5_990);
    assert_eq!(purchase["scale"], 2);
    let register = query_body(
        &platform,
        "CashRegisterGet",
        serde_json::json!({
            "account": "FI Roth",
            "period": "1M",
            "asOfDate": "2026-09-11"
        }),
    )
    .await;
    let row = register["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["label"] == "ETF Purchase")
        .expect("register lists the purchase");
    assert_eq!(row["transaction"], "Withdrawal");
    assert_eq!(row["source"], "activity");
    assert_eq!(row["withdrawalMinor"], 5_990);
    assert_eq!(row["scale"], 2);
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
            "priceScale": 2,
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

#[tokio::test]
async fn cart_scale4_last_is_dollars_not_ten_thousand() {
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
    let (amdw_id, _) = research_and_open(&platform, account_id, "AMDW", 10, 0, 6_000).await;
    let scene = must_ok(
        &platform,
        "CartScenarioCreate",
        serde_json::json!({
            "accountId": account_id,
            "asOf": "2026-09-29",
            "cashYieldBps": 334
        }),
    )
    .await;
    let scenario_id = scene["scenarioId"].as_str().unwrap();
    let added = must_ok(
        &platform,
        "CartBuyLineAdd",
        serde_json::json!({
            "scenarioId": scenario_id,
            "securityId": amdw_id,
            "qtyWhole": 6,
            "lastMinor": 10_147,
            "priceScale": 2,
            "planAnnualMinor": 17_160
        }),
    )
    .await;
    let line_id = added["buyLines"][0]["lineId"].as_str().unwrap();
    let next = must_ok(
        &platform,
        "CartBuyLineLastSet",
        serde_json::json!({
            "scenarioId": scenario_id,
            "lineId": line_id,
            "lastMinor": 1_014_703,
            "priceScale": 4
        }),
    )
    .await;
    let line = &next["buyLines"][0];
    assert_eq!(line["lastMinor"].as_i64().unwrap(), 1_014_703);
    assert_eq!(line["priceScale"].as_u64().unwrap(), 4);
    assert_eq!(line["spendMinor"].as_i64().unwrap(), 60_882);
}

/// Confirming MUIB must not be treated as confirming HAKY (index-order was the UI bug).
#[tokio::test]
async fn cart_execute_buy_rejects_lot_not_on_buy_list() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "ira"}),
    )
    .await;
    let account_id = account["accountId"].as_str().unwrap();
    let (_, cash_lot) = research_and_open(&platform, account_id, "SPAXX", 100_000, 2, 100_000).await;

    async fn register_named(platform: &LocalPlatform, symbol: &str) -> String {
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
        security_id
    }

    let haky_id = register_named(&platform, "HAKY").await;
    let muib_id = register_named(&platform, "MUIB").await;
    let amdw_id = register_named(&platform, "AMDW").await;

    let scene = must_ok(
        &platform,
        "CartScenarioCreate",
        serde_json::json!({
            "accountId": account_id,
            "asOf": "2026-09-26",
            "cashYieldBps": 349
        }),
    )
    .await;
    let scenario_id = scene["scenarioId"].as_str().unwrap();
    must_ok(
        &platform,
        "CartSellLineAdd",
        serde_json::json!({
            "scenarioId": scenario_id,
            "lotId": cash_lot,
            "qtyMinor": 70_000,
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
            "qtyWhole": 1,
            "lastMinor": 3_181,
            "priceScale": 2,
            "planAnnualMinor": 456
        }),
    )
    .await;
    must_ok(
        &platform,
        "CartBuyLineAdd",
        serde_json::json!({
            "scenarioId": scenario_id,
            "securityId": muib_id,
            "qtyWhole": 20,
            "lastMinor": 3_272,
            "priceScale": 2,
            "planAnnualMinor": 5_520
        }),
    )
    .await;
    must_ok(
        &platform,
        "CartScenarioAgree",
        serde_json::json!({ "scenarioId": scenario_id }),
    )
    .await;
    must_ok(
        &platform,
        "CartExecuteSell",
        serde_json::json!({
            "scenarioId": scenario_id,
            "occurredOn": "2026-09-26"
        }),
    )
    .await;
    let muib_lot = must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account_id,
            "securityId": muib_id,
            "openedOn": "2026-09-26",
            "origin": "purchase",
            "quantityMinor": 20,
            "quantityScale": 0,
            "performanceBasisMinor": 65_440,
            "taxBasisMinor": 65_440,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;
    let muib_lot_id = muib_lot["lotId"].as_str().unwrap();
    let after_muib = must_ok(
        &platform,
        "CartExecuteBuyStep",
        serde_json::json!({
            "scenarioId": scenario_id,
            "lotId": muib_lot_id
        }),
    )
    .await;
    let buy_lots: Vec<&str> = after_muib["executeSteps"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["kind"].as_str() == Some("buy"))
        .filter_map(|s| s["lotId"].as_str())
        .collect();
    assert_eq!(buy_lots, vec![muib_lot_id]);

    let amdw_lot = must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account_id,
            "securityId": amdw_id,
            "openedOn": "2026-09-26",
            "origin": "purchase",
            "quantityMinor": 1,
            "quantityScale": 0,
            "performanceBasisMinor": 100,
            "taxBasisMinor": 100,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;
    let bad = execute_command_on(
        &platform,
        &platform,
        cmd(
            "CartExecuteBuyStep",
            serde_json::json!({
                "scenarioId": scenario_id,
                "lotId": amdw_lot["lotId"].as_str().unwrap()
            }),
        ),
    )
    .await;
    assert!(!bad.ok, "AMDW lot must not attach to a HAKY/MUIB cart");
    assert_eq!(bad.error_code.as_deref(), Some("buy_lot_symbol_mismatch"));
}

/// Sell and buy use stored Plan × frequency. MUIB $0.2300 × 24 at $33.23 is not ×12.
#[test]
fn cart_twice_monthly_yield_is_plan_times_24() {
    let at_24 = financial_domain::cart::plan_fwd_yield_bps(2_300, 4, 24, 3_323);
    let at_12 = financial_domain::cart::plan_fwd_yield_bps(2_300, 4, 12, 3_323);
    assert_eq!(at_24, Some(1_661), "Plan $0.2300 × 24 ÷ $33.23");
    assert_ne!(at_24, at_12);
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let plan = std::fs::read_to_string(
        root.join("apps/desktop/src/features/shopping-cart/cartPlan.ts"),
    )
    .unwrap();
    assert!(
        plan.contains("export function periodsForPlan"),
        "one periods function for sell and buy"
    );
    assert!(
        plan.contains("twice monthly") && plan.contains("return 24"),
        "Twice monthly is 24 before any stored period count"
    );
    let screen = std::fs::read_to_string(
        root.join("apps/desktop/src/features/shopping-cart/ShoppingCartScreen.tsx"),
    )
    .unwrap();
    assert!(
        screen.contains("periodsForPlan(body.paymentFrequency, body.planningPeriodsPerYear)"),
        "InvestmentGet facts and the sell quote must prefer frequency"
    );
    assert!(
        !screen.contains("body.planningPeriodsPerYear || periodsPerYear"),
        "a stale planningPeriodsPerYear of 12 must not win"
    );
    assert!(
        screen.matches("planAnnualForSymbol(").count() >= 2,
        "sell and buy sheets both annualize stored Plan through planAnnualForSymbol"
    );
}

/// Cash align sets the scenario complete. The close summary must still be on screen.
#[test]
fn completed_cart_keeps_the_close_summary_on_screen() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let panel = std::fs::read_to_string(
        root.join("apps/desktop/src/features/shopping-cart/ExecutePlanPanel.tsx"),
    )
    .unwrap();
    assert!(
        panel.contains("{cashReady || cashAligned ? cashBlock : null}"),
        "cashReady is false once cash is aligned — the confirmed block must still render"
    );
    assert!(
        panel.contains("{!cashReady && !cashAligned ? cashBlock : null}"),
        "the pre-confirm copy must not render a second copy of the confirmed block"
    );
    assert!(
        panel.contains("aria-label=\"Completed register summary\"")
            && panel.contains("Net dividend change"),
        "one completion row: account, purchases, net dividend change"
    );
    assert!(
        panel.contains("scenarioStatus === \"complete\"") && panel.contains("readOnly"),
        "a complete cart is read-only — no re-confirming a purchase"
    );
    let screen = std::fs::read_to_string(
        root.join("apps/desktop/src/features/shopping-cart/ShoppingCartScreen.tsx"),
    )
    .unwrap();
    let scene = screen
        .split("const agreedScene")
        .nth(1)
        .expect("agreedScene")
        .split(';')
        .next()
        .expect("agreedScene body");
    assert!(
        scene.contains("item.status === \"complete\""),
        "agreedScene must keep the execute panel mounted after cash aligns: {scene}"
    );
    assert!(
        !screen.contains("scene.status === \"complete\" || scene.status === \"discarded\""),
        "resume must restore a completed cart read-only, not fall back to the sell sheet"
    );
    assert!(
        screen.contains("if (scene.status === \"discarded\")"),
        "only a discarded cart is dropped on resume"
    );
    assert!(
        screen.contains("CartExecutedList")
            && screen.contains("executedCarts={executedCarts}")
            && screen.contains("onOpenExecuted=")
            && screen.contains("openExecutedCart")
            && screen.contains("row.accountId"),
        "opening an executed cart uses that cart's account; the home picker is empty"
    );
    let archive = std::fs::read_to_string(
        root.join("apps/desktop/src/features/shopping-cart/ExecutedCarts.tsx"),
    )
    .expect("ExecutedCarts.tsx");
    for needle in [
        "aria-label=\"Executed carts\"",
        "Sales (non-cash)",
        "Realized P/L",
        "Invested",
        "Monthly income change",
        "onClick={() => onOpen(row)}",
        "aria-label=\"Total of all executed carts\"",
        "className=\"executed-cart-total\"",
        "rows.reduce((sum, row) => sum + row.nonCashSalesMinor, 0)",
        "aria-sort={ariaSort}",
    ] {
        assert!(archive.contains(needle), "executed cart row needs {needle}");
    }
    assert!(
        !archive.contains("Open read-only") && archive.contains("\n                      Open\n"),
        "the executed cart button says Open"
    );
    assert!(
        archive.contains("useState<{ key: SortKey; dir: \"asc\" | \"desc\" } | null>(null)"),
        "the register arrives newest first and only reorders when a column header is clicked"
    );
    let css = std::fs::read_to_string(root.join("apps/desktop/src/App.css")).unwrap();
    let total_row = css
        .split(".executed-cart-total th,")
        .nth(1)
        .expect("executed cart total style")
        .split(".plan-basis")
        .next()
        .unwrap();
    assert!(
        total_row.contains("font-weight: 700;")
            && total_row.contains("border-top: 1px solid")
            && total_row.contains("border-bottom: 1px solid")
            && total_row.contains("border-left: 1px solid")
            && total_row.contains("border-right: 1px solid")
            && total_row.contains("background: color-mix"),
        "the total row is outlined, lightly filled, and bold"
    );
    let wizard = std::fs::read_to_string(
        root.join("apps/desktop/src/features/shopping-cart/CartStartWizard.tsx"),
    )
    .unwrap();
    // Owner, 2026-10-03, restated: the account picker and Next are how a cart starts. The
    // executed register is a different screen, every account, and it sits below Next. Putting
    // the register between the picker and Next made Next look like part of the register.
    let picker = wizard
        .split("{prompt === \"account\" ? (")
        .nth(1)
        .expect("account picker")
        .split("{prompt === \"plans\" ? (")
        .next()
        .expect("end of account picker");
    assert!(
        picker.contains("<AccountSelect") && !picker.contains("<ExecutedCarts"),
        "the account picker stands alone; the executed register is not inside it: {picker}"
    );
    let next_at = wizard
        .find("aria-label=\"Next cart step\"")
        .expect("Next cart step");
    let executed_at = wizard.find("<ExecutedCarts").expect("executed register");
    assert!(
        next_at < executed_at,
        "Next sits above the executed register"
    );
    let build = wizard
        .split("{prompt === \"plans\" ? (")
        .nth(1)
        .expect("plans prompt block")
        .split("aria-label=\"Next cart step\"")
        .next()
        .expect("plans block ends before Next");
    assert!(
        !build.contains("<ExecutedCarts"),
        "completed carts must not render on the open-carts step"
    );
    assert!(
        wizard.contains("Open carts for"),
        "open carts stay per account on the step after the picker"
    );
}

/// The close summary on a completed cart must report the delta the owner agreed to, read off
/// the stored evaluation. It used to re-derive it from the live calculator: on a reopened cart
/// `annualForQty` returned null for every buy, the `null ? sum` skip collapsed the buy term to
/// zero, and what was left was cash interest on the dollars spent. The Income cart's Done step
/// read -$31.36 while its own snapshot, and the executed register, said +$7.13.
#[test]
fn completed_cart_reports_the_agreed_delta_not_a_fresh_derivation() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let screen = std::fs::read_to_string(
        root.join("apps/desktop/src/features/shopping-cart/ShoppingCartScreen.tsx"),
    )
    .unwrap();
    assert!(
        screen.contains("const netDividendMinor = scenarioA?.eval?.netMonthlyMinor ?? null"),
        "the close summary reads the stored monthly delta, nothing else"
    );
    for gone in [
        "const leftoverAnnual = Math.round",
        "const baselineAnnual = Math.round",
        "return buyAnnual - sellAnnual + leftoverAnnual - baselineAnnual",
    ] {
        assert!(
            !screen.contains(gone),
            "the live re-derivation of the agreed delta must stay deleted: {gone}"
        );
    }
    // The detail the decision was made on, from the same snapshot. IncomeCompare existed and
    // was never mounted, so closing a cart erased the week/month/year comparison from view.
    assert!(
        screen.contains("<IncomeCompare eval={scenarioA.eval ?? null} />"),
        "an agreed or completed cart shows the stored income comparison"
    );
    let compare = std::fs::read_to_string(
        root.join("apps/desktop/src/features/shopping-cart/IncomeCompare.tsx"),
    )
    .expect("IncomeCompare.tsx");
    assert!(
        !compare.contains("annualForQty") && !compare.contains("calculator"),
        "the stored comparison must not recompute from live positions"
    );
}

/// Once a scenario loaded, the wizard stopped rendering and there was no way back to the cart
/// list. Clearing the resume token is the part that cannot be skipped: without it the resume
/// effect reopens the same cart and leaving looks broken.
#[test]
fn an_open_cart_has_a_way_back_to_the_cart_home_screen() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let screen = std::fs::read_to_string(
        root.join("apps/desktop/src/features/shopping-cart/ShoppingCartScreen.tsx"),
    )
    .unwrap();
    assert!(
        screen.contains("aria-label=\"Shopping Cart home\""),
        "an open, new or completed cart needs a control back to the cart home screen"
    );
    let home = screen
        .split("const returnToCartHome")
        .nth(1)
        .expect("returnToCartHome")
        .split("};")
        .next()
        .expect("returnToCartHome body");
    for needle in [
        "clearCartResume()",
        "setWizardPrompt(\"account\")",
        "setScenarioA(null)",
        "setScenarioB(null)",
    ] {
        assert!(
            home.contains(needle),
            "leaving a cart must {needle}, or the resume effect drags it back: {home}"
        );
    }
}

/// Archive row: actual sales, actual realized P/L, actual dollars invested, and a
/// Plan-only monthly income delta. A new declaration must not move the income column.
#[tokio::test]
async fn executed_cart_archive_uses_actual_dollars_and_plan_income() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "ira"}),
    )
    .await;
    let account_id = account["accountId"].as_str().unwrap();

    // $5,000 money market: the align baseline and the plan rate on leftover cash.
    let (spaxx_id, cash_lot) =
        research_and_open(&platform, account_id, "SPAXX", 500_000, 2, 500_000).await;
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

    // HAKY: 100 shares at $30 cost, Plan $0.38 monthly → $456/yr surrendered.
    let (haky_id, haky_lot) =
        research_and_open(&platform, account_id, "HAKY", 100, 0, 300_000).await;
    must_ok(
        &platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": haky_id,
            "amountPerShareMinor": 3_800,
            "amountScale": 4,
            "paymentPeriod": "2026-08-31",
            "source": "issuer",
            "enteredAt": "2026-08-31"
        }),
    )
    .await;
    must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": haky_id,
            "amountPerShareMinor": 3_800,
            "amountScale": 4,
            "planningPeriodsPerYear": 12,
            "effectiveFrom": "2026-09-01",
            "decisionReason": "collector",
            "incompleteAnalysisReason": "Fewer than 6 observations"
        }),
    )
    .await;

    // MUIB: twice monthly, Plan $0.2300 × 24 → $110.40/yr on 20 shares.
    let muib = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "MUIB", "name": "MUIB"}),
    )
    .await;
    let muib_id = muib["securityId"].as_str().unwrap();
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": muib_id,
            "priceSource": "public",
            "sourceSymbol": "MUIB",
            "declarationSource": "issuer",
            "sourceUrl": "https://example.test/muib/distributions",
            "calendarPolicy": "derived_walk",
            "collectorEnabled": true,
            "lookbackCount": 12
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot_as(
        &platform,
        muib_id,
        "MUIB",
        "Twice monthly",
        true,
    )
    .await
    .expect("complete collector");
    must_ok(
        &platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": muib_id,
            "amountPerShareMinor": 2_300,
            "amountScale": 4,
            "paymentPeriod": "2026-09-16",
            "source": "issuer",
            "enteredAt": "2026-09-16"
        }),
    )
    .await;
    must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": muib_id,
            "amountPerShareMinor": 2_300,
            "amountScale": 4,
            "planningPeriodsPerYear": 24,
            "effectiveFrom": "2026-09-01",
            "decisionReason": "Conservative vs Most Current",
            "incompleteAnalysisReason": "Recent inception date"
        }),
    )
    .await;

    let scene = must_ok(
        &platform,
        "CartScenarioCreate",
        serde_json::json!({
            "accountId": account_id,
            "asOf": "2026-09-26",
            "name": "Income - 9/26/26"
        }),
    )
    .await;
    let scenario_id = scene["scenarioId"].as_str().unwrap();
    must_ok(
        &platform,
        "CartSellLineAdd",
        serde_json::json!({
            "scenarioId": scenario_id,
            "lotId": cash_lot,
            "qtyMinor": 100_000,
            "unitMinor": 100,
            "isCash": true
        }),
    )
    .await;
    let with_sell = must_ok(
        &platform,
        "CartSellLineAdd",
        serde_json::json!({
            "scenarioId": scenario_id,
            "lotId": haky_lot,
            "qtyMinor": 100,
            "qtyScale": 0,
            "unitMinor": 3_200
        }),
    )
    .await;
    let haky_line = with_sell["sellLines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["symbol"] == "HAKY")
        .expect("HAKY sell line");
    assert_eq!(haky_line["proceedsMinor"].as_i64().unwrap(), 320_000);
    assert_eq!(haky_line["performanceGainMinor"].as_i64().unwrap(), 20_000);
    must_ok(
        &platform,
        "CartBuyLineAdd",
        serde_json::json!({
            "scenarioId": scenario_id,
            "securityId": muib_id,
            "qtyWhole": 20,
            "lastMinor": 3_272,
            "priceScale": 2,
            "planAnnualMinor": 11_040
        }),
    )
    .await;
    let pile = query_body(
        &platform,
        "CashPileGet",
        serde_json::json!({ "accountId": account_id }),
    )
    .await;
    assert_eq!(
        pile["dollarsMinor"].as_i64().unwrap_or(0),
        500_000,
        "the $5,000 money market is the align baseline: {pile}"
    );
    must_ok(
        &platform,
        "CartScenarioAgree",
        serde_json::json!({ "scenarioId": scenario_id }),
    )
    .await;
    let posted = must_ok(
        &platform,
        "CartExecuteSell",
        serde_json::json!({ "scenarioId": scenario_id, "occurredOn": "2026-09-26" }),
    )
    .await;
    assert_eq!(
        posted["executeCashBaselineMinor"].as_i64().unwrap_or(0),
        500_000,
        "the scenario body must carry the baseline or Confirm cash has no math: {posted}"
    );
    // Actual fill is $660, not the $654.40 plan.
    let muib_lot = must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account_id,
            "securityId": muib_id,
            "openedOn": "2026-09-26",
            "origin": "purchase",
            "quantityMinor": 20,
            "quantityScale": 0,
            "performanceBasisMinor": 66_000,
            "taxBasisMinor": 66_000,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "CartExecuteBuyStep",
        serde_json::json!({
            "scenarioId": scenario_id,
            "lotId": muib_lot["lotId"].as_str().unwrap()
        }),
    )
    .await;
    let closed = must_ok(
        &platform,
        "CartExecuteCashAlign",
        serde_json::json!({ "scenarioId": scenario_id, "occurredOn": "2026-09-26" }),
    )
    .await;
    assert_eq!(closed["status"], "complete");

    let archive = query_body(
        &platform,
        "CartExecutedList",
        serde_json::json!({ "accountId": account_id }),
    )
    .await;
    let rows = archive["items"].as_array().expect("archive items");
    assert_eq!(rows.len(), 1, "one executed cart: {archive}");
    let row = &rows[0];
    assert_eq!(row["accountName"], "Income");
    assert_eq!(row["name"], "Income - 9/26/26");
    assert_eq!(row["nonCashSalesMinor"].as_i64().unwrap(), 320_000);
    assert_eq!(row["realizedPlMinor"].as_i64().unwrap(), 20_000);
    assert_eq!(
        row["investedMinor"].as_i64().unwrap(),
        66_000,
        "invested is the actual lot cost, not the planned spend: {row}"
    );
    assert_eq!(row["cashBaselineMinor"].as_i64().unwrap(), 500_000);
    assert_eq!(row["cashTargetMinor"].as_i64().unwrap(), 754_000);

    // The executed row reports the delta the agreed scenario calculated, not a fresh
    // derivation. Re-deriving it here disagreed with the comparison screen the owner approved
    // and let a later Plan re-confirm rewrite history.
    let agreed = query_body(
        &platform,
        "CartScenarioGet",
        serde_json::json!({ "scenarioId": scenario_id }),
    )
    .await;
    let snapshot_annual = agreed["eval"]["netAnnualMinor"]
        .as_i64()
        .expect("the agreed scenario stored an income delta");
    let snapshot_monthly = agreed["eval"]["netMonthlyMinor"]
        .as_i64()
        .expect("the agreed scenario stored a monthly income delta");
    assert_eq!(
        row["deltaAnnualIncomeMinor"].as_i64().unwrap(),
        snapshot_annual,
        "executed row must report the agreed scenario's own delta: {row}"
    );
    assert_eq!(
        row["deltaMonthlyIncomeMinor"].as_i64().unwrap(),
        snapshot_monthly,
        "executed row must report the agreed scenario's own monthly delta: {row}"
    );
    // Not just equal to itself: MUIB Plan $110.40 a year less $21.85 of cash interest given up
    // on the $654.40 spent, at this account's 334 bps.
    //
    // Note what this figure does not contain: `evaluate_swap` surrenders cash interest only, so
    // the $456.00 a year the sold HAKY shares were paying is absent and the swap reads as a
    // gain. That is the same blind spot that made the Car cart's TSLW sale read +$167.02 a year.
    // Reporting the agreed snapshot is the owner's instruction; whether the snapshot should
    // subtract the income of positions sold is an open owner decision in the ledger.
    assert_eq!(snapshot_annual, 8_855, "agreed delta: {agreed}");
    assert_eq!(snapshot_monthly, 737, "agreed monthly delta: {agreed}");

    must_ok(
        &platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": haky_id,
            "amountPerShareMinor": 90_000,
            "amountScale": 5,
            "paymentPeriod": "2026-09-30",
            "source": "issuer",
            "enteredAt": "2026-09-30"
        }),
    )
    .await;
    let after_declaration = query_body(
        &platform,
        "CartExecutedList",
        serde_json::json!({ "accountId": account_id }),
    )
    .await;
    assert_eq!(
        after_declaration["items"][0]["deltaAnnualIncomeMinor"]
            .as_i64()
            .unwrap(),
        snapshot_annual,
        "a $0.90 declaration must not move a Plan-only column: {after_declaration}"
    );

    must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": haky_id,
            "amountPerShareMinor": 1_900,
            "amountScale": 4,
            "planningPeriodsPerYear": 12,
            "effectiveFrom": "2026-10-02",
            "decisionReason": "owner re-confirm",
            "incompleteAnalysisReason": "Fewer than 6 observations"
        }),
    )
    .await;
    let after_plan = query_body(
        &platform,
        "CartExecutedList",
        serde_json::json!({ "accountId": account_id }),
    )
    .await;
    // Owner decision, 2026-10-03: an executed cart is the record of what was agreed, so a Plan
    // re-confirmed afterwards must not move it. This assertion used to require the opposite
    // (-3_277), which is how the Car cart came to show a loss against a plan agreed as a gain.
    assert_eq!(
        after_plan["items"][0]["deltaAnnualIncomeMinor"]
            .as_i64()
            .unwrap(),
        snapshot_annual,
        "a Plan re-confirmed after execution must not rewrite the agreed delta: {after_plan}"
    );

    let open_carts = query_body(
        &platform,
        "CartScenarioList",
        serde_json::json!({ "accountId": account_id }),
    )
    .await;
    assert!(
        open_carts["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["status"] == "complete"),
        "the executed cart stays readable through CartScenarioGet/List: {open_carts}"
    );
}

/// Executing one slot of a plan must not leave the other slot an open draft. The owner saw
/// "Income - 9/26/26" in the saved list and the executed list at once: slot A was complete and
/// slot B, the comparison copy, stayed a draft forever.
#[tokio::test]
async fn executing_a_plan_leaves_no_sibling_slot_open() {
    let dir = golden_harness::profile_a_app_dir();
    if !dir.join("local.sqlite").is_file() {
        return;
    }
    let platform = LocalPlatform::open(&dir).await.expect("open live sqlite");
    let rows = live_accounts(&platform).await;
    let mut seen_plans = 0_usize;
    let mut straddling: Vec<String> = Vec::new();
    for account in rows {
        let Some(account_id) = account["accountId"].as_str() else {
            continue;
        };
        let listed = query_body(
            &platform,
            "CartScenarioList",
            serde_json::json!({ "accountId": account_id }),
        )
        .await;
        let items = listed["items"].as_array().cloned().unwrap_or_default();
        for item in &items {
            eprintln!(
                "  {} plan={:?} slot={:?} status={:?}",
                account["name"].as_str().unwrap_or("?"),
                item["planId"].as_str().map(|s| &s[..8.min(s.len())]),
                item["slot"].as_str(),
                item["status"].as_str()
            );
        }
        for item in &items {
            let Some(plan_id) = item["planId"].as_str() else {
                continue;
            };
            seen_plans += 1;
            let siblings: Vec<&serde_json::Value> = items
                .iter()
                .filter(|s| s["planId"].as_str() == Some(plan_id))
                .collect();
            let any_complete = siblings.iter().any(|s| s["status"] == "complete");
            // `superseded` is closed: execution sets it on the sibling slots, and migration
            // 0064 healed the rows written before that existed.
            let open: Vec<String> = siblings
                .iter()
                .filter(|s| s["status"] != "complete" && s["status"] != "superseded")
                .map(|s| {
                    format!(
                        "slot {} {}",
                        s["slot"].as_str().unwrap_or("?"),
                        s["status"].as_str().unwrap_or("?")
                    )
                })
                .collect();
            if any_complete && !open.is_empty() {
                let note = format!(
                    "{} (plan {}): executed, but {} still open",
                    item["name"].as_str().unwrap_or("cart"),
                    &plan_id[..8],
                    open.join(", ")
                );
                if !straddling.contains(&note) {
                    straddling.push(note);
                }
            }
        }
    }
    assert!(
        straddling.is_empty(),
        "a plan cannot be executed and open at the same time; the owner sees the same cart in \
         both lists:\n  {}",
        straddling.join("\n  ")
    );
    // An empty account list used to make this pass while checking nothing.
    assert!(seen_plans > 0, "no cart scenarios were read, so this guard proved nothing");
}

/// The Shopping Cart home screen asks for completed carts without an account, because it sits
/// above the account picker. The all-accounts answer must be exactly the per-account answers put
/// together, and every row must report the delta its own scenario stored — the two figures the
/// product shows for one cart, checked against each other rather than against my arithmetic.
#[tokio::test]
async fn executed_list_without_an_account_spans_every_account() {
    let dir = golden_harness::profile_a_app_dir();
    if !dir.join("local.sqlite").is_file() {
        return;
    }
    let platform = LocalPlatform::open(&dir).await.expect("open live sqlite");

    let everywhere = query_body(&platform, "CartExecutedList", serde_json::json!({})).await;
    let all_rows = everywhere["items"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let mut per_account = 0_usize;
    for account in live_accounts(&platform).await {
        let Some(account_id) = account["accountId"].as_str() else {
            continue;
        };
        let one = query_body(
            &platform,
            "CartExecutedList",
            serde_json::json!({ "accountId": account_id }),
        )
        .await;
        for row in one["items"].as_array().cloned().unwrap_or_default() {
            per_account += 1;
            assert!(
                all_rows
                    .iter()
                    .any(|r| r["scenarioId"] == row["scenarioId"]),
                "{} is in the {} register but missing from the home screen: {everywhere}",
                row["name"],
                account["name"]
            );
        }
    }
    assert_eq!(
        all_rows.len(),
        per_account,
        "the home register is every account's completed carts and nothing else: {everywhere}"
    );
    assert!(
        per_account > 0,
        "no completed carts were read, so this guard proved nothing"
    );

    for row in &all_rows {
        let scene = query_body(
            &platform,
            "CartScenarioGet",
            serde_json::json!({ "scenarioId": row["scenarioId"].as_str().unwrap_or_default() }),
        )
        .await;
        eprintln!(
            "  {} {}: row monthly {:?}, agreed monthly {:?}",
            row["accountName"].as_str().unwrap_or("?"),
            row["name"].as_str().unwrap_or("?"),
            row["deltaMonthlyIncomeMinor"].as_i64(),
            scene["eval"]["netMonthlyMinor"].as_i64()
        );
        assert_eq!(
            row["deltaMonthlyIncomeMinor"].as_i64(),
            scene["eval"]["netMonthlyMinor"].as_i64(),
            "the executed register and the cart's own agreed delta must not disagree: {} {}",
            row["accountName"],
            row["name"]
        );
        assert_eq!(
            row["deltaAnnualIncomeMinor"].as_i64(),
            scene["eval"]["netAnnualMinor"].as_i64(),
            "the executed register and the cart's own agreed annual delta must not disagree: {} {}",
            row["accountName"],
            row["name"]
        );
    }
}

/// Scenario A and B offer the Calculator performance views. A row fills the symbol only.
#[test]
fn scenario_dropdown_opens_calculator_filter_views() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sheet = std::fs::read_to_string(root.join("packages/ui-components/src/index.tsx")).unwrap();
    assert!(
        sheet.contains("export function matchesCalculatorPerformanceView"),
        "the cart list and the Calculator sheet share one membership test"
    );
    assert!(
        sheet.contains(
            "matchesCalculatorPerformanceView(row, masterBySymbol.get(row.symbol), performance, cadence)"
        ),
        "the Calculator sheet must call the shared test"
    );
    assert!(
        sheet.contains("P/L ≥ 0%, plan FWD yield, green plan check")
            && sheet.contains("P/L ≥ 0%, plan FWD yield, green plan check, 90% ROC"),
        "both performance view names"
    );
    let screen = std::fs::read_to_string(
        root.join("apps/desktop/src/features/shopping-cart/ShoppingCartScreen.tsx"),
    )
    .unwrap();
    assert!(
        screen.contains("aria-label={`Scenario ${slot} symbol`}")
            && screen.contains("CALCULATOR_PERFORMANCE_VIEWS.map((view) => ("),
        "the scenario symbol dropdown lists both views"
    );
    assert!(
        screen.contains("setFilterCadence(\"all\")"),
        "the popup frequency defaults to All"
    );
    for column in ["FWD", "MC FWD", "YOC", "Gain %"] {
        assert!(
            screen.contains(&format!("<th scope=\"col\">{column}</th>")),
            "popup column {column}"
        );
    }
    let choose = screen
        .split("onForm({ ...form, securityId: choice.securityId })")
        .nth(1)
        .expect("a filter row writes the scenario symbol");
    let handler = choose.split('}').next().expect("row handler");
    assert!(
        !handler.contains("onAdd"),
        "choosing a filter row must not add a cart line"
    );
    assert!(
        screen.contains("matchesCalculatorPerformanceView"),
        "the popup uses the shared membership test"
    );
}

/// $32.72 at scale 4 is minor 327200. Ten shares spend 32720 cents, and that fill fits a pile of $400.
#[tokio::test]
async fn scale4_fill_spends_cents_not_the_raw_minor() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let account_id = account["accountId"].as_str().unwrap();
    research_and_open(&platform, account_id, "SPAXX", 40_000, 2, 40_000).await;
    let (security_id, _) = research_and_open(&platform, account_id, "BUY4", 1, 0, 100).await;
    let scene = must_ok(
        &platform,
        "CartScenarioCreate",
        serde_json::json!({
            "accountId": account_id,
            "asOf": "2026-10-05",
            "name": "scale 4 fill",
            "fundingSource": "accountCash"
        }),
    )
    .await;
    let scenario_id = scene["scenarioId"].as_str().unwrap();
    let added = must_ok(
        &platform,
        "CartBuyLineAdd",
        serde_json::json!({
            "scenarioId": scenario_id,
            "securityId": security_id,
            "qtyWhole": 10,
            "lastMinor": 327_200,
            "priceScale": 4,
            "planAnnualMinor": 1_000
        }),
    )
    .await;
    let line = &added["buyLines"][0];
    assert_eq!(line["lastMinor"].as_i64().unwrap(), 327_200);
    assert_ne!(line["lastMinor"].as_i64().unwrap(), 3_272);
    assert_eq!(line["priceScale"].as_u64().unwrap(), 4);
    assert_eq!(line["spendMinor"].as_i64().unwrap(), 32_720);
    must_ok(
        &platform,
        "CartScenarioAgree",
        serde_json::json!({
            "scenarioId": scenario_id,
            "overrideReason": "pile covers the scale-4 fill"
        }),
    )
    .await;
    let line_id = line["lineId"].as_str().unwrap();
    let filled = must_ok(
        &platform,
        "CartExecuteFill",
        serde_json::json!({
            "scenarioId": scenario_id,
            "occurredOn": "2026-10-05",
            "fills": [{ "lineId": line_id, "fillMinor": 327_200 }]
        }),
    )
    .await;
    assert_eq!(filled["buyLines"][0]["spendMinor"].as_i64().unwrap(), 32_720);
    let blank = execute_command_on(
        &platform,
        &platform,
        cmd(
            "CartBuyLineAdd",
            serde_json::json!({
                "scenarioId": scenario_id,
                "securityId": security_id,
                "qtyWhole": 1,
                "lastMinor": 0,
                "priceScale": 4
            }),
        ),
    )
    .await;
    assert!(!blank.ok);
    assert_eq!(blank.error_code.as_deref(), Some("last_unknown"));
}

/// A missing add-lot scale stays blank. Scale 2 of minor 3272 is $32.72, not $0.3272.
#[test]
fn add_lot_uses_the_stored_price_scale() {
    let root = golden_harness::repo_root();
    let app = std::fs::read_to_string(root.join("apps/desktop/src/App.tsx")).unwrap();
    let price = std::fs::read_to_string(
        root.join("apps/desktop/src/features/shopping-cart/cartPrice.ts"),
    )
    .unwrap();
    assert!(
        !app.contains("priceScale ?? 4"),
        "a missing scale must not be invented as 4"
    );
    assert!(
        app.contains("prefill.priceScale == null")
            && app.contains("cartPriceInput(prefill.lastMinor, prefill.priceScale)"),
        "add lot uses the stored scale and leaves a missing scale blank"
    );
    assert!(
        price.contains("const s = Math.max(0, Math.trunc(scale));"),
        "the price text uses the scale it was given"
    );
    assert_eq!(cart_price_text(3_272, 2), "32.72");
    assert_eq!(cart_price_text(3_272, 4), "0.3272");
}

fn cart_price_text(minor: i64, scale: u32) -> String {
    let digits = minor.unsigned_abs().to_string();
    let width = (scale as usize) + 1;
    let digits = if digits.len() < width {
        format!("{:0>width$}", digits, width = width)
    } else {
        digits
    };
    let split = digits.len() - scale as usize;
    let whole = &digits[..split];
    let frac = &digits[split..];
    if scale == 0 {
        whole.to_string()
    } else {
        format!("{whole}.{frac}")
    }
}

/// Accounts come back as a bare JSON array, not `{items:[…]}`. Getting that wrong made a guard
/// pass while reading nothing, which is worse than a red test.
async fn live_accounts(platform: &LocalPlatform) -> Vec<serde_json::Value> {
    let accounts = query_body(platform, "AccountList", serde_json::json!({})).await;
    let rows = accounts
        .as_array()
        .or_else(|| accounts["items"].as_array())
        .or_else(|| accounts["accounts"].as_array())
        .cloned()
        .unwrap_or_default();
    assert!(!rows.is_empty(), "AccountList returned nothing: {accounts}");
    rows
}
