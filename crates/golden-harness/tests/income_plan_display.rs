//! Income Plan display goldens G-IP-01..10 and G-IP-P01..P10.

use application_core::contracts::{
    CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::ports::canonical::Canonical;
use application_core::queries::{execute_command_on, execute_query_on};
use golden_harness::repo_root;
use storage_sqlite::LocalPlatform;
use uuid::Uuid;

const PACK: &str =
    "docs/Authority/Income_Plan_Display_Cursor_Pack_2026-09-08/income_plan_display_golden_pack_2026-09-08.json";
const HTML_MOCK: &str = "Income_Plan_Pattern_A_B_Simulation.html";
const AS_OF: &str = "2026-09-08";

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
            "calendarPolicy": "issuer_calendar",
            "collectorEnabled": true,
            "lookbackCount": 12
        }),
    )
    .await;
}

async fn confirm_plan(
    platform: &LocalPlatform,
    security_id: &str,
    symbol: &str,
    cadence: &str,
    amount_minor: i64,
    scale: u8,
    periods: u8,
    effective_from: &str,
) {
    research_template(platform, security_id, symbol).await;
    must_ok(
        platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": cadence,
            "replaceCadence": true,
            "riskTier": "Core"
        }),
    )
    .await;
    must_ok(
        platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": amount_minor,
            "amountScale": scale,
            "paymentPeriod": "2026-07-15",
            "source": "provider-site",
            "enteredAt": "2026-08-01"
        }),
    )
    .await;
    must_ok(
        platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": amount_minor,
            "amountScale": scale,
            "planningPeriodsPerYear": periods,
            "effectiveFrom": effective_from,
            "decisionReason": "Most Current",
            "incompleteAnalysisReason": "Fewer than 6 observations"
        }),
    )
    .await;
}

async fn open_lot(
    platform: &LocalPlatform,
    account_id: &serde_json::Value,
    security_id: &str,
    qty: i64,
    key: &str,
) {
    golden_harness::complete_collector_for_first_lot_as(
        platform,
        security_id,
        key,
        match key {
            "WEEK1" => "Weekly",
            "QTR1" => "Quarterly",
            _ => "Monthly",
        },
        false,
    )
    .await
    .expect("complete collector");
    must_ok(
        platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "openedOn": "2026-01-02",
            "origin": "purchase",
            "quantityMinor": qty,
            "quantityScale": 0,
            "performanceBasisMinor": 100_000,
            "taxBasisMinor": 100_000,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;
}

#[allow(dead_code)]
struct PackIds {
    week1: String,
    mon1: String,
    qtr1: String,
    cash1: String,
}

async fn seed_pack(platform: &LocalPlatform) -> PackIds {
    for name in [
        "Income",
        "Car",
        "Health",
        "FI Roth",
        "9",
        "Speculation",
        "Energy",
        "Robinhood",
    ] {
        must_ok(
            platform,
            "AccountRegister",
            serde_json::json!({"name": name, "kind": "taxable"}),
        )
        .await;
    }
    let accounts = query_json(platform, "AccountList", serde_json::json!({})).await;
    let id_for = |name: &str| {
        accounts
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["name"] == name)
            .unwrap()["accountId"]
            .clone()
    };
    let car = id_for("Car");
    let income = id_for("Income");
    let nine = id_for("9");

    let week1 = must_ok(
        platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "WEEK1", "name": "WEEK1"}),
    )
    .await;
    let week1_id = week1["securityId"].as_str().unwrap().to_string();
    confirm_plan(platform, &week1_id, "WEEK1", "Weekly", 15, 2, 52, "2026-01-01").await;
    must_ok(
        platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": week1_id,
            "amountPerShareMinor": 17,
            "amountScale": 2,
            "planningPeriodsPerYear": 52,
            "effectiveFrom": "2026-08-15",
            "decisionReason": "Most Current",
            "incompleteAnalysisReason": "Fewer than 6 observations"
        }),
    )
    .await;
    open_lot(platform, &car, &week1_id, 100, "WEEK1").await;

    let mon1 = must_ok(
        platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "MON1", "name": "MON1"}),
    )
    .await;
    let mon1_id = mon1["securityId"].as_str().unwrap().to_string();
    confirm_plan(platform, &mon1_id, "MON1", "Monthly", 100, 2, 12, "2026-01-01").await;
    open_lot(platform, &income, &mon1_id, 10, "MON1").await;
    must_ok(
        platform,
        "IssuerPayDateReplace",
        serde_json::json!({
            "securityId": mon1_id,
            "asOfDate": "2026-01-01",
            "dates": [{"payOn": "2026-08-28", "source": "test"}]
        }),
    )
    .await;

    let qtr1 = must_ok(
        platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "QTR1", "name": "QTR1"}),
    )
    .await;
    let qtr1_id = qtr1["securityId"].as_str().unwrap().to_string();
    confirm_plan(platform, &qtr1_id, "QTR1", "Quarterly", 50, 2, 4, "2026-01-01").await;
    open_lot(platform, &income, &qtr1_id, 20, "QTR1").await;
    must_ok(
        platform,
        "IssuerPayDateReplace",
        serde_json::json!({
            "securityId": qtr1_id,
            "asOfDate": "2026-01-01",
            "dates": [{"payOn": "2026-08-14", "source": "test"}]
        }),
    )
    .await;

    let cash1 = must_ok(
        platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "CASH1", "name": "CASH1"}),
    )
    .await;
    let cash1_id = cash1["securityId"].as_str().unwrap().to_string();
    confirm_plan(platform, &cash1_id, "CASH1", "Monthly", 10, 2, 12, "2026-01-01").await;
    open_lot(platform, &nine, &cash1_id, 1000, "CASH1").await;

    PackIds {
        week1: week1_id,
        mon1: mon1_id,
        qtr1: qtr1_id,
        cash1: cash1_id,
    }
}

