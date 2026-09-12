use application_core::contracts::{
    CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::queries::{execute_command_on, execute_query_on};
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

/// Process B requires a researched identity before LotOpen.
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

async fn record_decl(platform: &LocalPlatform, security_id: &str, period: &str) {
    must_ok(
        platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 1000,
            "amountScale": 4,
            "paymentPeriod": period,
            "source": "provider-site",
            "enteredAt": "2026-08-22"
        }),
    )
    .await;
}

#[tokio::test]
async fn week_dividend_actuals_match_dividend_get_by_account() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let income = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
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
        serde_json::json!({"name": "Account 9", "kind": "taxable"}),
    )
    .await;
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "VTI", "name": "Vanguard Total"}),
    )
    .await;
    must_ok(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": income["accountId"],
            "securityId": security["securityId"],
            "activityType": "dividend",
            "amountMinor": 12_500,
            "scale": 2,
            "occurredOn": "2026-08-17",
            "idempotencyKey": "week-income-vti"
        }),
    )
    .await;
    let car = query_json(&platform, "AccountList", serde_json::json!({})).await;
    let car_id = car
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["name"] == "Car")
        .unwrap()["accountId"]
        .clone();
    must_ok(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": car_id,
            "securityId": security["securityId"],
            "activityType": "dividend",
            "amountMinor": 4_000,
            "scale": 2,
            "occurredOn": "2026-08-19",
            "idempotencyKey": "week-car-vti"
        }),
    )
    .await;
    must_ok(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": income["accountId"],
            "securityId": security["securityId"],
            "activityType": "dividend",
            "amountMinor": 99_000,
            "scale": 2,
            "occurredOn": "2026-08-10",
            "idempotencyKey": "prior-week"
        }),
    )
    .await;

    let dividend = query_json(&platform, "DividendGet", serde_json::json!({})).await;
    let week_actuals: i64 = dividend["actuals"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| {
            let on = row["occurredOn"].as_str().unwrap_or("");
            on >= "2026-08-15" && on <= "2026-08-21"
        })
        .map(|row| row["amountMinor"].as_i64().unwrap_or(0))
        .sum();

    let week = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({"asOfDate": "2026-08-18"}),
    )
    .await;
    assert_eq!(week["start"], "2026-08-15");
    assert_eq!(week["end"], "2026-08-21");
    assert_eq!(week["weekYear"], 2026);
    assert_eq!(week["weekNumber"], 33);
    assert_eq!(week["status"], "Open");
    let income_line = week["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["accountName"] == "Income")
        .unwrap();
    assert_eq!(income_line["actualMinor"].as_i64().unwrap(), 12_500);
    assert_eq!(income_line["planKnown"], false);
    let car_line = week["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["accountName"] == "Car")
        .unwrap();
    assert_eq!(car_line["actualMinor"].as_i64().unwrap(), 4_000);
    let nine = week["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["accountName"] == "9")
        .unwrap();
    assert_eq!(nine["actualMinor"].as_i64().unwrap(), 0);
    let vti = week["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "VTI")
        .expect("VTI position week row");
    assert_eq!(vti["actualMinor"].as_i64().unwrap(), 16_500);
    assert_eq!(vti["actualKnown"], true);
    assert_eq!(vti["planKnown"], false);
    assert_eq!(vti["declarationKnown"], false);
    let grid_total: i64 = week["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["actualMinor"].as_i64().unwrap_or(0))
        .sum();
    assert_eq!(grid_total, week_actuals);

    let burn = query_json(
        &platform,
        "DashboardBurndownGet",
        serde_json::json!({"asOfDate": "2026-08-18"}),
    )
    .await;
    let burn_nine = burn["lines"]
        .as_array()
        .unwrap()
        .iter()
        .any(|l| l["accountName"] == "Account 9");
    assert!(!burn_nine, "Account 9 must not appear on burndown");
    let burn_income = burn["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["accountName"] == "Income")
        .unwrap();
    assert_eq!(burn_income["inflowMinor"].as_i64().unwrap(), 12_500);
    assert_eq!(burn_income["floorKnown"], false);

    let latest = query_json(&platform, "IncomePlanWeekGet", serde_json::json!({})).await;
    assert_eq!(latest["latestActualOn"], "2026-08-19");
    assert_eq!(latest["yieldCount"].as_u64().unwrap(), 3);
    let latest_income = latest["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["accountName"] == "Income")
        .unwrap();
    assert_eq!(latest_income["actualMinor"].as_i64().unwrap(), 12_500);

    let summary = query_json(&platform, "DataSummaryGet", serde_json::json!({})).await;
    assert_eq!(summary["accountCount"].as_u64().unwrap(), 3);
    assert_eq!(summary["yieldCount"].as_u64().unwrap(), 3);
    assert_eq!(summary["latestYieldOn"], "2026-08-19");
}

#[tokio::test]
async fn weekly_payer_lists_every_week_with_plan_even_without_that_week_actual() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let income = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "NVDW", "name": "NVDW"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    research_template(&platform, security_id, "NVDW").await;
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
        "NVDW",
        "Weekly",
        true,
    )
    .await
    .expect("complete collector");
    record_decl(&platform, security_id, "2026-07-28").await;
    must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 35,
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
            "accountId": income["accountId"],
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
        "ActivityPost",
        serde_json::json!({
            "accountId": income["accountId"],
            "securityId": security_id,
            "activityType": "dividend",
            "amountMinor": 3_500,
            "scale": 2,
            "occurredOn": "2026-07-28",
            "idempotencyKey": "nvdw-prior"
        }),
    )
    .await;
    for as_of in ["2026-08-29", "2026-09-05"] {
        let week = query_json(
            &platform,
            "IncomePlanWeekGet",
            serde_json::json!({ "asOfDate": as_of }),
        )
        .await;
        let row = week["positions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["symbol"] == "NVDW")
            .unwrap_or_else(|| panic!("NVDW missing from week {as_of}: {week}"));
        assert_eq!(row["cadence"], "Weekly", "{as_of}");
        assert_eq!(row["planKnown"], true, "{as_of}");
        assert!(row["plannedMinor"].as_i64().unwrap_or(0) > 0, "{as_of} {row}");
        assert_eq!(row["actualKnown"], false, "{as_of}");
        assert_eq!(row["actualMinor"].as_i64().unwrap(), 0, "{as_of}");
        assert_eq!(row["declarationKnown"], false, "{as_of}");
    }
}

#[tokio::test]
async fn weekly_last_week_issuer_row_does_not_clone_onto_next_forecast_friday() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let income = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "AMDY", "name": "AMDY"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    research_template(&platform, security_id, "AMDY").await;
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
        "AMDY",
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
            "amountPerShareMinor": 3763,
            "amountScale": 4,
            "paymentPeriod": "2026-09-11",
            "source": "yieldmax",
            "enteredAt": "2026-09-09"
        }),
    )
    .await;
    must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 40,
            "amountScale": 2,
            "planningPeriodsPerYear": 52,
            "effectiveFrom": "2026-08-01",
            "decisionReason": "owner",
            "incompleteAnalysisReason": "Fewer than 6 observations"
        }),
    )
    .await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": income["accountId"],
            "securityId": security_id,
            "openedOn": "2026-01-02",
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
        "IssuerPayDateReplace",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-09-12",
            "dates": [
                {"payOn": "2026-09-11", "source": "yieldmax"},
                {"payOn": "2026-09-18", "source": "derived_walk"}
            ]
        }),
    )
    .await;
    let w36 = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({ "asOfDate": "2026-09-11" }),
    )
    .await;
    let last = w36["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "AMDY")
        .unwrap_or_else(|| panic!("AMDY missing W36: {w36}"));
    assert_eq!(last["declarationKnown"], true, "{last}");
    assert_eq!(last["payOn"], "2026-09-11");
    let w37 = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({ "asOfDate": "2026-09-12" }),
    )
    .await;
    let next = w37["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "AMDY")
        .unwrap_or_else(|| panic!("AMDY missing W37 forecast row: {w37}"));
    assert_eq!(next["declarationKnown"], false, "{next}");
    assert_eq!(next["payOn"], "2026-09-18");
}

