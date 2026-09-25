use application_core::contracts::{
    CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::queries::{execute_command_on, execute_query_on};
use golden_harness::repo_root;
use storage_sqlite::LocalPlatform;
use uuid::Uuid;

const YTD_ROC_REASONS: &[&str] = &[
    "no Car payments this year yet",
    "Position.roc_pct_2026_estimate blank on names that paid",
    "needs_roc_research still set",
    "2026_actual empty until 1099 (expected)",
];

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

fn is_known_reason(reason: &str) -> bool {
    YTD_ROC_REASONS.contains(&reason)
        || reason.starts_with("mixed: some names estimated, some unclassified")
}

fn cash_submenu(lib: &str) -> &str {
    lib.split("SubmenuBuilder::new(app, \"Cash Management\")")
        .nth(1)
        .and_then(|rest| rest.split("SubmenuBuilder::new(app,").next())
        .unwrap_or("")
}

fn cash_menu_group(app: &str) -> &str {
    let Some(at) = app.find(r#"cmDeskButton("elements", "Element Management")"#) else {
        return "";
    };
    let start = at.saturating_sub(200);
    &app[start..at + 620]
}

#[test]
fn m1_top_level_cash_management_has_five_children() {
    let root = repo_root();
    let app = std::fs::read_to_string(root.join("apps/desktop/src/App.tsx")).unwrap();
    let lib = std::fs::read_to_string(root.join("apps/desktop/src-tauri/src/lib.rs")).unwrap();
    assert!(
        app.contains("menuGroup(") && app.contains("\"cash\"") && app.contains("\"Cash Management\""),
        "M1: in-app top-level Cash Management group"
    );
    let group = cash_menu_group(&app);
    assert!(
        group.contains("Element Management")
            && group.contains("Cashflow Manager")
            && group.contains("System update tasks and confirmations")
            && group.contains("Tax Planning")
            && group.contains("Coverage"),
        "M1: in-app five child labels: {group}"
    );
    let elements_at = group.find("Element Management").unwrap_or(usize::MAX);
    let cashflow_at = group.find("Cashflow Manager").unwrap_or(usize::MAX);
    let weekly_at = group.find("System update tasks and confirmations").unwrap_or(usize::MAX);
    let car_at = group.find("Tax Planning").unwrap_or(usize::MAX);
    let coverage_at = group.find("Coverage").unwrap_or(usize::MAX);
    assert!(
        elements_at < cashflow_at
            && cashflow_at < weekly_at
            && weekly_at < car_at
            && car_at < coverage_at,
        "M1: in-app child order Element / Cashflow / Weekly / Tax / Coverage"
    );
    assert!(
        !group.contains("Car ROC Plan") && !group.contains("Car ROC plan"),
        "M1: no Car ROC Plan menu string"
    );
    assert!(
        app.contains("navButton(\"cash-management\", \"Cash Management\")"),
        "M1: Plan keeps Cash Management shortcut"
    );

    let native = cash_submenu(&lib);
    assert!(
        native.contains(".text(\"cash-elements\", \"Element Management\")")
            && native.contains(".text(\"cash-cashflow\", \"Cashflow Manager\")")
            && native.contains(".text(\"cash-weekly\", \"System update tasks and confirmations\")")
            && native.contains(".text(\"cash-car-tax\", \"Tax Planning\")")
            && native.contains(".text(\"cash-coverage\", \"Coverage\")"),
        "M1: native five child labels: {native}"
    );
    let texts: Vec<_> = native
        .lines()
        .filter(|l| l.contains(".text(\""))
        .collect();
    assert_eq!(texts.len(), 5, "M1: native Cash Management has five children: {texts:?}");
    assert!(
        lib.contains(".text(\"cash-management\", \"Cash Management\")"),
        "M1: Plan keeps native cash-management shortcut"
    );
}

#[test]
fn m2_element_management_is_catalog_only() {
    let root = repo_root();
    let host = std::fs::read_to_string(root.join("apps/desktop/src/CashManagement.tsx")).unwrap();
    let register =
        std::fs::read_to_string(root.join("apps/desktop/src/features/cash/CashRegister.tsx"))
            .unwrap();
    let elements_block = host
        .split("if (desk === \"elements\")")
        .nth(1)
        .and_then(|rest| rest.split("if (desk ===").next())
        .unwrap_or("");
    assert!(
        elements_block.contains("cashElements") && !elements_block.contains("cashRegister"),
        "M2: Element Management mounts catalog only"
    );
    let cashflow_block = host
        .split("if (desk === \"cashflow\")")
        .nth(1)
        .and_then(|rest| rest.split("if (").next())
        .unwrap_or("");
    assert!(
        cashflow_block.contains("cashRegister") && !cashflow_block.contains("cashYtd"),
        "M2: Cashflow Manager mounts the register without YTD"
    );
    let tax_block = host
        .split("if (desk === \"car\")")
        .nth(1)
        .and_then(|rest| rest.split("if (desk ===").next())
        .unwrap_or("");
    assert!(
        tax_block.contains("cashYtd"),
        "M2: Tax Planning mounts Cash YTD at the bottom"
    );
    assert!(
        register.contains("Calendar")
            && register.contains("Trend")
            && register.contains("calendarBody?.series")
            && register.contains("cashflow manager")
            && register.contains("exactlyOne"),
        "M2: Cashflow Manager Calendar|Trend consume register.series; tick picker is exactly one"
    );
    let modules = std::fs::read_to_string(root.join("docs/architecture/ui-modules.json")).unwrap();
    assert!(
        modules.contains("\"title\": \"Cashflow Manager\"")
            && !modules.contains("\"title\": \"Cash register\""),
        "M2: catalog has one first-class Cashflow Manager, not a leftover Register"
    );
}

#[test]
fn m3_weekly_updates_keeps_capture_and_week_ahead() {
    let root = repo_root();
    let host = std::fs::read_to_string(root.join("apps/desktop/src/CashManagement.tsx")).unwrap();
    let app = std::fs::read_to_string(root.join("apps/desktop/src/App.tsx")).unwrap();
    let ahead =
        std::fs::read_to_string(root.join("apps/desktop/src/features/cash/WeekAhead.tsx")).unwrap();
    assert!(
        host.contains("desk === \"weekly\"") && host.contains("weekAhead"),
        "M3: System update tasks and confirmations still mounts Week Ahead"
    );
    assert!(
        app.contains("aria-label=\"Week capture grid\"")
            || std::fs::read_to_string(
                root.join("apps/desktop/src/features/graphing/TrendsCapture.tsx")
            )
            .unwrap()
            .contains("aria-label=\"Week capture grid\""),
        "M3: Week capture grid stays"
    );
    assert!(
        ahead.contains("Week ahead") || ahead.contains("aria-label=\"Week ahead\""),
        "M3: Week Ahead heading stays"
    );
    let weekly_host = app
        .split("aria-label=\"System update tasks and confirmations\"")
        .nth(1)
        .and_then(|s| s.split("cmDesk === \"coverage\" ? (").next())
        .unwrap_or("");
    let income_jsx = std::fs::read_to_string(
        root.join("apps/desktop/src/features/income-plan/IncomePlanScreen.tsx"),
    )
    .unwrap();
    assert!(
        weekly_host.contains("<PlanHorizonPrompt")
            && !income_jsx.contains("<PlanHorizonPrompt"),
        "M3: 2027 pay-date Confirm sits on System update tasks and confirmations, not Income Plan"
    );
}

#[test]
fn m4_no_visible_car_roc_plan_label() {
    let root = repo_root();
    for rel in [
        "apps/desktop/src/App.tsx",
        "apps/desktop/src/CashManagement.tsx",
        "apps/desktop/src/features/cash/CashRegister.tsx",
        "apps/desktop/src/features/cash/CashElementsCatalog.tsx",
        "apps/desktop/src/features/cash/CashElementEditor.tsx",
        "apps/desktop/src/features/cash/CashYtd.tsx",
        "apps/desktop/src/features/cash/CashWeekDesk.tsx",
        "apps/desktop/src/features/cash/CashCoverage.tsx",
        "apps/desktop/src/features/cash/WeekAhead.tsx",
    ] {
        let body = std::fs::read_to_string(root.join(rel)).unwrap_or_default();
        assert!(
            !body.contains("Car ROC Plan") && !body.contains("Car ROC plan"),
            "M4: {rel} must not show Car ROC Plan"
        );
    }
}

#[tokio::test]
async fn m5_unknown_ytd_roc_is_null_plus_reason() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let result = execute_command_on(
        &platform,
        &platform,
        cmd(
            "AccountRegister",
            serde_json::json!({"name": "Car", "kind": "taxable"}),
        ),
    )
    .await;
    assert!(result.ok, "AccountRegister Car {:?}", result.error_code);
    let plan = execute_query_on(
        &platform,
        &platform,
        qry("CarRocPlanGet", serde_json::json!({"asOfDate": "2026-09-13"})),
    )
    .await;
    assert!(plan.ok, "CarRocPlanGet {}", plan.error_code.unwrap_or_default());
    let body: serde_json::Value =
        serde_json::from_str(plan.body_json.as_deref().unwrap_or("{}")).unwrap();
    assert!(
        body["ytdRocMinor"].is_null(),
        "M5: unknown YTD ROC is null, never $0: {body}"
    );
    assert_ne!(
        body["ytdRocMinor"].as_i64(),
        Some(0),
        "M5: never 0 as the estimate dollar: {body}"
    );
    let reason = body["ytdRocUnknownReason"].as_str().unwrap_or("");
    assert!(
        is_known_reason(reason),
        "M5: unknown reason must be one of the owner strings: {body}"
    );
    let ui = std::fs::read_to_string(repo_root().join("apps/desktop/src/CashManagement.tsx")).unwrap();
    assert!(
        ui.contains("ytdRocUnknownReason")
            && ui.contains("formatRocCell")
            && !ui.contains("ytdRocMinor ?? 0")
            && !ui.contains("remainingRocMinor ?? 0"),
        "M5: table prints the reason, never $0 for unknown ROC"
    );
}

