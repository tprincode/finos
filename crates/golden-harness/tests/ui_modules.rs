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

fn collect_quoted_pairs(src: &str, fn_name: &str) -> Vec<(String, String)> {
    let needle = format!("{fn_name}(\"");
    let mut out = Vec::new();
    let mut rest = src;
    while let Some(at) = rest.find(&needle) {
        rest = &rest[at + needle.len()..];
        let Some((id, after_id)) = rest.split_once('"') else {
            break;
        };
        let after = after_id.trim_start().trim_start_matches(',').trim_start();
        if !after.starts_with('"') {
            continue;
        }
        let Some((label, next)) = after[1..].split_once('"') else {
            break;
        };
        rest = next;
        if NOT_A_SCREEN.contains(&id) {
            continue;
        }
        let label = label.replace(" -CCT", "");
        out.push((id.to_string(), label));
    }
    out
}

fn menu_screen_labels(app: &str) -> Vec<(String, String)> {
    let mut out = vec![
        ("home".into(), "Home".into()),
        ("income-plan".into(), "Income Plan".into()),
        ("trends".into(), "Trends".into()),
    ];
    out.extend(collect_quoted_pairs(app, "navButton"));
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
        "apps/desktop/src/features/shopping-cart/AffordStrip.tsx",
        "apps/desktop/src/features/cash/CashWeekDesk.tsx",
        "apps/desktop/src/features/cash/CashCoverage.tsx",
        "apps/desktop/src/features/cash/HouseholdIncomeReport.tsx",
        "apps/desktop/src/features/cash/magiForecast.ts",
        "apps/desktop/src/features/graphing/TrendsCharts.tsx",
        "apps/desktop/src/features/graphing/DividendWeeks.tsx",
        "apps/desktop/src/features/home/HomeDividendPlan.tsx",
        "apps/desktop/src/features/home/HomeScreen.tsx",
        "apps/desktop/src/features/collectors/CollectorsScreen.tsx",
        "apps/desktop/src/features/collectors/CollectorEstablishScreen.tsx",
        "apps/desktop/src/features/interest-rate/InterestRateCalculator.tsx",
        "apps/desktop/src/features/accounts/AccountManagement.tsx",
        "apps/desktop/src/features/field-intent/FieldIntentScreen.tsx",
        "apps/desktop/src/features/contracts/ContractPositions.tsx",
        "apps/desktop/src/features/roadmap/RoadmapScreen.tsx",
        "apps/desktop/src/features/task-manager/TaskManager.tsx",
        "apps/desktop/src/features/position-details/PositionDetailsScreen.tsx",
        "apps/desktop/src/features/add-lot/AddLotScreen.tsx",
        "apps/desktop/src/features/new-investment/NewInvestmentScreen.tsx",
        "apps/desktop/src/features/holdings/HoldingsScreen.tsx",
        "apps/desktop/src/features/income-plan/IncomePlanScreen.tsx",
        "apps/desktop/src/features/market-impact/MarketImpactPlanner.tsx",
        "apps/desktop/src/features/screen-atlas/ScreenAtlasScreen.tsx",
        "apps/desktop/src/features/components/ComponentRegistry.tsx",
        "packages/ui-components/src/index.tsx",
    ] {
        buf.push_str(&std::fs::read_to_string(root.join(rel)).unwrap_or_default());
        buf.push('\n');
    }
    buf
}

