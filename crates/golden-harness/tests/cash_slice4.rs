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
    must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "FI Roth", "kind": "fi_roth"}),
    )
    .await;
    must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "9", "kind": "taxable"}),
    )
    .await;
    (dir, platform)
}

#[test]
fn n1_menu_working_aria() {
    let app = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx")).unwrap();
    assert!(
        app.contains("Working…") && app.contains("aria-label=\"Menu working\""),
        "N1: Plan/CM/Tools menu shows Working… with aria-label Menu working"
    );
}

#[test]
fn n2_element_dirty_joins_leave_and_restart() {
    let app = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx")).unwrap();
    assert!(
        app.contains("elementDirty")
            && app.contains("leaveWithoutSaving")
            && app.contains("pdDirty || wizDirty || addLotDirty || cashDirty || elementDirty"),
        "N2: elementDirty joins leaveWithoutSaving"
    );
    assert!(
        app.contains("pdDirty || wizDirty || addLotDirty || cashDirty || elementDirty || weekWizardActive"),
        "N2: elementDirty joins Restart guard"
    );
    let editor = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/cash/CashElementEditor.tsx"),
    )
    .unwrap();
    assert!(
        editor.contains("is-owner-edit-active")
            && editor.contains("aria-label=\"Element editor working\"")
            && editor.contains("Working…"),
        "N2: element editor paints every button dirty and shows Working…"
    );
    let css = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.css")).unwrap();
    assert!(
        css.contains("button.is-unsaved:disabled")
            && !css.contains("button.is-unsaved:not(:disabled)"),
        "N2: is-unsaved stays orange while the action is in flight"
    );
}

#[tokio::test]
async fn n3_element_list_all_seeded_books() {
    let (_dir, platform) = seeded_platform().await;
    let _ahead = query_json(
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
    assert_eq!(list["account"], "all", "{list}");
    let items = list["items"].as_array().expect("items");
    let notes: Vec<&str> = items
        .iter()
        .map(|i| i["note"].as_str().unwrap_or(""))
        .collect();
    assert!(notes.contains(&"net") && notes.contains(&"fed") && notes.contains(&"state"));
    assert!(notes.contains(&"car"));
    assert!(notes.contains(&"hsa1") && notes.contains(&"hsa2"));
    assert!(notes.contains(&"tom") && notes.contains(&"barbara"));
    let income: Vec<_> = items.iter().filter(|i| i["account"] == "Income").collect();
    assert_eq!(income.len(), 3, "Income three elements: {list}");
    let health: Vec<_> = items.iter().filter(|i| i["account"] == "Health").collect();
    assert_eq!(health.len(), 2, "Health two elements: {list}");
    let ssa: Vec<_> = items.iter().filter(|i| i["account"] == "SSA_2026").collect();
    assert_eq!(ssa.len(), 2, "SSA two elements: {list}");
}

#[test]
fn n4_catalog_edit_opens_same_editor() {
    let catalog = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/cash/CashElementsCatalog.tsx"),
    )
    .unwrap();
    let editor = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/cash/CashElementEditor.tsx"),
    )
    .unwrap();
    assert!(
        catalog.contains("CashElementEditor")
            && catalog.contains("aria-label=\"Elements catalog\"")
            && catalog.contains(">Edit<")
            && catalog.contains("All managed accounts")
            && catalog.contains("allOrOne")
            && !catalog.contains("All books")
            && catalog.contains("canAdd")
            && catalog.contains(">Deposits<")
            && catalog.contains(">Withdrawals<")
            && catalog.contains("Add Deposit")
            && catalog.contains("Add Withdrawal")
            && editor.contains("Schedule Date")
            && editor.contains("Never expires")
            && editor.contains("Exceptions")
            && editor.contains("Open exceptions")
            && editor.contains("element-exceptions-link")
            && catalog.contains("CashElementExceptions")
            && editor.contains("element-editor-account"),
        "N4: catalog Edit mounts CashElementEditor; picker is All-or-one managed accounts"
    );
    let picker = std::fs::read_to_string(
        repo_root().join("packages/ui-components/src/AccountTickPicker.tsx"),
    )
    .unwrap();
    assert!(
        picker.contains("allOrOne")
            && picker.contains("anyCombination")
            && picker.contains("exactlyOne")
            && picker.contains("lockAccounts")
            && picker.contains("Pick at least one account"),
        "shared AccountTickPicker: All-or-one locks account ticks; combinations stay on Income Plan"
    );
    let register = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/cash/CashRegister.tsx"),
    )
    .unwrap();
    assert!(
        !register.contains("Register elements") && !register.contains("<ul"),
        "N4: Register no longer has a second element list"
    );
}

#[tokio::test]
async fn n5_calendar_and_trend_series_equal() {
    let (_dir, platform) = seeded_platform().await;
    let _ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-08-29"}),
    )
    .await;
    let reg = query_json(
        &platform,
        "CashRegisterGet",
        serde_json::json!({
            "account": "Car",
            "period": "1Y",
            "asOfDate": "2026-09-10"
        }),
    )
    .await;
    let series = reg["series"].as_array().expect("series");
    assert!(!series.is_empty(), "{reg}");
    let ui = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/cash/CashRegister.tsx"),
    )
    .unwrap();
    assert!(
        ui.contains("aria-label=\"Register calendar\"")
            && ui.contains("aria-label=\"Register trend\"")
            && ui.contains("calendarBody?.series")
            && ui.contains("className=\"register-calendar\"")
            && ui.contains("Sun")
            && ui.contains("calendarWeeks")
            && ui.contains("Previous month")
            && ui.contains("cashflow manager")
            && ui.contains("Month starting balance")
            && ui.contains("exactlyOne")
            && ui.contains("Manage Elements")
            && ui.contains("aria-label=\"Register view\"")
            && ui.contains("<AccountCashFlow")
            && ui.contains("weeks={weeks}")
            && ui.contains("asOf={asOfDate}")
            && ui.contains("cashflow-picker-row")
            && ui.contains("row.occurredOn === focusDay")
            && ui.contains("Day transactions"),
        "N5: Calendar is a one-account month grid; Trend is AccountCashFlow"
    );
}

