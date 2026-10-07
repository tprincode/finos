use application_core::contracts::{
    AssumedPayDateRecord, CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::ports::canonical::Canonical;
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
        .find(|l| l["accountName"] == "Account 9")
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
async fn dividend_performance_known_plan_percent_includes_the_open_week() {
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
        weeks.iter().any(|w| w["start"] == "2026-08-29" && w["end"] == "2026-09-04"),
        "the week containing asOf must appear: {perf}"
    );
    let paid = weeks
        .iter()
        .find(|w| w["end"] == "2026-07-31")
        .unwrap_or_else(|| panic!("week ending 2026-07-31 missing: {perf}"));
    assert_eq!(paid["actualMinor"].as_i64().unwrap(), 3_500);
    assert_eq!(
        paid["planKnown"], false,
        "plan effective 2026-08-12 is not in force for the 2026-07-31 week: {paid}"
    );
    let open = weeks
        .iter()
        .find(|w| w["end"] == "2026-09-04")
        .unwrap_or_else(|| panic!("open week missing: {perf}"));
    assert_eq!(open["planKnown"], true, "open week is after plan effective 2026-08-12: {open}");
    assert!(open["plannedMinor"].as_i64().unwrap_or(0) > 0, "{open}");

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

#[tokio::test]
async fn dividend_performance_keeps_empty_gap_weeks() {
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
            "occurredOn": "2026-07-28",
            "idempotencyKey": "gap-week-vti"
        }),
    )
    .await;
    let perf = query_json(
        &platform,
        "DividendPerformanceGet",
        serde_json::json!({ "asOfDate": "2026-08-26", "range": "30d" }),
    )
    .await;
    let weeks = perf["weeks"].as_array().unwrap();
    let ends: Vec<&str> = weeks
        .iter()
        .map(|w| w["end"].as_str().unwrap())
        .collect();
    assert_eq!(
        ends,
        vec![
            "2026-07-31",
            "2026-08-07",
            "2026-08-14",
            "2026-08-21",
            "2026-08-28",
        ],
        "30d must list every Satâ€“Fri week through the week containing asOf, including empty gaps: {perf}"
    );
    let paid = weeks.iter().find(|w| w["end"] == "2026-07-31").unwrap();
    assert_eq!(paid["actualMinor"].as_i64().unwrap(), 12_500);
    let gap = weeks.iter().find(|w| w["end"] == "2026-08-07").unwrap();
    assert_eq!(gap["planKnown"], false);
    assert_eq!(gap["declarationKnown"], false);
}