#[test]
fn components_catalog_lists_every_current_menu_screen() {
    let app = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx"))
        .expect("in-app menu");
    let screens = menu_screen_labels(&app);
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
    let app = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx"))
        .expect("in-app menu");
    let ui = desktop_sources();
    for (_id, label) in menu_screen_labels(&app) {
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
        ("Income Plan Export", "aria-label=\"Export\""),
        ("Income Plan week Decl $", "<th>Decl $</th>"),
        ("Income Plan week Plan $", "<th>Plan $</th>"),
        ("Income Plan week Variance", "<th>Current variance</th>"),
        ("Income Plan remaining declarations", "Remaining declarations"),
        (
            "Income Plan cash skips declaration tickets",
            "Cash / money-market never receives an issuer declaration",
        ),
        (
            "Income Plan week payers exclude cash",
            "const weekPayers = positionRows.filter((r) => !isCashSymbol(r.symbol))",
        ),
        ("Income Plan Decl $/sh", "Decl $/sh"),
        ("Income Plan % of Plan", "% of Plan"),
        ("Income Plan week summary", "aria-label=\"Week summary\""),
        ("Trends Plan vs Decl", "Plan vs Decl"),
        ("Cart blend Week/Month/Year", "<th scope=\"col\">Week</th>"),
        ("Cart blend Annual each", "<th scope=\"col\">Annual each</th>"),
        ("Cart blend Yield", "<th scope=\"col\">Yield</th>"),
        ("Cart leftover yield from collector", "Leftover (still earns)"),
        ("Cash Management SSA both payees", "Barbara"),
        ("Distribution accounts match type", "accountsForCashType(accounts, activityType)"),
        ("Withdrawal accounts are taxable brokerage", "accountsForCashType(accounts, \"Withdrawal\")"),
        ("SSA account is External", "accountsForCashType(accounts, \"SSA\")"),
        ("Cash Management Tom amount", "286500"),
        ("Cash Management Barbara amount", "133100"),
        ("Distributions account totals", "aria-label=\"Distribution account totals\""),
        ("Distribution tax sections", "aria-label=\"Distribution tax sections\""),
        ("Tax Planning", "aria-label=\"Cash Management Tax Planning\""),
        ("Tax Planning income", "aria-label=\"Tax Planning income\""),
        ("Tax Planning MAGI", "aria-label=\"Tax Planning MAGI\""),
        ("MAGI forecast panel", "aria-label=\"MAGI forecast\""),
        ("MAGI forecast suggestions", "aria-label=\"MAGI suggestions\""),
        ("Coverage desk", "aria-label=\"Income vs Expense planner\""),
        ("Coverage plan table", "aria-label=\"Coverage plan\""),
        ("Coverage plan vs withdrawals", "planned income vs planned withdrawals"),
        ("Coverage period caption", "cash-coverage-caption"),
        ("Coverage period loading", "cash-coverage-loading"),
        ("Weekly comparison title", "Weekly comparison"),
        ("Car tax table", "aria-label=\"Car account tax planning\""),
        ("Car tax YTD", "Total YTD + Planned"),
        ("Car ordinary row", "Ordinary"),
        ("Car long-term gains", "Long Term Capital Gains"),
        ("Car short-term gains", "Short Term Capital Gains"),
        ("Trends stays charts", "aria-label=\"Trends graphing period\""),
        ("Trends does not own capture", "<TrendsChartsPanel"),
        ("Dashboard", "<h2>Dashboard</h2>"),
        ("Calculator", "<h2>Calculator</h2>"),
        ("Tickets", "<h2>Tickets</h2>"),
        ("Holdings", "<h2>Holdings</h2>"),
        ("Add Investment", "<h2>Add Investment</h2>"),
        ("Add Lot", "<h2>Add Lot</h2>"),
        ("Shopping Cart", "<h2>Shopping Cart</h2>"),
        ("Unsaved Save orange", "is-unsaved"),
    ] {
        assert!(ui.contains(needle.1), "{} must stay on screen: {}", needle.0, needle.1);
    }
    assert!(
        !ui.contains("<span>Misses</span>") && !ui.contains("Misses {missCount}"),
        "Income Plan week must not show Misses; remaining declarations is the status chip"
    );
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
        calc_body["rows"].as_array().map(|a| a.len()).unwrap_or(0) >= 38,
        "Calculator must show imported DIV-1 plans, not an empty table: {calc_body}"
    );
    assert!(
        calc_body["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["symbol"] != "SPAXX"
                && row["symbol"] != "FDRXX"
                && row["symbol"] != "SWVXX"),
        "cash stays off CalculatorGet: {calc_body}"
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
        "Add Investment",
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

#[test]
fn components_catalog_lists_household_income_and_magi_forecast() {
    let catalog = core_functions_catalog().expect("CoreFunctionsGet modules");
    let titles: Vec<_> = catalog
        .modules
        .iter()
        .map(|m| m.title.as_str())
        .collect();
    assert!(
        titles
            .iter()
            .any(|t| t.contains("Household income") && t.contains("MAGI")),
        "Components catalog must list Household income / MAGI: {titles:?}"
    );
    assert!(
        titles.iter().any(|t| *t == "MAGI forecast"),
        "Components catalog must list MAGI forecast: {titles:?}"
    );
    let task = catalog
        .modules
        .iter()
        .find(|m| m.id == "task-manager")
        .expect("task-manager module");
    assert_eq!(
        task.folder.as_str(),
        "apps/desktop/src/features/task-manager/"
    );
    assert!(
        task.core_function_ids
            .iter()
            .any(|c| c == "cash-management-magi"),
        "task-manager must stay wired to weekly MAGI cliff core: {:?}",
        task.core_function_ids
    );
}

#[test]
fn app_tsx_shell_rule_locks_extract_before_append() {
    let rule = std::fs::read_to_string(repo_root().join(".cursor/rules/app-tsx-shell.mdc"))
        .expect("app-tsx-shell.mdc must exist");
    assert!(
        rule.contains("alwaysApply: true"),
        "shell rule must always apply"
    );
    assert!(
        rule.contains("shell"),
        "shell rule must name App.tsx as a shell"
    );
    assert!(
        rule.contains("~20 lines") || rule.contains("20 lines"),
        "shell rule must cap App.tsx growth at ~20 lines"
    );
    assert!(
        rule.to_lowercase().contains("never") && rule.to_lowercase().contains("append"),
        "shell rule must forbid appending new tool bodies into App.tsx"
    );
}

/// A live process and HTTP 200 on 1420 do not mean Home painted. The shell writes
/// `page loaded Home` after the Home button and the home row are in the document.
#[test]
fn home_page_loaded_is_logged_after_the_home_screen_commits() {
    let mark = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/shell/HomePaintMark.tsx"),
    )
    .expect("HomePaintMark");
    assert!(
        mark.contains("button[aria-label='Home']") && mark.contains(".home-top-row"),
        "the paint mark greps the Home button and the home row"
    );
    assert!(
        mark.contains("page_loaded") && mark.contains("Home"),
        "the paint mark asks the host to log Home"
    );
    let app = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx")).unwrap();
    let home = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/home/HomeScreen.tsx"),
    )
    .unwrap();
    assert!(
        app.contains("<HomeScreen") && home.contains("<HomePaintMark"),
        "Home mounts the paint mark"
    );
    let boot = std::fs::read_to_string(repo_root().join("apps/desktop/src/main.tsx")).unwrap();
    assert!(
        boot.contains("Home failed"),
        "a render error writes Home failed instead of a paint line"
    );
    let host = std::fs::read_to_string(repo_root().join("apps/desktop/src-tauri/src/lib.rs"))
        .unwrap();
    assert!(
        host.contains("page loaded")
            && host.contains("dev-console.log")
            && host.contains("page-loaded.log"),
        "the host writes page loaded into the desktop log"
    );
    let rule = std::fs::read_to_string(repo_root().join(".cursor/rules/restart-the-app.mdc"))
        .unwrap();
    assert!(
        rule.contains("page loaded Home") && rule.contains("exactly one"),
        "restart proof requires one process and a Home paint line"
    );
    let check = std::fs::read_to_string(repo_root().join("scripts/app-up.ps1")).unwrap();
    assert!(
        check.contains("page loaded Home 20")
            && check.contains("Home failed")
            && check.contains("finos-desktop count")
            && check.contains("start-finos-dev.bat"),
        "app-up.ps1 counts the desktop and the dev launcher and requires the Home line"
    );
}

