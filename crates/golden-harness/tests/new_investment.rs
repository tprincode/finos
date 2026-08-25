use application_core::contracts::{
    CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::queries::{execute_command_on, execute_query_on};
use import_engine::{declaration_candidates, retrieve_result};
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

async fn seed_identity(platform: &LocalPlatform, symbol: &str) -> (String, String) {
    seed_identity_in(platform, symbol, "Income").await
}

async fn seed_identity_in(
    platform: &LocalPlatform,
    symbol: &str,
    account_name: &str,
) -> (String, String) {
    let account = must_ok(
        platform,
        "AccountRegister",
        serde_json::json!({"name": account_name, "kind": "taxable"}),
    )
    .await;
    let security = must_ok(
        platform,
        "SecurityRegister",
        serde_json::json!({"symbol": symbol, "name": symbol}),
    )
    .await;
    (
        account["accountId"].as_str().unwrap().to_string(),
        security["securityId"].as_str().unwrap().to_string(),
    )
}

async fn record_decls(platform: &LocalPlatform, security_id: &str, n: usize) {
    for i in 0..n {
        must_ok(
            platform,
            "IssuerDeclarationRecord",
            serde_json::json!({
                "securityId": security_id,
                "amountPerShareMinor": 10 + i as i64,
                "amountScale": 2,
                "paymentPeriod": format!("2026-0{}-01", (i % 9) + 1),
                "source": "provider-site",
                "enteredAt": "2026-08-21"
            }),
        )
        .await;
    }
}

#[tokio::test]
async fn wz_no_watch_omits_calculator_until_first_lot() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let (_account_id, security_id) = seed_identity(&platform, "NEW1").await;
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "priceSource": "public",
            "sourceSymbol": "NEW1",
            "declarationSource": "provider-site",
            "lookbackCount": 12
        }),
    )
    .await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Weekly",
            "riskTier": "HighRisk"
        }),
    )
    .await;
    record_decls(&platform, &security_id, 6).await;
    must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 12,
            "amountScale": 2,
            "planningPeriodsPerYear": 52,
            "effectiveFrom": "2026-08-21",
            "decisionReason": "owner"
        }),
    )
    .await;
    let calc = query_json(&platform, "CalculatorGet", serde_json::json!({})).await;
    let rows = calc["rows"].as_array().unwrap();
    assert!(
        rows.iter().all(|r| r["symbol"] != "NEW1"),
        "WZ-no-watch: Calculator must omit incomplete investment"
    );
    let set = query_json(&platform, "PriceRetrievalSetGet", serde_json::json!({})).await;
    let ids = set["securityIds"].as_array().cloned().unwrap_or_default();
    assert!(
        ids.iter().all(|id| id.as_str() != Some(security_id.as_str())),
        "WZ-daily-set: incomplete not in price retrieval set"
    );
}

#[tokio::test]
async fn wz_first_lot_then_calculator_and_income_plan() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let (account_id, security_id) = seed_identity(&platform, "NEW2").await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Weekly",
            "riskTier": "Core"
        }),
    )
    .await;
    record_decls(&platform, &security_id, 6).await;
    must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 100,
            "amountScale": 2,
            "planningPeriodsPerYear": 52,
            "effectiveFrom": "2026-08-21",
            "decisionReason": "owner"
        }),
    )
    .await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "openedOn": "2026-08-01",
            "origin": "purchase",
            "quantityMinor": 10,
            "quantityScale": 0,
            "performanceBasisMinor": 100_000,
            "taxBasisMinor": 100_000,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;
    let calc = query_json(&platform, "CalculatorGet", serde_json::json!({})).await;
    let row = calc["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["symbol"] == "NEW2")
        .expect("WZ-first-lot Calculator row");
    assert_eq!(row["remainingQuantityMinor"].as_i64(), Some(10));
    let week = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({"asOfDate": "2026-08-21"}),
    )
    .await;
    let income = week["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["accountName"] == "Income")
        .unwrap();
    assert!(
        income["plannedMinor"].as_i64().unwrap_or(0) > 0,
        "WZ-first-lot Income Plan future week has Plan × qty: {income}"
    );
    let set = query_json(&platform, "PriceRetrievalSetGet", serde_json::json!({})).await;
    let ids = set["securityIds"].as_array().cloned().unwrap_or_default();
    assert!(ids.iter().any(|id| id.as_str() == Some(security_id.as_str())));
    let dossier = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({"symbol": "NEW2", "asOfDate": "2026-08-21"}),
    )
    .await;
    assert_eq!(dossier["symbol"], "NEW2");
    assert_eq!(dossier["firstLotComplete"], true);
    assert_eq!(dossier["planKnown"], true);
    assert_eq!(dossier["declarationCount"].as_u64(), Some(6));
    assert_eq!(dossier["review"]["avg6Complete"], true);
    assert_eq!(
        dossier["declarations"].as_array().map(|a| a.len()),
        Some(6)
    );
    assert!(dossier["annualPlanMinor"].as_i64().unwrap_or(0) > 0);
}

