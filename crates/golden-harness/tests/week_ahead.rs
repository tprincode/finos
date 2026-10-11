use application_core::contracts::{
    CashElementRecord, CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::ports::canonical::Canonical;
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
    // Fresh horizon for 9/12 starts at Sat−4; Sept 1 SSA is not seeded yet, so still empty here.
    assert!(
        rows_for(&ahead, "SSA_2026").is_empty(),
        "SSA monthly on the 1st is not on this fresh horizon seed: {ahead}"
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
        "seed week lists no dividend note, no CLM or CRF account, and no Cash_Adjust: {ahead}"
    );
}

#[tokio::test]
async fn clm_account_is_listed_and_a_dividend_note_is_not() {
    let (_dir, platform) = seeded_platform().await;
    let _ = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    platform
        .cash_element_upsert(CashElementRecord {
            element_id: Uuid::new_v4(),
            account: "CLM".into(),
            kind: "Deposit".into(),
            cadence: "weekly".into(),
            amount_minor: 4_200,
            note: "cash".into(),
            weekday_or_month_day: "Sat".into(),
            start_on: String::new(),
            stop_on: String::new(),
            association_kind: String::new(),
        })
        .await
        .unwrap();
    platform
        .cash_element_upsert(CashElementRecord {
            element_id: Uuid::new_v4(),
            account: "CLM".into(),
            kind: "Deposit".into(),
            cadence: "weekly".into(),
            amount_minor: 1_100,
            note: "dividend".into(),
            weekday_or_month_day: "Sat".into(),
            start_on: String::new(),
            stop_on: String::new(),
            association_kind: String::new(),
        })
        .await
        .unwrap();
    let ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    let listed = rows_for(&ahead, "CLM");
    assert!(
        listed.iter().any(|r| r["note"] == "cash" && r["amountMinor"] == 4_200),
        "an account named CLM stays on the list: {ahead}"
    );
    assert!(
        ahead["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["note"] != "dividend"),
        "a dividend note stays off the list: {ahead}"
    );
}

#[test]
fn elements_grid_is_date_account_transaction_amount() {
    let ahead = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/cash/WeekAhead.tsx"),
    )
    .unwrap();
    assert!(
        ahead.contains("<th>Date</th>")
            && ahead.contains("<th>Account</th>")
            && ahead.contains("<th>Name</th>")
            && ahead.contains("{row.note || \"—\"}")
            && ahead.contains("<th>Transaction</th>")
            && ahead.contains("<th>Amount</th>")
            && ahead.contains("<th>Snooze</th>")
            && ahead.contains("Snooze till tomorrow")
            && !ahead.contains("Check tomorrow"),
        "Elements columns are Date, Account, Name, Transaction, Amount; Snooze stands in for Check tomorrow"
    );
    let query = std::fs::read_to_string(
        repo_root().join("crates/application-core/src/week_ahead.rs"),
    )
    .unwrap();
    assert!(
        query.contains("note.eq_ignore_ascii_case(\"dividend\")")
            && !query.contains("eq_ignore_ascii_case(\"CLM\")")
            && !query.contains("eq_ignore_ascii_case(\"CRF\")"),
        "dividends drop by note; account names CLM and CRF do not"
    );
}

#[tokio::test]
async fn w2_confirm_car_posts_withdrawal_and_leaves_list() {
    let (_dir, platform) = seeded_platform().await;
    let _ = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-08-29"}),
    )
    .await;
    let ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-01"}),
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
        serde_json::json!({"asOfDate": "2026-09-01"}),
    )
    .await;
    assert!(
        rows_for(&after, "Car").is_empty(),
        "confirmed Car leaves the list: {after}"
    );
    let week = query_json(
        &platform,
        "CashManagementWeekGet",
        serde_json::json!({"asOfDate": "2026-09-01"}),
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
    let same_day = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    assert!(
        same_day["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["occurrenceId"] != net["occurrenceId"]),
        "deferred to tomorrow is not yet due on Saturday: {same_day}"
    );
    let after = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-13"}),
    )
    .await;
    let moved = after["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["occurrenceId"] == net["occurrenceId"])
        .expect("deferred Income net returns on its new due day");
    assert_eq!(moved["occurredOn"], "2026-09-13");
    assert_eq!(moved["note"], "net");
}

#[tokio::test]
async fn w4_unconfirmed_elements_stay_past_due_until_confirm() {
    let (_dir, platform) = seeded_platform().await;
    // Before the 1st: SSA is seeded on the horizon but not yet due — hidden.
    let before_due = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-08-29"}),
    )
    .await;
    assert!(
        rows_for(&before_due, "SSA_2026").is_empty(),
        "not-yet-due SSA (9/1) must stay hidden on 8/29: {before_due}"
    );
    // On the due day it appears.
    let on_due = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-01"}),
    )
    .await;
    let tom = find_row(&on_due, "SSA_2026", "tom");
    let barbara = find_row(&on_due, "SSA_2026", "barbara");
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
        "SSA past due stays until Confirm: {after}"
    );
    assert!(
        ssa.iter()
            .any(|r| r["note"] == "barbara" && r["occurredOn"] == "2026-09-07"),
        "unconfirmed element past the old 4-day lookback still stays: {after}"
    );
    // Last week's Income Saturday must still list when the next week opens.
    let prior = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    let income_prior_sat = rows_for(&prior, "Income");
    assert!(
        income_prior_sat
            .iter()
            .any(|r| r["occurredOn"] == "2026-09-12" && r["note"] == "net"),
        "this week's Income still listed: {prior}"
    );
    let next = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-19"}),
    )
    .await;
    assert_eq!(next["periodStart"], "2026-09-19");
    let income_next = rows_for(&next, "Income");
    assert!(
        income_next
            .iter()
            .any(|r| r["occurredOn"] == "2026-09-12" && r["note"] == "net"),
        "unconfirmed Income from last Saturday stays on Week Ahead: {next}"
    );
}

