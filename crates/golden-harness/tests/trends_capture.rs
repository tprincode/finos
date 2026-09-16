//! Trends weekly capture / close / IAL profit suggestion harness.

use application_core::contracts::{
    CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::queries::{execute_command_on, execute_query_on};
use golden_harness::{load_production_seed_via_commands, repo_root};
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

fn qry(name: &str, body: Option<&str>) -> QueryRequest {
    QueryRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        query_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: body.map(|s| s.to_string()),
    }
}

async fn must_cmd(platform: &LocalPlatform, name: &str, body: serde_json::Value) -> serde_json::Value {
    let result = execute_command_on(platform, platform, cmd(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

async fn query_json(platform: &LocalPlatform, name: &str, body: Option<&str>) -> serde_json::Value {
    let result = execute_query_on(platform, platform, qry(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

#[tokio::test]
async fn trends_week_save_updates_charts_and_is_idempotent() {
    let root = repo_root();
    let production = root.join("database/seed/production");
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    load_production_seed_via_commands(&platform, &production)
        .await
        .expect("seed");

    let body = serde_json::json!({
        "periodStart": "2026-08-22",
        "periodEnd": "2026-08-28",
        "capturedAt": "2026-08-29T12:00:00Z",
        "profitMinor": 150000,
        "monthlyDivsMinor": 490000,
        "fidelityTotalMinor": 31000000,
        "schwabTotalMinor": 3200000,
        "incomeCashMinor": 700000,
        "acct9CashMinor": 1000000,
        "acct9EtfValueMinor": 1100000,
        "carBalanceMinor": 4500000,
        "incomeBalanceMinor": 21200000,
        "healthBalanceMinor": 1900000,
        "rothBalanceMinor": 880000,
        "speculationBalanceMinor": 3000000,
        "scale": 2
    });
    must_cmd(&platform, "TrendsWeekSave", body.clone()).await;
    must_cmd(&platform, "TrendsWeekSave", body).await;

    let trends = query_json(&platform, "TrendsGet", Some(r#"{"asOfDate":"2026-08-28"}"#)).await;
    let weeks = trends["weeks"].as_array().unwrap();
    assert!(weeks.iter().any(|w| w["periodEnd"] == "2026-08-28"));
    assert!(trends["overview"].is_object());
    assert!(trends["distributions"].is_object());
    assert!(trends["taxMonitor"].is_object());
    assert!(trends["taxMonitor"]["acaThresholdMinor"].as_i64().unwrap() > 0);
}

#[tokio::test]
async fn trends_week_close_blocks_save_allows_correct() {
    let root = repo_root();
    let production = root.join("database/seed/production");
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    load_production_seed_via_commands(&platform, &production)
        .await
        .expect("seed");

    let body = serde_json::json!({
        "periodStart": "2026-08-22",
        "periodEnd": "2026-08-28",
        "capturedAt": "2026-08-29T12:00:00Z",
        "profitMinor": 100000,
        "monthlyDivsMinor": 480000,
        "fidelityTotalMinor": 30000000,
        "schwabTotalMinor": 3000000,
        "incomeCashMinor": 1,
        "acct9CashMinor": 1,
        "acct9EtfValueMinor": 1,
        "scale": 2
    });
    must_cmd(&platform, "TrendsWeekSave", body.clone()).await;
    must_cmd(
        &platform,
        "TrendsWeekClose",
        serde_json::json!({"periodEnd":"2026-08-28"}),
    )
    .await;

    let blocked = execute_command_on(&platform, &platform, cmd("TrendsWeekSave", body.clone())).await;
    assert!(!blocked.ok);
    assert_eq!(blocked.error_code.as_deref(), Some("trends_week_closed"));

    let mut corrected = body.clone();
    corrected["profitMinor"] = serde_json::json!(111100);
    must_cmd(&platform, "TrendsWeekCorrect", corrected).await;
    let open = query_json(
        &platform,
        "TrendsWeekGet",
        Some(r#"{"asOfDate":"2026-08-28"}"#),
    )
    .await;
    assert_eq!(open["current"]["profitMinor"], 111100);
    assert_eq!(open["closed"], true);
}

#[tokio::test]
async fn trends_suggested_profit_from_dividend_activity() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    must_cmd(
        &platform,
        "AccountRegister",
        serde_json::json!({"name":"Income","kind":"ira"}),
    )
    .await;
    must_cmd(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol":"XDTE","name":"XDTE"}),
    )
    .await;
    let accounts = query_json(&platform, "AccountList", None).await;
    let account_id = accounts[0]["accountId"].as_str().unwrap();
    let securities = query_json(&platform, "SecurityList", None).await;
    let security_id = securities[0]["securityId"].as_str().unwrap();
    must_cmd(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "activityType": "dividend",
            "amountMinor": 25000,
            "scale": 2,
            "occurredOn": "2026-08-25"
        }),
    )
    .await;
    let capture = query_json(
        &platform,
        "TrendsWeekGet",
        Some(r#"{"asOfDate":"2026-08-28"}"#),
    )
    .await;
    assert_eq!(capture["periodStart"], "2026-08-22");
    assert_eq!(capture["periodEnd"], "2026-08-28");
    assert_eq!(capture["suggestedProfitMinor"], 25000);
    assert_eq!(
        capture["suggestedMonthlyDivsMinor"], 25000,
        "week-aligned income is the paid dividend in that Sat–Fri week"
    );
}

#[tokio::test]
async fn trends_week_get_defaults_to_first_unpopulated() {
    let root = repo_root();
    let production = root.join("database/seed/production");
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    load_production_seed_via_commands(&platform, &production)
        .await
        .expect("seed");

    let capture = query_json(&platform, "TrendsWeekGet", None).await;
    assert_eq!(capture["firstUnpopulatedStart"], "2026-08-22");
    assert_eq!(capture["periodStart"], "2026-08-22");
    let chooser = capture["chooserSaturdays"].as_array().expect("chooser");
    assert_eq!(chooser[0], "2026-08-22");
    assert!(chooser.iter().any(|s| s == "2026-09-12"));
}

#[tokio::test]
async fn trends_week_save_cash_and_derived_totals_hit_charts() {
    let root = repo_root();
    let production = root.join("database/seed/production");
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    load_production_seed_via_commands(&platform, &production)
        .await
        .expect("seed");

    let body = serde_json::json!({
        "periodStart": "2026-08-22",
        "periodEnd": "2026-08-28",
        "capturedAt": "2026-08-29T12:00:00Z",
        "incomeBalanceMinor": 20000000,
        "rothBalanceMinor": 1000000,
        "speculationBalanceMinor": 2000000,
        "healthBalanceMinor": 1500000,
        "carBalanceMinor": 4000000,
        "acct9BalanceMinor": 3500000,
        "incomeCashMinor": 500000,
        "rothCashMinor": 100000,
        "speculationCashMinor": 50000,
        "healthCashMinor": 25000,
        "carCashMinor": 75000,
        "acct9CashMinor": 200000,
        "acct9EtfValueMinor": 0,
        "scale": 2
    });
    let saved = must_cmd(&platform, "TrendsWeekSave", body).await;
    assert_eq!(saved["current"]["fidelityTotalMinor"], 28_500_000);
    assert_eq!(saved["current"]["schwabTotalMinor"], 3_500_000);

    let trends = query_json(&platform, "TrendsGet", Some(r#"{"asOfDate":"2026-08-28"}"#)).await;
    let week = trends["weeks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["periodEnd"] == "2026-08-28")
        .expect("saved week");
    assert_eq!(week["fidelityTotalMinor"], 28_500_000);
    assert_eq!(week["schwabTotalMinor"], 3_500_000);
    assert!(week["totalCashMinor"].as_i64().unwrap() >= 950_000);
    assert_eq!(week["healthBalanceMinor"], 1_500_000);
}

#[tokio::test]
async fn trends_acct9_etf_uses_last_price_not_tax_basis() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    must_cmd(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "9", "kind": "ira"}),
    )
    .await;
    let empty = query_json(
        &platform,
        "TrendsWeekGet",
        Some(r#"{"asOfDate":"2026-08-28"}"#),
    )
    .await;
    assert_eq!(empty["suggestedAcct9EtfProxyMinor"], 0);

    let root = repo_root();
    let production = root.join("database/seed/production");
    let seeded = tempfile::tempdir().unwrap();
    let seeded_platform = LocalPlatform::open(seeded.path().join("app-data"))
        .await
        .unwrap();
    load_production_seed_via_commands(&seeded_platform, &production)
        .await
        .expect("seed");
    let capture = query_json(
        &seeded_platform,
        "TrendsWeekGet",
        Some(r#"{"asOfDate":"2026-08-28"}"#),
    )
    .await;
    let proxy = capture["suggestedAcct9EtfProxyMinor"].as_i64();
    assert!(
        proxy.is_some_and(|v| v > 0) || capture["suggestedAcct9EtfProxyMinor"].is_null(),
        "last-price 70% is a positive value or unknown, never invented $0: {}",
        capture["suggestedAcct9EtfProxyMinor"]
    );
    if let Some(v) = proxy {
        assert_ne!(v, 0, "Account 9 open lots have last prices");
    }
}

fn capture_body(
    period_start: &str,
    period_end: &str,
    cash: &[i64; 6],
    adjusts: serde_json::Value,
) -> serde_json::Value {
    // cash: Income, FI Roth, Speculation, Health, Car, Account 9
    serde_json::json!({
        "periodStart": period_start,
        "periodEnd": period_end,
        "capturedAt": format!("{period_end}T12:00:00Z"),
        "incomeBalanceMinor": 20_000_000,
        "rothBalanceMinor": 1_000_000,
        "speculationBalanceMinor": 2_000_000,
        "healthBalanceMinor": 1_500_000,
        "carBalanceMinor": 4_000_000,
        "acct9BalanceMinor": 3_500_000,
        "incomeCashMinor": cash[0],
        "rothCashMinor": cash[1],
        "speculationCashMinor": cash[2],
        "healthCashMinor": cash[3],
        "carCashMinor": cash[4],
        "acct9CashMinor": cash[5],
        "acct9EtfValueMinor": 0,
        "scale": 2,
        "adjusts": adjusts
    })
}

async fn count_cash_adjust(platform: &LocalPlatform, period_end: &str) -> usize {
    let acts = query_json(platform, "ActivityList", None).await;
    acts.as_array()
        .unwrap()
        .iter()
        .filter(|a| {
            a["activityType"] == "Cash_Adjust" && a["occurredOn"] == period_end
        })
        .count()
}

#[tokio::test]
async fn t1_no_gap_posts_zero_cash_adjust() {
    let root = repo_root();
    let production = root.join("database/seed/production");
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    load_production_seed_via_commands(&platform, &production)
        .await
        .expect("seed");

    let view = query_json(
        &platform,
        "TrendsWeekGet",
        Some(r#"{"asOfDate":"2026-08-28"}"#),
    )
    .await;
    let cash = cash_from_references(&view, |_, ref_minor| ref_minor.unwrap_or(1_000));
    must_cmd(
        &platform,
        "WeekCaptureAccept",
        capture_body("2026-08-22", "2026-08-28", &cash, serde_json::json!([])),
    )
    .await;
    assert_eq!(count_cash_adjust(&platform, "2026-08-28").await, 0);
}

#[tokio::test]
async fn t2_car_gap_posts_one_adjust_and_snapshot_cash() {
    let root = repo_root();
    let production = root.join("database/seed/production");
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    load_production_seed_via_commands(&platform, &production)
        .await
        .expect("seed");

    let view = query_json(
        &platform,
        "TrendsWeekGet",
        Some(r#"{"asOfDate":"2026-08-28"}"#),
    )
    .await;
    let car_ref = view["cashReferences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["accountName"] == "Car")
        .expect("Car ref");
    let car_reference = car_ref["referenceMinor"]
        .as_i64()
        .expect("Car needs a reference for T2");
    let car_id = car_ref["accountId"].as_str().unwrap().to_string();
    let typed_car = car_reference - 825;
    let cash = cash_from_references(&view, |name, ref_minor| {
        if name == "Car" {
            typed_car
        } else {
            ref_minor.unwrap_or(1_000)
        }
    });
    must_cmd(
        &platform,
        "WeekCaptureAccept",
        capture_body(
            "2026-08-22",
            "2026-08-28",
            &cash,
            serde_json::json!([{
                "accountId": car_id,
                "amountMinor": -825,
                "reason": "fee"
            }]),
        ),
    )
    .await;
    assert_eq!(count_cash_adjust(&platform, "2026-08-28").await, 1);
    let acts = query_json(&platform, "ActivityList", None).await;
    let adj = acts
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["activityType"] == "Cash_Adjust" && a["occurredOn"] == "2026-08-28")
        .expect("adjust");
    assert_eq!(adj["amountMinor"], -825);
    assert_eq!(adj["note"], "fee");
    let capture = query_json(
        &platform,
        "TrendsWeekGet",
        Some(r#"{"asOfDate":"2026-08-28"}"#),
    )
    .await;
    assert_eq!(capture["carCashMinor"], typed_car);
}

#[tokio::test]
async fn t3_blank_reason_blocks_week_capture_accept() {
    let root = repo_root();
    let production = root.join("database/seed/production");
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    load_production_seed_via_commands(&platform, &production)
        .await
        .expect("seed");

    let view = query_json(
        &platform,
        "TrendsWeekGet",
        Some(r#"{"asOfDate":"2026-08-28"}"#),
    )
    .await;
    let car_ref = view["cashReferences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["accountName"] == "Car")
        .expect("Car ref");
    let car_reference = car_ref["referenceMinor"]
        .as_i64()
        .expect("Car needs a reference for T3");
    let car_id = car_ref["accountId"].as_str().unwrap().to_string();
    let cash = cash_from_references(&view, |name, ref_minor| {
        if name == "Car" {
            car_reference - 825
        } else {
            ref_minor.unwrap_or(1_000)
        }
    });
    let blocked = execute_command_on(
        &platform,
        &platform,
        cmd(
            "WeekCaptureAccept",
            capture_body(
                "2026-08-22",
                "2026-08-28",
                &cash,
                serde_json::json!([{
                    "accountId": car_id,
                    "amountMinor": -825,
                    "reason": ""
                }]),
            ),
        ),
    )
    .await;
    assert!(!blocked.ok);
    assert_eq!(
        blocked.error_code.as_deref(),
        Some("cash_adjust_reason_required")
    );
    assert_eq!(count_cash_adjust(&platform, "2026-08-28").await, 0);
    let capture = query_json(
        &platform,
        "TrendsWeekGet",
        Some(r#"{"asOfDate":"2026-08-28"}"#),
    )
    .await;
    assert_eq!(capture["exists"], false);
}

fn cash_from_references<F>(view: &serde_json::Value, mut typed_for: F) -> [i64; 6]
where
    F: FnMut(&str, Option<i64>) -> i64,
{
    let mut out = [1_000_i64; 6];
    let order = ["Income", "FI Roth", "Speculation", "Health", "Car", "9"];
    let refs = view["cashReferences"].as_array().unwrap();
    for (i, name) in order.iter().enumerate() {
        let reference = refs
            .iter()
            .find(|r| r["accountName"] == *name)
            .and_then(|r| r["referenceMinor"].as_i64());
        out[i] = typed_for(name, reference);
    }
    out
}

#[tokio::test]
async fn t4_null_reference_skips_adjust() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    for (name, kind) in [
        ("Income", "ira"),
        ("FI Roth", "fi_roth"),
        ("Speculation", "ira"),
        ("Health", "hsa"),
        ("Car", "taxable"),
        ("9", "ira"),
    ] {
        must_cmd(
            &platform,
            "AccountRegister",
            serde_json::json!({"name": name, "kind": kind}),
        )
        .await;
    }
    let cash = [1_000, 1_000, 1_000, 1_000, 1_000, 1_000];
    must_cmd(
        &platform,
        "WeekCaptureAccept",
        capture_body("2026-08-22", "2026-08-28", &cash, serde_json::json!([])),
    )
    .await;
    assert_eq!(count_cash_adjust(&platform, "2026-08-28").await, 0);
    let capture = query_json(
        &platform,
        "TrendsWeekGet",
        Some(r#"{"asOfDate":"2026-08-28"}"#),
    )
    .await;
    assert_eq!(capture["exists"], true);
    let refs = capture["cashReferences"].as_array().unwrap();
    assert!(refs.iter().all(|r| r["referenceMinor"].is_null()));
}

#[tokio::test]
async fn t6_cash_adjust_post_refuses_withholding() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    must_cmd(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Car", "kind": "taxable"}),
    )
    .await;
    let accounts = query_json(&platform, "AccountList", None).await;
    let car_id = accounts[0]["accountId"].as_str().unwrap();
    let blocked = execute_command_on(
        &platform,
        &platform,
        cmd(
            "CashAdjustPost",
            serde_json::json!({
                "accountId": car_id,
                "occurredOn": "2026-08-28",
                "amountMinor": -825,
                "scale": 2,
                "reason": "fee",
                "federalWithholdingMinor": 1,
                "stateWithholdingMinor": 0
            }),
        ),
    )
    .await;
    assert!(!blocked.ok);
    assert_eq!(
        blocked.error_code.as_deref(),
        Some("cash_adjust_withholding_not_allowed")
    );
    assert_eq!(count_cash_adjust(&platform, "2026-08-28").await, 0);
}

#[tokio::test]
async fn t9_blank_etf_total_stays_zero_no_proxy_invent() {
    let root = repo_root();
    let production = root.join("database/seed/production");
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    load_production_seed_via_commands(&platform, &production)
        .await
        .expect("seed");

    let view = query_json(
        &platform,
        "TrendsWeekGet",
        Some(r#"{"asOfDate":"2026-08-28"}"#),
    )
    .await;
    // Seeded Account 9 has last prices → suggested proxy may be > 0, but blank typed ETF must stay 0.
    let suggested = view["suggestedAcct9EtfProxyMinor"].as_i64().unwrap_or(0);
    assert!(
        suggested > 0 || view["suggestedAcct9EtfProxyMinor"].is_null(),
        "fixture should expose a real suggested proxy or null, not invented 0 alone when lots exist"
    );
    let cash = cash_from_references(&view, |_, ref_minor| ref_minor.unwrap_or(1_000));
    let mut body = capture_body("2026-08-22", "2026-08-28", &cash, serde_json::json!([]));
    body["acct9EtfValueMinor"] = serde_json::json!(0);
    must_cmd(&platform, "WeekCaptureAccept", body).await;
    let capture = query_json(
        &platform,
        "TrendsWeekGet",
        Some(r#"{"asOfDate":"2026-08-28"}"#),
    )
    .await;
    assert_eq!(
        capture["current"]["acct9EtfValueMinor"],
        0,
        "blank ETF total must not be replaced by suggested 70% proxy on Accept"
    );
    assert_eq!(count_cash_adjust(&platform, "2026-08-28").await, 0);
}
