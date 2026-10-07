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

async fn seeded_platform() -> (tempfile::TempDir, LocalPlatform, serde_json::Value) {
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
    let car = must_ok(
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
    (dir, platform, car)
}

async fn seed_week(platform: &LocalPlatform) -> serde_json::Value {
    query_json(
        platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-08-29"}),
    )
    .await
}

async fn seed_car_start(platform: &LocalPlatform, _car_id: &str) {
    must_ok(
        platform,
        "TrendsWeekSave",
        serde_json::json!({
            "periodStart": "2026-08-22",
            "periodEnd": "2026-08-28",
            "carBalanceMinor": 500_000,
            "carCashMinor": 500_000,
            "scale": 2,
            "capturedAt": "2026-08-29T12:00:00Z"
        }),
    )
    .await;
}

fn car_rows(reg: &serde_json::Value) -> Vec<&serde_json::Value> {
    reg["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["transaction"] == "Withdrawal" || r["transaction"] == "Deposit")
        .collect()
}

#[tokio::test]
async fn r1_car_1m_actual_past_projected_future_running_signs() {
    let (_dir, platform, car) = seeded_platform().await;
    seed_car_start(&platform, car["accountId"].as_str().unwrap()).await;
    seed_week(&platform).await;
    must_ok(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": car["accountId"],
            "activityType": "Withdrawal",
            "amountMinor": 50_000,
            "scale": 2,
            "occurredOn": "2026-09-03",
            "idempotencyKey": "car-past-wd"
        }),
    )
    .await;
    must_ok(
        &platform,
        "CashAdjustPost",
        serde_json::json!({
            "accountId": car["accountId"],
            "occurredOn": "2026-09-04",
            "amountMinor": 10_000,
            "scale": 2,
            "reason": "other",
            "idempotencyKey": "car-adjust-dep"
        }),
    )
    .await;
    let reg = query_json(
        &platform,
        "CashRegisterGet",
        serde_json::json!({
            "account": "Car",
            "period": "1M",
            "asOfDate": "2026-09-10"
        }),
    )
    .await;
    assert_eq!(reg["account"], "Car");
    assert_eq!(reg["periodStart"], "2026-09-01");
    assert_eq!(reg["periodEnd"], "2026-09-30");
    assert_eq!(reg["startKnown"], true);
    let rows = car_rows(&reg);
    assert!(
        rows.iter().any(|r| {
            r["occurredOn"] == "2026-09-03"
                && r["status"] == "Actual"
                && r["transaction"] == "Withdrawal"
                && r["withdrawalMinor"] == 50_000
        }),
        "Actual past Car withdrawal: {reg}"
    );
    assert!(
        !rows.iter().any(|r| r["occurredOn"] == "2026-09-12"),
        "Car monthly is not dumped onto Saturday: {reg}"
    );
    let year = query_json(
        &platform,
        "CashRegisterGet",
        serde_json::json!({
            "account": "Car",
            "period": "1Y",
            "asOfDate": "2026-09-10"
        }),
    )
    .await;
    assert!(
        year["rows"].as_array().unwrap().iter().any(|r| {
            r["occurredOn"] == "2026-10-01"
                && r["status"] == "Planned"
                && r["transaction"] == "Withdrawal"
        }),
        "Planned next Car 1st: {year}"
    );
    let wd = rows
        .iter()
        .find(|r| r["occurredOn"] == "2026-09-03")
        .unwrap();
    let dep = rows
        .iter()
        .find(|r| r["occurredOn"] == "2026-09-04" && r["transaction"] == "Deposit")
        .expect(&format!("Adjust deposit: {reg}"));
    let start = reg["startMinor"].as_i64().unwrap();
    let wd_run = wd["runningMinor"].as_i64().unwrap();
    let dep_run = dep["runningMinor"].as_i64().unwrap();
    assert!(wd_run < start, "withdrawal decreases running {wd_run} {start}");
    assert!(dep_run > wd_run, "deposit increases running {dep_run} {wd_run}");
}