#[test]
fn elements_desk_does_not_paint_false_week_loading() {
    let ui = std::fs::read_to_string(repo_root().join("apps/desktop/src/CashManagement.tsx")).unwrap();
    assert!(
        ui.contains("if (desk === \"elements\")")
            && ui.contains("children || weekAhead ? null")
            && ui.contains("Loading Cash Management…"),
        "Elements desk returns without waiting on week; loading only if weekly has no pane"
    );
    let elements_at = ui.find("if (desk === \"elements\")").unwrap_or(0);
    let loading_at = ui.rfind("Loading Cash Management…").unwrap_or(0);
    let week_null_at = ui.find("if (!week)").unwrap_or(0);
    assert!(
        elements_at < week_null_at && week_null_at < loading_at,
        "Elements return is before the week-null loading fallback"
    );
}

#[test]
fn m6_label_goldens_updated_not_deleted() {
    let root = repo_root();
    for rel in [
        "crates/golden-harness/tests/cash_management.rs",
        "crates/golden-harness/tests/accessibility.rs",
        "crates/golden-harness/tests/ui_modules.rs",
        "crates/golden-harness/tests/week_ahead.rs",
        "crates/golden-harness/tests/cash_register.rs",
    ] {
        assert!(
            root.join(rel).is_file(),
            "M6: keep {rel}"
        );
    }
    let acc = std::fs::read_to_string(root.join("crates/golden-harness/tests/accessibility.rs"))
        .unwrap();
    assert!(
        acc.contains("fn g1_g6_slice1b_capture_grid_one_table"),
        "M6: G1–G7 capture goldens stay"
    );
}