#[tokio::test]
async fn n6_ytd_account_and_tax_remaining_null_not_zero() {
    let (_dir, platform) = seeded_platform().await;
    let _ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    let account = query_json(
        &platform,
        "CashYtdGet",
        serde_json::json!({"asOfDate": "2026-09-12", "view": "account"}),
    )
    .await;
    let rows = account["rows"].as_array().expect("account rows");
    let labels: Vec<&str> = rows
        .iter()
        .map(|r| r["label"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(
        labels,
        vec!["Income", "FI Roth", "Health", "Car", "Account 9", "SSA_2026"],
        "{account}"
    );
    let blank = rows.iter().any(|r| r["remainingMinor"].is_null());
    assert!(blank, "N6: at least one remaining plan is null, not 0: {account}");
    assert!(
        rows.iter()
            .filter(|r| r["remainingMinor"].is_null())
            .all(|r| r["eoyMinor"].is_null()),
        "EOY stays — when remaining is unknown: {account}"
    );
    let tax = query_json(
        &platform,
        "CashYtdGet",
        serde_json::json!({"asOfDate": "2026-09-12", "view": "tax"}),
    )
    .await;
    let tax_labels: Vec<&str> = tax["rows"]
        .as_array()
        .expect("tax rows")
        .iter()
        .map(|r| r["label"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(
        tax_labels,
        vec![
            "IRA ordinary",
            "SSA",
            "Roth",
            "Health (not MAGI)",
            "1099 job"
        ],
        "{tax}"
    );
}

#[tokio::test]
async fn ssa_ytd_is_tom_jan_sep_plus_barbara_may_sep() {
    let (_dir, platform) = seeded_platform().await;
    let accounts = query_json(&platform, "AccountList", serde_json::json!({})).await;
    let external = accounts
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["name"] == "External")
        .expect("External");
    let account_id = &external["accountId"];
    for (on, amt, key) in [
        ("2026-01-09", 286_500, "ssa-hist-tom-2026-01"),
        ("2026-02-06", 286_500, "ssa-hist-tom-2026-02"),
        ("2026-03-06", 286_500, "ssa-hist-tom-2026-03"),
        ("2026-04-10", 286_500, "ssa-hist-tom-2026-04"),
        ("2026-05-08", 286_500, "ssa-hist-tom-2026-05"),
        ("2026-06-05", 286_500, "ssa-hist-tom-2026-06a"),
        ("2026-06-26", 286_500, "ssa-hist-tom-2026-06b"),
        ("2026-08-10", 286_500, "ssa-tom-2026-08"),
        ("2026-09-10", 286_500, "ssa-tom-2026-09"),
        ("2026-05-15", 133_100, "ssa-hist-barbara-2026-05"),
        ("2026-06-12", 133_100, "ssa-hist-barbara-2026-06"),
        ("2026-07-03", 133_100, "ssa-hist-barbara-2026-07"),
        ("2026-08-10", 133_100, "ssa-barbara-2026-08"),
        ("2026-09-10", 133_100, "ssa-barbara-2026-09"),
    ] {
        must_ok(
            &platform,
            "CashDistributionPost",
            serde_json::json!({
                "accountId": account_id,
                "activityType": "SSA",
                "occurredOn": on,
                "grossMinor": amt,
                "federalWithholdingMinor": 0,
                "stateWithholdingMinor": 0,
                "scale": 2,
                "idempotencyKey": key
            }),
        )
        .await;
    }
    let ytd = query_json(
        &platform,
        "CashYtdGet",
        serde_json::json!({"asOfDate": "2026-09-21", "view": "account"}),
    )
    .await;
    let ssa = ytd["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["label"] == "SSA_2026")
        .expect("SSA_2026");
    assert_eq!(
        ssa["actualMinor"],
        3_244_000,
        "Tom 9×$2,865 + Barbara 5×$1,331 = $32,440: {ytd}"
    );
}

#[tokio::test]
async fn car_withdrawal_ytd_is_cash_only_not_tax_type() {
    let (_dir, platform) = seeded_platform().await;
    let accounts = query_json(&platform, "AccountList", serde_json::json!({})).await;
    let car = accounts
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["name"] == "Car")
        .expect("Car");
    let account_id = &car["accountId"];
    for (on, amt, key) in [
        ("2026-01-23", 60_000, "car-wd-2026-01-23"),
        ("2026-03-13", 65_000, "car-wd-2026-03-13"),
        ("2026-04-10", 85_000, "car-wd-2026-04-10"),
        ("2026-05-15", 85_000, "car-wd-2026-05-15"),
        ("2026-06-12", 85_000, "car-wd-2026-06-12"),
        ("2026-08-10", 120_000, "car-wd-2026-08-10"),
        ("2026-09-08", 120_000, "car-wd-2026-09-08"),
    ] {
        must_ok(
            &platform,
            "CashDistributionPost",
            serde_json::json!({
                "accountId": account_id,
                "activityType": "Withdrawal",
                "occurredOn": on,
                "grossMinor": amt,
                "federalWithholdingMinor": 0,
                "stateWithholdingMinor": 0,
                "scale": 2,
                "idempotencyKey": key
            }),
        )
        .await;
    }
    let account = query_json(
        &platform,
        "CashYtdGet",
        serde_json::json!({"asOfDate": "2026-09-21", "view": "account"}),
    )
    .await;
    let car_row = account["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["label"] == "Car")
        .expect("Car");
    assert_eq!(
        car_row["actualMinor"],
        620_000,
        "Car cash-out YTD $6,200: {account}"
    );
    let tax = query_json(
        &platform,
        "CashYtdGet",
        serde_json::json!({"asOfDate": "2026-09-21", "view": "tax"}),
    )
    .await;
    let labels: Vec<&str> = tax["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["label"].as_str().unwrap_or(""))
        .collect();
    assert!(
        !labels.iter().any(|l| l.contains("Car")),
        "Car withdrawals are cash, not tax-type YTD: {tax}"
    );
}

#[test]
fn n7_week_ahead_and_capture_grid_still_locked() {
    let ahead = std::fs::read_to_string(
        repo_root().join("crates/golden-harness/tests/week_ahead.rs"),
    )
    .unwrap();
    for name in [
        "w1_unconfirmed_income_triple_and_car_no_dividends",
        "w2_confirm_car_posts_withdrawal_and_leaves_list",
        "w3_check_tomorrow_saturday_income_net_stays_listed",
        "w4_ssa_saturday_minus_4_listed_minus_5_not",
        "w5_edit_health_amount_then_confirm_hsa_withdrawal",
        "w6_capture_grid_still_one_six_row_table",
        "w7_desk_planned_reported_blank_not_zero",
    ] {
        assert!(ahead.contains(name), "N7: week_ahead still has {name}");
    }
    assert!(
        ahead.contains("async fn w1_")
            && ahead.contains("fn w6_")
            && ahead.contains("fn w7_"),
        "N7: W1–W7 stay in week_ahead.rs"
    );
    let capture = std::fs::read_to_string(
        repo_root().join("crates/golden-harness/tests/accessibility.rs"),
    )
    .unwrap();
    assert!(
        capture.contains("fn g1_g6_slice1b_capture_grid_one_table"),
        "N7: G1–G6 stay in accessibility.rs"
    );
}

#[tokio::test]
async fn save_monthly_schedule_moves_next_date_and_amount() {
    let (_dir, platform) = seeded_platform().await;
    let _ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    let list = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "all", "asOfDate": "2026-09-18"}),
    )
    .await;
    let hsa1 = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["note"] == "hsa1")
        .expect("hsa1");
    assert_eq!(hsa1["nextOccurredOn"], "2026-10-01");
    assert_eq!(hsa1["nextAmountMinor"], 20_000);
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
            "asOfDate": "2026-09-18",
            "startOn": "",
            "stopOn": "",
            "occurrences": []
        }),
    )
    .await;
    let after = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "all", "asOfDate": "2026-09-18"}),
    )
    .await;
    let row = after["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["note"] == "MyClearbalance")
        .expect("renamed");
    assert_eq!(row["nextOccurredOn"], "2026-10-15", "{after}");
    assert_eq!(row["nextAmountMinor"], 20_300, "{after}");
}