#[tokio::test]
async fn r2_calendar_and_trend_series_equal() {
    let (_dir, platform, car) = seeded_platform().await;
    seed_car_start(&platform, car["accountId"].as_str().unwrap()).await;
    seed_week(&platform).await;
    let reg = query_json(
        &platform,
        "CashRegisterGet",
        serde_json::json!({
            "account": "Car",
            "period": "1M",
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
            && ui.contains("Sat")
            && ui.contains("calendarWeeks")
            && ui.contains("Previous month")
            && ui.contains("Next month")
            && ui.contains("cashflow manager")
            && ui.contains("Month starting balance")
            && ui.contains("Month ending balance")
            && ui.contains("Month planned income")
            && ui.contains("Month actual income")
            && ui.contains("Month actual withdrawals")
            && ui.contains("Actual Income")
            && ui.contains("Actual withdrawals")
            && ui.contains("exactlyOne")
            && ui.contains("Manage Elements")
            && ui.contains("aria-label=\"Register view\"")
            && ui.contains("is-debit")
            && ui.contains("is-credit")
            && ui.contains("<AccountCashFlow")
            && ui.contains("weeks={weeks}")
            && ui.contains("asOf={asOfDate}")
            && ui.contains("cashflow-picker-row")
            && ui.contains("register-row-")
            && ui.contains("row.occurredOn === focusDay")
            && ui.contains("Day transactions")
            && ui.contains("useState(\"Income\")") == false,
        "Calendar is a one-account month grid; Trend is AccountCashFlow"
    );
    let app = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx")).unwrap();
    assert!(
        app.contains("useState(\"Income\")")
            && app.contains("registerBookRef = useRef(\"Income\")")
            && app.contains("periodStart: monthStart")
            && app.contains("periodEnd: monthEnd")
            && !app.contains("includeUnconfirmedPast: true"),
        "Register default book is Income; month calendar uses posted facts plus future elements"
    );
}

#[tokio::test]
async fn r3_confirm_car_register_row_actual_uses_posted() {
    let (_dir, platform, car) = seeded_platform().await;
    seed_car_start(&platform, car["accountId"].as_str().unwrap()).await;
    let ahead = seed_week(&platform).await;
    let car_row = ahead["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["account"] == "Car")
        .expect("seeded Car");
    must_ok(
        &platform,
        "WeekAheadEdit",
        serde_json::json!({
            "occurrenceId": car_row["occurrenceId"],
            "amountMinor": 77_000
        }),
    )
    .await;
    let posted = must_ok(
        &platform,
        "WeekAheadConfirm",
        serde_json::json!({"occurrenceId": car_row["occurrenceId"]}),
    )
    .await;
    assert_eq!(posted["amountMinor"], 77_000);
    let reg = query_json(
        &platform,
        "CashRegisterGet",
        serde_json::json!({
            "account": "Car",
            "period": "1M",
            "asOfDate": "2026-09-18"
        }),
    )
    .await;
    let row = reg["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["occurrenceId"] == car_row["occurrenceId"])
        .expect(&format!("confirmed occurrence on register: {reg}"));
    assert_eq!(row["status"], "Actual");
    assert_eq!(row["withdrawalMinor"], 77_000);
    assert_eq!(row["posted"], true);
    let start = reg["startMinor"].as_i64().unwrap();
    let run = row["runningMinor"].as_i64().unwrap();
    assert!(run <= start - 77_000, "running uses posted 77000: {row} start={start}");
}

#[tokio::test]
async fn r4_add_element_future_and_series_edit_skips_past_actual() {
    let (_dir, platform, car) = seeded_platform().await;
    seed_week(&platform).await;
    must_ok(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": car["accountId"],
            "activityType": "Withdrawal",
            "amountMinor": 50_000,
            "scale": 2,
            "occurredOn": "2026-09-03",
            "idempotencyKey": "car-past-actual"
        }),
    )
    .await;
    let saved = must_ok(
        &platform,
        "CashElementSave",
        serde_json::json!({
            "account": "Car",
            "name": "extra",
            "kind": "Withdrawal",
            "cadence": "one-time",
            "weekdayOrMonthDay": "",
            "amountMinor": 12_345,
            "asOfDate": "2026-09-10",
            "occurrences": [{
                "occurredOn": "2026-09-25",
                "amountMinor": 12_345
            }]
        }),
    )
    .await;
    let occ_id = saved["occurrences"][0]["occurrenceId"].clone();
    let before = query_json(
        &platform,
        "CashRegisterGet",
        serde_json::json!({
            "account": "Car",
            "period": "1M",
            "asOfDate": "2026-09-10"
        }),
    )
    .await;
    assert!(
        before["rows"].as_array().unwrap().iter().any(|r| {
            r["occurredOn"] == "2026-09-25"
                && r["status"] == "Planned"
                && r["withdrawalMinor"] == 12_345
        }),
        "one-time future is Planned: {before}"
    );
    must_ok(
        &platform,
        "CashElementSave",
        serde_json::json!({
            "account": "Car",
            "elementId": saved["element"]["elementId"],
            "name": "extra",
            "kind": "Withdrawal",
            "cadence": "one-time",
            "weekdayOrMonthDay": "",
            "amountMinor": 99_999,
            "asOfDate": "2026-09-10",
            "occurrences": [{
                "occurrenceId": occ_id,
                "occurredOn": "2026-09-25",
                "amountMinor": 99_999
            }]
        }),
    )
    .await;
    let after = query_json(
        &platform,
        "CashRegisterGet",
        serde_json::json!({
            "account": "Car",
            "period": "1M",
            "asOfDate": "2026-09-10"
        }),
    )
    .await;
    assert!(
        after["rows"].as_array().unwrap().iter().any(|r| {
            r["occurredOn"] == "2026-09-03"
                && r["status"] == "Actual"
                && r["withdrawalMinor"] == 50_000
        }),
        "past Actual stays 50000: {after}"
    );
    assert!(
        after["rows"].as_array().unwrap().iter().any(|r| {
            r["occurredOn"] == "2026-09-25" && r["withdrawalMinor"] == 99_999
        }),
        "future series amount updated: {after}"
    );
}

#[tokio::test]
async fn r5_missing_cash_start_running_null_not_zero() {
    let (_dir, platform, _car) = seeded_platform().await;
    seed_week(&platform).await;
    let reg = query_json(
        &platform,
        "CashRegisterGet",
        serde_json::json!({
            "account": "Car",
            "period": "1M",
            "asOfDate": "2026-09-10"
        }),
    )
    .await;
    assert_eq!(reg["startKnown"], false);
    assert!(reg["startMinor"].is_null(), "{reg}");
    for row in reg["rows"].as_array().unwrap() {
        assert!(
            row["runningMinor"].is_null(),
            "running is — not 0: {row}"
        );
        assert_ne!(row["runningMinor"], 0);
    }
    let ui = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/cash/CashRegister.tsx"),
    )
    .unwrap();
    assert!(
        ui.contains("minor == null ? \"—\"") || ui.contains("runningMinor == null"),
        "UI prints — when running is unknown"
    );
}

#[tokio::test]
async fn later_cash_lot_does_not_invent_july_running() {
    let (_dir, platform, car) = seeded_platform().await;
    seed_week(&platform).await;
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "SPAXX", "name": "SPAXX"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    research_template(&platform, security_id, "SPAXX").await;
    golden_harness::complete_collector_for_first_lot_as(
        &platform,
        security_id,
        "SPAXX",
        "Monthly",
        false,
    )
    .await
    .expect("complete collector");
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": car["accountId"],
            "securityId": security_id,
            "openedOn": "2026-08-04",
            "origin": "purchase",
            "quantityMinor": 542_716,
            "quantityScale": 2,
            "performanceBasisMinor": 542_716,
            "taxBasisMinor": 542_716,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;
    let july = query_json(
        &platform,
        "CashRegisterGet",
        serde_json::json!({
            "account": "Car",
            "period": "1M",
            "asOfDate": "2026-07-15"
        }),
    )
    .await;
    assert_eq!(july["startKnown"], false, "{july}");
    assert!(july["startMinor"].is_null(), "{july}");
    for row in july["rows"].as_array().unwrap() {
        assert!(
            row["runningMinor"].is_null(),
            "August leftover is not July cash: {row}"
        );
    }
    let year = query_json(
        &platform,
        "CashRegisterGet",
        serde_json::json!({
            "account": "Car",
            "period": "1Y",
            "asOfDate": "2026-07-15"
        }),
    )
    .await;
    assert_eq!(year["startKnown"], false, "{year}");
    for point in year["series"].as_array().unwrap() {
        if point["occurredOn"] == "2026-07-01" {
            assert!(
                point["runningMinor"].is_null(),
                "July 1 running stays —: {point}"
            );
        }
    }
}

#[tokio::test]
async fn car_september_opens_on_first_with_owner_cash() {
    let (_dir, platform, car) = seeded_platform().await;
    must_ok(
        &platform,
        "TrendsWeekSave",
        serde_json::json!({
            "periodStart": "2026-08-22",
            "periodEnd": "2026-08-28",
            "carBalanceMinor": 304_251,
            "carCashMinor": 304_251,
            "scale": 2,
            "capturedAt": "2026-08-29T12:00:00Z"
        }),
    )
    .await;
    seed_week(&platform).await;
    must_ok(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": car["accountId"],
            "activityType": "Withdrawal",
            "amountMinor": 120_000,
            "scale": 2,
            "occurredOn": "2026-09-08",
            "idempotencyKey": "owner-car-wd-2026-09-08"
        }),
    )
    .await;
    let reg = query_json(
        &platform,
        "CashRegisterGet",
        serde_json::json!({
            "account": "Car",
            "period": "1M",
            "asOfDate": "2026-09-18"
        }),
    )
    .await;
    assert_eq!(reg["startKnown"], true, "{reg}");
    assert_eq!(reg["startMinor"], 304_251, "{reg}");
    assert_eq!(reg["periodStart"], "2026-09-01");
    let open = reg["series"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["occurredOn"] == "2026-09-01")
        .expect("Trend starts on the 1st");
    assert_eq!(open["runningMinor"], 304_251, "{open}");
    assert!(
        !reg["rows"].as_array().unwrap().iter().any(|r| {
            r["occurredOn"] == "2026-09-01" && r["withdrawalMinor"] == 85_000
        }),
        "past unconfirmed $850 is not a September fact: {reg}"
    );
    let wd = reg["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["occurredOn"] == "2026-09-08" && r["withdrawalMinor"] == 120_000)
        .expect("9/8 $1200");
    assert_eq!(wd["runningMinor"], 184_251, "{wd}");
    for row in reg["rows"].as_array().unwrap() {
        if let Some(run) = row["runningMinor"].as_i64() {
            assert!(run >= 0, "Car running stays non-negative: {row}");
        }
    }
}