#[tokio::test]
async fn tax_planning_lists_ira_roth_roc_ordinary_and_gains() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    for (name, kind) in [
        ("Income", "ira"),
        ("Speculation", "ira"),
        ("9", "ira"),
        ("FI Roth", "fi_roth"),
        ("Health", "hsa"),
        ("Car", "taxable"),
        ("External", "taxable"),
    ] {
        let result = execute_command_on(
            &platform,
            &platform,
            cmd(
                "AccountRegister",
                serde_json::json!({"name": name, "kind": kind}),
            ),
        )
        .await;
        assert!(result.ok, "AccountRegister {name} {:?}", result.error_code);
    }
    let plan = execute_query_on(
        &platform,
        &platform,
        qry("TaxPlanningGet", serde_json::json!({"asOfDate": "2026-09-19"})),
    )
    .await;
    assert!(plan.ok, "TaxPlanningGet {}", plan.error_code.unwrap_or_default());
    let body: serde_json::Value =
        serde_json::from_str(plan.body_json.as_deref().unwrap_or("{}")).unwrap();
    let keys: Vec<&str> = body["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["key"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(
        keys,
        vec!["ira", "roth", "hsa", "roc", "ordinary", "job1099", "ssa", "ltcg", "stcg"],
        "{body}"
    );
    assert_eq!(body["allSources"]["label"], "All income sources");
    assert_eq!(body["magiIncluded"]["label"], "MAGI-included");
    assert_eq!(body["notMagi"]["label"], "No MAGI impact");
    let job = body["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["key"] == "job1099")
        .expect("1099 job row");
    assert_eq!(job["label"], "1099 job");
    assert_eq!(job["ytdMinor"], 1_462_500);
    assert_eq!(job["projectedMinor"], 0);
    assert_eq!(job["totalMinor"], 1_462_500);
    assert_eq!(job["magiImpact"], "magi");
}