#[tokio::test]
async fn monthly_with_prior_actual_uses_issuer_date_not_thirty_day_walk() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let car = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Car", "kind": "taxable"}),
    )
    .await;
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "HAKY", "name": "HAKY"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    research_template(&platform, security_id, "HAKY").await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "riskTier": "Risk On"
        }),
    )
    .await;
    record_decl(&platform, security_id, "2026-07-15").await;
    must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 3800,
            "amountScale": 4,
            "planningPeriodsPerYear": 12,
            "effectiveFrom": "2026-08-29",
            "decisionReason": "Most Current",
            "incompleteAnalysisReason": "Fewer than 6 observations"
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot_as(
        &platform,
        security_id,
        "HAKY",
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
            "openedOn": "2026-08-21",
            "origin": "purchase",
            "quantityMinor": 70,
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
        "ActivityPost",
        serde_json::json!({
            "accountId": car["accountId"],
            "securityId": security_id,
            "activityType": "dividend",
            "amountMinor": 2_600,
            "scale": 2,
            "occurredOn": "2026-07-15",
            "idempotencyKey": "haky-prior"
        }),
    )
    .await;
    must_ok(
        &platform,
        "IssuerPayDateReplace",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-08-29",
            "dates": [
                {"payOn": "2026-08-31", "source": "amplify"},
                {"payOn": "2026-09-30", "source": "amplify"}
            ]
        }),
    )
    .await;
    let week = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({ "asOfDate": "2026-08-30" }),
    )
    .await;
    let row = week["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "HAKY")
        .unwrap_or_else(|| panic!("HAKY missing; 30-day walk from 2026-07-15 is not 2026-08-31: {week}"));
    assert_eq!(row["cadence"], "Monthly");
    assert_eq!(row["payOn"], "2026-08-31");
    assert_eq!(row["planKnown"], true);
    assert!(row["plannedMinor"].as_i64().unwrap_or(0) > 0);
    let miss = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({ "asOfDate": "2026-08-22" }),
    )
    .await;
    let absent = miss["positions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["symbol"] == "HAKY");
    assert!(!absent, "HAKY must not appear in a week without a remaining pay date: {miss}");
}