/// Later this week is not due yet on Saturday — must not list until as-of reaches that day.
#[tokio::test]
async fn w4b_not_yet_due_this_week_stays_hidden() {
    let (_dir, platform) = seeded_platform().await;
    // Seed monthly Health on the 1st into the horizon, then view from prior Saturday.
    let sat = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-08-29"}),
    )
    .await;
    assert_eq!(sat["periodStart"], "2026-08-29");
    assert_eq!(sat["periodEnd"], "2026-09-04");
    assert!(
        rows_for(&sat, "Health").is_empty(),
        "Health 9/1 is inside Sat–Fri week but not yet due on 8/29: {sat}"
    );
    assert!(
        rows_for(&sat, "Car").is_empty(),
        "Car 9/1 is inside Sat–Fri week but not yet due on 8/29: {sat}"
    );
    let loans = sat["loans"].as_array().cloned().unwrap_or_default();
    assert!(
        loans.iter().all(|l| {
            l["dueOn"]
                .as_str()
                .map(|d| d <= "2026-08-29")
                .unwrap_or(false)
        }),
        "loan due after as-of must stay hidden: {sat}"
    );
}

#[tokio::test]
async fn w5_edit_health_amount_then_confirm_hsa_withdrawal() {
    let (_dir, platform) = seeded_platform().await;
    // Warm horizon, then open on the due day (not-yet-due stays hidden before 9/1).
    let _ = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-08-29"}),
    )
    .await;
    let ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-01"}),
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
        serde_json::json!({"asOfDate": "2026-09-01"}),
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
    let before_due = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    assert!(
        rows_for(&before_due, "Health")
            .iter()
            .all(|r| r["occurredOn"].as_str().unwrap_or("") <= "2026-09-12"),
        "Health 9/15 is not yet due on 9/12: {before_due}"
    );
    let mid = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-15"}),
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
    let health: Vec<_> = oct["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["account"] == "Health" && r["note"] == "MyClearbalance")
        .collect();
    assert!(
        health.iter().any(|r| r["occurredOn"] == "2026-09-15"),
        "unconfirmed Sept 15 stays past due: {oct}"
    );
    assert!(
        health
            .iter()
            .all(|r| r["occurredOn"].as_str().unwrap_or("") <= "2026-10-10"),
        "Oct 15 is not yet due on Oct 10: {oct}"
    );
    let on_oct_due = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-10-15"}),
    )
    .await;
    assert!(
        on_oct_due["rows"].as_array().unwrap().iter().any(|r| {
            r["account"] == "Health"
                && r["note"] == "MyClearbalance"
                && r["occurredOn"] == "2026-10-15"
                && r["amountMinor"] == 20_300
        }),
        "October series date lists on its due day: {on_oct_due}"
    );
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

#[tokio::test]
async fn w4c_ancient_past_due_leaves_week_ahead_board() {
    let (_dir, platform) = seeded_platform().await;
    must_ok(
        &platform,
        "CashElementSave",
        serde_json::json!({
            "account": "Health",
            "name": "old-hsa",
            "kind": "Withdrawal",
            "cadence": "one-time",
            "weekdayOrMonthDay": "",
            "startOn": "2025-01-01",
            "amountMinor": 20_300,
            "asOfDate": "2025-01-01",
            "occurrences": [{
                "occurredOn": "2025-01-15",
                "amountMinor": 20_300
            }]
        }),
    )
    .await;
    let ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-10-10"}),
    )
    .await;
    let rows = ahead["rows"].as_array().expect("rows");
    assert!(
        rows.iter().all(|r| {
            r["occurredOn"]
                .as_str()
                .map(|d| d >= "2026-09-05" && d <= "2026-10-10")
                .unwrap_or(false)
        }),
        "Week Ahead is due/recent past-due only (as_of−35d .. as_of): {ahead}"
    );
    assert!(
        !rows.iter().any(|r| r["note"] == "old-hsa" || r["occurredOn"] == "2025-01-15"),
        "2025 Health backlog must leave the board: {ahead}"
    );
    let catalog = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "Health", "asOfDate": "2026-10-10"}),
    )
    .await;
    assert!(
        catalog["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["note"] == "old-hsa"),
        "series remains available outside Week Ahead (catalog/history): {catalog}"
    );
    let loans = ahead["loans"].as_array().cloned().unwrap_or_default();
    assert!(
        loans.iter().all(|l| {
            l["dueOn"]
                .as_str()
                .map(|d| d >= "2026-09-05" && d <= "2026-10-10")
                .unwrap_or(false)
        }),
        "loan due outside 35-day lookback must stay off the board: {ahead}"
    );
    let ui = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/cash/WeekAhead.tsx"),
    )
    .unwrap();
    assert!(
        ui.contains("recent past-due (35 days)")
            && ui.contains("Element history")
            && !ui.contains("Due and past-due only"),
        "caption matches the 35-day board"
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
            && capture.contains("label: \"Speculation (capture only)\"")
            && capture.contains("label: \"Health\"")
            && capture.contains("label: \"Car\"")
            && capture.contains("label: \"Account 9\""),
        "W6: capture still one six-row table; Speculation is capture-only"
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