#[tokio::test]
async fn save_with_stale_editor_occurrences_moves_next() {
    let (_dir, platform) = seeded_platform().await;
    let _ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    let list = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "all", "asOfDate": "2026-09-18"}),
    )
    .await;
    let hsa1 = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["note"] == "hsa1")
        .expect("hsa1");
    let register = query_json(
        &platform,
        "CashRegisterGet",
        serde_json::json!({
            "account": "Health",
            "period": "1Y",
            "asOfDate": "2026-09-18"
        }),
    )
    .await;
    let stale: Vec<serde_json::Value> = register["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["elementId"] == hsa1["elementId"] && r["occurrenceId"].is_string())
        .map(|r| {
            let amt = if r["withdrawalMinor"].as_i64().unwrap_or(0) > 0 {
                r["withdrawalMinor"].clone()
            } else {
                r["depositMinor"].clone()
            };
            serde_json::json!({
                "occurrenceId": r["occurrenceId"],
                "occurredOn": r["occurredOn"],
                "amountMinor": amt
            })
        })
        .collect();
    assert!(
        stale.iter().any(|o| o["occurredOn"] == "2026-10-01"),
        "editor still lists leftover 1sts: {stale:?}"
    );
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
            "asOfDate": "2026-09-18",
            "startOn": "",
            "stopOn": "",
            "occurrences": stale
        }),
    )
    .await;
    let after = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "all", "asOfDate": "2026-09-18"}),
    )
    .await;
    for item in after["items"].as_array().unwrap() {
        let cadence = item["cadence"].as_str().unwrap_or("");
        if cadence == "one-time" {
            continue;
        }
        if let Some(next) = item["nextOccurredOn"].as_str() {
            assert_eq!(
                item["nextAmountMinor"], item["amountMinor"],
                "catalog next amount must match series {}: {after}",
                item["note"]
            );
            if cadence == "monthly" {
                let day: u32 = item["weekdayOrMonthDay"]
                    .as_str()
                    .unwrap_or("1")
                    .parse()
                    .unwrap_or(1);
                let got: u32 = next[8..].parse().unwrap();
                assert_eq!(
                    got, day,
                    "catalog next date must follow schedule {}: {after}",
                    item["note"]
                );
            }
        }
    }
    let row = after["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["note"] == "MyClearbalance")
        .expect("renamed");
    assert_eq!(row["nextOccurredOn"], "2026-10-15", "{after}");
    assert_eq!(row["nextAmountMinor"], 20_300, "{after}");
}

