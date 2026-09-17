//! Components catalog must track today’s owner screens and calculated fields.
//! A pass means an owner would agree those screens still do what the product designed.

use application_core::contracts::{QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION};
use application_core::core_functions::core_functions_catalog;
use application_core::queries::execute_query_on;
use golden_harness::{profile_a_app_dir, repo_root};
use storage_sqlite::LocalPlatform;
use uuid::Uuid;

fn qry(name: &str, body: serde_json::Value) -> QueryRequest {
    QueryRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        query_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: Some(body.to_string()),
    }
}

/// File-only actions are not Components rows.
const NOT_A_SCREEN: &[&str] = &[
    "data-snapshot",
    "app-restart",
    "app-exit",
    "components",
];

fn native_screen_labels(lib: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = lib;
    while let Some(at) = rest.find(".text(\"") {
        rest = &rest[at + 7..];
        let Some((id, after_id)) = rest.split_once('"') else {
            break;
        };
        let after_id = after_id.trim_start();
        if !after_id.starts_with(',') {
            continue;
        }
        let after_comma = after_id[1..].trim_start();
        if !after_comma.starts_with('"') {
            continue;
        }
        let after_q = &after_comma[1..];
        let Some((label, next)) = after_q.split_once('"') else {
            break;
        };
        rest = next;
        if NOT_A_SCREEN.contains(&id) {
            continue;
        }
        out.push((id.to_string(), label.to_string()));
    }
    out
}

fn desktop_sources() -> String {
    let root = repo_root();
    let mut buf = String::new();
    for rel in [
        "apps/desktop/src/App.tsx",
        "apps/desktop/src/CashManagement.tsx",
        "apps/desktop/src/features/shopping-cart/ShoppingCartScreen.tsx",
        "apps/desktop/src/features/shopping-cart/CartBlendTable.tsx",
        "apps/desktop/src/features/cash/CashWeekDesk.tsx",
        "apps/desktop/src/features/graphing/TrendsCharts.tsx",
        "apps/desktop/src/features/graphing/DividendWeeks.tsx",
        "apps/desktop/src/features/home/HomeDividendPlan.tsx",
        "packages/ui-components/src/index.tsx",
    ] {
        buf.push_str(&std::fs::read_to_string(root.join(rel)).unwrap_or_default());
        buf.push('\n');
    }
    buf
}

#[test]
fn components_catalog_lists_every_current_menu_screen() {
    let lib = std::fs::read_to_string(repo_root().join("apps/desktop/src-tauri/src/lib.rs"))
        .expect("native menu");
    let screens = native_screen_labels(&lib);
    assert!(
        screens.len() >= 16,
        "native menu should keep growing with product screens, got {screens:?}"
    );
    let catalog = core_functions_catalog().expect("CoreFunctionsGet modules");
    let titles: Vec<_> = catalog
        .modules
        .iter()
        .map(|m| m.title.clone())
        .collect();
    let mut missing = Vec::new();
    for (id, label) in &screens {
        if !titles.iter().any(|t| t == label) {
            missing.push(format!("{id} → {label}"));
        }
    }
    assert!(
        missing.is_empty(),
        "Tools → Components must list every current menu screen, not a launch-day subset. Missing: {}",
        missing.join("; ")
    );
}

#[test]
fn each_catalog_screen_has_an_on_screen_heading_the_owner_can_see() {
    let lib = std::fs::read_to_string(repo_root().join("apps/desktop/src-tauri/src/lib.rs"))
        .expect("native menu");
    let ui = desktop_sources();
    for (_id, label) in native_screen_labels(&lib) {
        let heading = format!("<h2>{label}</h2>");
        let labeled = format!("aria-label=\"{label}\"");
        assert!(
            ui.contains(&heading) || ui.contains(&labeled),
            "owner must see {label} as a screen heading or aria-label"
        );
    }
}