#[tokio::test]
async fn wz_maintain_corrects_identity_without_wiping_roc() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let (_account_id, security_id) = seed_identity(&platform, "HAKY").await;
    must_ok(
        &platform,
        "SecurityUpdate",
        serde_json::json!({
            "securityId": security_id,
            "name": "Amplify HACK Cybersecurity Cove"
        }),
    )
    .await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "riskTier": "Risk On",
            "provider": "",
            "rocPct2025ActualMinor": 1234,
            "rocScale": 2
        }),
    )
    .await;
    let before = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({"symbol": "HAKY", "asOfDate": "2026-08-21"}),
    )
    .await;
    assert_eq!(before["name"], "Amplify HACK Cybersecurity Cove");
    assert_eq!(before["provider"], "");
    assert_eq!(before["riskTier"], "Risk On");
    assert_eq!(before["rocPct2025ActualMinor"].as_i64(), Some(1234));

    must_ok(
        &platform,
        "SecurityUpdate",
        serde_json::json!({
            "securityId": security_id,
            "name": "HAKY fund"
        }),
    )
    .await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "provider": "Amplify",
            "riskTier": "Core"
        }),
    )
    .await;
    let after = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({"symbol": "HAKY", "asOfDate": "2026-08-21"}),
    )
    .await;
    assert_eq!(after["name"], "HAKY fund");
    assert_eq!(after["provider"], "Amplify");
    assert_eq!(after["riskTier"], "Core");
    assert_eq!(after["paymentFrequency"], "Monthly");
    assert_eq!(
        after["rocPct2025ActualMinor"].as_i64(),
        Some(1234),
        "WZ-maintain must not wipe ROC when only provider/risk change"
    );
}

#[tokio::test]
async fn wz_price_live_and_prompt() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let (_account_id, security_id) = seed_identity(&platform, "PX").await;
    must_ok(
        &platform,
        "PriceQuoteRecord",
        serde_json::json!({
            "securityId": security_id,
            "priceMinor": 1250,
            "scale": 2,
            "asOfAt": "2026-08-21",
            "source": "fixture"
        }),
    )
    .await;
    let current = query_json(
        &platform,
        "CurrentPriceGet",
        serde_json::json!({"securityId": security_id, "asOfDate": "2026-08-21"}),
    )
    .await;
    assert_eq!(current["freshness"], "current");
    assert_eq!(current["priceMinor"].as_i64(), Some(1250));
    assert_eq!(current["priceDerivedValid"], true);

    let zero = execute_command_on(
        &platform,
        &platform,
        cmd(
            "PriceQuoteRecord",
            serde_json::json!({
                "securityId": security_id,
                "priceMinor": 0,
                "scale": 2,
                "asOfAt": "2026-08-21",
                "source": "fixture"
            }),
        ),
    )
    .await;
    assert!(!zero.ok);
    assert_eq!(zero.error_code.as_deref(), Some("nonpositive_price"));

    let (_a2, sid2) = seed_identity_in(&platform, "PY", "Car").await;
    let missing = query_json(
        &platform,
        "CurrentPriceGet",
        serde_json::json!({"securityId": sid2, "asOfDate": "2026-08-21"}),
    )
    .await;
    assert_eq!(missing["freshness"], "unavailable");
    assert!(missing["priceMinor"].is_null());
    must_ok(
        &platform,
        "ManualPriceOverride",
        serde_json::json!({
            "securityId": sid2,
            "priceMinor": 800,
            "scale": 2,
            "reason": "prompt after live miss",
            "effectiveFrom": "2026-08-21"
        }),
    )
    .await;
    let prompted = query_json(
        &platform,
        "CurrentPriceGet",
        serde_json::json!({"securityId": sid2, "asOfDate": "2026-08-21"}),
    )
    .await;
    assert_eq!(prompted["freshness"], "manual_override");
    assert_eq!(prompted["priceMinor"].as_i64(), Some(800));
}