#[tokio::test]
async fn list_get_prunes_leftover_off_schedule_without_save() {
    let (_dir, platform) = seeded_platform().await;
    let _ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    let list = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "all", "asOfDate": "2026-09-18"}),
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
            "elementId": hsa1["elementId"],
            "name": "MyClearbalance",
            "kind": "Withdrawal",
            "cadence": "monthly",
            "weekdayOrMonthDay": "15",
            "amountMinor": 20_300,
            "asOfDate": "2026-09-18",
            "startOn": "",
            "stopOn": "",
            "occurrences": [{
                "occurredOn": "2026-10-01",
                "amountMinor": 20_000
            }]
        }),
    )
    .await;
    let first = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "all", "asOfDate": "2026-09-18"}),
    )
    .await;
    let after = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "all", "asOfDate": "2026-09-18"}),
    )
    .await;
    assert_eq!(
        upcoming_count(&first),
        upcoming_count(&after),
        "list get must not insert occurrences"
    );
    let row = after["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["note"] == "MyClearbalance")
        .expect("renamed");
    assert_eq!(row["nextOccurredOn"], "2026-10-15", "{after}");
    assert_eq!(row["nextAmountMinor"], 20_300, "{after}");
}

#[tokio::test]
async fn saturday_asof_save_updates_income_state_list_and_plan() {
    let (_dir, platform) = seeded_platform().await;
    let _ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-19"}),
    )
    .await;
    let list = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "Income", "asOfDate": "2026-09-19"}),
    )
    .await;
    let state = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["note"] == "state" && i["account"] == "Income")
        .expect("Income state");
    must_ok(
        &platform,
        "CashElementSave",
        serde_json::json!({
            "account": "Income",
            "elementId": state["elementId"],
            "name": "state",
            "kind": "Withdrawal",
            "cadence": "weekly",
            "weekdayOrMonthDay": "Sat",
            "amountMinor": 5_500,
            "asOfDate": "2026-09-19",
            "startOn": "",
            "stopOn": "",
            "occurrences": []
        }),
    )
    .await;
    let after = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "Income", "asOfDate": "2026-09-19"}),
    )
    .await;
    let row = after["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["note"] == "state")
        .expect("state after save");
    assert_eq!(row["amountMinor"], 5_500, "{after}");
    assert_eq!(row["nextOccurredOn"], "2026-09-19", "{after}");
    assert_eq!(row["nextAmountMinor"], 5_500, "{after}");
    let ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-19"}),
    )
    .await;
    let plan = ahead["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["note"] == "state")
        .expect("Week Ahead state");
    assert_eq!(plan["amountMinor"], 5_500, "{ahead}");
    let reg = query_json(
        &platform,
        "CashRegisterGet",
        serde_json::json!({
            "account": "Income",
            "period": "1Y",
            "asOfDate": "2026-09-19",
            "periodStart": "2026-09-01",
            "periodEnd": "2026-10-31",
            "includeUnconfirmedPast": true
        }),
    )
    .await;
    assert!(
        reg["rows"].as_array().unwrap().iter().any(|r| {
            r["label"] == "state"
                && r["occurredOn"] == "2026-09-19"
                && r["withdrawalMinor"] == 5_500
        }),
        "Register this Saturday uses saved state: {reg}"
    );
}

#[test]
fn n8_exceptions_screen_and_save() {
    let catalog = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/cash/CashElementsCatalog.tsx"),
    )
    .unwrap();
    let editor = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/cash/CashElementEditor.tsx"),
    )
    .unwrap();
    let exceptions = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/cash/CashElementExceptions.tsx"),
    )
    .unwrap();
    let edit = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/cash/CashElementExceptionEdit.tsx"),
    )
    .unwrap();
    let queries = std::fs::read_to_string(
        repo_root().join("crates/application-core/src/queries.rs"),
    )
    .unwrap();
    let save = std::fs::read_to_string(
        repo_root().join("crates/application-core/src/cash_register.rs"),
    )
    .unwrap();
    assert!(
        catalog.contains("CashElementExceptions")
            && catalog.contains("onSaveExceptions")
            && editor.contains("onOpenExceptions")
            && editor.contains("{exceptionCount} Exceptions")
            && !editor.contains("Future occurrences")
            && exceptions.contains("Upcoming Transactions")
            && exceptions.contains("aria-label=\"Upcoming transactions\"")
            && exceptions.contains("CashElementExceptionEdit")
            && edit.contains("Cancel this transaction")
            && edit.contains("Modify this transaction")
            && edit.contains("Save exception")
            && queries.contains("PlannedOccurrenceSave")
            && save.contains("planned_occurrence_edit")
            && save.contains("if row.is_exception"),
        "N8: Exceptions lists upcoming hits; selected row opens Edit transaction"
    );
}