/// The desktop died at boot on 2026-10-03 with "Cannot read properties of null (reading
/// 'useState')". Cause: the workspace packages are served from source, not pre-bundled, so
/// adding one dependency edge inside `@finos/ui-components` re-ran Vite's dep optimizer
/// mid-session. The page then held `react.js?v=OLD` while the package loaded
/// `react.js?v=NEW` — two React instances, null hook dispatcher, blank app. Nothing caught
/// it because no test reads the dev-server config.
#[test]
fn vite_config_pins_one_react_copy() {
    let cfg = std::fs::read_to_string(repo_root().join("apps/desktop/vite.config.ts"))
        .expect("apps/desktop/vite.config.ts must exist");
    assert!(
        cfg.contains("dedupe"),
        "vite.config must dedupe react so a workspace package cannot load a second copy"
    );
    for pkg in ["\"react\"", "\"react-dom\""] {
        assert!(
            cfg.contains(pkg),
            "vite.config dedupe must name {pkg}; a duplicate React blanks the whole app at boot"
        );
    }
    assert!(
        cfg.contains("optimizeDeps"),
        "vite.config must pin React's entry points into the first optimizer pass, or a new \
         workspace import re-optimizes mid-session and splits the browser hash"
    );
    for entry in ["react/jsx-dev-runtime", "react-dom/client"] {
        assert!(
            cfg.contains(entry),
            "optimizeDeps.include must list {entry}: it is imported by the workspace packages, \
             which are the ones that triggered the split"
        );
    }
}

#[tokio::test]
async fn income_plan_week_declared_positions_have_account_slices() {
    let dir = profile_a_app_dir();
    let db = dir.join("local.sqlite");
    assert!(db.is_file(), "data file missing at {}", db.display());
    let platform = LocalPlatform::open(&dir).await.expect("open data sqlite");
    let week = execute_query_on(
        &platform,
        &platform,
        qry(
            "IncomePlanWeekGet",
            serde_json::json!({"asOfDate": "2026-09-30"}),
        ),
    )
    .await;
    assert!(week.ok, "IncomePlanWeekGet {}", week.error_code.unwrap_or_default());
    let body: serde_json::Value =
        serde_json::from_str(week.body_json.as_deref().unwrap_or("{}")).unwrap();
    let positions = body["positions"].as_array().cloned().unwrap_or_default();
    let declared: Vec<_> = positions
        .iter()
        .filter(|p| p["declarationKnown"] == true)
        .collect();
    assert!(
        !declared.is_empty(),
        "this week should have declared names: {body}"
    );
    let sample = declared[0];
    let accts = sample["accounts"].as_array().cloned().unwrap_or_default();
    assert!(
        !accts.is_empty(),
        "declared {} must carry account slices so by-account Current variance can fill",
        sample["symbol"]
    );
    assert!(
        accts.iter().any(|a| a["declarationKnown"] == true && a["planKnown"] == true),
        "declared {} slices need declaration+plan: {accts:?}",
        sample["symbol"]
    );
}