fn grid_body(accounts: &[&str], week_ending: &str) -> serde_json::Value {
    serde_json::json!({
        "asOfDate": AS_OF,
        "historicalWeeks": 6,
        "futureWeeks": 6,
        "accounts": accounts,
        "weekEnding": week_ending
    })
}

fn table2_row<'a>(grid: &'a serde_json::Value, symbol: &str) -> Option<&'a serde_json::Value> {
    grid["table2"].as_array()?.iter().find_map(|g| {
        g["rows"]
            .as_array()?
            .iter()
            .find(|r| r["symbol"] == symbol)
    })
}

fn cell_for<'a>(row: &'a serde_json::Value, week_end: &str) -> Option<&'a serde_json::Value> {
    row["cells"]
        .as_array()?
        .iter()
        .find(|c| c["weekEnd"] == week_end)
}

fn walk_src(root: &std::path::Path, needle: &str) -> Vec<String> {
    let mut hits = Vec::new();
    let dirs = [
        root.join("apps/desktop/src"),
        root.join("apps/desktop/src-tauri"),
        root.join("packages"),
        root.join("crates"),
    ];
    for dir in dirs {
        fn rec(path: &std::path::Path, needle: &str, hits: &mut Vec<String>) {
            if path.is_dir() {
                if let Ok(rd) = std::fs::read_dir(path) {
                    for e in rd.flatten() {
                        rec(&e.path(), needle, hits);
                    }
                }
                return;
            }
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name == needle {
                hits.push(path.display().to_string());
            }
        }
        rec(&dir, needle, &mut hits);
    }
    hits
}

#[test]
fn pack_lists_every_golden_id() {
    let pack: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo_root().join(PACK)).expect("golden pack"),
    )
    .unwrap();
    let ids: Vec<String> = pack["goldens"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["id"].as_str().unwrap().to_string())
        .collect();
    for n in 1..=10 {
        assert!(ids.iter().any(|id| id == &format!("G-IP-{n:02}")), "missing G-IP-{n:02}");
        assert!(
            ids.iter().any(|id| id == &format!("G-IP-P{n:02}")),
            "missing G-IP-P{n:02}"
        );
    }
    let banned = pack["banned_in_tests"].as_str().unwrap_or_default();
    assert!(banned.contains("live household"));
}

