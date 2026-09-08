//! Trends weekly AccountBalanceSnapshot series from production Template_Trends_Weekly.xlsx.

use application_core::contracts::{QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION};
use application_core::queries::execute_query_on;
use golden_harness::{load_production_seed_via_commands, repo_root};
use storage_sqlite::LocalPlatform;
use uuid::Uuid;

async fn query_json(platform: &LocalPlatform, name: &str, body: Option<&str>) -> serde_json::Value {
    let result = execute_query_on(
        platform,
        platform,
        QueryRequest {
            contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
            query_name: name.to_string(),
            correlation_id: Uuid::new_v4(),
            body_json: body.map(|s| s.to_string()),
        },
    )
    .await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

#[tokio::test]
async fn production_trends_weeks_seed_and_query() {
    let root = repo_root();
    let production = root.join("database/seed/production");
    assert!(
        production.join("Template_Trends_Weekly.xlsx").exists(),
        "Template_Trends_Weekly.xlsx missing"
    );
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    load_production_seed_via_commands(&platform, &production)
        .await
        .expect("production seed");

    let trends = query_json(&platform, "TrendsGet", None).await;
    let weeks = trends["weeks"].as_array().expect("weeks array");
    assert!(
        weeks.len() >= 80,
        "expected ~2y of weekly Trends rows, got {}",
        weeks.len()
    );
    assert_eq!(weeks[0]["periodEnd"], "2025-01-03");
    let last = weeks.last().unwrap();
    assert_eq!(last["periodEnd"], "2026-08-21");
    // Owner extract: late Aug 2026 monthly DIVS ~4889, FID+SCH ~348049
    assert_eq!(last["monthlyDivsMinor"].as_i64().unwrap(), 488_900);
    assert_eq!(last["profitMinor"].as_i64().unwrap(), 153_800);
    assert!(weeks[1]["fidSchCombinedMinor"].as_i64().unwrap() > 0);
    assert!(weeks[1]["totalCashMinor"].as_i64().unwrap() > 0);
    assert_eq!(weeks[0]["divDeltaMinor"].as_i64().unwrap(), 0);
    // First week cash matches Trends tab (Income + Acct9 + Acct9 70% already applied)
    // 13325.06 + 11506.02 + 18776.925 = 43608.005 → 4360801 cents (rounded)
    let first_cash = weeks[0]["totalCashMinor"].as_i64().unwrap();
    assert!(
        (first_cash - 4_360_801).abs() <= 1,
        "first totalCashMinor={first_cash}"
    );
}

#[tokio::test]
async fn dashboard_burndown_reuses_latest_trends_ending_balance() {
    let root = repo_root();
    let production = root.join("database/seed/production");
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    load_production_seed_via_commands(&platform, &production)
        .await
        .expect("production seed");

    let burn = query_json(
        &platform,
        "DashboardBurndownGet",
        Some(r#"{"asOfDate":"2026-08-21"}"#),
    )
    .await;
    let lines = burn["lines"].as_array().unwrap();
    let car = lines
        .iter()
        .find(|l| l["accountName"] == "Car")
        .expect("Car line");
    assert_eq!(car["endingBalanceKnown"], true);
    // Owner last Car = 44741.27
    assert_eq!(car["endingBalanceMinor"].as_i64().unwrap(), 4_474_127);
    let income = lines
        .iter()
        .find(|l| l["accountName"] == "Income")
        .expect("Income line");
    assert_eq!(income["endingBalanceKnown"], true);
    assert_eq!(income["endingBalanceMinor"].as_i64().unwrap(), 21_224_572);
}

#[tokio::test]
async fn trends_template_parses_owner_extract_without_coercing_blank_income() {
    let root = repo_root();
    let production = root.join("database/seed/production");
    let doc = import_engine::parse_production_templates(&production).expect("parse");
    assert!(doc.trends_weeks.len() >= 80);
    let early = doc
        .trends_weeks
        .iter()
        .find(|w| w.period_end == "2025-01-03")
        .expect("2025-01-03 week");
    assert!(
        early.income_balance_minor.is_none(),
        "early 2025 income balance stays unknown"
    );
    assert_eq!(early.profit_minor, 168_600);
    assert_eq!(early.monthly_divs_minor, 611_900);
    let late = doc
        .trends_weeks
        .iter()
        .find(|w| w.period_end == "2026-08-21")
        .expect("late week");
    assert_eq!(late.income_balance_minor, Some(21_224_572));
    assert_eq!(late.roth_balance_minor, Some(879_996));
    assert_eq!(late.speculation_balance_minor, Some(2_980_433));
}