#[tokio::test]
async fn r6_income_three_withdrawal_lines() {
    let (_dir, platform, _) = seeded_platform().await;
    seed_week(&platform).await;
    let reg = query_json(
        &platform,
        "CashRegisterGet",
        serde_json::json!({
            "account": "Income",
            "period": "1M",
            "asOfDate": "2026-09-10"
        }),
    )
    .await;
    let sat: Vec<_> = reg["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["occurredOn"] == "2026-09-12" && r["transaction"] == "Withdrawal")
        .collect();
    assert_eq!(sat.len(), 3, "Income net/fed/state on seed Saturday: {reg}");
    assert!(sat.iter().any(|r| r["label"] == "net"));
    assert!(sat.iter().any(|r| r["label"] == "fed"));
    assert!(sat.iter().any(|r| r["label"] == "state"));
}

#[test]
fn r7_week_ahead_and_capture_grid_still_locked() {
    let app = std::fs::read_to_string(repo_root().join("apps/desktop/src/App.tsx")).unwrap();
    assert!(
        app.contains("reportedMinor: trendsCapture.reportedWeeklyIncomeMinor || null"),
        "W7: open-week Reported stays null when blank, not 0"
    );
    assert!(app.contains("WeekCaptureAccept"), "W6/G Accept still WeekCaptureAccept");
    let capture = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/graphing/TrendsCapture.tsx"),
    )
    .unwrap();
    assert!(
        capture.contains("aria-label=\"Week capture grid\"")
            && capture.contains("label: \"Speculation (capture only)\"")
            && capture.contains("accountName: \"Speculation\""),
        "G1–G7 / W6 capture grid stays one six-row table; Speculation is capture-only"
    );
    let ahead = std::fs::read_to_string(
        repo_root().join("apps/desktop/src/features/cash/WeekAhead.tsx"),
    )
    .unwrap();
    assert!(
        ahead.contains("onOpenEditor")
            && ahead.contains("<h3>Elements</h3>")
            && ahead.contains("aria-label={`Edit ${row.account}"),
        "Week Ahead Edit opens the shared editor"
    );
    assert!(
        ahead
            .match_indices("inputMode=\"decimal\"")
            .all(|(idx, _)| ahead[idx.saturating_sub(80)..idx].contains("interest")),
        "inline decimal is loan interest only — plan Edit uses onOpenEditor"
    );
}