#[tokio::test]
async fn wz_decl_12_candidates_are_not_cash() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let (account_id, security_id) = seed_identity(&platform, "D12").await;
    let injected = (0..12)
        .map(|i| {
            serde_json::json!({
                "amountPerShareMinor": 50 + i,
                "paymentPeriod": format!("p{i}"),
                "source": "provider-site"
            })
        })
        .collect::<Vec<_>>();
    let retrieved = execute_command_on(
        &platform,
        &platform,
        cmd(
            "DeclarationRetrieve",
            retrieve_result(declaration_candidates(Some(&serde_json::Value::Array(
                injected.clone(),
            )))),
        ),
    )
    .await;
    assert!(retrieved.ok);
    let body: serde_json::Value =
        serde_json::from_str(retrieved.body_json.as_deref().unwrap()).unwrap();
    assert_eq!(body["candidates"].as_array().unwrap().len(), 12);
    for cand in body["candidates"].as_array().unwrap() {
        must_ok(
            &platform,
            "IssuerDeclarationRecord",
            serde_json::json!({
                "securityId": security_id,
                "amountPerShareMinor": cand["amountPerShareMinor"],
                "amountScale": 2,
                "paymentPeriod": cand["paymentPeriod"],
                "source": cand["source"],
                "enteredAt": "2026-08-21"
            }),
        )
        .await;
    }
    let missing_source = execute_command_on(
        &platform,
        &platform,
        cmd(
            "IssuerDeclarationRecord",
            serde_json::json!({
                "securityId": security_id,
                "amountPerShareMinor": 1,
                "amountScale": 2,
                "paymentPeriod": "no-source",
                "source": "",
                "enteredAt": "2026-08-21"
            }),
        ),
    )
    .await;
    assert!(!missing_source.ok);
    assert_eq!(
        missing_source.error_code.as_deref(),
        Some("missing_declaration_source")
    );
    let dividend = query_json(&platform, "DividendGet", serde_json::json!({})).await;
    assert_eq!(dividend["actualTotalMinor"].as_i64().unwrap_or(0), 0);
    must_ok(
        &platform,
        "DividendActualRecord",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "occurredOn": "2026-08-14",
            "amountMinor": 500,
            "scale": 2,
            "idempotencyKey": "cash-1"
        }),
    )
    .await;
    let after = query_json(&platform, "DividendGet", serde_json::json!({})).await;
    assert_eq!(after["actualTotalMinor"].as_i64(), Some(500));
    let review = query_json(
        &platform,
        "PlanReviewGet",
        serde_json::json!({"securityId": security_id}),
    )
    .await;
    assert_eq!(review["observationCount"].as_u64(), Some(12));
    assert_eq!(review["fullAnalysisPossible"], true);
}

