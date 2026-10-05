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

async fn must_ok(platform: &LocalPlatform, name: &str, body: serde_json::Value) -> serde_json::Value {
    let result = execute_command_on(platform, platform, cmd(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

async fn query_body(platform: &LocalPlatform, name: &str, body: serde_json::Value) -> serde_json::Value {
    let result = execute_query_on(platform, platform, qry_body(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

async fn query_json(platform: &LocalPlatform, name: &str) -> serde_json::Value {
    let result = execute_query_on(platform, platform, qry(name)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("[]")).unwrap()
}

fn ira_rows(activities: &serde_json::Value) -> Vec<&serde_json::Value> {
    activities
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["activityType"] == "ira-contribution")
        .collect()
}

async fn open_cash(
    platform: &LocalPlatform,
    account_id: &str,
    symbol: &str,
    qty_minor: i64,
) {
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
    must_ok(
        platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "openedOn": "2026-09-01",
            "origin": "purchase",
            "quantityMinor": qty_minor,
            "quantityScale": 2,
            "performanceBasisMinor": qty_minor,
            "taxBasisMinor": qty_minor,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;
}

async fn register_account(platform: &LocalPlatform, name: &str) -> String {
    let account = must_ok(
        platform,
        "AccountRegister",
        serde_json::json!({"name": name, "kind": "ira"}),
    )
    .await;
    account["accountId"].as_str().unwrap().to_string()
}

async fn sync_cliff(platform: &LocalPlatform, overage: i64) -> String {
    let synced = must_ok(
        platform,
        "MagiCliffTaskSync",
        serde_json::json!({
            "asOfDate": "2026-10-04",
            "overageMinor": overage,
            "creditAtRiskMinor": 1805400,
            "suggestions": [
                "Cut remaining Traditional IRA draws by the gap.",
                "Or book a Traditional IRA contribution of the gap."
            ]
        }),
    )
    .await;
    synced["task"]["taskId"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn g1_opening_lists_both_lines_and_inserts_no_activity() {
    let panel = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/features/task-manager/MagiCliffPanel.tsx"),
    )
    .unwrap();
    assert!(
        panel.contains("MAGI over cliff")
            && panel.contains("formatUsd(overageMinor)")
            && panel.contains(">IRA Contribution<")
            && panel.contains("size={8}")
            && panel.contains(">Cut remaining IRA draws<")
            && panel.contains("aria-label=\"Cut remaining Traditional IRA draws\"")
            && !panel.contains("Or both"),
        "the title shows the gap once, then one row per option"
    );
    assert_eq!(
        panel.matches("executeCommand(\"").count(),
        1,
        "opening the ticket does not post; confirm is the only command"
    );
    assert!(panel.contains("MagiIraContributionPost"));
    assert!(panel.contains("executeQuery(\"TaskList\")"));

    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let before = query_json(&platform, "ActivityList").await;
    assert!(before.as_array().unwrap().is_empty());
    let task_id = sync_cliff(&platform, 250_000).await;
    let after = query_json(&platform, "ActivityList").await;
    assert!(
        ira_rows(&after).is_empty(),
        "syncing the cliff task inserts no activity"
    );
    let tasks = query_json(&platform, "TaskList").await;
    let task = tasks["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["taskId"] == task_id)
        .unwrap();
    let payload: serde_json::Value = serde_json::from_str(task["payloadJson"].as_str().unwrap()).unwrap();
    let lines = payload["suggestions"].as_array().unwrap();
    assert!(lines.iter().any(|line| line.as_str().unwrap().starts_with("Cut remaining")));
    assert!(lines.iter().any(|line| line.as_str().unwrap().starts_with("Or book")));
}

#[tokio::test]
async fn g2_g3_g6_income_confirm_increases_spaxx_once_and_closes() {
    let register = std::fs::read_to_string(
        golden_harness::repo_root().join("crates/application-core/src/cash_register.rs"),
    )
    .unwrap();
    let chart = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/features/cash/AccountCashFlow.tsx"),
    )
    .unwrap();
    assert!(
        register.contains("ira-contribution") && register.contains("\"Deposit\""),
        "the cash-flow register lists the contribution as a deposit mark"
    );
    assert!(
        chart.contains("day > start"),
        "a contribution on as-of stays inside the lot and is not added again"
    );

    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let income = register_account(&platform, "Income").await;
    open_cash(&platform, &income, "SPAXX", 100_000).await;
    let task_id = sync_cliff(&platform, 25_000).await;
    let amount = 25_000_i64;

    let posted = must_ok(
        &platform,
        "MagiIraContributionPost",
        serde_json::json!({
            "taskId": task_id,
            "accountName": "Income",
            "amountMinor": amount,
            "occurredOn": "2026-10-04"
        }),
    )
    .await;
    assert_eq!(posted["status"], "done", "G6 confirm sets the task done");

    let pile = query_body(
        &platform,
        "CashPileGet",
        serde_json::json!({ "accountId": income }),
    )
    .await;
    assert_eq!(pile["symbol"], "SPAXX");
    assert_eq!(pile["dollarsMinor"].as_i64().unwrap(), 100_000 + amount, "G2 lot");

    let activities = query_json(&platform, "ActivityList").await;
    let rows = ira_rows(&activities);
    assert_eq!(rows.len(), 1, "G2 one activity");
    assert_eq!(rows[0]["amountMinor"].as_i64().unwrap(), amount);
    assert_eq!(rows[0]["scale"].as_i64().unwrap(), 2);
    assert_eq!(
        rows[0]["idempotencyKey"].as_str().unwrap(),
        format!("magi-ira:{task_id}")
    );

    let plan = query_body(
        &platform,
        "TaxPlanningGet",
        serde_json::json!({ "asOfDate": "2026-10-04" }),
    )
    .await;
    assert_eq!(
        plan["iraContributionMinor"].as_i64().unwrap(),
        amount,
        "G2 tax planning contribution matches the lot increase"
    );

    let cash = query_body(
        &platform,
        "CashRegisterGet",
        serde_json::json!({
            "account": "Income",
            "period": "1M",
            "asOfDate": "2026-10-04"
        }),
    )
    .await;
    let hit = cash["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["label"] == "ira-contribution")
        .expect("G2 cash flow lists the contribution");
    assert_eq!(hit["transaction"], "Deposit");
    assert_eq!(hit["depositMinor"].as_i64().unwrap(), amount);
    assert_eq!(hit["source"], "activity");

    let again = must_ok(
        &platform,
        "MagiIraContributionPost",
        serde_json::json!({
            "taskId": task_id,
            "accountName": "Income",
            "amountMinor": amount,
            "occurredOn": "2026-10-04"
        }),
    )
    .await;
    assert_eq!(again["status"], "done", "G6 stays done");
    let pile_again = query_body(
        &platform,
        "CashPileGet",
        serde_json::json!({ "accountId": income }),
    )
    .await;
    assert_eq!(pile_again["dollarsMinor"].as_i64().unwrap(), 100_000 + amount, "G3 lot");
    let activities_again = query_json(&platform, "ActivityList").await;
    assert_eq!(ira_rows(&activities_again).len(), 1, "G3 activity");

    let prior = must_ok(
        &platform,
        "TaskAdd",
        serde_json::json!({
            "title": "Prior year contribution",
            "dueOn": "2025-12-15",
            "note": ""
        }),
    )
    .await;
    let prior_task = prior["taskId"].as_str().unwrap();
    must_ok(
        &platform,
        "MagiIraContributionPost",
        serde_json::json!({
            "taskId": prior_task,
            "accountName": "Income",
            "amountMinor": 1_000,
            "occurredOn": "2025-12-15"
        }),
    )
    .await;
    let plan_year = query_body(
        &platform,
        "TaxPlanningGet",
        serde_json::json!({ "asOfDate": "2026-10-04" }),
    )
    .await;
    assert_eq!(
        plan_year["iraContributionMinor"].as_i64().unwrap(),
        amount,
        "a prior-year contribution stays out of this as-of year"
    );
    let pile_both = query_body(
        &platform,
        "CashPileGet",
        serde_json::json!({ "accountId": income }),
    )
    .await;
    assert_eq!(pile_both["dollarsMinor"].as_i64().unwrap(), 100_000 + amount + 1_000);
}

#[tokio::test]
async fn g4_speculation_and_account_9_accepted_roth_health_car_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let speculation = register_account(&platform, "Speculation").await;
    let account_9 = register_account(&platform, "Account 9").await;
    register_account(&platform, "FI Roth").await;
    register_account(&platform, "Health").await;
    register_account(&platform, "Car").await;
    open_cash(&platform, &speculation, "SPAXX", 50_000).await;
    open_cash(&platform, &account_9, "SWVXX", 40_000).await;

    let spec_task = sync_cliff(&platform, 8_000).await;
    must_ok(
        &platform,
        "MagiIraContributionPost",
        serde_json::json!({
            "taskId": spec_task,
            "accountName": "Speculation",
            "amountMinor": 8_000,
            "occurredOn": "2026-10-04"
        }),
    )
    .await;
    let spec_pile = query_body(
        &platform,
        "CashPileGet",
        serde_json::json!({ "accountId": speculation }),
    )
    .await;
    assert_eq!(spec_pile["symbol"], "SPAXX");
    assert_eq!(spec_pile["dollarsMinor"].as_i64().unwrap(), 58_000);

    let nine_task = {
        let synced = must_ok(
            &platform,
            "TaskAdd",
            serde_json::json!({
                "title": "Account 9 contribution",
                "dueOn": "2026-10-04",
                "note": ""
            }),
        )
        .await;
        synced["taskId"].as_str().unwrap().to_string()
    };
    must_ok(
        &platform,
        "MagiIraContributionPost",
        serde_json::json!({
            "taskId": nine_task,
            "accountName": "Account 9",
            "amountMinor": 4_000,
            "occurredOn": "2026-10-04"
        }),
    )
    .await;
    let nine_pile = query_body(
        &platform,
        "CashPileGet",
        serde_json::json!({ "accountId": account_9 }),
    )
    .await;
    assert_eq!(nine_pile["symbol"], "SWVXX");
    assert_eq!(nine_pile["dollarsMinor"].as_i64().unwrap(), 44_000);

    for name in ["FI Roth", "Health", "Car"] {
        let rejected = execute_command_on(
            &platform,
            &platform,
            cmd(
                "MagiIraContributionPost",
                serde_json::json!({
                    "taskId": spec_task,
                    "accountName": name,
                    "amountMinor": 1_000,
                    "occurredOn": "2026-10-04"
                }),
            ),
        )
        .await;
        assert!(!rejected.ok, "{name} must be rejected");
        assert_eq!(rejected.error_code.as_deref(), Some("magi_account_not_offered"));
    }
    let activities = query_json(&platform, "ActivityList").await;
    assert_eq!(ira_rows(&activities).len(), 2);
}

#[test]
fn g5_cut_draws_writes_nothing_and_return_names_the_task_screen() {
    let nav = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/features/task-manager/magiCutNav.ts"),
    )
    .unwrap();
    let filter = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/features/cash/magiDrawFilter.ts"),
    )
    .unwrap();
    let catalog = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/features/cash/CashElementsCatalog.tsx"),
    )
    .unwrap();
    assert!(
        nav.contains("\"task-manager\"") && nav.contains("\"week-ahead\""),
        "G5 the return frame names the task screen"
    );
    assert!(nav.contains("setScreen(\"task-manager\")"));
    assert!(nav.contains("setCmDesk(\"weekly\")"));
    assert!(!nav.contains("activity_post") && !nav.contains("MagiIraContributionPost"));
    assert!(
        nav.contains("executeCommand(\"TaskResolve\"")
            && nav.contains("how: \"done\"")
            && nav.contains("setCmDesk(\"car\")"),
        "adjusting an element closes the task and returns to the Tax / ACA monitor"
    );
    assert!(!filter.contains("executeCommand") && !filter.contains("lot_qty"));
    assert!(
        filter.contains("\"Income\"")
            && filter.contains("\"Speculation\"")
            && filter.contains("\"Account 9\"")
    );
    assert!(
        catalog.contains("MANAGED_ELEMENT_ACCOUNTS")
            && catalog.contains("\"FI Roth\"")
            && catalog.contains("magiFutureUnconfirmedWithdrawal"),
        "the everyday picker stays; the MAGI entry is a filter"
    );
}