#[tokio::test]
async fn dividend_performance_week_plan_survives_one_unplanned_name() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let income = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let planned = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "NVDW", "name": "NVDW"}),
    )
    .await;
    let planned_id = planned["securityId"].as_str().unwrap();
    research_template(&platform, planned_id, "NVDW").await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": planned_id,
            "paymentFrequency": "Weekly",
            "replaceCadence": true,
            "riskTier": "Risk On"
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot_as(
        &platform,
        planned_id,
        "NVDW",
        "Weekly",
        true,
    )
    .await
    .expect("complete collector");
    record_decl(&platform, planned_id, "2026-08-17").await;
    must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": planned_id,
            "amountPerShareMinor": 35,
            "amountScale": 2,
            "planningPeriodsPerYear": 52,
            "effectiveFrom": "2026-01-03",
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
            "securityId": planned_id,
            "openedOn": "2026-01-05",
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
    let leftover = must_ok(
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
            "securityId": leftover["securityId"],
            "activityType": "dividend",
            "amountMinor": 12_500,
            "scale": 2,
            "occurredOn": "2026-08-17",
            "idempotencyKey": "week-income-vti-mixed"
        }),
    )
    .await;
    let perf = query_json(
        &platform,
        "DividendPerformanceGet",
        serde_json::json!({ "asOfDate": "2026-08-26", "range": "all" }),
    )
    .await;
    let paid = perf["weeks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["end"] == "2026-08-21")
        .unwrap_or_else(|| panic!("week ending 2026-08-21 missing: {perf}"));
    assert_eq!(
        paid["planKnown"], true,
        "one unplanned name must not blank the week's Plan: {paid}"
    );
    assert_eq!(paid["plannedMinor"].as_i64().unwrap(), 3_500);
    let vti = paid["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "VTI")
        .unwrap();
    assert_eq!(vti["planKnown"], false);
}

/// Plan $ is owner. Declaration $ is the week's money (issuer payable in the Satâ€“Fri week).
/// Collect does not write plan. Missing declaration stays unknown, never $0.
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
fn weekly_grid_week_window_applies_on_select() {
    let ui = std::fs::read_to_string(
        golden_harness::repo_root().join("packages/ui-components/src/index.tsx"),
    )
    .expect("ui-components");
    let app = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/App.tsx"),
    )
    .expect("App.tsx");
    let income_plan = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/features/income-plan/IncomePlanScreen.tsx"),
    )
    .expect("IncomePlanScreen");
    assert!(
        ui.contains("aria-label=\"Historical weeks\"")
            && ui.contains("aria-label=\"Future weeks\""),
        "week window controls must stay labeled"
    );
    assert!(
        ui.contains("<select\n            aria-label=\"Historical weeks\"")
            && ui.contains("<select\n            aria-label=\"Future weeks\""),
        "week window is a closed list, not a spinner that refreshes each tick"
    );
    assert!(
        !ui.contains("Apply week window") && !ui.contains("Go weeks"),
        "week window is a view filter — no second commit button"
    );
    assert!(
        (app.contains("onHistoricalWeeks") || income_plan.contains("onHistoricalWeeks"))
            && (app.contains("withIncomeLoading") || income_plan.contains("withIncomeLoading")),
        "choosing a week count must reload the grid after the list closes"
    );
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
        ui.contains("<th className=\"ip-plan-sh\">Plan $/sh</th>"),
        "Plan $/sh column must sit between Pay date and Decl $/sh"
    );
    assert!(
        ui.contains("formatMonthDay"),
        "Pay date and Last Update must format as MM-DD without year"
    );
    assert!(
        ui.contains("planPerShareMinor"),
        "week rows must bind plan per-share"
    );
    assert!(
        ui.contains("declarationPerShareMinor"),
        "week rows must still bind per-share declaration"
    );
    assert!(
        !ui.contains("<th>Actual $</th>"),
        "Weekly report must not show broker Actual $; Decl $ is the week's money"
    );
    assert!(
        ui.contains("Week declared") && !ui.contains("Week actual"),
        "KPI must label declared, not broker actual"
    );
    assert!(
        !ui.contains("Amount exceptions"),
        "paid-vs-plan exceptions stay off the weekly report"
    );
    assert!(
        ui.contains("ip-current") && ui.contains("Current"),
        "Weekly report must show a Current subtotals row"
    );
    assert!(
        ui.contains("ip-grand") && ui.contains("Grand Total"),
        "Weekly report must keep Grand Total"
    );
    assert!(
        ui.contains("function declShareTone")
            && ui.contains("ip-decl-${tone}")
            && ui.contains("tone === \"stale\""),
        "Decl $/sh must green-code current only; stale stays unhighlighted"
    );
    assert!(
        ui.contains("Declarations within the last 5 days are Green")
            && ui.contains("ip-pos-head")
            && ui.contains("ip-pos-legend"),
        "By position heading must carry the green Decl legend"
    );
    assert!(
        ui.contains("planShortOfDecl") && ui.contains("ip-plan-short"),
        "Plan $/sh must light-red when declaration is below plan"
    );
    let css = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/App.css"),
    )
    .expect("App.css");
    assert!(
        css.contains("td.numeric.ip-decl-current")
            && css.contains("td.numeric.ip-decl-none")
            && !css.contains("td.numeric.ip-decl-stale"),
        "Decl $/sh keeps current green and empty gray; no yellow stale"
    );
    assert!(
        css.contains("td.numeric.ip-plan-short") && css.contains("#fde8ea"),
        "Plan $/sh short-of-decl highlight must stay light red"
    );
    assert!(
        css.contains("tr.ip-current td.numeric")
            && css.contains("tr.ip-grand td.numeric")
            && css.contains("color: #fff")
            && css.contains("font-weight: 700"),
        "Current and Grand Total money cells must be bold white on the blue/green bars"
    );
}