/// Every golden in the gate must be announceable before it runs. The owner cannot see tool
/// calls, so an unannounced multi-minute suite is indistinguishable from a hang; a suite added
/// to the Pass block without a duration entry would be announced as unmeasured for ever.
#[test]
fn every_pass_suite_has_a_measured_duration() {
    let root = repo_root();
    let execution = std::fs::read_to_string(root.join("docs/architecture/execution.md"))
        .expect("execution.md");
    let record = std::fs::read_to_string(root.join("docs/architecture/golden-durations.json"))
        .expect("golden-durations.json");
    // A byte order mark is "expected value at 1:1" to serde_json. Any Windows editor can add
    // one, and losing the gate to an invisible character is not a useful failure.
    let record: serde_json::Value = serde_json::from_str(record.trim_start_matches('\u{feff}'))
        .expect("golden-durations.json parses");
    let suites = record["suites"]
        .as_object()
        .expect("golden-durations.json has a suites object");

    let mut gated: Vec<&str> = Vec::new();
    for chunk in execution.split("--test ").skip(1) {
        let name = chunk
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .trim_end_matches('`');
        if !name.is_empty() && !gated.contains(&name) {
            gated.push(name);
        }
    }
    assert!(
        !gated.is_empty(),
        "no --test suites found in execution.md, so this guard proved nothing"
    );

    let missing: Vec<&str> = gated
        .iter()
        .copied()
        .filter(|name| !suites.contains_key(*name))
        .collect();
    assert!(
        missing.is_empty(),
        "these gate suites have no entry in docs/architecture/golden-durations.json, so \
         scripts/golden.ps1 cannot say how long they take: {}",
        missing.join(", ")
    );

    assert!(
        root.join("scripts/golden.ps1").is_file(),
        "scripts/golden.ps1 is what announces and records the duration"
    );
}

