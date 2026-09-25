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

async fn seeded_platform() -> (tempfile::TempDir, LocalPlatform) {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "ira"}),
    )
    .await;
    must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Health", "kind": "hsa"}),
    )
    .await;
    must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Car", "kind": "taxable"}),
    )
    .await;
    must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "External", "kind": "taxable"}),
    )
    .await;
    (dir, platform)
}

fn rows_for<'a>(ahead: &'a serde_json::Value, account: &str) -> Vec<&'a serde_json::Value> {
    ahead["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["account"] == account)
        .collect()
}

fn find_row<'a>(ahead: &'a serde_json::Value, account: &str, note: &str) -> &'a serde_json::Value {
    ahead["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["account"] == account && r["note"] == note)
        .unwrap_or_else(|| panic!("missing {account} {note} in {ahead}"))
}

#[tokio::test]
async fn w1_unconfirmed_income_triple_and_car_no_dividends() {
    let (_dir, platform) = seeded_platform().await;
    let ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    assert_eq!(ahead["periodStart"], "2026-09-12");
    assert_eq!(ahead["periodEnd"], "2026-09-18");
    let income = rows_for(&ahead, "Income");
    assert_eq!(income.len(), 3, "Income net/fed/state are three Elements: {ahead}");
    assert!(income.iter().any(|r| r["note"] == "net"));
    assert!(income.iter().any(|r| r["note"] == "fed"));
    assert!(income.iter().any(|r| r["note"] == "state"));
    assert!(income.iter().all(|r| r["transaction"] == "Withdrawal"));
    assert!(
        income.iter().all(|r| r["occurredOn"] == "2026-09-12"),
        "weekly Income stays on Saturday: {ahead}"
    );
    assert!(
        rows_for(&ahead, "Car").is_empty(),
        "Car monthly on the 1st is not dumped onto Saturday: {ahead}"
    );
    assert!(
        rows_for(&ahead, "Health").is_empty(),
        "Health monthly is not dumped onto Saturday: {ahead}"
    );
    assert!(
        rows_for(&ahead, "SSA_2026").is_empty(),
        "SSA monthly on the 1st is outside 9/12 lookback: {ahead}"
    );
    assert!(
        ahead["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["note"] != "dividend"
                && r["account"] != "CLM"
                && r["account"] != "CRF"
                && r["transaction"] != "Cash_Adjust"),
        "Week Ahead never lists dividends, Cash_Adjust, or CLM/CRF: {ahead}"
    );
}

#[tokio::test]
async fn w2_confirm_car_posts_withdrawal_and_leaves_list() {
    let (_dir, platform) = seeded_platform().await;
    let ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-08-29"}),
    )
    .await;
    let car = find_row(&ahead, "Car", "car");
    assert_eq!(car["occurredOn"], "2026-09-01");
    let posted = must_ok(
        &platform,
        "WeekAheadConfirm",
        serde_json::json!({"occurrenceId": car["occurrenceId"]}),
    )
    .await;
    assert_eq!(posted["activityType"], "Withdrawal");
    assert_eq!(posted["amountMinor"], car["amountMinor"]);
    let after = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-08-29"}),
    )
    .await;
    assert!(
        rows_for(&after, "Car").is_empty(),
        "confirmed Car leaves the list: {after}"
    );
    let week = query_json(
        &platform,
        "CashManagementWeekGet",
        serde_json::json!({"asOfDate": "2026-08-29"}),
    )
    .await;
    assert!(
        week["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["activityType"] == "Withdrawal" && r["accountName"] == "Car"),
        "Car Withdrawal posted on the week board: {week}"
    );
}

#[tokio::test]
async fn w3_check_tomorrow_saturday_income_net_stays_listed() {
    let (_dir, platform) = seeded_platform().await;
    let ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    let net = find_row(&ahead, "Income", "net");
    must_ok(
        &platform,
        "WeekAheadDefer",
        serde_json::json!({"occurrenceId": net["occurrenceId"]}),
    )
    .await;
    let after = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    let moved = find_row(&after, "Income", "net");
    assert_eq!(moved["occurredOn"], "2026-09-13");
    assert!(moved["occurrenceId"] == net["occurrenceId"]);
}

#[tokio::test]
async fn w4_ssa_saturday_minus_4_listed_minus_5_not() {
    let (_dir, platform) = seeded_platform().await;
    let ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-08-29"}),
    )
    .await;
    let tom = find_row(&ahead, "SSA_2026", "tom");
    let barbara = find_row(&ahead, "SSA_2026", "barbara");
    assert_eq!(tom["occurredOn"], "2026-09-01");
    assert_eq!(barbara["occurredOn"], "2026-09-01");
    must_ok(
        &platform,
        "WeekAheadEdit",
        serde_json::json!({
            "occurrenceId": tom["occurrenceId"],
            "occurredOn": "2026-09-08"
        }),
    )
    .await;
    must_ok(
        &platform,
        "WeekAheadEdit",
        serde_json::json!({
            "occurrenceId": barbara["occurrenceId"],
            "occurredOn": "2026-09-07"
        }),
    )
    .await;
    let after = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    let ssa = rows_for(&after, "SSA_2026");
    assert!(
        ssa.iter().any(|r| r["note"] == "tom" && r["occurredOn"] == "2026-09-08"),
        "SSA dated Saturday-4 listed: {after}"
    );
    assert!(
        ssa.iter().all(|r| r["note"] != "barbara"),
        "SSA dated Saturday-5 is not listed: {after}"
    );
}