#[tokio::test]
async fn wz_plan_confirm_gates() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let (_account_id, security_id) = seed_identity(&platform, "PG").await;
    let blocked = execute_command_on(
        &platform,
        &platform,
        cmd(
            "PlanHistoryConfirm",
            serde_json::json!({
                "securityId": security_id,
                "amountPerShareMinor": 10,
                "amountScale": 2,
                "planningPeriodsPerYear": 52,
                "effectiveFrom": "2026-08-21",
                "decisionReason": "owner"
            }),
        ),
    )
    .await;
    assert!(!blocked.ok);
    assert_eq!(blocked.error_code.as_deref(), Some("plan_confirm_blocked"));

    record_decls(&platform, &security_id, 3).await;
    let incomplete = execute_command_on(
        &platform,
        &platform,
        cmd(
            "PlanHistoryConfirm",
            serde_json::json!({
                "securityId": security_id,
                "amountPerShareMinor": 99,
                "amountScale": 2,
                "planningPeriodsPerYear": 52,
                "effectiveFrom": "2026-08-21",
                "decisionReason": "owner"
            }),
        ),
    )
    .await;
    assert!(!incomplete.ok);
    assert_eq!(
        incomplete.error_code.as_deref(),
        Some("incomplete_analysis_required")
    );
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Weekly",
            "riskTier": "Core"
        }),
    )
    .await;
    let ok_incomplete = must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 99,
            "amountScale": 2,
            "planningPeriodsPerYear": 52,
            "effectiveFrom": "2026-08-21",
            "decisionReason": "owner",
            "incompleteAnalysisReason": "only three observations"
        }),
    )
    .await;
    assert_eq!(ok_incomplete["amountPerShareMinor"].as_i64(), Some(99));
    let review = query_json(
        &platform,
        "PlanReviewGet",
        serde_json::json!({"securityId": security_id}),
    )
    .await;
    assert_eq!(review["avg6Complete"], false);
    assert_ne!(review["mostCurrentMinor"].as_i64(), Some(99));
}

#[tokio::test]
async fn wz_roc_does_not_change_original_cost() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let (account_id, security_id) = seed_identity(&platform, "ROC1").await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "openedOn": "2026-01-01",
            "origin": "purchase",
            "quantityMinor": 10,
            "quantityScale": 0,
            "performanceBasisMinor": 50_000,
            "taxBasisMinor": 50_000,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;
    let actual = must_ok(
        &platform,
        "DividendActualRecord",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "occurredOn": "2026-06-01",
            "amountMinor": 1_000,
            "scale": 2,
            "idempotencyKey": "roc-div"
        }),
    )
    .await;
    let before = query_json(&platform, "BasisGet", serde_json::json!({})).await;
    must_ok(
        &platform,
        "DistributionCharacterize",
        serde_json::json!({
            "activityId": actual["activityId"],
            "category": "roc",
            "amountMinor": 1_000,
            "scale": 2
        }),
    )
    .await;
    let after = query_json(&platform, "BasisGet", serde_json::json!({})).await;
    assert_eq!(
        before["openPerformanceMinor"],
        after["openPerformanceMinor"],
        "WZ-roc original economic cost unchanged"
    );
}

#[tokio::test]
async fn wz_monthly_remaining_year_without_actuals_schedules_income_plan() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let (account_id, security_id) = seed_identity(&platform, "HAKY2").await;
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
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 1000,
            "amountScale": 4,
            "planningPeriodsPerYear": 12,
            "effectiveFrom": "2026-08-22",
            "decisionReason": "Match Most Current",
            "incompleteAnalysisReason": "Fewer than 6 observations"
        }),
    )
    .await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "openedOn": "2026-08-21",
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
    let remaining = query_json(
        &platform,
        "RemainingYearIncomeGet",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-08-22"
        }),
    )
    .await;
    assert_eq!(remaining["known"], true);
    assert_eq!(remaining["remainingPeriods"].as_i64(), Some(5));
    assert!(remaining["yearToGoMinor"].as_i64().unwrap() > 0);
    let dates: Vec<_> = remaining["payments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["payOn"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        dates,
        ["2026-08-30", "2026-09-29", "2026-10-29", "2026-11-28", "2026-12-28"]
    );
    assert_eq!(remaining["months"].as_array().unwrap().len(), 5);
    let week = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({"asOfDate": "2026-08-30"}),
    )
    .await;
    let income = week["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["accountName"] == "Income")
        .unwrap();
    assert!(
        income["plannedMinor"].as_i64().unwrap_or(0) > 0,
        "monthly new position with declarations and no actuals is scheduled: {income}"
    );
    let empty_week = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({"asOfDate": "2026-08-22"}),
    )
    .await;
    let empty_income = empty_week["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["accountName"] == "Income")
        .unwrap();
    assert_eq!(
        empty_income["plannedMinor"].as_i64(),
        Some(0),
        "week without a remaining payment date is not invented as $0 Plan of a pay week; it is not a pay week"
    );
}