#[tokio::test]
async fn r8_income_plan_day_is_register_projected_not_week_ahead() {
    let (_dir, platform, _) = seeded_platform().await;
    let income = query_json(&platform, "AccountList", serde_json::json!({})).await;
    let income_id = income
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["name"] == "Income")
        .unwrap()["accountId"]
        .clone();
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
    seed_week(&platform).await;
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
    let pay_on = pos["payOn"].as_str().unwrap_or("");
    assert!(!pay_on.is_empty(), "{pos}");
    let reg = query_json(
        &platform,
        "CashRegisterGet",
        serde_json::json!({
            "account": "Income",
            "period": "1M",
            "asOfDate": "2026-09-10"
        }),
    )
    .await;
    assert!(
        reg["rows"].as_array().unwrap().iter().any(|r| {
            r["source"] == "income-plan"
                && r["occurredOn"] == pay_on
                && r["transaction"] == "Deposit"
                && r["status"] == "Planned"
                && r["label"] == pos["symbol"]
        }),
        "Income Plan payable day is a Register Planned Deposit: pay_on={pay_on} {reg}"
    );
    let ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    assert!(
        ahead["rows"].as_array().unwrap().iter().all(|r| {
            r["note"] != "dividend"
                && r["note"] != "Income Plan"
                && r["transaction"] != "Cash_Adjust"
        }),
        "Week Ahead list has no dividend / Income Plan virtual rows: {ahead}"
    );
    let two_m = query_json(
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
    let plan_days: Vec<&str> = two_m["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["source"] == "income-plan" && r["label"] == "REG1")
        .filter_map(|r| r["occurredOn"].as_str())
        .collect();
    assert!(
        plan_days.iter().all(|d| *d >= "2026-09-19"),
        "unpaid plan stays off dates before as-of: {plan_days:?}"
    );
    assert!(
        plan_days.len() >= 4,
        "Home 2m window must list future weekly planned income, got {}: {two_m}",
        plan_days.len()
    );
    let paid_on = "2026-09-18";
    must_ok(
        &platform,
        "DividendActualRecord",
        serde_json::json!({
            "accountId": income_id,
            "securityId": security_id,
            "occurredOn": paid_on,
            "amountMinor": 12_50,
            "scale": 2,
            "idempotencyKey": "reg1-paid"
        }),
    )
    .await;
    let after_pay = query_json(
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
        after_pay["rows"].as_array().unwrap().iter().any(|r| {
            r["source"] == "dividend-actual"
                && r["label"] == "REG1"
                && r["occurredOn"] == paid_on
                && r["status"] == "Actual"
                && r["depositMinor"] == 12_50
        }),
        "paid REG1 is Actual: {after_pay}"
    );
    assert!(
        after_pay["rows"].as_array().unwrap().iter().all(|r| {
            !(r["source"] == "income-plan" && r["label"] == "REG1" && r["occurredOn"] == paid_on)
        }),
        "paid REG1 must not keep a plan row: {after_pay}"
    );
}

/// Vendor-printed 2027 survives a 2026–2027 Register window. remaining-year stays 2026.
#[tokio::test]
async fn register_1y_sees_vendor_2027_after_2026_weeks() {
    let (_dir, platform, _) = seeded_platform().await;
    let income = query_json(&platform, "AccountList", serde_json::json!({})).await;
    let income_id = income
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["name"] == "Income")
        .unwrap()["accountId"]
        .clone();
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "HOR1", "name": "HOR1"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    research_template(&platform, security_id, "HOR1").await;
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "priceSource": "public",
            "sourceSymbol": "HOR1",
            "declarationSource": "issuer",
            "sourceUrl": "https://example.test/hor1/distributions",
            "calendarPolicy": "issuer_calendar",
            "collectorEnabled": true,
            "lookbackCount": 12
        }),
    )
    .await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "replaceCadence": true,
            "riskTier": "Risk On"
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot_as(
        &platform,
        security_id,
        "HOR1",
        "Monthly",
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
            "paymentPeriod": "2026-08-31",
            "source": "issuer",
            "enteredAt": "2026-08-31"
        }),
    )
    .await;
    must_ok(
        &platform,
        "IssuerPayDateReplace",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-09-20",
            "dates": [
                {"payOn": "2026-10-30", "source": "issuer"},
                {"payOn": "2026-11-28", "source": "issuer"},
                {"payOn": "2026-12-31", "source": "issuer"},
                {"payOn": "2027-01-30", "source": "issuer"}
            ]
        }),
    )
    .await;
    must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 10,
            "amountScale": 2,
            "planningPeriodsPerYear": 12,
            "effectiveFrom": "2026-01-02",
            "decisionReason": "owner",
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
    let year = query_json(
        &platform,
        "RemainingYearIncomeGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-09-20" }),
    )
    .await;
    let year_dates: Vec<_> = year["payments"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|p| p["payOn"].as_str())
        .collect();
    assert!(
        !year_dates.iter().any(|d| d.starts_with("2027")),
        "Tax/remaining-year stays this 31 Dec: {year_dates:?}"
    );
    let reg = query_json(
        &platform,
        "CashRegisterGet",
        serde_json::json!({
            "account": "Income",
            "period": "1Y",
            "asOfDate": "2026-09-20",
            "periodStart": "2026-09-01",
            "periodEnd": "2027-02-28",
            "hitsOnly": true
        }),
    )
    .await;
    let plan_days: Vec<&str> = reg["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["source"] == "income-plan" && r["label"] == "HOR1")
        .filter_map(|r| r["occurredOn"].as_str())
        .collect();
    assert!(
        plan_days.iter().any(|d| *d == "2026-10-30"),
        "2026 vendor date in window: {plan_days:?}"
    );
    assert!(
        plan_days.iter().any(|d| *d == "2027-01-30"),
        "vendor 2027 must plot after 2026 weeks in the same range: {plan_days:?} {reg}"
    );
}