#[tokio::test]
async fn g_ip_01_table1_has_no_last_update() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    seed_pack(&platform).await;
    let grid = query_json(
        &platform,
        "IncomePlanGridGet",
        grid_body(&["Income", "Car", "Health", "FI Roth", "9"], "2026-09-04"),
    )
    .await;
    let cols = grid["table1Columns"].as_array().unwrap();
    assert_eq!(cols[0], "label");
    assert!(!cols.iter().any(|c| c.as_str().unwrap_or("").to_ascii_lowercase().contains("last")));
    assert!(!application_core::income_plan_display::table1_has_last_update(
        &serde_json::from_value(grid.clone()).unwrap()
    ));
}

#[tokio::test]
async fn g_ip_02_table2_last_update_replaces_qty_total_annual() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    seed_pack(&platform).await;
    let grid = query_json(
        &platform,
        "IncomePlanGridGet",
        grid_body(&["Income", "Car", "Health", "FI Roth", "9"], "2026-09-04"),
    )
    .await;
    assert!(!grid.to_string().contains("ytd_total"));
    assert!(!grid.to_string().contains("annual_pct"));
    assert!(!grid.to_string().contains("quantity"));
    let row = table2_row(&grid, "WEEK1").expect("WEEK1");
    assert!(row.get("lastUpdate").is_some());
}

#[tokio::test]
async fn g_ip_03_plan_lock_past_weeks() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    seed_pack(&platform).await;
    let grid = query_json(
        &platform,
        "IncomePlanGridGet",
        grid_body(&["Car"], "2026-08-14"),
    )
    .await;
    let row = table2_row(&grid, "WEEK1").expect("WEEK1");
    let c14 = cell_for(row, "2026-08-14").expect("08-14");
    let c21 = cell_for(row, "2026-08-21").expect("08-21");
    assert_eq!(c14["amountMinor"].as_i64(), Some(1500), "{c14}");
    assert_eq!(c21["amountMinor"].as_i64(), Some(1700), "{c21}");
}

#[tokio::test]
async fn g_ip_04_monthly_only_pay_week() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    seed_pack(&platform).await;
    let grid = query_json(
        &platform,
        "IncomePlanGridGet",
        grid_body(&["Income", "Car", "Health", "FI Roth", "9"], "2026-08-28"),
    )
    .await;
    let row = table2_row(&grid, "MON1").expect("MON1");
    let pay = cell_for(row, "2026-08-28").expect("pay week");
    assert!(pay["amountMinor"].as_i64().unwrap_or(0) > 0, "{pay}");
    for end in ["2026-08-14", "2026-08-21", "2026-09-04"] {
        if let Some(cell) = cell_for(row, end) {
            assert!(
                cell["amountMinor"].is_null(),
                "MON1 {end} should be empty: {cell}"
            );
            assert_ne!(cell["tone"], "miss", "off-cycle must not be red");
        }
    }
}

#[tokio::test]
async fn g_ip_05_future_actual_blank() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    seed_pack(&platform).await;
    let grid = query_json(
        &platform,
        "IncomePlanGridGet",
        grid_body(&["Income", "Car", "Health", "FI Roth", "9"], "2026-09-11"),
    )
    .await;
    for id in ["total_actual", "total_difference"] {
        let row = grid["table1"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == id)
            .unwrap();
        let cell = cell_for(row, "2026-09-11").expect("09-11");
        assert!(cell["amountMinor"].is_null(), "{id} {cell}");
    }
}

#[tokio::test]
async fn g_ip_06_account_filter_car_only() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    seed_pack(&platform).await;
    let grid = query_json(&platform, "IncomePlanGridGet", grid_body(&["Car"], "2026-09-04")).await;
    let labels: Vec<String> = grid["table1"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["label"].as_str().unwrap().to_string())
        .collect();
    assert!(labels.iter().any(|l| l.contains("Car")));
    assert!(!labels.iter().any(|l| l.contains("Income")));
    assert!(!labels.iter().any(|l| l.contains("Health")));
    assert!(!labels.iter().any(|l| l.contains("Roth") || l.contains("FI Roth")));
    assert!(!labels.iter().any(|l| l == "Plan 9" || l == "Actual 9"));
    assert!(table2_row(&grid, "WEEK1").is_some());
    assert!(table2_row(&grid, "MON1").is_none());
}