/// Leftover record 9/15 must not plan W37 after collect writes payable 9/30 (W39).
/// Cash/Trends planned week money must equal Income Plan Plan $ for that week.
#[tokio::test]
async fn leftover_record_pay_date_plans_on_payable_week_not_record_week() {
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
        serde_json::json!({"symbol": "CLM", "name": "CLM"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "priceSource": "public",
            "sourceSymbol": "CLM",
            "declarationSource": "cornerstone",
            "sourceUrl": "https://www.cornerstonestrategicinvestmentfund.com/press-releases",
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
            "provider": "Cornerstone",
            "divType": "DIV-1",
            "riskTier": "Core"
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot_as(
        &platform,
        security_id,
        "CLM",
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
            "amountPerShareMinor": 1215,
            "amountScale": 4,
            "paymentPeriod": "2026-08-31",
            "source": "cornerstone",
            "enteredAt": "2026-08-08"
        }),
    )
    .await;
    must_ok(
        &platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 1215,
            "amountScale": 4,
            "paymentPeriod": "2026-09-15",
            "source": "cornerstone",
            "enteredAt": "2026-08-08"
        }),
    )
    .await;
    must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 1215,
            "amountScale": 4,
            "planningPeriodsPerYear": 12,
            "effectiveFrom": "2026-01-01",
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
            "asOfDate": "2026-08-01",
            "dates": [
                {"payOn": "2026-09-15", "source": "derived_walk"},
                {"payOn": "2026-09-30", "source": "vendor_payable"}
            ]
        }),
    )
    .await;
    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "CLM",
            "declarationSource": "cornerstone",
            "asOfDate": "2026-09-18",
            "candidates": [{
                "paymentPeriod": "2026-09-30",
                "amountPerShareMinor": 1215,
                "amountScale": 4,
                "source": "cornerstone",
                "recordDate": "2026-09-15"
            }],
            "upcomingPays": [{"payOn": "2026-09-30", "source": "vendor_payable"}]
        }),
    )
    .await;

    let w37 = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({ "asOfDate": "2026-09-18" }),
    )
    .await;
    assert_eq!(w37["start"], "2026-09-12");
    assert_eq!(w37["end"], "2026-09-18");
    let w37_row = w37["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "CLM");
    assert!(
        w37_row.is_none() || w37_row.unwrap()["planKnown"] == false,
        "record leftover must not put Plan $ on W37: {w37}"
    );

    let w39 = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({ "asOfDate": "2026-09-30" }),
    )
    .await;
    assert_eq!(w39["start"], "2026-09-26");
    assert_eq!(w39["end"], "2026-10-02");
    let w39_row = w39["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "CLM")
        .unwrap_or_else(|| panic!("CLM must plan on payable W39: {w39}"));
    assert_eq!(w39_row["payOn"], "2026-09-30");
    assert_eq!(w39_row["planKnown"], true);
    assert_eq!(w39_row["plannedMinor"], 122);

    let planned_w39: i64 = w39["positions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["planKnown"] == true)
        .map(|p| p["plannedMinor"].as_i64().unwrap_or(0))
        .sum();
    let capture = query_json(
        &platform,
        "TrendsWeekGet",
        serde_json::json!({ "asOfDate": "2026-09-30" }),
    )
    .await;
    assert_eq!(
        capture["plannedWeeklyIncomeMinor"].as_i64().unwrap(),
        planned_w39,
        "Cash/Trends Planned weekly income must equal Income Plan Plan $: {capture}"
    );
}

/// Broker cash on the wrong day is Reported only. Plan $ stays on the payable week.
#[tokio::test]
async fn off_calendar_actual_does_not_move_plan_week() {
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
        serde_json::json!({"symbol": "GLAD", "name": "GLAD"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "priceSource": "public",
            "sourceSymbol": "GLAD",
            "declarationSource": "gladstone",
            "sourceUrl": "https://www.gladstonecapital.com/investors/stock-data/dividend-history",
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
            "provider": "Gladstone",
            "divType": "DIV-1",
            "riskTier": "Core"
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot_as(
        &platform,
        security_id,
        "GLAD",
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
            "amountPerShareMinor": 1500,
            "amountScale": 4,
            "paymentPeriod": "2026-08-31",
            "source": "gladstone",
            "enteredAt": "2026-07-14"
        }),
    )
    .await;
    must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 1500,
            "amountScale": 4,
            "planningPeriodsPerYear": 12,
            "effectiveFrom": "2026-01-01",
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
            "asOfDate": "2026-08-01",
            "dates": [{"payOn": "2026-09-30", "source": "vendor_payable"}]
        }),
    )
    .await;
    must_ok(
        &platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 1500,
            "amountScale": 4,
            "paymentPeriod": "2026-09-30",
            "source": "gladstone",
            "enteredAt": "2026-07-14"
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
            "amountMinor": 5706,
            "scale": 2,
            "occurredOn": "2026-09-16",
            "idempotencyKey": "glad-w37-actual"
        }),
    )
    .await;

    let w37 = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({ "asOfDate": "2026-09-18" }),
    )
    .await;
    assert!(
        w37["positions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["symbol"] != "GLAD"),
        "off-calendar 9/16 actual must not put GLAD on W37 Plan: {w37}"
    );

    let w39 = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({ "asOfDate": "2026-09-30" }),
    )
    .await;
    let w39_row = w39["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "GLAD")
        .unwrap_or_else(|| panic!("GLAD Plan $ stays on payable W39: {w39}"));
    assert_eq!(w39_row["payOn"], "2026-09-30");
    assert_eq!(w39_row["planKnown"], true);
    assert_eq!(w39_row["plannedMinor"], 150);

    let planned_w37: i64 = w37["positions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["planKnown"] == true)
        .map(|p| p["plannedMinor"].as_i64().unwrap_or(0))
        .sum();
    let capture_w37 = query_json(
        &platform,
        "TrendsWeekGet",
        serde_json::json!({ "asOfDate": "2026-09-18" }),
    )
    .await;
    assert_eq!(
        capture_w37["plannedWeeklyIncomeMinor"].as_i64().unwrap(),
        planned_w37,
        "Cash/Trends W37 Planned weekly income must equal Income Plan Plan $: {capture_w37}"
    );
}

#[tokio::test]
async fn week_plan_excludes_lot_opened_after_pay_on() {
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
        serde_json::json!({"symbol": "MUIB", "name": "MUIB"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    research_template(&platform, security_id, "MUIB").await;
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
        "MUIB",
        "Monthly",
        true,
    )
    .await
    .expect("complete collector");
    record_decl(&platform, security_id, "2026-09-01").await;
    must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 10,
            "amountScale": 2,
            "planningPeriodsPerYear": 12,
            "effectiveFrom": "2026-08-01",
            "decisionReason": "owner",
            "incompleteAnalysisReason": "Fewer than 6 observations"
        }),
    )
    .await;
    must_ok(
        &platform,
        "IssuerPayDateReplace",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-10-02",
            "dates": [
                {"payOn": "2026-10-01", "source": "issuer"},
                {"payOn": "2026-11-01", "source": "issuer"}
            ]
        }),
    )
    .await;
    // First lot opened the day after pay — must not appear on the 10/1 week Plan.
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": income["accountId"],
            "securityId": security_id,
            "openedOn": "2026-10-02",
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
    let pay_week = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({ "asOfDate": "2026-10-01" }),
    )
    .await;
    assert!(
        pay_week["positions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["symbol"] != "MUIB"),
        "lot opened after pay_on must not be on that week Plan: {pay_week}"
    );
    // Older lot + add-on after pay: Plan $ uses only the older qty.
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": income["accountId"],
            "securityId": security_id,
            "openedOn": "2026-09-01",
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
    let with_old = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({ "asOfDate": "2026-10-01" }),
    )
    .await;
    let row = with_old["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "MUIB")
        .unwrap_or_else(|| panic!("MUIB with older lot should be on pay week: {with_old}"));
    assert_eq!(row["planKnown"], true, "{row}");
    assert_eq!(
        row["plannedMinor"].as_i64().unwrap(),
        500,
        "Plan $ = 50 shares × $0.10 only (not 150): {row}"
    );
    let pos_plan: i64 = with_old["positions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["planKnown"] == true)
        .map(|p| p["plannedMinor"].as_i64().unwrap_or(0))
        .sum();
    let line_plan: i64 = with_old["lines"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|l| l["planKnown"] == true)
        .map(|l| l["plannedMinor"].as_i64().unwrap_or(0))
        .sum();
    assert_eq!(
        pos_plan, line_plan,
        "account Plan $ must equal sum of position Plan $ after eligibility: pos={pos_plan} lines={line_plan} {with_old}"
    );
    let acct = row["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["accountName"] == "Income")
        .expect("Income account slice");
    assert_eq!(acct["planKnown"], true);
    assert_eq!(acct["plannedMinor"].as_i64().unwrap(), 500);
}

/// Plan confirmed the day after pay is not that week's ticket (MUIB 10/02 vs pay 10/01).
#[tokio::test]
async fn plan_effective_after_pay_on_is_not_a_week_ticket() {
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
        serde_json::json!({"symbol": "MUIB", "name": "MUIB"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    research_template(&platform, security_id, "MUIB").await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Twice monthly",
            "replaceCadence": true,
            "riskTier": "Risk On"
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot_as(
        &platform,
        security_id,
        "MUIB",
        "Twice monthly",
        true,
    )
    .await
    .expect("complete collector");
    record_decl(&platform, security_id, "2026-10-01").await;
    must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 10,
            "amountScale": 2,
            "planningPeriodsPerYear": 24,
            "effectiveFrom": "2026-10-02",
            "decisionReason": "owner",
            "incompleteAnalysisReason": "Fewer than 6 observations"
        }),
    )
    .await;
    must_ok(
        &platform,
        "IssuerPayDateReplace",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-10-02",
            "dates": [
                {"payOn": "2026-10-01", "source": "issuer"},
                {"payOn": "2026-11-01", "source": "issuer"}
            ]
        }),
    )
    .await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": income["accountId"],
            "securityId": security_id,
            "openedOn": "2026-09-01",
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
    let pay_week = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({ "asOfDate": "2026-10-01" }),
    )
    .await;
    let early = pay_week["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "MUIB");
    if let Some(row) = early {
        assert_ne!(
            row["planKnown"], true,
            "plan effective 2026-10-02 must not ticket pay 2026-10-01: {row}"
        );
        assert_eq!(row["plannedMinor"].as_i64().unwrap_or(0), 0, "{row}");
    }
    let later = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({ "asOfDate": "2026-11-01" }),
    )
    .await;
    let row = later["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "MUIB")
        .unwrap_or_else(|| panic!("in-force plan must still count: {later}"));
    assert_eq!(row["planKnown"], true, "{row}");
    assert!(row["plannedMinor"].as_i64().unwrap_or(0) > 0, "{row}");
}