#[tokio::test]
async fn last_price_stale_still_shows_on_calculator() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let (account_id, security_id) = seed_identity(&platform, "LP1").await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "openedOn": "2026-08-01",
            "origin": "purchase",
            "quantityMinor": 10,
            "quantityScale": 0,
            "performanceBasisMinor": 100_000,
            "taxBasisMinor": 100_000,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;
    let none = query_json(&platform, "CalculatorGet", serde_json::json!({})).await;
    let empty = none["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["symbol"] == "LP1")
        .expect("calculator row");
    assert!(empty["lastPriceMinor"].is_null());
    assert!(empty["marketValueMinor"].is_null());
    let summary_none = query_json(&platform, "HouseholdSummaryGet", serde_json::json!({})).await;
    assert_eq!(summary_none["openPerformanceMinor"].as_i64(), Some(100_000));
    assert_eq!(summary_none["lastPriceCount"].as_u64(), Some(0));
    assert!(summary_none["marketValueMinor"].is_null());

    let refresh = must_ok(
        &platform,
        "LastPriceRefresh",
        serde_json::json!({
            "quotes": [{
                "securityId": security_id,
                "priceMinor": 1250,
                "scale": 2,
                "asOfAt": "2026-08-20",
                "source": "fixture"
            }]
        }),
    )
    .await;
    assert_eq!(refresh["recorded"].as_u64(), Some(1));

    let stale = query_json(
        &platform,
        "CurrentPriceGet",
        serde_json::json!({"securityId": security_id, "asOfDate": "2026-08-21"}),
    )
    .await;
    assert_eq!(stale["freshness"], "stale");
    assert_eq!(stale["priceDerivedValid"], true);
    assert_eq!(stale["priceMinor"].as_i64(), Some(1250));

    let calc = query_json(&platform, "CalculatorGet", serde_json::json!({})).await;
    let row = calc["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["symbol"] == "LP1")
        .expect("calculator row after last price");
    assert_eq!(row["lastPriceMinor"].as_i64(), Some(1250));
    assert_eq!(row["marketValueMinor"].as_i64(), Some(12_500));
    assert_ne!(row["priceFreshness"], "unavailable");

    let summary = query_json(&platform, "HouseholdSummaryGet", serde_json::json!({})).await;
    assert_eq!(summary["lastPriceCount"].as_u64(), Some(1));
    assert_eq!(summary["marketValueMinor"].as_i64(), Some(12_500));
    assert_eq!(summary["marketValueComplete"], true);

    let zero = must_ok(
        &platform,
        "LastPriceRefresh",
        serde_json::json!({
            "quotes": [{
                "securityId": security_id,
                "priceMinor": 0,
                "scale": 2,
                "asOfAt": "2026-08-21",
                "source": "fixture"
            }]
        }),
    )
    .await;
    assert_eq!(zero["recorded"].as_u64(), Some(0));
    assert_eq!(zero["skipped"].as_u64(), Some(1));
    let still = query_json(
        &platform,
        "CurrentPriceGet",
        serde_json::json!({"securityId": security_id, "asOfDate": "2026-08-21"}),
    )
    .await;
    assert_eq!(still["priceMinor"].as_i64(), Some(1250));
}

#[tokio::test]
async fn wz_cadence_required_to_add_and_is_one_value() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let (_account_id, security_id) = seed_identity(&platform, "CAD1").await;
    let missing = execute_command_on(
        &platform,
        &platform,
        cmd(
            "PositionCharacteristicUpsert",
            serde_json::json!({
                "securityId": security_id,
                "riskTier": "Core"
            }),
        ),
    )
    .await;
    assert!(!missing.ok);
    assert_eq!(
        missing.error_code.as_deref(),
        Some("payment_cadence_required")
    );

    let stored = must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "12",
            "riskTier": "Core"
        }),
    )
    .await;
    assert_eq!(stored["paymentFrequency"], "Monthly");

    record_decls(&platform, &security_id, 6).await;
    let mismatch = execute_command_on(
        &platform,
        &platform,
        cmd(
            "PlanHistoryConfirm",
            serde_json::json!({
                "securityId": security_id,
                "amountPerShareMinor": 10,
                "amountScale": 2,
                "planningPeriodsPerYear": 52,
                "effectiveFrom": "2026-08-21",
                "decisionReason": "owner"
            }),
        ),
    )
    .await;
    assert!(!mismatch.ok);
    assert_eq!(
        mismatch.error_code.as_deref(),
        Some("payment_cadence_mismatch")
    );

    let plan = must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 10,
            "amountScale": 2,
            "effectiveFrom": "2026-08-21",
            "decisionReason": "owner"
        }),
    )
    .await;
    assert_eq!(plan["planningPeriodsPerYear"].as_u64(), Some(12));

    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({"symbol": "CAD1", "asOfDate": "2026-08-21"}),
    )
    .await;
    assert_eq!(inv["paymentFrequency"], "Monthly");
    assert_eq!(inv["planningPeriodsPerYear"].as_u64(), Some(12));
}