#[tokio::test]
async fn pay_week_without_plan_history_still_lists_position_plan_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let income = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "OPEN", "name": "OPEN"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    research_template(&platform, security_id, "OPEN").await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "riskTier": "Core"
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot_as(
        &platform,
        security_id,
        "OPEN",
        "Monthly",
        false,
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
            "paymentPeriod": "2026-07-01",
            "source": "provider-site",
            "enteredAt": "2026-08-22"
        }),
    )
    .await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": income["accountId"],
            "securityId": security_id,
            "openedOn": "2026-08-21",
            "origin": "purchase",
            "quantityMinor": 50,
            "quantityScale": 0,
            "performanceBasisMinor": 100_000,
            "taxBasisMinor": 100_000,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;
    let week = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({ "asOfDate": "2026-08-30" }),
    )
    .await;
    let row = week["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "OPEN")
        .unwrap_or_else(|| panic!("OPEN should list on pay week without PlanHistory: {week}"));
    assert_eq!(row["planKnown"], false);
    assert_eq!(row["plannedMinor"].as_i64().unwrap(), 0);
    assert_eq!(row["payOn"], "2026-08-31");
}

#[tokio::test]
async fn dividend_performance_known_plan_percent_excludes_this_and_future_week() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let income = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "NVDW", "name": "NVDW"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    research_template(&platform, security_id, "NVDW").await;
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
        "NVDW",
        "Weekly",
        true,
    )
    .await
    .expect("complete collector");
    record_decl(&platform, security_id, "2026-07-28").await;
    must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 35,
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
            "accountId": income["accountId"],
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
        "ActivityPost",
        serde_json::json!({
            "accountId": income["accountId"],
            "securityId": security_id,
            "activityType": "dividend",
            "amountMinor": 3_500,
            "scale": 2,
            "occurredOn": "2026-07-28",
            "idempotencyKey": "nvdw-prior"
        }),
    )
    .await;

    let this_week = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({ "asOfDate": "2026-08-29" }),
    )
    .await;
    assert_eq!(this_week["start"], "2026-08-29");

    let perf = query_json(
        &platform,
        "DividendPerformanceGet",
        serde_json::json!({ "asOfDate": "2026-08-29", "range": "30d" }),
    )
    .await;
    assert_eq!(perf["thisWeekStart"], "2026-08-29");
    assert_eq!(perf["range"], "30d");
    let weeks = perf["weeks"].as_array().unwrap();
    assert!(
        weeks.iter().all(|w| w["end"].as_str().unwrap() < "2026-08-29"),
        "this/future week must not appear: {perf}"
    );
    let paid = weeks
        .iter()
        .find(|w| w["end"] == "2026-07-31")
        .unwrap_or_else(|| panic!("week ending 2026-07-31 missing: {perf}"));
    assert_eq!(paid["actualMinor"].as_i64().unwrap(), 3_500);
    assert_eq!(paid["planKnown"], true);
    assert_eq!(paid["pctOfPlanMinor"].as_i64().unwrap(), 10_000);
    let nvdw = paid["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "NVDW")
        .unwrap();
    assert_eq!(nvdw["pctOfPlanMinor"].as_i64().unwrap(), 10_000);

    let dividend = query_json(&platform, "DividendGet", serde_json::json!({})).await;
    let window_actual: i64 = dividend["actuals"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| {
            let on = row["occurredOn"].as_str().unwrap_or("");
            weeks.iter().any(|w| {
                let start = w["start"].as_str().unwrap_or("");
                let end = w["end"].as_str().unwrap_or("");
                on >= start && on <= end
            })
        })
        .map(|row| row["amountMinor"].as_i64().unwrap_or(0))
        .sum();
    assert_eq!(perf["summary"]["actualMinor"].as_i64().unwrap(), window_actual);
    assert_eq!(window_actual, 3_500);
}