async fn seed_horizon_monthly(
    platform: &LocalPlatform,
    symbol: &str,
    paid_on: &str,
) -> (String, String) {
    let income = query_json(platform, "AccountList", serde_json::json!({})).await;
    let income_id = income
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["name"] == "Income")
        .unwrap()["accountId"]
        .as_str()
        .unwrap()
        .to_string();
    let security = must_ok(
        platform,
        "SecurityRegister",
        serde_json::json!({"symbol": symbol, "name": symbol}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap().to_string();
    research_template(platform, &security_id, symbol).await;
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
    must_ok(
        platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "replaceCadence": true,
            "riskTier": "Risk On"
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot_as(
        platform,
        &security_id,
        symbol,
        "Monthly",
        true,
    )
    .await
    .expect("complete collector");
    must_ok(
        platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 1000,
            "amountScale": 4,
            "paymentPeriod": paid_on,
            "source": "issuer",
            "enteredAt": paid_on
        }),
    )
    .await;
    must_ok(
        platform,
        "IssuerPayDateReplace",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-09-20",
            "dates": [
                {"payOn": "2026-10-15", "source": "issuer"},
                {"payOn": "2026-11-15", "source": "issuer"},
                {"payOn": "2026-12-15", "source": "issuer"}
            ]
        }),
    )
    .await;
    must_ok(
        platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 10,
            "amountScale": 2,
            "planningPeriodsPerYear": 12,
            "effectiveFrom": "2026-01-02",
            "decisionReason": "owner",
            "incompleteAnalysisReason": "Fewer than 6 observations"
        }),
    )
    .await;
    must_ok(
        platform,
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
    (security_id, symbol.to_string())
}

/// June prompt writes assumed 2027 holes. 1Y sees them; Tax stays this 31 Dec.
#[tokio::test]
async fn register_1y_sees_assumed_2027_after_confirm() {
    let (_dir, platform, _) = seeded_platform().await;
    let (security_id, symbol) = seed_horizon_monthly(&platform, "HOR2", "2026-08-15").await;
    let preview = query_json(
        &platform,
        "PlanHorizonGet",
        serde_json::json!({ "asOfDate": "2026-09-20" }),
    )
    .await;
    assert_eq!(preview["assumeYear"], 2027);
    assert!(
        preview["holeCount"].as_u64().unwrap_or(0) >= 12,
        "2027 monthly holes: {preview}"
    );
    must_ok(
        &platform,
        "PlanHorizonAssumeNextYear",
        serde_json::json!({ "asOfDate": "2026-09-20" }),
    )
    .await;
    let after = query_json(
        &platform,
        "PlanHorizonGet",
        serde_json::json!({ "asOfDate": "2026-09-20" }),
    )
    .await;
    let still_open = after["names"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["symbol"] == symbol);
    assert!(
        !still_open,
        "assumed 2027 dates close the Confirm task for {symbol}: {after}"
    );
    let year = query_json(
        &platform,
        "RemainingYearIncomeGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-09-20" }),
    )
    .await;
    let year_dates: Vec<_> = year["payments"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|p| p["payOn"].as_str())
        .collect();
    assert!(
        !year_dates.iter().any(|d| d.starts_with("2027")),
        "Tax/remaining-year stays this 31 Dec: {year_dates:?}"
    );
    let reg = query_json(
        &platform,
        "CashRegisterGet",
        serde_json::json!({
            "account": "Income",
            "period": "1Y",
            "asOfDate": "2026-09-20",
            "periodStart": "2026-09-01",
            "periodEnd": "2027-02-28",
            "hitsOnly": true
        }),
    )
    .await;
    let plan_days: Vec<&str> = reg["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["source"] == "income-plan" && r["label"] == symbol)
        .filter_map(|r| r["occurredOn"].as_str())
        .collect();
    assert!(
        plan_days.iter().any(|d| *d == "2026-10-15"),
        "2026 issuer date in window: {plan_days:?}"
    );
    assert!(
        plan_days.iter().any(|d| *d == "2027-01-15"),
        "assumed 2027 hole must plot after Confirm: {plan_days:?} {reg}"
    );
}

