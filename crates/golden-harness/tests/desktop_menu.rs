use application_core::contracts::{
    CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::queries::{execute_command_on, execute_query_on};
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

fn cmd(name: &str, body: serde_json::Value) -> CommandRequest {
    CommandRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        command_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: Some(body.to_string()),
        expected_version: None,
    }
}

/// Every FinanceClient query the desktop nav actually issues.
const MENU_QUERIES: &[&str] = &[
    "HealthGet",
    "CoreFunctionsGet",
    "ConfigGet",
    "HandoffStatusGet",
    "IncomePlanWeekGet",
    "IncomePlanGridGet",
    "DividendPerformanceGet",
    "DashboardBurndownGet",
    "HoldingsGet",
    "WorkTicketList",
    "ExceptionList",
    "DataSummaryGet",
    "CalculatorGet",
    "CashManagementWeekGet",
    "CashManagementRemindersGet",
    "CashManagementMonthGet",
    "DeclarationHistoryGet",
    "AccountList",
    "SecurityList",
    "PositionDetailsGet",
    "PositionMasterGet",
];

#[test]
fn app_execute_query_names_are_registered() {
    let app = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx")).unwrap();
    let queries = std::fs::read_to_string(
        repo_root().join("crates/application-core/src/queries.rs"),
    )
    .unwrap();
    let on = queries
        .split("pub async fn execute_query_on")
        .nth(1)
        .expect("execute_query_on");
    for name in MENU_QUERIES {
        assert!(
            app.contains(&format!("executeQuery(\"{name}\"")),
            "App.tsx must call {name}"
        );
        assert!(
            on.contains(&format!("\"{name}\"")),
            "execute_query_on must register {name}"
        );
    }
}