#[tokio::test]
async fn planned_occurrence_save_keeps_off_cadence_through_series_save() {
    let (_dir, platform) = seeded_platform().await;
    let _ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    let list = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "all", "asOfDate": "2026-09-18"}),
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
            "asOfDate": "2026-09-18",
            "startOn": "",
            "stopOn": "",
            "occurrences": []
        }),
    )
    .await;
    must_ok(
        &platform,
        "PlannedOccurrenceSave",
        serde_json::json!({
            "elementId": hsa1["elementId"],
            "asOfDate": "2026-09-18",
            "occurrences": [{
                "occurredOn": "2026-10-01",
                "amountMinor": 15_000
            }]
        }),
    )
    .await;
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
            "asOfDate": "2026-09-18",
            "startOn": "",
            "stopOn": "",
            "occurrences": []
        }),
    )
    .await;
    let after = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "all", "asOfDate": "2026-09-18"}),
    )
    .await;
    let row = after["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["note"] == "MyClearbalance")
        .expect("renamed");
    assert_eq!(row["exceptionCount"], 1, "{after}");
    assert_eq!(row["nextOccurredOn"], "2026-10-01", "{after}");
    assert_eq!(row["nextAmountMinor"], 15_000, "{after}");
    let exceptions = row["exceptions"].as_array().expect("exceptions");
    assert_eq!(exceptions[0]["occurredOn"], "2026-10-01");
    assert_eq!(exceptions[0]["amountMinor"], 15_000);
}

#[tokio::test]
async fn planned_occurrence_edit_moves_selected_upcoming_date() {
    let (_dir, platform) = seeded_platform().await;
    let _ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    let list = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "all", "asOfDate": "2026-09-18"}),
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
            "asOfDate": "2026-09-18",
            "startOn": "",
            "stopOn": "",
            "occurrences": []
        }),
    )
    .await;
    let ready = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "all", "asOfDate": "2026-09-18"}),
    )
    .await;
    let row = ready["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["note"] == "MyClearbalance")
        .expect("renamed");
    let source = row["upcoming"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["occurredOn"] == "2026-10-15")
        .expect("15th upcoming");
    must_ok(
        &platform,
        "PlannedOccurrenceSave",
        serde_json::json!({
            "elementId": hsa1["elementId"],
            "occurrenceId": source["occurrenceId"],
            "asOfDate": "2026-09-18",
            "occurredOn": "2026-10-01",
            "amountMinor": 13_699
        }),
    )
    .await;
    let after = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "all", "asOfDate": "2026-09-18"}),
    )
    .await;
    let got = after["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["note"] == "MyClearbalance")
        .expect("after edit");
    let upcoming = got["upcoming"].as_array().expect("upcoming");
    assert!(
        upcoming.iter().any(|o| {
            o["occurredOn"] == "2026-10-01"
                && o["amountMinor"] == 13_699
                && o["isException"] == true
                && o["isCancelled"] == false
        }),
        "modified date is listed: {after}"
    );
    assert!(
        upcoming.iter().any(|o| {
            o["occurredOn"] == "2026-10-15" && o["isCancelled"] == true
        }),
        "original cadence date stays cancelled so horizon does not refill: {after}"
    );
    assert_eq!(got["nextOccurredOn"], "2026-10-01", "{after}");
    assert_eq!(got["nextAmountMinor"], 13_699, "{after}");
}

#[test]
fn n9_element_history_section_on_catalog() {
    let catalog = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/cash/CashElementsCatalog.tsx"),
    )
    .unwrap();
    let history = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/cash/CashElementHistory.tsx"),
    )
    .unwrap();
    let css = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.css")).unwrap();
    assert!(
        catalog.contains("CashElementHistory")
            && history.contains("aria-label=\"Element history\"")
            && history.contains("aria-label=\"History element\"")
            && history.contains("aria-label=\"History duration\"")
            && history.contains("aria-label=\"Element transaction register\"")
            && history.contains("YTD")
            && history.contains("6 months")
            && history.contains("3 months")
            && history.contains("1 month")
            && history.contains("value: \"all\"")
            && history.contains("(retired)")
            && history.contains("Debit")
            && history.contains("Credit")
            && css.contains(".element-history-break")
            && css.contains(".element-history-kicker")
            && catalog.contains("PageActivityCard")
            && catalog.contains("page=\"elements\""),
        "N9: Element Management has a history register for current and retired elements"
    );
    let bar = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/shared/PageActivityBar.tsx"),
    )
    .unwrap();
    let app = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx")).unwrap();
    let client = std::fs::read_to_string(repo_root().join("apps/desktop/src/financeClient.ts")).unwrap();
    assert!(
        app.contains("PageActivityBar")
            && bar.contains("aria-label=\"Page activity\"")
            && bar.contains("menubar-activity-chip")
            && !bar.contains("<progress")
            && client.contains("beginPageActivity")
            && client.contains("activityLabel"),
        "N9: page activity is a menubar last-entry chip, not a progress bar; client announces reads/writes"
    );
}