/// Vendor-printed 2027 prunes the assumed hole for that slot.
#[tokio::test]
async fn vendor_2027_prunes_assumed_hole() {
    let (_dir, platform, _) = seeded_platform().await;
    let (security_id, symbol) = seed_horizon_monthly(&platform, "HOR3", "2026-08-15").await;
    must_ok(
        &platform,
        "PlanHorizonAssumeNextYear",
        serde_json::json!({ "asOfDate": "2026-09-20" }),
    )
    .await;
    must_ok(
        &platform,
        "IssuerPayDateReplace",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-09-20",
            "dates": [
                {"payOn": "2026-10-15", "source": "issuer"},
                {"payOn": "2026-11-15", "source": "issuer"},
                {"payOn": "2026-12-15", "source": "issuer"},
                {"payOn": "2027-01-30", "source": "issuer"}
            ]
        }),
    )
    .await;
    let preview = query_json(
        &platform,
        "PlanHorizonGet",
        serde_json::json!({ "asOfDate": "2026-09-20" }),
    )
    .await;
    let hor3 = preview["names"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["symbol"] == symbol);
    if let Some(row) = hor3 {
        let holes: Vec<_> = row["holes"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|d| d.as_str())
            .collect();
        assert!(
            !holes.iter().any(|d| d.starts_with("2027-01")),
            "January slot is vendor, not assumed: {holes:?}"
        );
    }
    let reg = query_json(
        &platform,
        "CashRegisterGet",
        serde_json::json!({
            "account": "Income",
            "period": "1Y",
            "asOfDate": "2026-09-20",
            "periodStart": "2026-09-01",
            "periodEnd": "2027-02-28",
            "hitsOnly": true
        }),
    )
    .await;
    let plan_days: Vec<&str> = reg["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["source"] == "income-plan" && r["label"] == symbol)
        .filter_map(|r| r["occurredOn"].as_str())
        .collect();
    assert!(
        plan_days.iter().any(|d| *d == "2027-01-30"),
        "vendor 2027 replaces assumed 15th: {plan_days:?}"
    );
    assert!(
        !plan_days.iter().any(|d| *d == "2027-01-15"),
        "pruned assumed hole must not plot: {plan_days:?}"
    );
}

#[tokio::test]
async fn week_report_splits_income_ira_into_net_fed_state_and_ytd() {
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
    must_ok(
        &platform,
        "CashDistributionPost",
        serde_json::json!({
            "accountId": income["accountId"],
            "activityType": "IRA_Distribution",
            "occurredOn": "2026-02-06",
            "grossMinor": 400000,
            "federalWithholdingMinor": 40000,
            "stateWithholdingMinor": 16000,
            "scale": 2,
            "idempotencyKey": "week-report-feb6"
        }),
    )
    .await;
    let report = query_json(
        &platform,
        "DisbursementWeekReportGet",
        serde_json::json!({"asOfDate": "2026-09-26"}),
    )
    .await;
    let columns = report["columns"].as_array().unwrap();
    let index = |key: &str| {
        columns
            .iter()
            .position(|c| c["key"] == key)
            .unwrap_or_else(|| panic!("missing column {key}"))
    };
    let income_i = index("income");
    let fed_i = index("fed");
    let state_i = index("state");
    let total_i = index("total");
    let rows = report["rows"].as_array().unwrap();
    assert!(
        rows.iter().all(|row| row["periodEnd"].as_str().unwrap() <= "2026-09-26"
            || row["periodStart"].as_str().unwrap() <= "2026-09-26"),
        "report is posted weeks only"
    );
    assert_eq!(rows.len(), 1, "empty weeks stay off the to-date report");
    let week = rows
        .iter()
        .find(|row| row["periodEnd"] == "2026-02-06")
        .expect("week ending 2026-02-06");
    let cell = |i: usize| &week["cells"][i];
    assert_eq!(cell(income_i)["amountMinor"].as_i64(), Some(344_000));
    assert_eq!(cell(income_i)["planned"].as_bool(), Some(false));
    assert_eq!(cell(fed_i)["amountMinor"].as_i64(), Some(40_000));
    assert_eq!(cell(state_i)["amountMinor"].as_i64(), Some(16_000));
    assert_eq!(cell(total_i)["amountMinor"].as_i64(), Some(400_000));
    assert_eq!(report["ytdMinor"][income_i].as_i64(), Some(344_000));
    assert_eq!(report["ytdMinor"][fed_i].as_i64(), Some(40_000));
    assert_eq!(report["ytdMinor"][state_i].as_i64(), Some(16_000));
    assert_eq!(report["ytdMinor"][total_i].as_i64(), Some(400_000));
    let exported = query_json(
        &platform,
        "DisbursementWeekReportExportGet",
        serde_json::json!({"asOfDate": "2026-09-26"}),
    )
    .await;
    assert_eq!(
        exported["defaultFileName"],
        "current-year-totals-2026-09-26.xlsx"
    );
    assert!(exported["bytesBase64"].as_str().unwrap().len() > 32);
}