#[tokio::test]
async fn issuer_calendar_remaining_year_uses_published_dates() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let (account_id, security_id) = seed_identity(&platform, "HAKY3").await;
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
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "priceSource": "public",
            "sourceSymbol": "HAKY3",
            "declarationSource": "amplify",
            "sourceUrl": "https://amplifyetfs.com/haky3/",
            "calendarPolicy": "issuer_calendar",
            "lookbackCount": 12,
            "paymentSource": "import"
        }),
    )
    .await;
    must_ok(
        &platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 1000,
            "amountScale": 4,
            "paymentPeriod": "2026-07-01",
            "source": "amplify",
            "enteredAt": "2026-08-22"
        }),
    )
    .await;
    must_ok(
        &platform,
        "IssuerPayDateReplace",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-08-22",
            "dates": [
                {"payOn": "2026-08-31", "source": "amplify"},
                {"payOn": "2026-09-30", "source": "amplify"},
                {"payOn": "2026-10-30", "source": "amplify"},
                {"payOn": "2026-11-30", "source": "amplify"},
                {"payOn": "2026-12-31", "source": "amplify"}
            ]
        }),
    )
    .await;
    must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 1000,
            "amountScale": 4,
            "planningPeriodsPerYear": 12,
            "effectiveFrom": "2026-08-22",
            "decisionReason": "owner",
            "incompleteAnalysisReason": "Fewer than 6 observations"
        }),
    )
    .await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "openedOn": "2026-01-15",
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
    let remaining = query_json(
        &platform,
        "RemainingYearIncomeGet",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-08-22"
        }),
    )
    .await;
    assert_eq!(remaining["known"], true);
    assert_eq!(remaining["calendarPolicy"], "issuer_calendar");
    let dates: Vec<_> = remaining["payments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["payOn"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        dates,
        ["2026-08-31", "2026-09-30", "2026-10-30", "2026-11-30", "2026-12-31"]
    );
    assert_ne!(dates[0], "2026-08-30");
    assert_eq!(remaining["payments"][0]["dateProvenance"], "issuer_calendar");
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({"symbol": "HAKY3", "asOfDate": "2026-08-22"}),
    )
    .await;
    assert_eq!(inv["template"]["sourceUrl"], "https://amplifyetfs.com/haky3/");
    assert_eq!(inv["template"]["calendarPolicy"], "issuer_calendar");
}

#[tokio::test]
async fn add_lot_mid_year_increases_later_pay_dates_only() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let (account_id, security_id) = seed_identity(&platform, "ADD2").await;
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
    must_ok(
        &platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 100,
            "amountScale": 2,
            "paymentPeriod": "2026-07-01",
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
            "planningPeriodsPerYear": 12,
            "effectiveFrom": "2026-08-22",
            "decisionReason": "owner",
            "incompleteAnalysisReason": "Fewer than 6 observations"
        }),
    )
    .await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "openedOn": "2026-01-01",
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
    let preview = query_json(
        &platform,
        "RemainingYearIncomeGet",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-08-22",
            "thisLotQuantityMinor": 10,
            "thisLotOpenedOn": "2026-10-01"
        }),
    )
    .await;
    let payments = preview["payments"].as_array().unwrap();
    let aug = payments.iter().find(|p| p["payOn"] == "2026-08-30").unwrap();
    let oct = payments.iter().find(|p| p["payOn"] == "2026-10-29").unwrap();
    assert_eq!(aug["thisLotCashMinor"].as_i64(), Some(0));
    assert!(oct["thisLotCashMinor"].as_i64().unwrap() > 0);
    assert!(
        oct["positionAfterCashMinor"].as_i64().unwrap() > aug["positionAfterCashMinor"].as_i64().unwrap()
    );
}