#[tokio::test]
async fn dividend_performance_unknown_plan_is_na_not_zero_percent() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let income = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "VTI", "name": "Vanguard Total"}),
    )
    .await;
    must_ok(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": income["accountId"],
            "securityId": security["securityId"],
            "activityType": "dividend",
            "amountMinor": 12_500,
            "scale": 2,
            "occurredOn": "2026-08-17",
            "idempotencyKey": "week-income-vti"
        }),
    )
    .await;
    let perf = query_json(
        &platform,
        "DividendPerformanceGet",
        serde_json::json!({ "asOfDate": "2026-08-26", "range": "all" }),
    )
    .await;
    let weeks = perf["weeks"].as_array().unwrap();
    let paid = weeks
        .iter()
        .find(|w| w["end"] == "2026-08-21")
        .unwrap_or_else(|| panic!("week ending 2026-08-21 missing: {perf}"));
    assert_eq!(paid["actualMinor"].as_i64().unwrap(), 12_500);
    assert_eq!(paid["planKnown"], false);
    assert!(paid["pctOfPlanMinor"].is_null(), "unknown Plan must be N/A: {paid}");
    assert!(
        perf["summary"]["pctOfPlanMinor"].is_null(),
        "range % must not treat unknown Plan as 0%: {perf}"
    );
}