#[tokio::test]
async fn n10_element_history_state_tax_durations_and_retired() {
    let (_dir, platform) = seeded_platform().await;
    let jan = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-01-10"}),
    )
    .await;
    let jan_state = jan["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["account"] == "Income" && r["note"] == "state")
        .expect("jan state");
    must_ok(
        &platform,
        "WeekAheadConfirm",
        serde_json::json!({ "occurrenceId": jan_state["occurrenceId"] }),
    )
    .await;
    let sep = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    let sep_state = sep["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["account"] == "Income" && r["note"] == "state")
        .expect("sep state");
    must_ok(
        &platform,
        "WeekAheadConfirm",
        serde_json::json!({ "occurrenceId": sep_state["occurrenceId"] }),
    )
    .await;
    let _register = query_json(
        &platform,
        "CashRegisterGet",
        serde_json::json!({
            "account": "Income",
            "period": "1M",
            "asOfDate": "2026-09-12"
        }),
    )
    .await;
    let list = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "all", "asOfDate": "2026-09-12"}),
    )
    .await;
    let state = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["note"] == "state")
        .expect("state element");
    let element_id = state["elementId"].as_str().unwrap();
    let ytd = query_json(
        &platform,
        "CashElementHistoryGet",
        serde_json::json!({
            "elementId": element_id,
            "duration": "ytd",
            "asOfDate": "2026-09-12"
        }),
    )
    .await;
    assert_eq!(ytd["elementName"], "state", "{ytd}");
    assert_eq!(ytd["account"], "Income", "{ytd}");
    assert_eq!(ytd["kind"], "Withdrawal", "{ytd}");
    assert_eq!(ytd["retired"], false, "{ytd}");
    assert_eq!(ytd["periodStart"], "2026-01-01", "{ytd}");
    assert_eq!(ytd["periodEnd"], "2026-09-12", "{ytd}");
    let ytd_actual: Vec<&str> = ytd["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["status"] == "Actual")
        .map(|r| r["occurredOn"].as_str().unwrap_or(""))
        .collect();
    assert!(
        ytd_actual.contains(&"2026-01-10") && ytd_actual.contains(&"2026-09-12"),
        "YTD includes both posted state weeks: {ytd}"
    );
    assert_eq!(ytd["totals"]["actualCount"], 2, "{ytd}");
    assert_eq!(ytd["totals"]["actualDebitMinor"], 9_000, "{ytd}");
    assert_eq!(ytd["totals"]["actualCreditMinor"], 0, "{ytd}");
    assert_eq!(ytd["totals"]["actualNetMinor"], -9_000, "{ytd}");
    assert!(
        ytd["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["side"] == "Debit" && r["elementName"] == "state"),
        "{ytd}"
    );
    let month = query_json(
        &platform,
        "CashElementHistoryGet",
        serde_json::json!({
            "elementId": element_id,
            "duration": "1m",
            "asOfDate": "2026-09-12"
        }),
    )
    .await;
    let month_actual: Vec<&str> = month["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["status"] == "Actual")
        .map(|r| r["occurredOn"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(month_actual, vec!["2026-09-12"], "1 month is lookback: {month}");
    assert!(!month_actual.iter().any(|d| *d == "2026-01-10"), "{month}");
    let future = state["upcoming"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["occurredOn"].as_str().unwrap_or("") > "2026-09-12" && o["isCancelled"] != true)
        .expect("future state occ");
    must_ok(
        &platform,
        "PlannedOccurrenceSave",
        serde_json::json!({
            "elementId": element_id,
            "occurrenceId": future["occurrenceId"],
            "asOfDate": "2026-09-12",
            "cancel": true,
            "occurredOn": future["occurredOn"],
            "amountMinor": future["amountMinor"]
        }),
    )
    .await;
    let all = query_json(
        &platform,
        "CashElementHistoryGet",
        serde_json::json!({
            "elementId": element_id,
            "duration": "all",
            "asOfDate": "2026-09-12"
        }),
    )
    .await;
    assert!(
        all["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["occurrenceId"] != future["occurrenceId"]),
        "cancelled leftover is not a history row: {all}"
    );
    must_ok(
        &platform,
        "CashElementSave",
        serde_json::json!({
            "account": "Income",
            "elementId": element_id,
            "name": "state",
            "kind": "Withdrawal",
            "cadence": "weekly",
            "weekdayOrMonthDay": "Sat",
            "amountMinor": 5_000,
            "asOfDate": "2026-09-12",
            "startOn": "",
            "stopOn": "2026-08-01",
            "occurrences": []
        }),
    )
    .await;
    let retired = query_json(
        &platform,
        "CashElementHistoryGet",
        serde_json::json!({
            "elementId": element_id,
            "duration": "ytd",
            "asOfDate": "2026-09-12"
        }),
    )
    .await;
    assert_eq!(retired["retired"], true, "{retired}");
    assert_eq!(retired["totals"]["actualCount"], 2, "{retired}");
}

#[tokio::test]
async fn n11_income_history_includes_seed_ira_withholding() {
    let (_dir, platform) = seeded_platform().await;
    let _ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    let accounts = query_json(&platform, "AccountList", serde_json::json!({})).await;
    let income = accounts
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["name"] == "Income")
        .expect("Income");
    must_ok(
        &platform,
        "CashDistributionPost",
        serde_json::json!({
            "accountId": income["accountId"],
            "activityType": "IRA_Distribution",
            "occurredOn": "2026-01-02",
            "grossMinor": 110_000,
            "federalWithholdingMinor": 11_000,
            "stateWithholdingMinor": 4_400,
            "scale": 2,
            "idempotencyKey": "production-disb-73-2026-Trends R82 2026-01-02"
        }),
    )
    .await;
    let list = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "all", "asOfDate": "2026-09-12"}),
    )
    .await;
    let fed = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["note"] == "fed")
        .expect("fed");
    let net = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["note"] == "net")
        .expect("net");
    let hist = query_json(
        &platform,
        "CashElementHistoryGet",
        serde_json::json!({
            "elementId": fed["elementId"],
            "duration": "ytd",
            "asOfDate": "2026-09-12"
        }),
    )
    .await;
    assert!(
        hist["rows"].as_array().unwrap().iter().any(|r| {
            r["occurredOn"] == "2026-01-02"
                && r["amountMinor"] == 11_000
                && r["status"] == "Actual"
                && r["side"] == "Debit"
                && r["activityType"] == "IRA_Distribution"
        }),
        "fed YTD must list seed withholding, not only confirmed Saturdays: {hist}"
    );
    assert_eq!(hist["totals"]["actualDebitMinor"], 11_000, "{hist}");
    let net_hist = query_json(
        &platform,
        "CashElementHistoryGet",
        serde_json::json!({
            "elementId": net["elementId"],
            "duration": "ytd",
            "asOfDate": "2026-09-12"
        }),
    )
    .await;
    assert!(
        net_hist["rows"].as_array().unwrap().iter().any(|r| {
            r["occurredOn"] == "2026-01-02" && r["amountMinor"] == 94_600
        }),
        "net is gross minus withholding: {net_hist}"
    );
}

