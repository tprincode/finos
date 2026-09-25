use application_core::contracts::{
    CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
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

async fn must_ok(platform: &LocalPlatform, name: &str, body: serde_json::Value) -> serde_json::Value {
    let result = execute_command_on(platform, platform, cmd(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

async fn query_json(platform: &LocalPlatform, name: &str, body: serde_json::Value) -> serde_json::Value {
    let result = execute_query_on(platform, platform, qry(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

async fn research_template(platform: &LocalPlatform, security_id: &str, symbol: &str) {
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
}

#[test]
fn coverage_menu_is_fourth_cash_management_child() {
    let root = repo_root();
    let app = std::fs::read_to_string(root.join("apps/desktop/src/App.tsx")).unwrap();
    let lib = std::fs::read_to_string(root.join("apps/desktop/src-tauri/src/lib.rs")).unwrap();
    let ui = std::fs::read_to_string(root.join("apps/desktop/src/features/cash/CashCoverage.tsx"))
        .unwrap();
    assert!(
        app.contains(r#"cmDeskButton("coverage", "Coverage")"#)
            && app.contains(r#"id === "cash-coverage""#)
            && app.contains(r#"aria-label="Coverage""#),
        "in-app Coverage desk"
    );
    assert!(
        lib.contains(".text(\"cash-coverage\", \"Coverage\")"),
        "native Coverage item"
    );
    assert!(
        ui.contains("aria-label=\"Cash Management Coverage\"")
            && ui.contains("aria-label=\"Coverage plan\"")
            && ui.contains("aria-label=\"Coverage income math\"")
            && ui.contains("aria-label=\"Coverage expense math\"")
            && ui.contains("cash-coverage-caption")
            && ui.contains("Weekly comparison")
            && ui.contains("next 12 months")
            && ui.contains("amount per payment × periods")
            && ui.contains("cash-coverage-loading")
            && ui.contains("Loading {periodChip}")
            && ui.contains("minor == null ? \"—\"")
            && !ui.contains("?? 0")
            && !ui.contains("??0"),
        "Coverage prints — for unknown, never $0; comparison is the forward plan; math tables show amount × periods"
    );
}

#[tokio::test]
async fn income_week_plan_minus_unpaid_element_is_known_plan_unknown_actual() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let income = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "ira"}),
    )
    .await;
    let income_id = income["accountId"].clone();
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "REG1", "name": "REG1"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    research_template(&platform, security_id, "REG1").await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Weekly",
            "replaceCadence": true,
            "riskTier": "Risk On"
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot_as(
        &platform,
        security_id,
        "REG1",
        "Weekly",
        true,
    )
    .await
    .expect("complete collector");
    must_ok(
        &platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 1000,
            "amountScale": 4,
            "paymentPeriod": "2026-07-28",
            "source": "provider-site",
            "enteredAt": "2026-08-22"
        }),
    )
    .await;
    must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 100,
            "amountScale": 2,
            "planningPeriodsPerYear": 52,
            "effectiveFrom": "2026-08-12",
            "decisionReason": "Locked weekly Plan",
            "incompleteAnalysisReason": "Fewer than 6 observations"
        }),
    )
    .await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": income_id,
            "securityId": security_id,
            "openedOn": "2025-11-03",
            "origin": "purchase",
            "quantityMinor": 100,
            "quantityScale": 0,
            "performanceBasisMinor": 200_000,
            "taxBasisMinor": 200_000,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "CashElementSave",
        serde_json::json!({
            "account": "Income",
            "name": "coverage-week",
            "kind": "Withdrawal",
            "cadence": "weekly",
            "weekdayOrMonthDay": "Sat",
            "amountMinor": 100_000,
            "asOfDate": "2026-09-18",
            "startOn": "",
            "stopOn": "",
            "occurrences": []
        }),
    )
    .await;
    must_ok(
        &platform,
        "CashElementSave",
        serde_json::json!({
            "account": "Car",
            "name": "coverage-car",
            "kind": "Withdrawal",
            "cadence": "monthly",
            "weekdayOrMonthDay": "1",
            "amountMinor": 85_000,
            "asOfDate": "2026-09-18",
            "startOn": "",
            "stopOn": "",
            "occurrences": []
        }),
    )
    .await;
    must_ok(
        &platform,
        "CashElementSave",
        serde_json::json!({
            "account": "Health",
            "name": "coverage-hsa",
            "kind": "Withdrawal",
            "cadence": "monthly",
            "weekdayOrMonthDay": "1",
            "amountMinor": 20_000,
            "asOfDate": "2026-09-18",
            "startOn": "",
            "stopOn": "",
            "occurrences": []
        }),
    )
    .await;

    let plan = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({"asOfDate": "2026-09-18"}),
    )
    .await;
    let pos = plan["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "REG1")
        .unwrap_or_else(|| panic!("REG1 missing: {plan}"));
    assert_eq!(pos["planKnown"], true, "{pos}");

    let cover = query_json(
        &platform,
        "CashCoverageGet",
        serde_json::json!({"asOfDate": "2026-09-18", "period": "week"}),
    )
    .await;
    assert_eq!(cover["period"], "week");
    assert_eq!(cover["periodStart"], "2026-09-18");
    assert_eq!(cover["periodEnd"], "2027-09-18");
    let rows = cover["rows"].as_array().unwrap();
    let names: Vec<&str> = rows.iter().filter_map(|r| r["account"].as_str()).collect();
    assert_eq!(
        names,
        vec!["Income", "FI Roth", "Car", "Health", "Account 9", "Total"],
        "{cover}"
    );
    assert!(!names.iter().any(|n| n.contains("SSA")), "{cover}");
    let income = rows.iter().find(|r| r["account"] == "Income").unwrap();
    assert_eq!(
        income["planIncomeMinor"].as_i64(),
        Some(10_000),
        "100 sh × $1.00 weekly, week = year/52: {cover}"
    );
    assert_eq!(income["planExpenseMinor"].as_i64(), Some(100_000), "{cover}");
    assert_eq!(
        income["planMinor"].as_i64(),
        Some(10_000 - 100_000),
        "Δ is planned income minus planned withdrawals: {cover}"
    );
    let lines = cover["incomeLines"].as_array().expect("income math");
    let reg = lines.iter().find(|l| l["symbol"] == "REG1").unwrap();
    assert_eq!(reg["perPeriodMinor"].as_i64(), Some(10_000), "{reg}");
    assert_eq!(reg["periods"].as_i64(), Some(52), "{reg}");
    assert_eq!(reg["yearMinor"].as_i64(), Some(520_000), "{reg}");
    let car = rows.iter().find(|r| r["account"] == "Car").unwrap();
    assert_eq!(
        car["planExpenseMinor"].as_i64(),
        Some(19_615),
        "Car $850/mo × 12 / 52: {cover}"
    );
    let health = rows.iter().find(|r| r["account"] == "Health").unwrap();
    assert_eq!(
        health["planExpenseMinor"].as_i64(),
        Some(4_615),
        "Health $200/mo × 12 / 52: {cover}"
    );
}
