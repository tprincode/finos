//! Forward Plan $ is qty × periods × plan. History is locked Plan vs Declared.
use application_core::contracts::{
    CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::queries::{execute_command_on, execute_query_on};
use chrono::{Duration, NaiveDate};
use golden_harness::profile_a_app_dir;
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

async fn seed_monthly(
    platform: &LocalPlatform,
    income_id: &serde_json::Value,
) -> String {
    let security = must_ok(
        platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "MONF", "name": "MONF"}),
    )
    .await;
    let sid = security["securityId"].as_str().unwrap().to_string();
    must_ok(
        platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": sid,
            "priceSource": "public",
            "sourceSymbol": "MONF",
            "declarationSource": "issuer",
            "sourceUrl": "https://example.test/monf/distributions",
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
            "securityId": sid,
            "paymentFrequency": "Monthly",
            "replaceCadence": true,
            "riskTier": "Core"
        }),
    )
    .await;
    must_ok(
        platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": sid,
            "amountPerShareMinor": 100,
            "amountScale": 2,
            "paymentPeriod": "2026-07-31",
            "source": "provider-site",
            "enteredAt": "2026-08-01"
        }),
    )
    .await;
    must_ok(
        platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": sid,
            "amountPerShareMinor": 100,
            "amountScale": 2,
            "planningPeriodsPerYear": 12,
            "effectiveFrom": "2026-01-01",
            "decisionReason": "Most Current",
            "incompleteAnalysisReason": "Fewer than 6 observations"
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot_as(platform, &sid, "MONF", "Monthly", true)
        .await
        .expect("complete collector");
    must_ok(
        platform,
        "LotOpen",
        serde_json::json!({
            "accountId": income_id,
            "securityId": sid,
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
        platform,
        "IssuerPayDateReplace",
        serde_json::json!({
            "securityId": sid,
            "asOfDate": "2026-01-01",
            "dates": [
                {"payOn": "2026-07-31", "source": "test"},
                {"payOn": "2026-08-31", "source": "test"},
                {"payOn": "2026-09-30", "source": "test"}
            ]
        }),
    )
    .await;
    sid
}

async fn income_week_plan(platform: &LocalPlatform, as_of: &str, week_ending: &str) -> i64 {
    let body = query_json(
        platform,
        "IncomePlanWeekGet",
        serde_json::json!({ "asOfDate": as_of, "weekEnding": week_ending }),
    )
    .await;
    body["lines"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|ln| ln["accountName"] == "Income")
        .and_then(|ln| {
            ln["planKnown"]
                .as_bool()
                .unwrap_or(false)
                .then(|| ln["plannedMinor"].as_i64())
        })
        .flatten()
        .unwrap_or(0)
}

#[tokio::test]
async fn monthly_without_2027_vendor_dates_still_fills_twelve_months() {
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
    seed_monthly(&platform, &income["accountId"]).await;

    let as_of = "2026-09-20";
    let mut sum = 0i64;
    let start = NaiveDate::from_ymd_opt(2026, 9, 25).unwrap();
    for i in 0..52 {
        let end = start + Duration::days(7 * i);
        sum += income_week_plan(&platform, as_of, &end.format("%Y-%m-%d").to_string()).await;
    }
    // 100 sh × $1.00 × 12 = $1,200. Sep-20 window keeps Sep 30 2026 through Aug 2027.
    assert!(
        sum >= 120_000 && sum <= 130_000,
        "12m monthly Plan $ must land near qty×12×plan, not 3 vendor dates: {sum}"
    );

    let cover = query_json(
        &platform,
        "CashCoverageGet",
        serde_json::json!({ "asOfDate": as_of, "period": "year" }),
    )
    .await;
    assert_eq!(cover["periodStart"], "2026-09-20");
    assert_eq!(cover["periodEnd"], "2027-09-20");
    assert_eq!(cover["lookbackStart"], "2026-09-20");
    assert_eq!(cover["lookbackEnd"], "2027-09-20");
    let income_row = cover["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["account"] == "Income")
        .unwrap();
    let forecast = income_row["planIncomeMinor"].as_i64().expect("year forecast");
    assert!(
        forecast >= 120_000,
        "Coverage Year forecast is next 12 months, not 2025 lookback: {cover}"
    );
}

#[tokio::test]
async fn confirm_plan_moves_future_week_not_locked_past_week() {
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
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "WKF", "name": "WKF"}),
    )
    .await;
    let sid = security["securityId"].as_str().unwrap().to_string();
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": sid,
            "priceSource": "public",
            "sourceSymbol": "WKF",
            "declarationSource": "issuer",
            "sourceUrl": "https://example.test/wkf/distributions",
            "calendarPolicy": "derived_walk",
            "collectorEnabled": true,
            "lookbackCount": 12
        }),
    )
    .await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": sid,
            "paymentFrequency": "Weekly",
            "replaceCadence": true,
            "riskTier": "Core"
        }),
    )
    .await;
    must_ok(
        &platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": sid,
            "amountPerShareMinor": 100,
            "amountScale": 2,
            "paymentPeriod": "2026-07-28",
            "source": "provider-site",
            "enteredAt": "2026-08-01"
        }),
    )
    .await;
    must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": sid,
            "amountPerShareMinor": 100,
            "amountScale": 2,
            "planningPeriodsPerYear": 52,
            "effectiveFrom": "2026-01-01",
            "decisionReason": "Most Current",
            "incompleteAnalysisReason": "Fewer than 6 observations"
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot_as(&platform, &sid, "WKF", "Weekly", true)
        .await
        .expect("complete collector");
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": income["accountId"],
            "securityId": sid,
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

    let as_of = "2026-09-20";
    let past = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({ "asOfDate": as_of, "weekEnding": "2026-09-04" }),
    )
    .await;
    let future = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({ "asOfDate": as_of, "weekEnding": "2026-10-02" }),
    )
    .await;
    let past_before = past["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "WKF")
        .and_then(|p| p["plannedMinor"].as_i64())
        .expect("past WKF");
    let future_before = future["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "WKF")
        .and_then(|p| p["plannedMinor"].as_i64())
        .expect("future WKF");
    assert_eq!(past_before, 10_000);
    assert_eq!(future_before, 10_000);

    must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": sid,
            "amountPerShareMinor": 200,
            "amountScale": 2,
            "planningPeriodsPerYear": 52,
            "effectiveFrom": "2026-09-20",
            "decisionReason": "Raise Plan",
            "incompleteAnalysisReason": "Fewer than 6 observations"
        }),
    )
    .await;

    let past_after = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({ "asOfDate": as_of, "weekEnding": "2026-09-04" }),
    )
    .await;
    let future_after = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({ "asOfDate": as_of, "weekEnding": "2026-10-02" }),
    )
    .await;
    let past_plan = past_after["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "WKF")
        .and_then(|p| p["plannedMinor"].as_i64())
        .expect("past after");
    let future_plan = future_after["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "WKF")
        .and_then(|p| p["plannedMinor"].as_i64())
        .expect("future after");
    assert_eq!(past_plan, 10_000, "locked past week stays on the old Plan $");
    assert_eq!(future_plan, 20_000, "unlocked future week uses the new Plan $");
}

#[tokio::test]
async fn live_income_next_12m_matches_home_annual() {
    let sqlite = profile_a_app_dir().join("local.sqlite");
    if !sqlite.exists() {
        return;
    }
    let platform = LocalPlatform::open(profile_a_app_dir())
        .await
        .expect("open live sqlite");
    let as_of = "2026-09-20";
    let home = query_json(
        &platform,
        "DividendPlanHomeGet",
        serde_json::json!({ "asOfDate": as_of }),
    )
    .await;
    let annual = home["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["accountName"] == "Income")
        .and_then(|r| r["annualDividendMinor"].as_i64())
        .expect("Income annual");
    let weekly = annual / 52;
    let mut week_sum = 0i64;
    let start = NaiveDate::from_ymd_opt(2026, 9, 25).unwrap();
    for i in 0..52 {
        let end = start + Duration::days(7 * i);
        week_sum += income_week_plan(&platform, as_of, &end.format("%Y-%m-%d").to_string()).await;
    }
    // Home is qty×52/12/4. 52 Sat–Fri weeks from this Friday can land one extra
    // monthly slot (mixed vendor days) or a 53rd weekly; never the old $26k year.
    assert!(
        week_sum >= annual && week_sum - annual <= weekly * 2,
        "52 Income Plan weeks {week_sum} must be ≥ Home {annual} and within two weekly {weekly}"
    );

    let cover = query_json(
        &platform,
        "CashCoverageGet",
        serde_json::json!({ "asOfDate": as_of, "period": "year" }),
    )
    .await;
    let forecast = cover["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["account"] == "Income")
        .and_then(|r| r["planIncomeMinor"].as_i64())
        .expect("year forecast");
    assert_ne!(forecast, 2_614_404, "must not be 2025 lookback $26,144.04");
    // Year chip is as-of → as-of+12m (can hold a 53rd weekly). Home is 52/12/4.
    assert!(
        forecast >= annual && forecast - annual <= weekly * 3,
        "Coverage Year {forecast} vs Home {annual}"
    );

    let register = query_json(
        &platform,
        "CashRegisterGet",
        serde_json::json!({
            "asOfDate": as_of,
            "account": "Income",
            "period": "1Y",
            "periodStart": as_of,
            "periodEnd": "2027-09-20",
            "hitsOnly": true
        }),
    )
    .await;
    let hits: i64 = register["rows"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|h| h["source"] == "income-plan")
        .filter_map(|h| h["depositMinor"].as_i64().or_else(|| h["amountMinor"].as_i64()))
        .sum();
    let slack = weekly.max(1);
    assert!(
        (hits - week_sum).abs() <= slack,
        "cash-flow income-plan hits {hits} vs 52-week Plan $ {week_sum}"
    );
}

#[tokio::test]
async fn live_health_12m_planned_income_matches_dividend_plan() {
    let sqlite = profile_a_app_dir().join("local.sqlite");
    if !sqlite.exists() {
        return;
    }
    let platform = LocalPlatform::open(profile_a_app_dir())
        .await
        .expect("open live sqlite");
    let as_of = "2026-09-20";
    let home = query_json(
        &platform,
        "DividendPlanHomeGet",
        serde_json::json!({ "asOfDate": as_of }),
    )
    .await;
    let health = home["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["accountName"] == "Health")
        .expect("Health row");
    let annual = health["annualDividendMinor"].as_i64().expect("Health annual");
    let monthly = health["monthlyMedicalMinor"]
        .as_i64()
        .or_else(|| health["monthlyIncomeMinor"].as_i64())
        .expect("Health monthly");
    assert_eq!(monthly, annual / 12, "table monthly is annual ÷ 12");

    let register = query_json(
        &platform,
        "CashRegisterGet",
        serde_json::json!({
            "asOfDate": as_of,
            "account": "Health",
            "period": "1Y",
            "periodStart": as_of,
            "periodEnd": "2027-09-20",
            "hitsOnly": true
        }),
    )
    .await;
    let planned: i64 = register["rows"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|h| h["source"] == "income-plan")
        .filter(|h| {
            h["occurredOn"]
                .as_str()
                .map(|d| d >= as_of && d <= "2027-09-20")
                .unwrap_or(false)
        })
        .filter_map(|h| h["depositMinor"].as_i64())
        .sum();
    let slack = (annual / 12).max(1);
    assert!(
        planned >= annual && planned - annual <= slack * 2,
        "Health 12m Planned Income {planned} must agree with Dividend Plan annual {annual} (monthly {monthly})"
    );
}