#[tokio::test]
async fn n12_car_and_ssa_history_use_posted_facts() {
    let (_dir, platform) = seeded_platform().await;
    let _ = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-21"}),
    )
    .await;
    let accounts = query_json(&platform, "AccountList", serde_json::json!({})).await;
    let car_id = accounts
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["name"] == "Car")
        .expect("Car")["accountId"]
        .clone();
    let ext_id = accounts
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["name"] == "External")
        .expect("External")["accountId"]
        .clone();
    for (on, amt, key) in [
        ("2026-01-23", 60_000, "car-hist-2026-01-23"),
        ("2026-03-13", 65_000, "car-hist-2026-03-13"),
        ("2026-04-10", 85_000, "car-hist-2026-04-10"),
        ("2026-05-15", 85_000, "car-hist-2026-05-15"),
        ("2026-06-12", 85_000, "car-hist-2026-06-12"),
        ("2026-08-10", 120_000, "car-hist-2026-08-10"),
        ("2026-09-08", 120_000, "car-hist-2026-09-08"),
    ] {
        must_ok(
            &platform,
            "CashDistributionPost",
            serde_json::json!({
                "accountId": car_id,
                "activityType": "Withdrawal",
                "occurredOn": on,
                "grossMinor": amt,
                "federalWithholdingMinor": 0,
                "stateWithholdingMinor": 0,
                "scale": 2,
                "idempotencyKey": key
            }),
        )
        .await;
    }
    must_ok(
        &platform,
        "CashDistributionPost",
        serde_json::json!({
            "accountId": ext_id,
            "activityType": "SSA",
            "occurredOn": "2026-06-26",
            "grossMinor": 286_500,
            "federalWithholdingMinor": 0,
            "stateWithholdingMinor": 0,
            "scale": 2,
            "idempotencyKey": "ssa-hist-tom-june-26"
        }),
    )
    .await;
    must_ok(
        &platform,
        "CashDistributionPost",
        serde_json::json!({
            "accountId": ext_id,
            "activityType": "SSA",
            "occurredOn": "2026-05-15",
            "grossMinor": 133_100,
            "federalWithholdingMinor": 0,
            "stateWithholdingMinor": 0,
            "scale": 2,
            "idempotencyKey": "ssa-hist-barbara-may-15"
        }),
    )
    .await;
    let list = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "all", "asOfDate": "2026-09-21"}),
    )
    .await;
    let car = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["account"] == "Car")
        .expect("Car element");
    let tom = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| {
            i["account"] == "SSA_2026"
                && i["note"]
                    .as_str()
                    .unwrap_or("")
                    .to_ascii_lowercase()
                    .contains("tom")
        })
        .expect("SSA Tom");
    let barbara = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| {
            i["account"] == "SSA_2026"
                && i["note"]
                    .as_str()
                    .unwrap_or("")
                    .to_ascii_lowercase()
                    .contains("barbara")
        })
        .expect("SSA Barbara");
    let car_hist = query_json(
        &platform,
        "CashElementHistoryGet",
        serde_json::json!({
            "elementId": car["elementId"],
            "duration": "ytd",
            "asOfDate": "2026-09-21"
        }),
    )
    .await;
    let car_actual: Vec<(String, i64)> = car_hist["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["status"] == "Actual")
        .map(|r| {
            (
                r["occurredOn"].as_str().unwrap_or("").to_string(),
                r["amountMinor"].as_i64().unwrap_or(0),
            )
        })
        .collect();
    assert!(
        car_actual.contains(&("2026-01-23".into(), 60_000))
            && car_actual.contains(&("2026-03-13".into(), 65_000))
            && car_actual.contains(&("2026-09-08".into(), 120_000)),
        "Car history uses posted dates and amounts: {car_hist}"
    );
    assert!(
        !car_actual.iter().any(|(on, amt)| on.ends_with("-01") && *amt == 85_000),
        "Car leftover firsts at $850 stay off Actual: {car_hist}"
    );
    let tom_hist = query_json(
        &platform,
        "CashElementHistoryGet",
        serde_json::json!({
            "elementId": tom["elementId"],
            "duration": "ytd",
            "asOfDate": "2026-09-21"
        }),
    )
    .await;
    assert!(
        tom_hist["rows"].as_array().unwrap().iter().any(|r| {
            r["occurredOn"] == "2026-06-26"
                && r["amountMinor"] == 286_500
                && r["status"] == "Actual"
                && r["side"] == "Credit"
        }),
        "SSA Tom posted facts list on history: {tom_hist}"
    );
    let barb_hist = query_json(
        &platform,
        "CashElementHistoryGet",
        serde_json::json!({
            "elementId": barbara["elementId"],
            "duration": "ytd",
            "asOfDate": "2026-09-21"
        }),
    )
    .await;
    assert!(
        barb_hist["rows"].as_array().unwrap().iter().any(|r| {
            r["occurredOn"] == "2026-05-15"
                && r["amountMinor"] == 133_100
                && r["status"] == "Actual"
        }),
        "SSA Barbara posted facts list on history: {barb_hist}"
    );
}

fn upcoming_count(list: &serde_json::Value) -> usize {
    list["items"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|i| i["upcoming"].as_array().map(|a| a.len()).unwrap_or(0))
        .sum()
}