/// Live screens, Screen Atlas, the catalog, and Template_UiModules are one set.
/// A failure means add the missing row. Do not delete a screen to force a match.
#[test]
fn live_screens_atlas_catalog_and_snapshot_are_one_set() {
    let root = repo_root();
    let app = std::fs::read_to_string(root.join("apps/desktop/src/App.tsx")).expect("App.tsx");
    let screens = union_literals(&app, "Screen");
    let desks = union_literals(&app, "CmDesk");
    let atlas = std::fs::read_to_string(
        root.join("apps/desktop/src/features/screen-atlas/atlasTargets.ts"),
    )
    .expect("atlas");
    let groups = std::fs::read_to_string(
        root.join("apps/desktop/src/features/components/screenGroups.ts"),
    )
    .expect("screen groups");
    let snapshot_src =
        std::fs::read_to_string(root.join("crates/application-core/src/data_snapshot.rs"))
            .expect("data_snapshot");
    let registry = std::fs::read_to_string(
        root.join("apps/desktop/src/features/components/ComponentRegistry.tsx"),
    )
    .expect("ComponentRegistry");
    let atlas_ui = std::fs::read_to_string(
        root.join("apps/desktop/src/features/screen-atlas/ScreenAtlasScreen.tsx"),
    )
    .expect("ScreenAtlasScreen");

    assert_eq!(
        screens,
        const_string_list(&groups, "SCREEN_ORDER"),
        "add the missing screen to screenGroups SCREEN_ORDER. Do not delete the screen."
    );
    assert_eq!(
        screens,
        const_string_list(&atlas, "ATLAS_SCREEN_IDS"),
        "add the missing screen to ATLAS_SCREEN_IDS. Do not delete the screen."
    );
    assert_eq!(
        screens,
        const_string_list(&snapshot_src, "UI_SCREEN_ORDER"),
        "add the missing screen to UI_SCREEN_ORDER. Do not delete the screen."
    );
    assert_eq!(
        desks,
        const_string_list(&groups, "DESK_ORDER"),
        "add the missing desk to DESK_ORDER. Do not delete the desk."
    );
    assert_eq!(
        desks,
        const_string_list(&snapshot_src, "UI_DESK_ORDER"),
        "add the missing desk to UI_DESK_ORDER. Do not delete the desk."
    );

    let atlas_screens = quoted_field(&atlas, "screen");
    let atlas_desks = quoted_field(&atlas, "cmDesk");
    for screen in &screens {
        assert!(
            atlas_screens.iter().any(|id| id == screen),
            "add the missing Screen Atlas row for `{screen}`. Do not delete the screen."
        );
    }
    for screen in &atlas_screens {
        assert!(
            screens.iter().any(|id| id == screen),
            "Screen Atlas names `{screen}`, which is not a live screen. Add the screen. Do not delete a live screen to force a match."
        );
    }
    for desk in &desks {
        assert!(
            atlas_desks.iter().any(|id| id == desk),
            "add the missing Screen Atlas row for desk `{desk}`. Do not delete the desk."
        );
    }
    for desk in &atlas_desks {
        assert!(
            desks.iter().any(|id| id == desk),
            "Screen Atlas names desk `{desk}`, which is not live. Add the desk. Do not delete a live desk to force a match."
        );
    }

    let catalog = core_functions_catalog().expect("catalog");
    for id in [
        "graphing",
        "new-investment-readiness",
        "shopping-cart",
        "cash-management",
        "cash-week-desk",
        "week-ahead",
        "household-income",
        "magi-forecast",
        "cash-elements",
        "cash-management-ytd",
        "cm-element-management",
        "cm-cashflow-manager",
        "cm-weekly-updates",
        "cm-car-account-tax",
        "cm-coverage",
        "cm-external",
        "home",
        "income-plan",
        "calculator",
        "market-impact",
        "dashboard",
        "trends",
        "position-details",
        "holdings",
        "add-position",
        "add-lot",
        "interest-rate",
        "contract-positions",
        "task-manager",
        "reevaluate-collector",
        "tickets",
        "collectors",
        "import",
        "lots",
        "settings",
        "components",
        "screen-atlas",
        "shell",
    ] {
        assert!(
            catalog.modules.iter().any(|module| module.id == id),
            "add the missing catalog row `{id}`. Do not delete a module to force a match."
        );
    }
    assert!(
        catalog
            .modules
            .iter()
            .any(|module| module.id == "cm-element-management" && module.title == "Planned Transactions"),
        "keep the Planned Transactions catalog title. Add a row; do not rename a shipped title away."
    );
    for module in &catalog.modules {
        if !module.screen.is_empty() {
            assert!(
                screens.iter().any(|screen| screen == &module.screen),
                "catalog module {} names screen `{}`, which is not live. Add the screen. Do not delete a live screen.",
                module.id,
                module.screen
            );
        }
        if !module.cm_desk.is_empty() {
            assert!(
                desks.iter().any(|desk| desk == &module.cm_desk),
                "catalog module {} names desk `{}`, which is not live. Add the desk. Do not delete a live desk.",
                module.id,
                module.cm_desk
            );
        }
    }
    for screen in &screens {
        assert!(
            catalog.modules.iter().any(|module| &module.screen == screen),
            "add the missing catalog row for screen `{screen}`. Do not delete the screen."
        );
    }
    for desk in &desks {
        assert!(
            catalog.modules.iter().any(|module| &module.cm_desk == desk),
            "add the missing catalog row for desk `{desk}`. Do not delete the desk."
        );
    }
    assert!(
        catalog.modules.iter().any(|module| module.id == "lots" && module.screen.is_empty())
            && catalog
                .modules
                .iter()
                .any(|module| module.id == "shell" && module.screen.is_empty()),
        "lots and shell stay in the catalog with no screen id"
    );

    let rows = application_core::data_snapshot::ui_module_sheet_rows().expect("module rows");
    let mut sheet_ids: Vec<_> = rows.iter().map(|row| row[2].clone()).collect();
    let mut catalog_ids: Vec<_> = catalog.modules.iter().map(|module| module.id.clone()).collect();
    sheet_ids.sort();
    catalog_ids.sort();
    assert_eq!(
        sheet_ids, catalog_ids,
        "Template_UiModules id set must equal the catalog. Add the missing row. Do not delete a module."
    );
    for screen in &screens {
        assert!(
            rows.iter().any(|row| &row[0] == screen),
            "add the missing Template_UiModules row for screen `{screen}`. Do not delete the screen."
        );
    }
    for desk in &desks {
        assert!(
            rows.iter().any(|row| &row[1] == desk),
            "add the missing Template_UiModules row for desk `{desk}`. Do not delete the desk."
        );
    }

    let bytes = application_core::data_snapshot::write_data_workbook(
        &[
            "screen",
            "cmDesk",
            "id",
            "title",
            "status",
            "folder",
            "menu areas",
            "sections",
        ],
        &rows,
    )
    .expect("workbook");
    let dir = tempfile::tempdir().expect("temp");
    let path = dir.path().join("Template_UiModules.xlsx");
    std::fs::write(&path, bytes).expect("write workbook");
    let read = import_engine::read_data_rows(&path).expect("read workbook");
    let mut read_ids: Vec<_> = read
        .iter()
        .filter_map(|row| row.get("id").cloned())
        .collect();
    read_ids.sort();
    assert_eq!(
        read_ids, catalog_ids,
        "Template_UiModules.xlsx id set must equal the catalog. Add the missing row. Do not delete a module."
    );
    for screen in &screens {
        assert!(
            read.iter()
                .any(|row| row.get("screen").map(String::as_str) == Some(screen.as_str())),
            "add the missing Template_UiModules.xlsx row for screen `{screen}`. Do not delete the screen."
        );
    }
    for desk in &desks {
        assert!(
            read.iter()
                .any(|row| row.get("cmDesk").map(String::as_str) == Some(desk.as_str())),
            "add the missing Template_UiModules.xlsx row for desk `{desk}`. Do not delete the desk."
        );
    }

    assert!(
        app.contains("<ComponentRegistry"),
        "App mounts the extracted component registry"
    );
    assert!(
        registry.contains("Also registered") && registry.contains("groupByScreen"),
        "Components groups by screen and keeps modules that have no screen id"
    );
    assert!(
        registry.contains("Add description")
            && registry.contains("Edit description")
            && registry.contains("Capture Page")
            && registry.contains("Export to Excel")
            && registry.contains("not proven")
            && registry.contains("Money rule"),
        "registry shows Add description, page Excel, capture, and the forensic record"
    );
    assert!(
        !registry.contains("description missing")
            && !registry.contains("Export to Excel ${line.name}"),
        "component lines do not print a missing description or their own Excel button"
    );
    assert!(
        registry.contains("function sectionGroups(")
            && registry.contains("aria-label={`Section ${group.label}`}")
            && registry.contains("UNSECTIONED"),
        "the registry groups components under their section and names the unplaced ones"
    );
    for module in &catalog.modules {
        assert!(
            application_core::component_export::export_covers(&module.id, ""),
            "add an export arm for module {}",
            module.id
        );
        for part in &module.parts {
            assert!(
                part.export.is_empty() || part.export == "none",
                "component {} on {} has export `{}`. Use `none` or leave it empty — \
                 a typo must not waive the export arm.",
                part.id,
                module.id,
                part.export
            );
            if part.export == "none" {
                continue;
            }
            assert!(
                application_core::component_export::export_covers(&module.id, &part.id),
                "add an export arm for part {} on {}, or set its export to `none`",
                part.id,
                module.id
            );
        }
        for cite in &module.sqlite_tables {
            let source = std::fs::read_to_string(root.join(&cite.path))
                .unwrap_or_else(|_| panic!("citation path {}", cite.path));
            assert!(
                source.contains(&cite.needle),
                "table {} is not proven in {}",
                cite.name,
                cite.path
            );
        }
    }
    assert!(
        atlas_ui.contains("groupByScreen") && atlas_ui.contains("Also registered"),
        "Screen Atlas uses the same groups, including Also registered"
    );
}