#[tokio::test]
async fn week_report_remaining_uses_plan_through_year_end() {
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
    for (name, amount) in [("net", 82_000), ("fed", 13_000), ("state", 5_000)] {
        must_ok(
            &platform,
            "CashElementSave",
            serde_json::json!({
                "account": "Income",
                "name": name,
                "kind": "Withdrawal",
                "cadence": "weekly",
                "weekdayOrMonthDay": "Sat",
                "amountMinor": amount,
                "asOfDate": "2026-09-26"
            }),
        )
        .await;
    }
    let report = query_json(
        &platform,
        "DisbursementWeekReportGet",
        serde_json::json!({"asOfDate": "2026-09-26", "scope": "remaining"}),
    )
    .await;
    assert_eq!(report["reportKind"], "remaining");
    let columns = report["columns"].as_array().unwrap();
    let index = |key: &str| {
        columns
            .iter()
            .position(|c| c["key"] == key)
            .unwrap_or_else(|| panic!("missing column {key}"))
    };
    let income_i = index("income");
    let fed_i = index("fed");
    let state_i = index("state");
    let total_i = index("total");
    let rows = report["rows"].as_array().unwrap();
    assert!(
        rows.iter().all(|row| row["periodEnd"].as_str().unwrap() >= "2026-09-26"),
        "remaining starts at the as-of week"
    );
    assert_eq!(rows.len(), 14, "Saturdays from 2026-09-26 through 2026-12-26");
    let week = rows
        .iter()
        .find(|row| row["periodEnd"] == "2026-10-02")
        .expect("week ending 2026-10-02");
    assert_eq!(week["cells"][income_i]["amountMinor"].as_i64(), Some(82_000));
    assert_eq!(week["cells"][income_i]["planned"].as_bool(), Some(true));
    assert_eq!(week["cells"][fed_i]["amountMinor"].as_i64(), Some(13_000));
    assert_eq!(week["cells"][state_i]["amountMinor"].as_i64(), Some(5_000));
    assert_eq!(week["cells"][total_i]["amountMinor"].as_i64(), Some(100_000));
    assert_eq!(report["ytdMinor"][income_i].as_i64(), Some(14 * 82_000));
    assert_eq!(report["ytdMinor"][total_i].as_i64(), Some(14 * 100_000));
    let posted = query_json(
        &platform,
        "DisbursementWeekReportGet",
        serde_json::json!({"asOfDate": "2026-09-26", "scope": "posted"}),
    )
    .await;
    assert_eq!(posted["reportKind"], "posted");
    assert!(posted["rows"].as_array().unwrap().is_empty());
    let exported = query_json(
        &platform,
        "DisbursementWeekReportExportGet",
        serde_json::json!({"asOfDate": "2026-09-26", "scope": "remaining"}),
    )
    .await;
    assert_eq!(
        exported["defaultFileName"],
        "remaining-this-year-2026-09-26.xlsx"
    );
}

#[tokio::test]
async fn one_time_on_tenth_fifth_shows_in_this_week() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "FI Roth", "kind": "roth"}),
    )
    .await;
    let saved = must_ok(
        &platform,
        "CashElementSave",
        serde_json::json!({
            "account": "FI Roth",
            "name": "Roth transfer",
            "kind": "Withdrawal",
            "cadence": "one-time",
            "weekdayOrMonthDay": "10/5",
            "startOn": "2026-10-05",
            "amountMinor": 15_000,
            "scale": 2,
            "asOfDate": "2026-10-09",
            "occurrences": []
        }),
    )
    .await;
    let occ_id = saved["occurrences"][0]["occurrenceId"].as_str().unwrap();
    assert_eq!(saved["occurrences"][0]["occurredOn"], "2026-10-05");
    assert_eq!(saved["occurrences"][0]["amountMinor"], 15_000);
    let ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-10-04"}),
    )
    .await;
    assert!(
        ahead["rows"].as_array().unwrap().iter().any(|row| {
            row["account"] == "FI Roth" && row["occurredOn"] == "2026-10-05"
        }),
        "10/5 is inside Sat 10/3–Fri 10/9: {ahead}"
    );
    must_ok(
        &platform,
        "PlannedOccurrenceDelete",
        serde_json::json!({ "occurrenceId": occ_id }),
    )
    .await;
    let again = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-10-04"}),
    )
    .await;
    assert!(
        again["rows"].as_array().unwrap().iter().any(|row| {
            row["account"] == "FI Roth" && row["occurredOn"] == "2026-10-05"
        }),
        "a saved one-time with no row is created when the week opens: {again}"
    );
}

#[tokio::test]
async fn income_withdrawal_tenth_tenth_edits_as_an_exception() {
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
    let saved = must_ok(
        &platform,
        "CashElementSave",
        serde_json::json!({
            "account": "Income",
            "name": "net",
            "kind": "Withdrawal",
            "cadence": "weekly",
            "weekdayOrMonthDay": "Sat",
            "amountMinor": 10_000,
            "scale": 2,
            "asOfDate": "2026-10-04",
            "occurrences": []
        }),
    )
    .await;
    let element_id = saved["element"]["elementId"].as_str().unwrap();
    let list = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "Income", "asOfDate": "2026-10-04"}),
    )
    .await;
    let item = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["elementId"] == element_id)
        .expect("income withdrawal");
    let first = &item["upcoming"][0];
    assert_eq!(first["occurredOn"], "2026-10-10");
    assert_eq!(item["amountMinor"], 10_000);
    let occurrence_id = first["occurrenceId"].as_str().unwrap();
    must_ok(
        &platform,
        "PlannedOccurrenceSave",
        serde_json::json!({
            "elementId": element_id,
            "occurrenceId": occurrence_id,
            "occurredOn": "2026-10-10",
            "amountMinor": 2_500,
            "scale": 2,
            "asOfDate": "2026-10-04",
            "cancel": false
        }),
    )
    .await;
    let after = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "Income", "asOfDate": "2026-10-04"}),
    )
    .await;
    let edited = after["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["elementId"] == element_id)
        .unwrap();
    assert_eq!(edited["amountMinor"], 10_000, "the series amount stays");
    let hit = edited["upcoming"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["occurredOn"] == "2026-10-10")
        .unwrap();
    assert_eq!(hit["amountMinor"], 2_500);
    assert_eq!(hit["isException"], true);
}