#[tokio::test]
async fn n13_element_list_get_does_not_insert_occurrences() {
    let (_dir, platform) = seeded_platform().await;
    let empty = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "all", "asOfDate": "2026-09-22"}),
    )
    .await;
    assert!(
        empty["items"].as_array().unwrap().is_empty(),
        "list get does not seed elements: {empty}"
    );
    let _ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    let after_week = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "all", "asOfDate": "2026-01-01"}),
    )
    .await;
    assert!(
        !after_week["items"].as_array().unwrap().is_empty(),
        "WeekAheadGet still seeds the catalog: {after_week}"
    );
    let n_week = upcoming_count(&after_week);
    let _later = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "all", "asOfDate": "2026-09-22"}),
    )
    .await;
    let after_list = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "all", "asOfDate": "2026-01-01"}),
    )
    .await;
    assert_eq!(
        n_week,
        upcoming_count(&after_list),
        "list get must not mint planned_occurrence"
    );
}

#[tokio::test]
async fn n13_income_plan_week_returns_while_list_get_in_flight() {
    let (_dir, platform) = seeded_platform().await;
    let (list, week) = tokio::join!(
        query_json(
            &platform,
            "CashElementListGet",
            serde_json::json!({"account": "all", "asOfDate": "2026-09-22"}),
        ),
        query_json(
            &platform,
            "IncomePlanWeekGet",
            serde_json::json!({"asOfDate": "2026-09-19"}),
        ),
    );
    assert!(
        list["items"].as_array().is_some(),
        "element list still returns during concurrent week get: {list}"
    );
    assert!(
        week.get("weekEnding").is_some() || week.get("periodEnd").is_some() || week.is_object(),
        "Income Plan week query still returns while list get is in flight: {week}"
    );
}

#[test]
fn n13_idle_warm_is_read_only_next_desk() {
    let app = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx")).unwrap();
    let list = std::fs::read_to_string(
        repo_root().join("crates/application-core/src/cash_register.rs"),
    )
    .unwrap();
    let nav = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/cash/fetchCashNav.ts"),
    )
    .unwrap();
    let warm = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/shared/idleWarm.ts"),
    )
    .unwrap();
    let start = list
        .find("Catalog list is read-only")
        .expect("list-get comment");
    let slice = &list[start..];
    let end = slice[40..]
        .find("pub async fn")
        .map(|i| i + 40)
        .unwrap_or(slice.len());
    let body = &slice[..end];
    assert!(
        !body.contains("ensure_horizon")
            && !body.contains("ensure_seed")
            && !body.contains("prune_off_schedule")
            && app.contains("loadElementsPack")
            && app.contains("cmDesk === \"elements\"")
            && app.contains("scheduleIdleWarm")
            && app.contains("income-plan")
            && app.contains("if (busy || elementDirty || lastPriceBusy || declarationBusy)")
            && app.contains("screen === \"home\" && !accountValues")
            && app.contains("IncomePlanWeekGet")
            && !app.contains("await client.executeQuery(\"CashDividendCoverageGet\"")
            && app.contains("runBackgroundReads")
            && app.contains("incomeGridMemoryMatches")
            && nav.contains("fetchElementList")
            && nav.contains("fetchTaxPlanning")
            && warm.contains("scheduleIdleWarm"),
        "N13: Element open is a read-only list pack; idle warm is Tax + list, not horizon"
    );
}

#[tokio::test]
async fn n14_background_grid_read_does_not_insert_occurrences() {
    let (_dir, platform) = seeded_platform().await;
    let _ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    let before = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "all", "asOfDate": "2026-01-01"}),
    )
    .await;
    let n_before = upcoming_count(&before);
    let (grid, week) = tokio::join!(
        query_json(
            &platform,
            "IncomePlanGridGet",
            serde_json::json!({
                "asOfDate": "2026-09-19",
                "historicalWeeks": 6,
                "futureWeeks": 6,
                "accounts": ["Income"],
                "weekEnding": "2026-09-19"
            }),
        ),
        query_json(
            &platform,
            "IncomePlanWeekGet",
            serde_json::json!({"asOfDate": "2026-09-19"}),
        ),
    );
    assert!(
        grid.get("weeks").is_some() || grid.get("table1").is_some(),
        "grid read returns: {grid}"
    );
    assert!(
        week.is_object(),
        "Income Plan week query still returns while the grid read is in flight: {week}"
    );
    let after = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "all", "asOfDate": "2026-01-01"}),
    )
    .await;
    assert_eq!(
        n_before,
        upcoming_count(&after),
        "grid read must not mint planned_occurrence"
    );
}

#[test]
fn n14_weekly_grid_opens_from_memory() {
    let app = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx")).unwrap();
    let reads = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/shared/backgroundReads.ts"),
    )
    .unwrap();
    let click = app
        .split("aria-label=\"Report type\"")
        .nth(1)
        .and_then(|s| s.split("Weekly report").next())
        .unwrap_or("");
    assert!(
        click.contains("incomeGridMemoryMatches")
            && click.find("incomeGridMemoryMatches").unwrap()
                < click.find("withIncomeLoading").unwrap_or(usize::MAX),
        "N14: Weekly grid shows memory before Loading Data"
    );
    for name in [
        "IncomePlanGridGet",
        "DividendPerformanceGet",
        "CashElementListGet",
        "TaxPlanningGet",
    ] {
        assert!(reads.contains(name), "background queue reads {name}");
    }
    for blocked in [
        "WeekAheadGet",
        "CashRegisterGet",
        "CashCoverageGet",
        "CashYtdGet",
    ] {
        assert!(
            !reads.contains(blocked),
            "background queue must not call {blocked}"
        );
    }
}