#[test]
fn owner_facing_calculated_fields_still_on_the_screens() {
    let ui = desktop_sources();
    for needle in [
        ("Home averages", "aria-label=\"Average monthly income\""),
        ("Income Plan Print Export", "aria-label=\"Print Export\""),
        ("Income Plan week Decl $", "<th>Decl $</th>"),
        ("Income Plan week Plan $", "<th>Plan $</th>"),
        ("Income Plan week Variance", "<th>Variance</th>"),
        ("Income Plan Decl $/sh", "Decl $/sh"),
        ("Income Plan % of Plan", "% of Plan"),
        ("Income Plan week summary", "aria-label=\"Week summary\""),
        ("Trends Plan vs Decl", "Plan vs Decl"),
        ("Cart blend Week/Month/Year", "<th scope=\"col\">Week</th>"),
        ("Cart blend Annual each", "<th scope=\"col\">Annual each</th>"),
        ("Cart blend Yield", "<th scope=\"col\">Yield</th>"),
        ("Cart leftover yield from collector", "aria-label=\"Cart cash yield from collector\""),
        ("Cash Management SSA both payees", "Barbara"),
        ("Distribution accounts match type", "accountsForCashType(accounts, activityType)"),
        ("Withdrawal accounts are taxable brokerage", "accountsForCashType(accounts, \"Withdrawal\")"),
        ("SSA account is External", "accountsForCashType(accounts, \"SSA\")"),
        ("Cash Management Tom amount", "286500"),
        ("Cash Management Barbara amount", "133100"),
        ("Distributions account totals", "aria-label=\"Distribution account totals\""),
        ("Distribution tax sections", "aria-label=\"Distribution tax sections\""),
        ("Car ROC plan", "aria-label=\"Cash Management Car ROC plan\""),
        ("Car remaining ordinary", "<dt>Remaining ordinary</dt>"),
        ("Car remaining ROC", "<dt>Remaining ROC</dt>"),
        ("Car YTD ordinary estimate", "<dt>YTD ordinary (estimate)</dt>"),
        ("Car YTD ROC estimate", "<dt>YTD ROC (estimate)</dt>"),
        ("Car long-term gain/loss", "<dt>Long-term capital gain/loss</dt>"),
        ("Car short-term gain/loss", "<dt>Short-term capital gain/loss</dt>"),
        ("Trends stays charts", "Enter the week on Cash"),
        ("Trends does not own capture", "Finish this week on Cash Management"),
        ("Dashboard", "<h2>Dashboard</h2>"),
        ("Calculator", "<h2>Calculator</h2>"),
        ("Tickets", "<h2>Tickets</h2>"),
        ("Holdings", "<h2>Holdings</h2>"),
        ("Add Position", "<h2>Add Position</h2>"),
        ("Add Lot", "<h2>Add Lot</h2>"),
        ("Shopping Cart", "<h2>Shopping Cart</h2>"),
        ("Unsaved Save orange", "is-unsaved"),
    ] {
        assert!(ui.contains(needle.1), "{} must stay on screen: {}", needle.0, needle.1);
    }
    let trends = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/graphing/TrendsCharts.tsx"),
    )
    .unwrap();
    assert!(
        !trends.contains("TrendsCapturePanel"),
        "Trends must not host the week-entry wizard"
    );
}