/// A later plan must not erase the window that still covers this pay date.
#[tokio::test]
async fn prior_plan_window_still_supplies_plan_per_share() {
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
            "replaceCadence": true,
            "riskTier": "Foundation"
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot_as(
        &platform,
        security_id,
        "HAKY",
        "Monthly",
        true,
    )
    .await
    .expect("complete collector");
    record_decl(&platform, security_id, "2026-09-30").await;
    must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 38,
            "amountScale": 2,
            "planningPeriodsPerYear": 12,
            "effectiveFrom": "2026-08-29",
            "decisionReason": "owner",
            "incompleteAnalysisReason": "Fewer than 6 observations"
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
            "effectiveFrom": "2026-10-02",
            "decisionReason": "later plan",
            "incompleteAnalysisReason": "Fewer than 6 observations"
        }),
    )
    .await;
    must_ok(
        &platform,
        "IssuerPayDateReplace",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-10-02",
            "dates": [
                {"payOn": "2026-09-30", "source": "issuer"},
                {"payOn": "2026-10-30", "source": "issuer"}
            ]
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
            "quantityMinor": 70,
            "quantityScale": 0,
            "performanceBasisMinor": 200_000,
            "taxBasisMinor": 200_000,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;
    let week = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({ "asOfDate": "2026-09-30" }),
    )
    .await;
    let row = week["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "HAKY")
        .unwrap_or_else(|| panic!("HAKY should list on the 2026-09-30 week: {week}"));
    assert_eq!(row["planKnown"], true, "{row}");
    assert_eq!(
        row["planPerShareMinor"].as_i64(),
        Some(38),
        "pay 2026-09-30 stays on the 2026-08-29 plan, not the 2026-10-02 plan: {row}"
    );
    assert!(row["plannedMinor"].as_i64().unwrap_or(0) > 0, "{row}");
}