fn union_literals(src: &str, type_name: &str) -> Vec<String> {
    let marker = format!("type {type_name} =");
    let start = src
        .find(&marker)
        .unwrap_or_else(|| panic!("App.tsx missing {type_name}"));
    let rest = &src[start + marker.len()..];
    let end = rest.find(';').expect("union semicolon");
    quoted_words(&rest[..end])
}

fn const_string_list(src: &str, name: &str) -> Vec<String> {
    let marker = format!("const {name}");
    let start = src
        .find(&marker)
        .unwrap_or_else(|| panic!("missing {name}"));
    let rest = &src[start..];
    let eq = rest.find('=').unwrap_or_else(|| panic!("{name} equals"));
    let after = &rest[eq..];
    let open = after.find('[').unwrap_or_else(|| panic!("{name} array"));
    let close = after[open..].find(']').unwrap_or_else(|| panic!("{name} end"));
    quoted_words(&after[open..open + close])
}

fn quoted_field(src: &str, key: &str) -> Vec<String> {
    let needle = format!("{key}: \"");
    let mut out = Vec::new();
    let mut rest = src;
    while let Some(at) = rest.find(&needle) {
        rest = &rest[at + needle.len()..];
        let Some((value, next)) = rest.split_once('"') else {
            break;
        };
        rest = next;
        if !out.iter().any(|seen| seen == value) {
            out.push(value.to_string());
        }
    }
    out
}

fn attr_values(src: &str, attr: &str) -> Vec<String> {
    let needle = format!("{attr}=\"");
    let mut out = Vec::new();
    let mut rest = src;
    while let Some(at) = rest.find(&needle) {
        rest = &rest[at + needle.len()..];
        let Some((value, next)) = rest.split_once('"') else {
            break;
        };
        rest = next;
        out.push(value.to_string());
    }
    out
}

/// Every `.ts`/`.tsx` behind a module's `folder`. That is a directory once the
/// screen is extracted and a single file while it still sits in the shell.
fn module_sources(folder: &str) -> String {
    let path = repo_root().join(folder.trim_end_matches('/'));
    if path.is_file() {
        return std::fs::read_to_string(&path).unwrap_or_default();
    }
    let mut buf = String::new();
    let Ok(entries) = std::fs::read_dir(&path) else {
        return buf;
    };
    for entry in entries.flatten() {
        let file = entry.path();
        let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext == "ts" || ext == "tsx" {
            buf.push_str(&std::fs::read_to_string(&file).unwrap_or_default());
            buf.push('\n');
        }
    }
    buf
}