#[tokio::test]
async fn live_screens_return_the_numbers_an_owner_would_check() {
    let dir = profile_a_app_dir();
    let db = dir.join("local.sqlite");
    assert!(
        db.is_file(),
        "data file missing at {}; run npm run data-seed",
        db.display()
    );
    let platform = LocalPlatform::open(&dir).await.expect("open data sqlite");
    let as_of = serde_json::json!({"asOfDate": "2026-07-31"});

    let calc = execute_query_on(&platform, &platform, qry("CalculatorGet", serde_json::json!({})))
        .await;
    assert!(calc.ok, "CalculatorGet {}", calc.error_code.unwrap_or_default());
    let calc_body: serde_json::Value =
        serde_json::from_str(calc.body_json.as_deref().unwrap_or("{}")).unwrap();
    assert!(
        calc_body["rows"].as_array().map(|a| a.len()).unwrap_or(0) >= 40,
        "Calculator must show imported plans, not an empty table: {calc_body}"
    );

    let dash = execute_query_on(&platform, &platform, qry("DashboardBurndownGet", as_of.clone()))
        .await;
    assert!(dash.ok, "DashboardBurndownGet {}", dash.error_code.unwrap_or_default());
    let dash_body: serde_json::Value =
        serde_json::from_str(dash.body_json.as_deref().unwrap_or("{}")).unwrap();
    assert!(
        dash_body["lines"].as_array().is_some(),
        "Dashboard must show burndown lines: {dash_body}"
    );

    let tickets = execute_query_on(&platform, &platform, qry("WorkTicketList", serde_json::json!({})))
        .await;
    assert!(tickets.ok, "WorkTicketList {}", tickets.error_code.unwrap_or_default());
    let ticket_body: serde_json::Value =
        serde_json::from_str(tickets.body_json.as_deref().unwrap_or("{}")).unwrap();
    assert!(
        ticket_body.get("items").is_some() && ticket_body.get("openCount").is_some(),
        "Tickets must show a queue count and items: {ticket_body}"
    );

    let cm = execute_query_on(
        &platform,
        &platform,
        qry("CashManagementRemindersGet", serde_json::json!({"asOfDate": "2026-09-13"})),
    )
    .await;
    assert!(cm.ok, "CashManagementRemindersGet {}", cm.error_code.unwrap_or_default());
    let cm_body: serde_json::Value =
        serde_json::from_str(cm.body_json.as_deref().unwrap_or("{}")).unwrap();
    let payees = cm_body["ssaPayees"].as_array().cloned().unwrap_or_default();
    assert!(
        payees.iter().any(|p| p["payee"] == "barbara" && p["expectedMinor"] == 133100),
        "Barbara $1,331 must appear on Cash Management: {cm_body}"
    );
    assert!(
        payees.iter().any(|p| p["payee"] == "tom" && p["expectedMinor"] == 286500),
        "Tom $2,865 must appear on Cash Management: {cm_body}"
    );

    let accounts = execute_query_on(&platform, &platform, qry("AccountList", serde_json::json!({})))
        .await;
    assert!(accounts.ok, "AccountList {}", accounts.error_code.unwrap_or_default());
    let acct_body: serde_json::Value =
        serde_json::from_str(accounts.body_json.as_deref().unwrap_or("{}")).unwrap();
    let items = acct_body
        .as_array()
        .cloned()
        .or_else(|| acct_body["items"].as_array().cloned())
        .unwrap_or_default();
    let kind_of = |name: &str| -> String {
        items
            .iter()
            .find(|a| a["name"] == name)
            .and_then(|a| a["kind"].as_str())
            .unwrap_or("")
            .to_string()
    };
    assert_eq!(kind_of("Income"), "ira", "Income must stay IRA so Withdrawal is refused: {acct_body}");
    assert_eq!(kind_of("Speculation"), "ira");
    assert_eq!(kind_of("Car"), "taxable", "Car must stay taxable brokerage");
    let car_roc = execute_query_on(
        &platform,
        &platform,
        qry("CarRocPlanGet", serde_json::json!({"asOfDate": "2026-09-13"})),
    )
    .await;
    assert!(car_roc.ok, "CarRocPlanGet {}", car_roc.error_code.unwrap_or_default());
    let car_roc_body: serde_json::Value =
        serde_json::from_str(car_roc.body_json.as_deref().unwrap_or("{}")).unwrap();
    assert_eq!(car_roc_body["accountName"], "Car", "{car_roc_body}");
    assert!(
        car_roc_body["taxNote"]
            .as_str()
            .unwrap_or("")
            .contains("April 2027"),
        "Car tax stays unknown until 1099: {car_roc_body}"
    );
    assert!(
        car_roc_body["estimateNote"]
            .as_str()
            .unwrap_or("")
            .contains("prior-year ROC guidance"),
        "{car_roc_body}"
    );
    if let (Some(ord), Some(roc), Some(total)) = (
        car_roc_body["remainingOrdinaryMinor"].as_i64(),
        car_roc_body["remainingRocMinor"].as_i64(),
        car_roc_body["remainingTotalMinor"].as_i64(),
    ) {
        assert_eq!(ord + roc, total, "Car remaining split must use the plan estimate: {car_roc_body}");
    }
    assert!(
        car_roc_body["namesWithEstimate"].as_u64().unwrap_or(0) >= 1,
        "Car ROC estimates must be on the plan: {car_roc_body}"
    );
    assert_eq!(kind_of("Robinhood"), "taxable");
    assert_eq!(kind_of("FI Roth"), "fi_roth");
    assert!(
        items.iter().any(|a| a["name"] == "External"),
        "External must stay so SSA has a home: {acct_body}"
    );

    let modules = execute_query_on(&platform, &platform, qry("CoreFunctionsGet", serde_json::json!({})))
        .await;
    assert!(modules.ok);
    let module_body: serde_json::Value =
        serde_json::from_str(modules.body_json.as_deref().unwrap_or("{}")).unwrap();
    let titles: Vec<_> = module_body["modules"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|m| m["title"].as_str().map(|s| s.to_string()))
        .collect();
    for need in [
        "Calculator",
        "Dashboard",
        "Tickets",
        "Holdings",
        "Add Position",
        "Add Lot",
        "Trends",
        "Cash Management",
        "Reevaluate collector",
    ] {
        assert!(
            titles.iter().any(|t| t == need),
            "CoreFunctionsGet.modules must include {need} so Components cannot freeze at first-ship: {titles:?}"
        );
    }
}