/// Broker cash on an account with no lot must not clear the position plan.
#[tokio::test]
async fn broker_cash_without_a_lot_does_not_clear_position_plan() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let income = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let roth = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "FI Roth", "kind": "roth"}),
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
            "replaceCadence": true,
            "riskTier": "Foundation"
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot_as(
        &platform,
        security_id,
        "HAKY",
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
            "amountPerShareMinor": 39,
            "amountScale": 2,
            "paymentPeriod": "2026-09-30",
            "source": "provider-site",
            "enteredAt": "2026-09-29"
        }),
    )
    .await;
    must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 38,
            "amountScale": 2,
            "planningPeriodsPerYear": 12,
            "effectiveFrom": "2026-08-29",
            "decisionReason": "owner",
            "incompleteAnalysisReason": "Fewer than 6 observations"
        }),
    )
    .await;
    must_ok(
        &platform,
        "IssuerPayDateReplace",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-10-02",
            "dates": [{"payOn": "2026-09-30", "source": "issuer"}]
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
            "accountId": roth["accountId"],
            "securityId": security_id,
            "activityType": "dividend",
            "amountMinor": 429,
            "scale": 2,
            "occurredOn": "2026-09-30",
            "idempotencyKey": "haky-roth-no-lot"
        }),
    )
    .await;
    let week = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({ "asOfDate": "2026-09-30" }),
    )
    .await;
    let row = week["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "HAKY")
        .unwrap_or_else(|| panic!("HAKY should list: {week}"));
    assert_eq!(row["planKnown"], true, "{row}");
    assert_eq!(row["planPerShareMinor"].as_i64(), Some(38), "{row}");
    assert_eq!(row["declarationKnown"], true, "{row}");
    assert!(
        row["declarationPerShareMinor"].as_i64().unwrap_or(0) > 0,
        "{row}"
    );
    let accounts = row["accounts"].as_array().unwrap();
    let roth_slice = accounts
        .iter()
        .find(|a| a["accountName"] == "FI Roth")
        .unwrap_or_else(|| panic!("FI Roth cash must still list: {accounts:?}"));
    assert_eq!(roth_slice["planKnown"], false, "{roth_slice}");
    assert_eq!(roth_slice["actualKnown"], true, "{roth_slice}");
    let income_slice = accounts
        .iter()
        .find(|a| a["accountName"] == "Income")
        .unwrap_or_else(|| panic!("Income lot must list: {accounts:?}"));
    assert_eq!(income_slice["planKnown"], true, "{income_slice}");
}