/// Plan $ is owner. Declaration $ is issuer payable in the Sat–Fri week. Actual $ is broker import.
/// Collect does not write plan. Missing declaration or actual is unknown, never $0.
#[tokio::test]
async fn week_carries_plan_declaration_actual_as_three_amounts() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let income = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "PAY1", "name": "PAY1"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    research_template(&platform, security_id, "PAY1").await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "provider": "Issuer",
            "divType": "DIV-1",
            "riskTier": "Core"
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot_as(
        &platform,
        security_id,
        "PAY1",
        "Monthly",
        false,
    )
    .await
    .expect("complete collector");
    must_ok(
        &platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 1100,
            "amountScale": 4,
            "paymentPeriod": "2026-08-31",
            "source": "issuer",
            "enteredAt": "2026-08-28"
        }),
    )
    .await;
    must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 1200,
            "amountScale": 4,
            "planningPeriodsPerYear": 12,
            "effectiveFrom": "2026-08-01",
            "decisionReason": "owner",
            "incompleteAnalysisReason": "Fewer than 6 observations"
        }),
    )
    .await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": income["accountId"],
            "securityId": security_id,
            "openedOn": "2026-01-02",
            "origin": "purchase",
            "quantityMinor": 10,
            "quantityScale": 0,
            "performanceBasisMinor": 10_000,
            "taxBasisMinor": 10_000,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "IssuerPayDateReplace",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-08-29",
            "dates": [{"payOn": "2026-08-31", "source": "issuer"}]
        }),
    )
    .await;
    let week = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({ "asOfDate": "2026-08-30" }),
    )
    .await;
    let row = week["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "PAY1")
        .unwrap_or_else(|| panic!("PAY1 missing: {week}"));
    assert_eq!(row["planKnown"], true);
    assert_eq!(row["plannedMinor"], 120);
    assert_eq!(row["declarationKnown"], true);
    assert_eq!(row["declarationMinor"], 110);
    assert_eq!(row["declarationPerShareMinor"], 1100);
    assert_eq!(row["declarationPerShareScale"], 4);
    assert_eq!(row["declarationEnteredOn"], "2026-08-28");
    assert_eq!(row["declarationCurrent"], true);
    assert_eq!(row["actualKnown"], false);
    assert_eq!(row["actualMinor"], 0);
    assert_eq!(row["payOn"], "2026-08-31");

    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "PAY1",
            "declarationSource": "issuer",
            "asOfDate": "2026-08-30",
            "candidates": [{
                "securityId": security_id,
                "amountPerShareMinor": 1300,
                "amountScale": 4,
                "paymentPeriod": "2026-08-31",
                "source": "issuer"
            }]
        }),
    )
    .await;
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-08-30" }),
    )
    .await;
    assert_eq!(inv["planPerShareMinor"], 1200);
    assert_eq!(inv["planScale"], 4);
    let after_collect = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({ "asOfDate": "2026-08-30" }),
    )
    .await;
    let collected = after_collect["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "PAY1")
        .unwrap();
    assert_eq!(collected["plannedMinor"], 120, "collect must not write plan $");
    assert_eq!(collected["declarationKnown"], true);
    assert_eq!(collected["actualKnown"], false);

    must_ok(
        &platform,
        "DividendActualRecord",
        serde_json::json!({
            "accountId": income["accountId"],
            "securityId": security_id,
            "occurredOn": "2026-08-31",
            "amountMinor": 105,
            "scale": 2,
            "idempotencyKey": "pay1-broker"
        }),
    )
    .await;
    let after_import = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({ "asOfDate": "2026-08-30" }),
    )
    .await;
    let imported = after_import["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "PAY1")
        .unwrap();
    assert_eq!(imported["plannedMinor"], 120);
    assert_eq!(imported["actualKnown"], true);
    assert_eq!(imported["actualMinor"], 105);
    assert_eq!(imported["declarationKnown"], true);
}

#[test]
fn weekly_report_by_position_keeps_decl_per_share_column() {
    let ui = std::fs::read_to_string(
        golden_harness::repo_root().join("packages/ui-components/src/index.tsx"),
    )
    .expect("ui-components");
    assert!(
        ui.contains("By position for the week"),
        "Weekly report heading must stay"
    );
    assert!(
        ui.contains("aria-label=\"Income plan by position\""),
        "by-position table must stay"
    );
    assert!(
        ui.contains("<th className=\"ip-decl-sh\">Decl $/sh</th>"),
        "Decl $/sh column header must stay on Weekly report by position"
    );
    assert!(
        ui.contains("declarationPerShareMinor"),
        "week rows must still bind per-share declaration"
    );
    assert!(
        ui.contains("function declShareTone") && ui.contains("ip-decl-${declShareTone"),
        "Decl $/sh must stay color-coded current/stale/none"
    );
    let css = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/App.css"),
    )
    .expect("App.css");
    assert!(
        css.contains("td.numeric.ip-decl-current")
            && css.contains("td.numeric.ip-decl-stale")
            && css.contains("td.numeric.ip-decl-none"),
        "numeric Decl $/sh cells must keep current/stale/none colors"
    );
}