#[tokio::test]
async fn g_ip_07_week_nav_a_to_b_to_a() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    seed_pack(&platform).await;
    let week = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({ "asOfDate": "2026-09-04" }),
    )
    .await;
    assert_eq!(week["end"], "2026-09-04");
    let grid = query_json(
        &platform,
        "IncomePlanGridGet",
        grid_body(&["Income", "Car", "Health", "FI Roth", "9"], "2026-09-04"),
    )
    .await;
    assert_eq!(grid["selectedWeekEnd"], "2026-09-04");
    let app = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx")).unwrap();
    assert!(app.contains("setIncomePattern(\"B\")"));
    assert!(app.contains("setIncomePattern(\"A\")"));
    assert!(app.contains("incomeWeek?.end"));
}

#[tokio::test]
async fn g_ip_08_last_update_success_only() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let ids = seed_pack(&platform).await;
    let sid = Uuid::parse_str(&ids.week1).unwrap();
    platform
        .retrieval_template_touch_run(
            sid,
            false,
            "failed".into(),
            "2026-09-01T00:00:00Z".into(),
            String::new(),
            "",
        )
        .await
        .unwrap();
    let grid = query_json(
        &platform,
        "IncomePlanGridGet",
        grid_body(&["Car"], "2026-09-04"),
    )
    .await;
    let row = table2_row(&grid, "WEEK1").expect("WEEK1");
    assert!(row["lastUpdate"].is_null(), "{row}");
}

#[tokio::test]
async fn g_ip_09_default_accounts_exclude_speculation() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    seed_pack(&platform).await;
    let grid = query_json(
        &platform,
        "IncomePlanGridGet",
        serde_json::json!({
            "asOfDate": AS_OF,
            "historicalWeeks": 6,
            "futureWeeks": 6,
            "accounts": [],
            "weekEnding": "2026-09-04"
        }),
    )
    .await;
    let selected = grid["selectedAccounts"].as_array().unwrap();
    for off in ["Speculation", "Energy", "Robinhood"] {
        assert!(
            !selected.iter().any(|s| s == off),
            "{off} must start off: {selected:?}"
        );
    }
    let ui = std::fs::read_to_string(repo_root().join("packages/ui-components/src/index.tsx")).unwrap();
    assert!(ui.contains("INCOME_PLAN_DEFAULT_ACCOUNTS"));
    assert!(ui.contains("Speculation"));
}

#[test]
fn g_ip_10_html_not_imported() {
    let root = repo_root();
    let hits = walk_src(&root, HTML_MOCK);
    assert!(
        hits.is_empty(),
        "companion HTML must not ship under src/ or Tauri: {hits:?}"
    );
    let tauri_conf = std::fs::read_to_string(root.join("apps/desktop/src-tauri/tauri.conf.json")).unwrap();
    assert!(!tauri_conf.contains(HTML_MOCK));
}

#[tokio::test]
async fn g_ip_p01_pdf_pattern_a_contains_both_tables_not_b() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    seed_pack(&platform).await;
    let exp = query_json(
        &platform,
        "IncomePlanExportGet",
        serde_json::json!({
            "pattern": "A",
            "format": "pdf",
            "asOfDate": AS_OF,
            "historicalWeeks": 6,
            "futureWeeks": 6,
            "accounts": ["Income", "Car", "Health", "FI Roth", "9"],
            "printedAt": "2026-09-08T12:00Z"
        }),
    )
    .await;
    let pdf_bytes = b64_decode(exp["bytesBase64"].as_str().unwrap());
    let pdf = String::from_utf8_lossy(&pdf_bytes);
    assert!(pdf.contains("AccountRollup"));
    assert!(pdf.contains("PositionGrid"));
    assert!(!pdf.contains("WeekDetail"));
    assert!(!pdf.contains("Declaration $"));
    assert_eq!(exp["cover"]["pattern"], "A");
    assert_eq!(exp["landscape"], true);
}