/// Ids declared by every module that reads from the same `folder`. Several
/// modules share one folder, so a marker there belongs to any of them.
fn declared_by_folder(
    catalog: &application_core::contracts::CoreFunctionsGetBody,
    pick: impl Fn(&application_core::contracts::UiModuleItem) -> Vec<String>,
) -> std::collections::HashMap<String, Vec<String>> {
    let mut out: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for module in &catalog.modules {
        out.entry(module.folder.clone())
            .or_default()
            .extend(pick(module));
    }
    out
}

/// Screen, then section, then the components inside it. The catalog is the
/// crosscheck, so a section with no anchor, an anchor with no section, or a
/// component sitting outside every section all fail here rather than leaving a
/// shortcut the owner cannot reach.
#[test]
fn every_catalog_section_is_anchored_and_holds_its_components() {
    let catalog = core_functions_catalog().expect("catalog");
    let by_folder = declared_by_folder(&catalog, |module| {
        module.sections.iter().map(|s| s.id.clone()).collect()
    });
    for module in &catalog.modules {
        let src = module_sources(&module.folder);
        let declared: Vec<&str> = module.sections.iter().map(|s| s.id.as_str()).collect();
        let folder_declared = by_folder.get(&module.folder).cloned().unwrap_or_default();
        for found in attr_values(&src, "data-section") {
            assert!(
                folder_declared.contains(&found),
                "{} has data-section=\"{found}\" with no catalog section. \
                 Register it under module `{}`. Do not delete the block.",
                module.folder,
                module.id
            );
        }
        if module.sections.is_empty() {
            continue;
        }
        for section in &module.sections {
            assert!(
                src.contains(&format!("data-section=\"{}\"", section.id)),
                "mark the {} block in {} with data-section=\"{}\"",
                section.title,
                module.folder,
                section.id
            );
            assert!(
                src.contains(&format!("id=\"{}\"", section.anchor)),
                "section `{}` on `{}` jumps to anchor `{}`, which is not in {}",
                section.id,
                module.id,
                section.anchor,
                module.folder
            );
        }
        for part in &module.parts {
            assert!(
                declared.contains(&part.section.as_str()),
                "component `{}` on `{}` names section `{}`, which `{}` does not declare",
                part.id,
                module.id,
                part.section,
                module.id
            );
        }
    }
}

/// A component the registry lists must be findable on the screen, and a marked
/// component must be in the registry. The reverse rule holds everywhere from
/// the start; a module comes under the forward rule once it declares sections,
/// which is how each screen batch joins the gate.
#[test]
fn every_registered_component_is_marked_on_its_screen() {
    let catalog = core_functions_catalog().expect("catalog");
    let by_folder = declared_by_folder(&catalog, |module| {
        module.parts.iter().map(|p| p.id.clone()).collect()
    });
    for module in &catalog.modules {
        let src = module_sources(&module.folder);
        let folder_declared = by_folder.get(&module.folder).cloned().unwrap_or_default();
        for found in attr_values(&src, "data-part") {
            assert!(
                folder_declared.contains(&found),
                "{} has data-part=\"{found}\" with no registered component. \
                 Add it to module `{}` in ui-modules.json. Do not delete the block.",
                module.folder,
                module.id
            );
        }
        if module.sections.is_empty() {
            continue;
        }
        for part in &module.parts {
            // A component rendered by a `.map` cannot carry a literal attribute —
            // the JSX writes `data-part={spec.part}`. It declares its id on the
            // spec row instead, which keeps the registry id beside the definition.
            let marked = src.contains(&format!("data-part=\"{}\"", part.id))
                || src.contains(&format!("part: \"{}\"", part.id));
            assert!(
                marked,
                "mark the {} block in {} with data-part=\"{}\", \
                 or give its spec row part: \"{}\"",
                part.title, module.folder, part.id, part.id
            );
        }
    }
}

/// The nav row reads a static map so no screen pays for a catalog query. It
/// only stays true while it matches the catalog section for section.
#[test]
fn page_nav_row_sections_match_the_catalog() {
    let root = repo_root();
    let nav = std::fs::read_to_string(root.join("apps/desktop/src/features/navigation/pageSections.ts"))
        .expect("pageSections.ts");
    let catalog = core_functions_catalog().expect("catalog");
    for module in &catalog.modules {
        if module.sections.is_empty() {
            continue;
        }
        let key = if module.cm_desk.is_empty() {
            module.screen.clone()
        } else {
            format!("{}/{}", module.screen, module.cm_desk)
        };
        assert!(
            nav.contains(&format!("\"{key}\": [")),
            "add a `{key}` entry to pageSections.ts for module `{}`",
            module.id
        );
        for section in &module.sections {
            assert!(
                nav.contains(&format!(
                    "{{ id: \"{}\", label: \"{}\", anchor: \"{}\" }}",
                    section.id, section.title, section.anchor
                )),
                "pageSections.ts is missing `{}` ({} -> #{}) for `{key}`",
                section.id,
                section.title,
                section.anchor
            );
        }
    }
    let app = std::fs::read_to_string(root.join("apps/desktop/src/App.tsx")).expect("App.tsx");
    let row = std::fs::read_to_string(root.join("apps/desktop/src/features/navigation/PageNavRow.tsx"))
        .expect("PageNavRow.tsx");
    assert!(
        app.contains("<PageNavRow") && row.contains("<ReturnToPrevious") && row.contains("<SectionNav"),
        "the page nav row carries Return and the section shortcuts on every screen"
    );
}