#[tokio::test]
async fn w5_edit_health_amount_then_confirm_hsa_withdrawal() {
    let (_dir, platform) = seeded_platform().await;
    let ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-08-29"}),
    )
    .await;
    let health = find_row(&ahead, "Health", "hsa1");
    assert_eq!(health["occurredOn"], "2026-09-01");
    assert_ne!(health["amountMinor"].as_i64(), Some(33_333));
    must_ok(
        &platform,
        "WeekAheadEdit",
        serde_json::json!({
            "occurrenceId": health["occurrenceId"],
            "amountMinor": 33_333
        }),
    )
    .await;
    let posted = must_ok(
        &platform,
        "WeekAheadConfirm",
        serde_json::json!({"occurrenceId": health["occurrenceId"]}),
    )
    .await;
    assert_eq!(posted["activityType"], "HSA_Withdrawal");
    assert_eq!(posted["amountMinor"].as_i64(), Some(33_333));
    let after = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-08-29"}),
    )
    .await;
    assert!(
        after["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["occurrenceId"] != health["occurrenceId"]),
        "confirmed Health leaves the list: {after}"
    );
}

#[tokio::test]
async fn week_ahead_follows_element_series() {
    let (_dir, platform) = seeded_platform().await;
    let _ = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    let list = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "all", "asOfDate": "2026-09-12"}),
    )
    .await;
    let hsa1 = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["note"] == "hsa1")
        .expect("hsa1");
    must_ok(
        &platform,
        "CashElementSave",
        serde_json::json!({
            "account": "Health",
            "elementId": hsa1["elementId"],
            "name": "MyClearbalance",
            "kind": "Withdrawal",
            "cadence": "monthly",
            "weekdayOrMonthDay": "15",
            "amountMinor": 20_300,
            "asOfDate": "2026-09-12",
            "startOn": "",
            "stopOn": "",
            "occurrences": []
        }),
    )
    .await;
    let mid = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    let mid_health = find_row(&mid, "Health", "MyClearbalance");
    assert_eq!(mid_health["occurredOn"], "2026-09-15", "{mid}");
    assert_eq!(mid_health["amountMinor"], 20_300, "{mid}");
    assert!(
        mid["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["note"] != "hsa1"),
        "leftover hsa1 name is gone: {mid}"
    );
    let catalog = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "all", "asOfDate": "2026-09-12"}),
    )
    .await;
    let row = catalog["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["note"] == "MyClearbalance")
        .expect("renamed");
    assert_eq!(row["nextOccurredOn"], "2026-09-15", "{catalog}");
    assert_eq!(row["nextAmountMinor"], 20_300, "{catalog}");
    let oct = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-10-10"}),
    )
    .await;
    let health = find_row(&oct, "Health", "MyClearbalance");
    assert_eq!(health["occurredOn"], "2026-10-15");
    assert_eq!(health["amountMinor"], 20_300);
    let today = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-19"}),
    )
    .await;
    let els = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "all", "asOfDate": "2026-09-19"}),
    )
    .await;
    let net = els["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["note"] == "net")
        .expect("net");
    assert_eq!(
        net["nextOccurredOn"], "2026-09-19",
        "catalog Next includes today so it matches Week Ahead: {els}"
    );
    assert!(
        today["rows"].as_array().unwrap().iter().any(|r| {
            r["note"] == "net"
                && r["occurredOn"] == "2026-09-19"
                && r["amountMinor"] == net["nextAmountMinor"]
        }),
        "Week Ahead net is the catalog Next: {today} vs {net}"
    );
}

#[test]
fn w6_capture_grid_still_one_six_row_table() {
    let capture = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/graphing/TrendsCapture.tsx"),
    )
    .unwrap();
    assert!(
        capture.contains("aria-label=\"Week capture grid\"")
            && capture.contains("label: \"Income\"")
            && capture.contains("label: \"FI Roth\"")
            && capture.contains("label: \"Speculation\"")
            && capture.contains("label: \"Health\"")
            && capture.contains("label: \"Car\"")
            && capture.contains("label: \"Account 9\""),
        "W6: capture still one six-row table"
    );
    assert!(
        !capture.contains("const STEPS") && !capture.contains("Next Trends step"),
        "W6: no account-by-account rail"
    );
    let app = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx")).unwrap();
    assert!(app.contains("WeekCaptureAccept"), "W6: Accept still WeekCaptureAccept");
}

#[test]
fn w7_desk_planned_reported_blank_not_zero() {
    let desk = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/cash/CashWeekDesk.tsx"),
    )
    .unwrap();
    assert!(
        desk.contains("Planned weekly income") && desk.contains("Reported weekly income"),
        "W7: desk still has Planned / Reported headers"
    );
    assert!(
        !desk.contains("reportedMinor: 0") && !desk.contains("reportedWeekIncomeMinor: 0"),
        "W7: blank Reported is not stored as 0"
    );
    let app = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx")).unwrap();
    assert!(
        app.contains("reportedMinor: trendsCapture.reportedWeeklyIncomeMinor || null"),
        "W7: open-week Reported stays null when blank, not 0"
    );
}