#[tokio::test]
async fn g_ip_p02_pdf_pattern_b_single_week() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    seed_pack(&platform).await;
    let exp = query_json(
        &platform,
        "IncomePlanExportGet",
        serde_json::json!({
            "pattern": "B",
            "format": "pdf",
            "asOfDate": AS_OF,
            "weekEnding": "2026-09-04",
            "historicalWeeks": 6,
            "futureWeeks": 6,
            "accounts": ["Income", "Car"],
            "printedAt": "2026-09-08T12:00Z"
        }),
    )
    .await;
    let pdf_bytes = b64_decode(exp["bytesBase64"].as_str().unwrap());
    let pdf = String::from_utf8_lossy(&pdf_bytes);
    assert!(pdf.contains("WeekDetail"));
    assert!(pdf.contains("week-ending: 2026-09-04"));
    assert!(!pdf.contains("AccountRollup"));
}

#[tokio::test]
async fn g_ip_p03_print_respects_car_filter() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    seed_pack(&platform).await;
    let exp = query_json(
        &platform,
        "IncomePlanExportGet",
        serde_json::json!({
            "pattern": "A",
            "format": "html",
            "asOfDate": AS_OF,
            "accounts": ["Car"],
            "historicalWeeks": 6,
            "futureWeeks": 6,
            "printedAt": "2026-09-08T12:00Z"
        }),
    )
    .await;
    let html = exp["printHtml"].as_str().unwrap();
    assert!(html.contains("Car"));
    assert!(exp["cover"]["selectedAccounts"]
        .as_array()
        .unwrap()
        .iter()
        .all(|a| a == "Car"));
    assert!(!html.contains("Plan Income"));
}

#[tokio::test]
async fn g_ip_p04_excel_future_actual_blank() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    seed_pack(&platform).await;
    let exp = query_json(
        &platform,
        "IncomePlanExportGet",
        serde_json::json!({
            "pattern": "A",
            "format": "xlsx",
            "asOfDate": AS_OF,
            "accounts": ["Income", "Car", "Health", "FI Roth", "9"],
            "historicalWeeks": 6,
            "futureWeeks": 6,
            "printedAt": "2026-09-08T12:00Z"
        }),
    )
    .await;
    let html = exp["printHtml"].as_str().unwrap();
    assert!(html.contains("Total Actual"));
    let grid = query_json(
        &platform,
        "IncomePlanGridGet",
        grid_body(&["Income", "Car", "Health", "FI Roth", "9"], "2026-09-11"),
    )
    .await;
    let actual = grid["table1"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "total_actual")
        .unwrap();
    let cell = cell_for(actual, "2026-09-11").unwrap();
    assert!(cell["amountMinor"].is_null());
    assert!(
        html.contains("data-row=\"total_actual\" data-week-end=\"2026-09-11\"></td>"),
        "future Total Actual must be empty in the print snapshot"
    );
    assert!(
        !html.contains("data-row=\"total_actual\" data-week-end=\"2026-09-11\">0.00"),
        "future Total Actual must not serialize as 0.00"
    );
    assert!(
        html.contains("data-row=\"total_difference\" data-week-end=\"2026-09-11\"></td>"),
        "future Total Difference must be empty in the print snapshot"
    );
}

#[tokio::test]
async fn g_ip_p05_excel_table1_no_last_update() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    seed_pack(&platform).await;
    let exp = query_json(
        &platform,
        "IncomePlanExportGet",
        serde_json::json!({
            "pattern": "A",
            "format": "xlsx",
            "asOfDate": AS_OF,
            "accounts": ["Car"],
            "historicalWeeks": 6,
            "futureWeeks": 6,
            "printedAt": "2026-09-08T12:00Z"
        }),
    )
    .await;
    assert_eq!(
        exp["sheetNames"],
        serde_json::json!(["AccountRollup", "PositionGrid", "Cover"])
    );
    let html = exp["printHtml"].as_str().unwrap();
    let rollup = html.split("PositionGrid").next().unwrap();
    assert!(!rollup.to_ascii_lowercase().contains("last_update"));
}