#[tokio::test]
async fn issuer_calendar_replace_orphans_owner_override() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let (account_id, security_id) = seed_identity(&platform, "MOVE1").await;
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
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "declarationSource": "amplify",
            "calendarPolicy": "issuer_calendar"
        }),
    )
    .await;
    must_ok(
        &platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 100,
            "amountScale": 2,
            "paymentPeriod": "2026-07-01",
            "source": "amplify",
            "enteredAt": "2026-08-22"
        }),
    )
    .await;
    must_ok(
        &platform,
        "IssuerPayDateReplace",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-08-22",
            "dates": [
                {"payOn": "2026-09-10", "source": "amplify"},
                {"payOn": "2026-10-10", "source": "amplify"}
            ]
        }),
    )
    .await;
    must_ok(
        &platform,
        "RemainingPaymentDateOverride",
        serde_json::json!({
            "securityId": security_id,
            "originalPayOn": "2026-09-10",
            "payOn": "2026-09-11",
            "recordedAt": "2026-08-22"
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
            "planningPeriodsPerYear": 12,
            "effectiveFrom": "2026-08-22",
            "decisionReason": "owner",
            "incompleteAnalysisReason": "Fewer than 6 observations"
        }),
    )
    .await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "openedOn": "2026-01-01",
            "origin": "purchase",
            "quantityMinor": 1,
            "quantityScale": 0,
            "performanceBasisMinor": 100,
            "taxBasisMinor": 100,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;
    let first = query_json(
        &platform,
        "RemainingYearIncomeGet",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-08-22"
        }),
    )
    .await;
    assert_eq!(first["payments"][0]["payOn"], "2026-09-11");
    assert_eq!(first["payments"][0]["ownerOverride"], true);
    must_ok(
        &platform,
        "IssuerPayDateReplace",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-08-22",
            "dates": [
                {"payOn": "2026-09-15", "source": "amplify"},
                {"payOn": "2026-10-15", "source": "amplify"}
            ]
        }),
    )
    .await;
    let moved = query_json(
        &platform,
        "RemainingYearIncomeGet",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-08-22"
        }),
    )
    .await;
    assert_eq!(moved["payments"][0]["payOn"], "2026-09-15");
    assert_eq!(moved["payments"][0]["ownerOverride"], false);
    assert_eq!(moved["orphanedOverrides"].as_array().unwrap().len(), 1);
    assert_eq!(moved["orphanedOverrides"][0]["originalPayOn"], "2026-09-10");
}

#[tokio::test]
async fn declaration_retrieve_miss_writes_exception_not_zero() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let (_account_id, security_id) = seed_identity(&platform, "MISS1").await;
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "declarationSource": "amplify",
            "calendarPolicy": "derived_walk"
        }),
    )
    .await;
    must_ok(
        &platform,
        "DeclarationRefresh",
        serde_json::json!({
            "misses": [{
                "securityId": security_id,
                "symbol": "MISS1",
                "code": "declaration_retrieve_miss",
                "reason": "Issuer page empty — not using Yahoo."
            }]
        }),
    )
    .await;
    let exceptions = query_json(&platform, "ExceptionList", serde_json::json!({})).await;
    let rows = exceptions
        .as_array()
        .cloned()
        .or_else(|| exceptions["exceptions"].as_array().cloned())
        .unwrap_or_default();
    assert!(
        rows.iter().any(|e| e["code"] == "declaration_retrieve_miss"),
        "loud miss must land on ExceptionList: {exceptions}"
    );
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({"symbol": "MISS1", "asOfDate": "2026-08-22"}),
    )
    .await;
    assert_eq!(inv["template"]["lastRunOk"], false);
    assert!(inv["template"]["lastRunMessage"]
        .as_str()
        .unwrap_or("")
        .contains("not using Yahoo"));
}