/// Named components Home and Trends must still register. This is the literal
/// form of `every_registered_component_is_marked_on_its_screen`, kept until
/// those two screens declare sections and carry `data-part`; the general rule
/// covers them from that point. Do not delete it before then.
#[test]
fn home_component_lines_are_the_mounted_screen() {
    let catalog = core_functions_catalog().expect("catalog");
    let titles_on = |screen: &str| -> Vec<String> {
        catalog
            .modules
            .iter()
            .filter(|module| module.screen == screen)
            .flat_map(|module| module.parts.iter().map(|part| part.title.clone()))
            .collect()
    };
    let home = titles_on("home");
    for title in [
        "Portfolio summary",
        "Refresh declarations",
        "Work Tickets",
        "Refresh last prices",
        "Income through",
        "Income through transactions",
        "Dividend Plan",
        "Account cash flow projection",
        "Account values",
        "Graphing period",
        "Fidelity",
        "Schwab Total",
        "Income",
        "FI Roth",
        "Car",
        "Health",
        "Speculation",
        "Account 9",
    ] {
        assert!(
            home.iter().any(|line| line == title),
            "Home is missing the mounted component `{title}`"
        );
    }
    assert!(
        !home.iter().any(|line| line == "Live by risk"),
        "Live by risk is mounted on Trends"
    );
    let trends = titles_on("trends");
    for title in [
        "Live by risk",
        "Live by risk level",
        "Live by risk allocation",
        "Declared vs Plan",
        "Planned weekly income",
        "Reported weekly income",
        "Dividends paid by month",
        "All Cash",
        "Monthly Dividends",
        "Total Fidelity & Schwab",
    ] {
        assert!(
            trends.iter().any(|line| line == title),
            "Trends is missing `{title}`"
        );
    }
    let page_titles = [
        "Home",
        "Income Plan",
        "Calculator",
        "Market impact planner",
        "Dashboard",
        "Trends",
        "Cash Management",
        "Shopping Cart",
        "Holdings",
        "Import",
        "Settings",
        "Add Investment",
        "Add Lot",
        "Position Details",
        "Collectors",
        "Tickets",
        "Reevaluate collector",
        "Task Manager",
        "Interest rate calculator",
        "Contract positions",
        "Field intent",
        "Roadmap",
        "Component Registry",
        "Element Management",
        "Cashflow Manager",
        "Week ahead planner",
        "Tax Planning",
        "Income vs Expense planner",
        "External accounts",
        "Debt planner",
    ];
    let part_titles: Vec<String> = catalog
        .modules
        .iter()
        .flat_map(|module| module.parts.iter().map(|part| part.title.clone()))
        .collect();
    let root = repo_root();
    let mut headings = Vec::new();
    for rel in [
        "apps/desktop/src/App.tsx",
        "apps/desktop/src/features/graphing/HomeAccountCharts.tsx",
        "apps/desktop/src/features/home/HomeDividendPlan.tsx",
        "apps/desktop/src/features/home/HomeScreen.tsx",
        "apps/desktop/src/features/cash/AccountCashFlow.tsx",
        "apps/desktop/src/features/income-plan/IncomePlanScreen.tsx",
        "apps/desktop/src/ImportWizard.tsx",
        "apps/desktop/src/features/components/ComponentRegistry.tsx",
    ] {
        let src = std::fs::read_to_string(root.join(rel)).expect(rel);
        for line in src.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("<h2>") {
                if let Some(text) = rest.strip_suffix("</h2>") {
                    headings.push(text.to_string());
                }
            }
        }
    }
    for heading in headings {
        if page_titles.contains(&heading.as_str()) {
            continue;
        }
        assert!(
            part_titles.iter().any(|title| title == &heading),
            "heading `{heading}` has no component line"
        );
    }
}

fn quoted_words(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = src;
    while let Some(at) = rest.find('"') {
        rest = &rest[at + 1..];
        let Some((value, next)) = rest.split_once('"') else {
            break;
        };
        rest = next;
        if !value.is_empty() {
            out.push(value.to_string());
        }
    }
    out
}