#[tokio::test]
async fn g_ip_p06_excel_table2_has_last_update_not_qty() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    seed_pack(&platform).await;
    let exp = query_json(
        &platform,
        "IncomePlanExportGet",
        serde_json::json!({
            "pattern": "A",
            "format": "xlsx",
            "asOfDate": AS_OF,
            "accounts": ["Car"],
            "historicalWeeks": 6,
            "futureWeeks": 6,
            "printedAt": "2026-09-08T12:00Z"
        }),
    )
    .await;
    let html = exp["printHtml"].as_str().unwrap();
    let grid = html.split("PositionGrid").nth(1).unwrap_or("");
    assert!(grid.contains("last_update"));
    assert!(!grid.contains("quantity"));
    assert!(!grid.contains("ytd_total"));
    assert!(!grid.contains("annual_pct"));
}

#[test]
fn g_ip_p07_file_menu_delegates() {
    let app = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx")).unwrap();
    assert!(!app.contains("Print current view"));
    assert!(!app.contains("Export current view"));
    assert!(app.contains("runIncomeExport(\"print\")"));
    assert!(app.contains("aria-label=\"Print to page\""));
}

#[test]
fn g_ip_p08_html_not_print_engine() {
    let root = repo_root();
    for rel in [
        "crates/application-core/src/income_plan_display.rs",
        "crates/application-core/src/queries.rs",
        "apps/desktop/src/App.tsx",
        "packages/ui-components/src/index.tsx",
    ] {
        let src = std::fs::read_to_string(root.join(rel)).unwrap();
        assert!(
            !src.contains(HTML_MOCK) || src.contains("must not"),
            "{rel} must not invoke companion HTML as print engine"
        );
    }
    let display =
        std::fs::read_to_string(root.join("crates/application-core/src/income_plan_display.rs"))
            .unwrap();
    assert!(display.contains("must not invoke the companion HTML"));
}

#[tokio::test]
async fn g_ip_p09_pdf_write_local_first() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    seed_pack(&platform).await;
    let exp = query_json(
        &platform,
        "IncomePlanExportGet",
        serde_json::json!({
            "pattern": "A",
            "format": "pdf",
            "asOfDate": AS_OF,
            "accounts": ["Car"],
            "printedAt": "2026-09-08T12:00Z"
        }),
    )
    .await;
    assert_eq!(exp["destinationKind"], "local");
    let lib = std::fs::read_to_string(repo_root().join("apps/desktop/src-tauri/src/lib.rs")).unwrap();
    assert!(lib.contains("save_local_bytes"));
    assert!(lib.contains("finos-exports"));
    assert!(!lib.to_ascii_lowercase().contains("drive.google"));
}

#[tokio::test]
async fn g_ip_p10_cover_lists_filter_state() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    seed_pack(&platform).await;
    let exp = query_json(
        &platform,
        "IncomePlanExportGet",
        serde_json::json!({
            "pattern": "B",
            "format": "xlsx",
            "asOfDate": AS_OF,
            "weekEnding": "2026-09-04",
            "accounts": ["Car", "Income"],
            "historicalWeeks": 6,
            "futureWeeks": 6,
            "printedAt": "2026-09-08T12:00Z"
        }),
    )
    .await;
    assert_eq!(exp["cover"]["pattern"], "B");
    assert_eq!(exp["cover"]["printedAt"], "2026-09-08T12:00Z");
    assert_eq!(exp["cover"]["weekEnding"], "2026-09-04");
    assert_eq!(exp["cover"]["historicalWeeks"], 6);
    assert_eq!(exp["cover"]["futureWeeks"], 6);
    assert_eq!(
        exp["sheetNames"],
        serde_json::json!(["WeekDetail", "Cover"])
    );
}

fn b64_decode(s: &str) -> Vec<u8> {
    fn val(c: u8) -> u8 {
        match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => 0,
        }
    }
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 3 < bytes.len() {
        let n = ((val(bytes[i]) as u32) << 18)
            | ((val(bytes[i + 1]) as u32) << 12)
            | ((val(bytes[i + 2]) as u32) << 6)
            | (val(bytes[i + 3]) as u32);
        out.push(((n >> 16) & 255) as u8);
        if bytes[i + 2] != b'=' {
            out.push(((n >> 8) & 255) as u8);
        }
        if bytes[i + 3] != b'=' {
            out.push((n & 255) as u8);
        }
        i += 4;
    }
    out
}