#[test]
fn weekly_report_plan_per_share_ignores_actual_only_account() {
    let ui = std::fs::read_to_string(
        golden_harness::repo_root().join("packages/ui-components/src/index.tsx"),
    )
    .expect("ui-components");
    assert!(
        ui.contains("account.planKnown || account.declarationKnown"),
        "actual-only account slices must not join the Plan $/sh gate"
    );
    assert!(
        !ui.contains("const planKnown = slices.every((account) => account.planKnown)"),
        "every selected account, including broker cash with no lot, must not blank Plan $/sh"
    );
}

/// Owner Confirm stores `assumed_next_year`. The Pay date cell says assumed.
/// A vendor date stays unmarked. Collectors do not write those rows.
#[tokio::test]
async fn assumed_next_year_pay_date_is_marked_and_a_vendor_date_is_not() {
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
    let vendor_week = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({ "asOfDate": "2026-08-30" }),
    )
    .await;
    let vendor = vendor_week["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "PAY1")
        .unwrap_or_else(|| panic!("PAY1 vendor week missing: {vendor_week}"));
    assert_eq!(vendor["payOn"], "2026-08-31");
    assert!(
        vendor.get("payDateAssumed").is_none(),
        "a vendor pay date stays unmarked: {vendor}"
    );

    platform
        .assumed_pay_date_insert(AssumedPayDateRecord {
            assumed_pay_date_id: Uuid::new_v4(),
            security_id: Uuid::parse_str(security_id).unwrap(),
            pay_on: "2027-01-30".into(),
            cadence: "Monthly".into(),
            provenance: "assumed_next_year".into(),
            assumed_on: "2026-06-02".into(),
        })
        .await
        .expect("store assumed pay date");
    let assumed_week = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({ "asOfDate": "2027-02-01" }),
    )
    .await;
    let assumed = assumed_week["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "PAY1")
        .unwrap_or_else(|| panic!("PAY1 assumed week missing: {assumed_week}"));
    assert_eq!(assumed["payOn"], "2027-01-30", "{assumed}");
    assert_eq!(assumed["payDateAssumed"], true, "{assumed}");
    assert_eq!(assumed["plannedMinor"], 120, "{assumed}");
}