#[tokio::test]
async fn menu_queries_succeed_on_data_sqlite() {
    let dir = profile_a_app_dir();
    let db = dir.join("local.sqlite");
    assert!(
        db.is_file(),
        "data file missing at {}; run npm run data-seed",
        db.display()
    );
    let platform = LocalPlatform::open(&dir).await.expect("open data sqlite");
    let mut failures = Vec::new();
    for name in MENU_QUERIES {
        let body = if matches!(*name, "IncomePlanWeekGet" | "IncomePlanGridGet" | "DashboardBurndownGet") {
            serde_json::json!({"asOfDate": "2026-07-31"})
        } else if *name == "DividendPerformanceGet" {
            serde_json::json!({"asOfDate": "2026-07-31", "range": "ytd"})
        } else {
            serde_json::json!({})
        };
        let result = execute_query_on(&platform, &platform, qry(name, body)).await;
        if !result.ok {
            failures.push(format!(
                "{name}: {}",
                result.error_code.unwrap_or_else(|| "error".into())
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "menu queries failed: {}",
        failures.join("; ")
    );

    let securities = execute_query_on(
        &platform,
        &platform,
        qry("SecurityList", serde_json::json!({})),
    )
    .await;
    let securities_body: serde_json::Value =
        serde_json::from_str(securities.body_json.as_deref().unwrap_or("[]")).unwrap();
    let has_haky = securities_body
        .as_array()
        .into_iter()
        .flatten()
        .any(|s| s["symbol"].as_str() == Some("HAKY"));
    if !has_haky {
        // Include Package templates omit HAKY; the locked 40-name income fleet lives on the
        // household Profile A file. Seed-only DBs still exercise menu queries above.
        eprintln!("skip: planCount gate needs household Profile A (HAKY not in seed templates)");
        return;
    }

    let summary = execute_query_on(
        &platform,
        &platform,
        qry("DataSummaryGet", serde_json::json!({})),
    )
    .await;
    let body: serde_json::Value =
        serde_json::from_str(summary.body_json.as_deref().unwrap_or("{}")).unwrap();
    assert!(
        body["planCount"].as_u64().unwrap_or(0) >= 40,
        "Calculator plans not loaded: {body}"
    );
    let calc = execute_query_on(&platform, &platform, qry("CalculatorGet", serde_json::json!({})))
        .await;
    let calc_body: serde_json::Value =
        serde_json::from_str(calc.body_json.as_deref().unwrap_or("{}")).unwrap();
    assert!(
        calc_body["rows"].as_array().map(|a| a.len()).unwrap_or(0) >= 40,
        "CalculatorGet rows missing: {calc_body}"
    );
}

#[test]
fn last_prices_summary_replaces_refresh_banner() {
    let app = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx")).unwrap();
    assert!(
        !app.contains("Last prices updated:"),
        "refresh must not print recorded/skipped under the summary grid"
    );
    assert!(
        !app.contains("Applied issuer sources from provider:"),
        "issuer-source apply is not a last-price grid fact"
    );
    assert!(
        app.contains("Last refresh"),
        "Last prices cell must show last refresh date"
    );
    assert!(
        !app.contains("Stale still displays. Missing stays unknown."),
        "stale/unknown copy must not sit under Last prices"
    );
    assert!(
        app.contains("<dt>Income through</dt>"),
        "Home grid must show income current through date"
    );
    assert!(
        !app.contains("<dt>Accounts</dt>"),
        "Home grid must not show account count"
    );
    assert!(
        !app.contains("<dt>Last yield</dt>"),
        "Income through replaces Last yield"
    );
    assert!(
        app.contains("aria-label=\"Income through transactions\""),
        "Income through date must open the paid-dividend list"
    );
    assert!(
        app.contains("<th>Date</th>")
            && app.contains("<th>Acct</th>")
            && app.contains("<th>Symbol</th>"),
        "Income through dialog lists date, acct, symbol"
    );
    assert!(
        app.contains("income-tx-scroll"),
        "Income through list must scroll"
    );
    assert!(
        app.contains("aria-label=\"Income transaction period\""),
        "Income through list must offer a period dropdown"
    );
    assert!(app.contains("aria-label=\"Income transaction start\""));
    assert!(app.contains("aria-label=\"Income transaction end\""));
    assert!(
        app.contains("aria-label=\"Income transaction account\""),
        "Income through list must filter by account"
    );
    assert!(app.contains("All accounts"));
    let period = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/incomeTxPeriod.ts"),
    )
    .unwrap();
    assert!(period.contains(r#"label: "Today""#));
    assert!(period.contains(r#"label: "Current month""#));
    assert!(period.contains(r#"label: "Year to date""#));
    assert!(period.contains(r#"label: "Custom date range""#));
    assert!(
        period.contains(r#"{ period: "today" as const, today: "2026-09-11", startOn: "2026-09-11", endOn: "2026-09-11" }"#),
        "Today window must stay the local calendar day"
    );
    assert!(
        period.contains(r#"{ period: "month" as const, today: "2026-09-11", startOn: "2026-09-01", endOn: "2026-09-11" }"#),
        "Current month must stay calendar month"
    );
    assert!(
        period.contains(r#"{ period: "ytd" as const, today: "2026-09-11", startOn: "2026-01-01", endOn: "2026-09-11" }"#),
        "YTD must start 1 January"
    );
}

#[test]
fn native_and_in_app_menus_list_screens() {
    let root = repo_root();
    let app = std::fs::read_to_string(root.join("apps/desktop/src/App.tsx")).unwrap();
    let lib =
        std::fs::read_to_string(root.join("apps/desktop/src-tauri/src/lib.rs")).unwrap();
    let week =
        std::fs::read_to_string(root.join("packages/ui-components/src/week.ts")).unwrap();
    for needle in [
        "SubmenuBuilder::new(app, \"Income Plan\")",
        "SubmenuBuilder::new(app, \"Trends\")",
        "SubmenuBuilder::new(app, \"Plan\")",
        "SubmenuBuilder::new(app, \"Positions\")",
        "SubmenuBuilder::new(app, \"Data\")",
        "SubmenuBuilder::new(app, \"Tools\")",
        ".text(\"home\", \"Home\")",
        ".text(\"income-plan\", \"Income Plan\")",
        ".text(\"data-snapshot\", \"Save data snapshot\")",
        ".text(\"app-restart\", \"Restart Application\")",
        ".text(\"tickets\", \"Tickets\")",
        ".text(\"cash-management\", \"Cash Management\")",
        ".text(\"collector-establish\", \"Reevaluate collector\")",
        ".text(\"components\", \"Components\")",
        "finos-navigate",
    ] {
        assert!(lib.contains(needle), "native menu missing {needle}");
    }
    assert!(
        app.contains("aria-label=\"Home\""),
        "in-app menubar must include Home"
    );
    assert!(
        app.contains("aria-label=\"Income Plan\""),
        "in-app menubar must include Income Plan"
    );
    assert!(
        !app.contains("navButton(\"income-plan\", \"Income Plan\")"),
        "Income Plan is a top-level menubar item, not under Plan"
    );
    assert!(
        !app.contains("Print current view"),
        "Print current view is not a File menu item"
    );
    assert!(
        !app.contains("Export current view"),
        "Export current view is not a File menu item"
    );
    assert!(
        !lib.contains("income-print") && !lib.contains("income-export"),
        "native File menu must not include print/export current view"
    );
    assert!(
        app.contains("Save data snapshot"),
        "in-app File menu must include Save data snapshot"
    );
    assert!(
        app.contains("aria-label=\"Confirm save data snapshot\""),
        "Save data snapshot must ask before writing"
    );
    assert!(
        app.contains("Restart Application"),
        "in-app File menu must include Restart Application"
    );
    assert!(
        app.contains("Waiting for in-flight work, then closing the data file"),
        "Restart must wait for in-flight work before closing SQLite"
    );
    assert!(
        app.contains("aria-label=\"Restart in progress\""),
        "Restart must show an on-screen in-progress banner"
    );
    assert!(
        lib.contains("close_for_shutdown"),
        "native Restart must close the live SQLite pool"
    );
    assert!(
        lib.contains("data file still open"),
        "Restart must fail if the data file did not close"
    );
    assert!(
        app.contains("navButton(\"collector-establish\", \"Reevaluate collector\")"),
        "in-app Tools must include Reevaluate collector"
    );
    assert!(
        app.contains("navButton(\"cash-management\", \"Cash Management\")"),
        "in-app Plan must include Cash Management"
    );
    assert!(
        !app.contains("navButton(\"trends\", \"Trends\")"),
        "Trends is a top-level menubar item, not under Plan"
    );
    let plan_native = lib
        .split("SubmenuBuilder::new(app, \"Plan\")")
        .nth(1)
        .and_then(|rest| rest.split("SubmenuBuilder::new(app, \"Positions\")").next())
        .unwrap_or("");
    assert!(
        !plan_native.contains(".text(\"trends\""),
        "native Plan must not list Trends"
    );
    assert!(
        app.contains("formatMenuWeek"),
        "menubar must show the current week without weekday names"
    );
    assert!(
        week.contains("W36") || week.contains("formatMenuWeek"),
        "formatMenuWeek must exist"
    );
    assert!(
        week.contains("${formatWeekNumber(id)} ${id.start} – ${id.end}"),
        "menu week must be dates only"
    );
    let bat = std::fs::read_to_string(root.join("apps/desktop/start-finos-dev.bat")).unwrap();
    assert!(
        bat.contains("wait_port_free") && bat.contains("1420"),
        "dev start must wait until Vite port 1420 is free"
    );
    let supervisor =
        std::fs::read_to_string(root.join("apps/desktop/start-finos-supervisor.bat")).unwrap();
    assert!(
        supervisor.contains("finos supervisor")
            && supervisor.contains("restart.token")
            && supervisor.contains("stale restart.token")
            && supervisor.contains("Start-Process -FilePath")
            && supervisor.contains("start-finos-dev.bat"),
        "supervisor must Start-Process start-finos-dev.bat on restart.token"
    );
    assert!(
        !supervisor.contains("call start-finos-dev"),
        "supervisor must not call start-finos-dev.bat as a child"
    );
    assert!(
        lib.contains("restart.token")
            && lib.contains("close_for_shutdown")
            && lib.contains("ensure_coding_supervisor")
            && lib.contains("spawn_installed_release_relaunch"),
        "coding Restart writes restart.token, starts the supervisor if missing, then exits; installed release spawn_installed_release_relaunch-es the current exe"
    );
    for banned in [
        "schtasks",
        "FinosDevRestart",
        "wmic",
        "Invoke-CimMethod",
        "spawn_dev_stack",
    ] {
        assert!(
            !lib.contains(banned),
            "lib.rs must not contain {banned}"
        );
    }
    assert!(
        app.contains("supervisor is running"),
        "Restart copy must name the supervisor"
    );
    assert!(
        app.contains("Installed release relaunches this app"),
        "Restart copy must name installed release relaunch"
    );
    assert!(
        !root.join("apps/desktop/spawn-finos-dev.cmd").exists(),
        "spawn-finos-dev.cmd must be deleted"
    );
}

/// Hard gate: installed-release Restart must Start-Process / spawn the current
/// exe **before** any window destroy or `app.exit`, and must not rely on
/// `app.restart()` / `restart_on_exit` alone (off-thread Exit can quit with no
/// child). Source-shape only — not owner Windows NSIS relaunch proof.
#[test]
fn installed_release_restart_must_spawn_exe_before_exit() {
    let root = repo_root();
    let lib = std::fs::read_to_string(root.join("apps/desktop/src-tauri/src/lib.rs"))
        .expect("lib.rs");
    assert!(
        lib.contains("fn spawn_installed_release_relaunch"),
        "installed relaunch helper must exist"
    );
    assert!(
        lib.contains("Start-Process -FilePath"),
        "Windows installed relaunch must use PowerShell Start-Process (coding-supervisor style)"
    );
    assert!(
        lib.contains("current_exe()"),
        "installed relaunch must resolve the running exe path"
    );
    let start = lib
        .find("async fn app_restart")
        .expect("app_restart command must exist");
    let body = &lib[start..];
    let end = body
        .find("\nfn supervisor_lock_dir")
        .or_else(|| body.find("\nfn coding_supervisor_running"))
        .expect("app_restart must be followed by supervisor helpers");
    let fn_src = &body[..end];
    assert!(
        fn_src.contains("cfg!(debug_assertions)"),
        "coding vs installed release must branch on debug_assertions"
    );
    assert!(
        fn_src.contains("Do not call app.restart()"),
        "lib.rs must keep the installed-release spawn-before-exit comment"
    );
    assert!(
        !fn_src.contains("app.restart();"),
        "installed release must not call app.restart(); spawn then exit instead"
    );
    let debug_if = fn_src
        .find("if cfg!(debug_assertions)")
        .expect("app_restart must branch on debug_assertions");
    let before_debug = &fn_src[..debug_if];
    assert!(
        !before_debug.contains("window.destroy()")
            && !before_debug.contains("webview_windows()")
            && !before_debug.contains("spawn_installed_release_relaunch"),
        "must not destroy windows or spawn relaunch before the coding/release branch"
    );
    let after_debug_return = fn_src[debug_if..]
        .rsplit("return Ok(());")
        .next()
        .expect("coding branch must return Ok after exit");
    let spawn_at = after_debug_return
        .find("spawn_installed_release_relaunch()?")
        .expect("installed release must call spawn_installed_release_relaunch before exit");
    let after_spawn = &after_debug_return[spawn_at..];
    assert!(
        after_spawn.contains("window.destroy()"),
        "installed release may destroy windows only after spawn succeeds"
    );
    assert!(
        after_spawn.contains("app.exit(0)"),
        "installed release must exit after spawn"
    );
    let destroy_before_spawn = &after_debug_return[..spawn_at];
    assert!(
        !destroy_before_spawn.contains("window.destroy()")
            && !destroy_before_spawn.contains("webview_windows()")
            && !destroy_before_spawn.contains("app.exit"),
        "must not destroy windows or exit before spawn_installed_release_relaunch"
    );
    let destroy_count = fn_src.matches("window.destroy()").count();
    assert_eq!(
        destroy_count, 2,
        "window.destroy() once in coding path and once after installed spawn; found {destroy_count}"
    );
    assert!(
        !fn_src.to_ascii_lowercase().contains("household release"),
        "Restart comments must say installed release, not household slang"
    );
}

#[tokio::test]
async fn settings_config_set_is_registered() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let settings = execute_command_on(
        &platform,
        &platform,
        cmd("ConfigSet", serde_json::json!({"deviceName": "menu-test"})),
    )
    .await;
    assert!(
        settings.ok,
        "Settings ConfigSet failed: {:?}",
        settings.error_code
    );
}