#[tokio::test]
async fn income_plan_export_includes_current_grid_and_week_surfaces() {
    let dir = profile_a_app_dir();
    let db = dir.join("local.sqlite");
    assert!(db.is_file(), "data file missing at {}", db.display());
    let platform = LocalPlatform::open(&dir).await.expect("open data sqlite");

    let grid = execute_query_on(
        &platform,
        &platform,
        qry(
            "IncomePlanExportGet",
            serde_json::json!({
                "pattern": "A",
                "format": "html",
                "asOfDate": "2026-07-31",
                "historicalWeeks": 4,
                "futureWeeks": 4
            }),
        ),
    )
    .await;
    assert!(grid.ok, "grid export {}", grid.error_code.unwrap_or_default());
    let grid_body: serde_json::Value =
        serde_json::from_str(grid.body_json.as_deref().unwrap_or("{}")).unwrap();
    let grid_html = grid_body["printHtml"].as_str().unwrap_or("");
    for part in ["Income Plan", "AccountRollup", "PositionGrid", "decl_per_share"] {
        assert!(
            grid_html.contains(part),
            "Print Export grid must still include {part}"
        );
    }

    let week = execute_query_on(
        &platform,
        &platform,
        qry(
            "IncomePlanExportGet",
            serde_json::json!({
                "pattern": "B",
                "format": "html",
                "asOfDate": "2026-07-31",
                "weekEnding": "2026-07-31"
            }),
        ),
    )
    .await;
    assert!(week.ok, "week export {}", week.error_code.unwrap_or_default());
    let week_body: serde_json::Value =
        serde_json::from_str(week.body_json.as_deref().unwrap_or("{}")).unwrap();
    let week_html = week_body["printHtml"].as_str().unwrap_or("");
    for part in ["WeekDetail", "Plan $", "Declaration $", "Variance", "decl_per_share"] {
        assert!(
            week_html.contains(part),
            "Print Export week report must include current money columns, not a first-ship subset: missing {part}"
        );
    }
    assert!(
        !week_html.contains("Actual $"),
        "weekly export must not put broker Actual $ back on the report"
    );

    let app = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx")).unwrap();
    for action in [
        "aria-label=\"Print Export\"",
        "aria-label=\"Print to page\"",
        "aria-label=\"Save PDF\"",
        "aria-label=\"Export Excel\"",
    ] {
        assert!(
            app.contains(action),
            "export dialog must keep {action}"
        );
    }
}