#[tokio::test]
async fn open_income_on_october_third_stays_editable() {
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
    let saved = must_ok(
        &platform,
        "CashElementSave",
        serde_json::json!({
            "account": "Income",
            "name": "net",
            "kind": "Withdrawal",
            "cadence": "weekly",
            "weekdayOrMonthDay": "Sat",
            "amountMinor": 10_000,
            "scale": 2,
            "asOfDate": "2026-10-04",
            "occurrences": []
        }),
    )
    .await;
    assert_eq!(saved["element"]["amountMinor"], 10_000);
    let element_id = saved["element"]["elementId"].as_str().unwrap();
    let ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-10-04"}),
    )
    .await;
    assert!(
        ahead["rows"].as_array().unwrap().iter().any(|row| {
            row["account"] == "Income" && row["occurredOn"] == "2026-10-03"
        }),
        "Oct 3 is the open Saturday of this week: {ahead}"
    );
    let list = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "Income", "asOfDate": "2026-10-09"}),
    )
    .await;
    let item = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["elementId"] == element_id)
        .expect("income withdrawal");
    let open = item["upcoming"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["occurredOn"] == "2026-10-03")
        .expect("Open Oct 3 stays on the exception list when as-of is Friday");
    assert_eq!(open["amountMinor"], 10_000);
    assert_eq!(open["isCancelled"], false);
    assert_eq!(item["amountMinor"], 10_000);
    let dates: Vec<&str> = item["upcoming"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|row| row["occurredOn"].as_str())
        .collect();
    assert!(
        dates.contains(&"2026-10-03")
            && dates.contains(&"2026-10-10")
            && dates.contains(&"2026-10-17"),
        "the forward weeks stay, and the open Oct 3 is on the list: {dates:?}"
    );
    let occurrence_id = open["occurrenceId"].as_str().unwrap();
    must_ok(
        &platform,
        "PlannedOccurrenceSave",
        serde_json::json!({
            "elementId": element_id,
            "occurrenceId": occurrence_id,
            "occurredOn": "2026-10-03",
            "amountMinor": 2_500,
            "scale": 2,
            "asOfDate": "2026-10-09",
            "cancel": false
        }),
    )
    .await;
    let edited = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "Income", "asOfDate": "2026-10-04"}),
    )
    .await;
    let after = edited["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["elementId"] == element_id)
        .unwrap();
    assert_eq!(after["amountMinor"], 10_000, "the series amount stays");
    let hit = after["upcoming"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["occurredOn"] == "2026-10-03")
        .unwrap();
    assert_eq!(hit["amountMinor"], 2_500);
    assert_eq!(hit["isException"], true);
    must_ok(
        &platform,
        "WeekAheadConfirm",
        serde_json::json!({ "occurrenceId": occurrence_id }),
    )
    .await;
    let posted = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({"account": "Income", "asOfDate": "2026-10-04"}),
    )
    .await;
    let frozen = posted["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["elementId"] == element_id)
        .unwrap();
    assert!(
        !frozen["upcoming"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["occurrenceId"] == occurrence_id),
        "a confirmed payment leaves the Open list: {posted}"
    );
    let refused = execute_command_on(
        &platform,
        &platform,
        cmd(
            "PlannedOccurrenceSave",
            serde_json::json!({
                "elementId": element_id,
                "occurrenceId": occurrence_id,
                "occurredOn": "2026-10-03",
                "amountMinor": 1_000,
                "scale": 2,
                "asOfDate": "2026-10-04",
                "cancel": false
            }),
        ),
    )
    .await;
    assert!(!refused.ok, "posted history stays frozen");
    assert_eq!(refused.error_code.as_deref(), Some("occurrence_confirmed"));
}

#[tokio::test]
async fn fi_roth_confirm_posts_on_fi_roth_not_income() {
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
        serde_json::json!({"name": "FI Roth", "kind": "fi_roth"}),
    )
    .await;
    let saved = must_ok(
        &platform,
        "CashElementSave",
        serde_json::json!({
            "account": "FI Roth",
            "name": "One time Roth",
            "kind": "Withdrawal",
            "cadence": "one-time",
            "weekdayOrMonthDay": "10/5",
            "startOn": "2026-10-05",
            "amountMinor": 12_000,
            "scale": 2,
            "asOfDate": "2026-10-09",
            "occurrences": []
        }),
    )
    .await;
    assert_eq!(saved["element"]["account"], "FI Roth");
    assert_eq!(saved["element"]["amountMinor"], 12_000);
    let ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({"asOfDate": "2026-10-04"}),
    )
    .await;
    let row = ahead["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["account"] == "FI Roth" && r["occurredOn"] == "2026-10-05")
        .expect("FI Roth one-time is on the week");
    let posted = must_ok(
        &platform,
        "WeekAheadConfirm",
        serde_json::json!({ "occurrenceId": row["occurrenceId"] }),
    )
    .await;
    assert_eq!(posted["activityType"], "Roth_Distribution");
    assert_eq!(posted["amountMinor"], 12_000);
    assert_eq!(posted["scale"], 2);
    let income = query_json(
        &platform,
        "CashRegisterGet",
        serde_json::json!({
            "account": "Income",
            "period": "1M",
            "asOfDate": "2026-10-09"
        }),
    )
    .await;
    assert!(
        !income["rows"].as_array().unwrap().iter().any(|r| {
            r["occurredOn"] == "2026-10-05" || r["label"] == "One time Roth"
        }),
        "FI Roth confirmation stays off the Income register: {income}"
    );
    let roth = query_json(
        &platform,
        "CashRegisterGet",
        serde_json::json!({
            "account": "FI Roth",
            "period": "1M",
            "asOfDate": "2026-10-09"
        }),
    )
    .await;
    let hit = roth["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["occurredOn"] == "2026-10-05")
        .expect("confirmed row is on the FI Roth register");
    assert_eq!(hit["withdrawalMinor"], 12_000);
    assert_eq!(hit["scale"], 2);
    assert_eq!(hit["posted"], true);
    assert_eq!(hit["label"], "One time Roth");
}