#[test]
fn assumed_mark_is_on_the_pay_date_and_collectors_do_not_write_it() {
    let root = golden_harness::repo_root();
    let ui = std::fs::read_to_string(root.join("packages/ui-components/src/index.tsx"))
        .expect("ui-components");
    assert!(
        ui.contains("row.payDateAssumed ? <span> assumed</span> : null"),
        "Pay date shows the word assumed only when the week flag is set"
    );
    assert!(
        !ui.contains("<th>Reported") && !ui.contains("<th>Actual $</th>"),
        "the position table does not gain a Reported or Actual $ column"
    );
    let mut callers = Vec::new();
    let mut stack = vec![
        root.join("crates/application-core/src"),
        root.join("crates/financial-domain/src"),
        root.join("crates/storage-sqlite/src"),
    ];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            if text.contains(".assumed_pay_date_insert(") {
                let slashed = path.to_string_lossy().replace('\\', "/");
                let rel = slashed
                    .rsplit_once("/crates/")
                    .map(|(_, rest)| format!("crates/{rest}"))
                    .unwrap_or(slashed);
                callers.push(rel);
            }
        }
    }
    callers.sort();
    assert_eq!(
        callers,
        vec!["crates/application-core/src/plan_horizon.rs".to_string()],
        "only the June confirm writes an assumed pay date"
    );
}

