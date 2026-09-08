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
    let security_id = security["securityId"].as_str().unwrap().to_string();
    // Process B requires a researched identity (standing retrieval template) before LotOpen.
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
    (
        account["accountId"].as_str().unwrap().to_string(),
        security_id,
    )
}

async fn ready_first_lot(platform: &LocalPlatform, security_id: &str, symbol: &str) {
    golden_harness::complete_collector_for_first_lot(platform, security_id, symbol)
        .await
        .expect("complete collector");
}

async fn ready_first_lot_as(
    platform: &LocalPlatform,
    security_id: &str,
    symbol: &str,
    cadence: &str,
    replace_dates: bool,
) {
    golden_harness::complete_collector_for_first_lot_as(
        platform,
        security_id,
        symbol,
        cadence,
        replace_dates,
    )
    .await
    .expect("complete collector");
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
    ready_first_lot_as(&platform, &security_id, "NEW2", "Weekly", true).await;
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
        "WZ-first-lot Income Plan future week has Plan x qty: {income}"
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
    ready_first_lot(&platform, &security_id, "ROC1").await;
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
    ready_first_lot_as(&platform, &security_id, "HAKY2", "Monthly", false).await;
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
        ["2026-08-31", "2026-09-30", "2026-10-31", "2026-11-30", "2026-12-31"]
    );
    assert_eq!(remaining["months"].as_array().unwrap().len(), 5);
    let week = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({"asOfDate": "2026-08-31"}),
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
    let haky2 = week["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["symbol"] == "HAKY2")
        .expect("HAKY2 in by-position week");
    assert_eq!(haky2["cadence"], "Monthly");
    assert_eq!(haky2["payOn"], "2026-08-31");
    assert_eq!(haky2["planKnown"], true);
    assert!(haky2["plannedMinor"].as_i64().unwrap_or(0) > 0);
    assert_eq!(haky2["actualMinor"].as_i64().unwrap(), 0);
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
    ready_first_lot(&platform, &security_id, "LP1").await;
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
    let summary_none = query_json(&platform, "DataSummaryGet", serde_json::json!({})).await;
    assert_eq!(summary_none["openPerformanceMinor"].as_i64(), Some(100_000));
    assert_eq!(summary_none["lastPriceCount"].as_u64(), Some(0));
    assert_eq!(summary_none["declarationCount"].as_u64(), Some(0));
    assert!(
        summary_none["declarationCollectorCount"].as_u64().unwrap_or(0)
            <= summary_none["symbolCount"].as_u64().unwrap_or(0)
    );
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

    let summary = query_json(&platform, "DataSummaryGet", serde_json::json!({})).await;
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
            "paymentSource": "import",
            "inceptionOn": "2026-09-04"
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
    ready_first_lot_as(&platform, &security_id, "HAKY3", "Monthly", false).await;
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
    ready_first_lot_as(&platform, &security_id, "ADD2", "Monthly", false).await;
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
    let aug = payments.iter().find(|p| p["payOn"] == "2026-08-31").unwrap();
    let oct = payments.iter().find(|p| p["payOn"] == "2026-10-31").unwrap();
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
    ready_first_lot(&platform, &security_id, "MOVE1").await;
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
                "reason": "Issuer page empty."
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
        .contains("Issuer page empty"));
    assert!(
        !inv["template"]["lastRunMessage"]
            .as_str()
            .unwrap_or("")
            .contains("Yahoo")
    );
}

#[tokio::test]
async fn lookthrough_research_is_stored_and_does_not_apply_risk() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let (_account_id, security_id) = seed_identity(&platform, "HAKY").await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "riskTier": "Core",
            "provider": "Amplify",
            "underlying": "HACK",
            "lookthrough": {
                "themeStrategy": "Cybersecurity + Covered Call Equity",
                "primaryRiskDriver": "Look-through cybersecurity equity basket (HACK)",
                "concentrationStatus": "unknown",
                "topHoldings": [],
                "sectorWeights": [],
                "volProxy": "Slightly dampened version of HACK / cyber software basket",
                "taxCharacter": "Option premium (possible ROC component); ROC unknown until 19a-1",
                "riskTierSuggestion": "Risk On",
                "riskTierSuggestionReason": "Highest return potential + thematic concentration"
            }
        }),
    )
    .await;
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({"symbol": "HAKY", "asOfDate": "2026-08-24"}),
    )
    .await;
    assert_eq!(inv["riskTier"], "Core");
    assert_eq!(inv["underlying"], "HACK");
    assert_eq!(
        inv["lookthrough"]["themeStrategy"],
        "Cybersecurity + Covered Call Equity"
    );
    assert_eq!(inv["lookthrough"]["riskTierSuggestion"], "Risk On");
    assert_eq!(inv["lookthrough"]["concentrationStatus"], "unknown");
    assert!(inv["lookthrough"]["topHoldings"].as_array().unwrap().is_empty());
}

/// Process A (research-first): owner supplies symbol + distribution URL only.
/// No tier, frequency, provider, Plan, ROC, account, qty, or cost required.
#[tokio::test]
async fn process_a_seed_symbol_and_url_only() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let url = "https://amplifyetfs.com/haky/#distributions";
    let seed = must_ok(
        &platform,
        "PositionResearchSeed",
        serde_json::json!({
            "symbol": "HAKY",
            "sourceUrl": url
        }),
    )
    .await;
    assert_eq!(seed["symbol"], "HAKY");
    assert_eq!(seed["declarationSource"], "amplify");
    assert_eq!(seed["sourceUrl"], url);
    assert_eq!(seed["calendarPolicy"], "issuer_calendar");
    let security_id = seed["securityId"].as_str().expect("securityId");

    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-08-29" }),
    )
    .await;
    assert_eq!(inv["symbol"], "HAKY");
    assert_eq!(inv["template"]["sourceUrl"], url);
    assert_eq!(inv["template"]["declarationSource"], "amplify");
    assert_eq!(inv["template"]["calendarPolicy"], "issuer_calendar");
    assert_eq!(inv["template"]["collectorEnabled"], true);
    assert_eq!(inv["planKnown"], false);
    assert!(inv["lots"].as_array().unwrap().is_empty());
    assert_eq!(inv["remainingQuantityMinor"], 0);
    assert_eq!(inv["paymentFrequency"], "");
    assert_eq!(inv["riskTier"], "");
    assert_eq!(inv["provider"], "Amplify");
    assert_eq!(inv["divType"], "DIV-1");
    assert_eq!(inv["collectorComplete"], false);
    assert_eq!(inv["needsRocResearch"], true);

    let set = query_json(&platform, "CollectorSetGet", serde_json::json!({})).await;
    let items = set["items"].as_array().unwrap();
    assert!(
        !items.iter().any(|i| i["symbol"] == seed["symbol"]),
        "saved-but-incomplete names stay out of the daily fleet: {set}"
    );

    // Injected miss stays unknown - seed command still succeeds; no $0 declaration invented.
    let seeded_miss = must_ok(
        &platform,
        "PositionResearchSeed",
        serde_json::json!({
            "symbol": "HAKY",
            "sourceUrl": url,
            "misses": [{
                "securityId": security_id,
                "symbol": "HAKY",
                "code": "declaration_retrieve_miss",
                "reason": "Issuer page empty."
            }]
        }),
    )
    .await;
    assert_eq!(seeded_miss["retrieveOk"], false);
    assert_eq!(seeded_miss["retrieveCode"], "declaration_retrieve_miss");
    let inv2 = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-08-29" }),
    )
    .await;
    assert_eq!(inv2["declarationCount"], 0);
    assert!(inv2["lots"].as_array().unwrap().is_empty());
}

/// Process A: ≥2 paid monthly declarations (or issuer Monthly label) → frequency Monthly.
/// Owner does not type frequency. No lots. No auto Plan confirm.
/// Lookback stays a miss until 12 paid or inception — cadence inference is independent.
#[tokio::test]
async fn process_a_infers_monthly_from_seven_paid_haky_decls() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let url = "https://amplifyetfs.com/haky/#distributions";
    let periods = [
        "2026-01-30",
        "2026-02-27",
        "2026-03-31",
        "2026-04-30",
        "2026-05-29",
        "2026-06-30",
        "2026-07-31",
    ];
    let candidates: Vec<serde_json::Value> = periods
        .iter()
        .enumerate()
        .map(|(i, pay)| {
            serde_json::json!({
                "amountPerShareMinor": 10 + i as i64,
                "amountScale": 2,
                "paymentPeriod": pay,
                "source": "amplify"
            })
        })
        .collect();
    let seed = must_ok(
        &platform,
        "PositionResearchSeed",
        serde_json::json!({
            "symbol": "HAKY",
            "sourceUrl": url,
            "candidates": candidates,
            "suggestedFrequency": "Monthly"
        }),
    )
    .await;
    assert_eq!(seed["paymentFrequency"], "Monthly");
    assert_eq!(seed["retrieveOk"], false);
    assert_eq!(seed["retrieveCode"], "declaration_lookback_short");
    let security_id = seed["securityId"].as_str().unwrap();
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-08-29" }),
    )
    .await;
    assert_eq!(inv["paymentFrequency"], "Monthly");
    assert_eq!(inv["declarationCount"], 7);
    assert_eq!(inv["planKnown"], false);
    assert!(inv["lots"].as_array().unwrap().is_empty());
    assert_eq!(inv["riskTier"], "");
    // No 19a-1 injected -> ROC estimate stays unknown (never 0%).
    assert_eq!(inv["rocPct2026EstimateMinor"], serde_json::Value::Null);
    assert_eq!(inv["rocPct2026ActualMinor"], serde_json::Value::Null);

    // Spacing alone (no page label) still yields Monthly for ?2 paid monthly rows.
    let seed2 = must_ok(
        &platform,
        "PositionResearchSeed",
        serde_json::json!({
            "symbol": "HAKY2",
            "sourceUrl": url,
            "candidates": [
                {"amountPerShareMinor": 11, "amountScale": 2, "paymentPeriod": "2026-06-30", "source": "amplify"},
                {"amountPerShareMinor": 12, "amountScale": 2, "paymentPeriod": "2026-07-31", "source": "amplify"}
            ]
        }),
    )
    .await;
    assert_eq!(seed2["paymentFrequency"], "Monthly");
    let inv2 = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": seed2["securityId"], "asOfDate": "2026-08-29" }),
    )
    .await;
    assert_eq!(inv2["paymentFrequency"], "Monthly");
    assert_eq!(inv2["planKnown"], false);
    assert!(inv2["lots"].as_array().unwrap().is_empty());
}

/// Process A: after paid decls, propose Amplify 19a-1 current-year estimate.
/// Not research-complete. Not 1099 actual. No lots / Plan / tier.
#[tokio::test]
async fn process_a_proposes_19a1_roc_estimate_not_complete() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let url = "https://amplifyetfs.com/haky/#distributions";
    let notice = "https://amplifyetfs.com/wp-content/uploads/files/19a-1_Notice_04-30-26_HAKY.pdf";
    let periods = [
        "2026-01-30",
        "2026-02-27",
        "2026-03-31",
        "2026-04-30",
        "2026-05-29",
        "2026-06-30",
        "2026-07-31",
    ];
    let candidates: Vec<serde_json::Value> = periods
        .iter()
        .enumerate()
        .map(|(i, pay)| {
            serde_json::json!({
                "amountPerShareMinor": 10 + i as i64,
                "amountScale": 2,
                "paymentPeriod": pay,
                "source": "amplify"
            })
        })
        .collect();
    let seed = must_ok(
        &platform,
        "PositionResearchSeed",
        serde_json::json!({
            "symbol": "HAKY",
            "sourceUrl": url,
            "candidates": candidates,
            "rocCandidates": [{
                "rocPctMinor": 10000,
                "scale": 2,
                "taxYear": "2026",
                "source": "19a-1",
                "sourceUrl": notice,
                "method": "19a-1-current-year",
                "asOf": "2026-04-30",
                "kind": "estimate",
                "establishedHow": "current distribution 19a-1 estimate",
                "ownerOverride": false
            }]
        }),
    )
    .await;
    assert_eq!(seed["paymentFrequency"], "Monthly");
    assert_eq!(seed["rocPctMinor"], 10000);
    assert_eq!(seed["rocSourceUrl"], notice);
    assert_eq!(seed["rocKind"], "estimate");
    assert_eq!(seed["rocComplete"], false);
    assert_eq!(seed["rocMethod"], "19a-1-current-year");

    let security_id = seed["securityId"].as_str().unwrap();
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-08-29" }),
    )
    .await;
    assert_eq!(inv["paymentFrequency"], "Monthly");
    assert_eq!(inv["rocPct2026EstimateMinor"], 10000);
    assert_eq!(inv["rocPct2026ActualMinor"], serde_json::Value::Null);
    assert_eq!(inv["rocEstimateSourceUrl"], notice);
    assert_eq!(inv["rocResearchStatus"], "estimate-only · N/A 2025");
    assert_eq!(inv["needsRocResearch"], true);
    assert_eq!(inv["planKnown"], false);
    assert!(inv["lots"].as_array().unwrap().is_empty());
    assert_eq!(inv["riskTier"], "");

    let research = query_json(
        &platform,
        "RocResearchGet",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-08-29"
        }),
    )
    .await;
    // Usable 19a-1 suggestion exists, but research is not owner-confirmed / not 1099.
    assert_eq!(research["complete"], false);
    assert_eq!(research["systemRocPctMinor"], 10000);
    assert_eq!(research["sourceUrl"], notice);
    assert_eq!(research["kind"], "estimate");
    assert!(!research["reason"].as_str().unwrap_or("").contains("confirmed working"));
}

/// Validate current ROC estimate fills a hole: decls + stored URL, no prior ROC observation.
/// Does not re-ask symbol/URL. Does not complete research or invent 0%.
#[tokio::test]
async fn validate_current_roc_estimate_fills_hole_without_reasking() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let url = "https://amplifyetfs.com/haky/#distributions";
    let notice = "https://amplifyetfs.com/wp-content/uploads/files/19a-1_Notice_04-30-26_HAKY.pdf";
    let periods = [
        "2026-01-30",
        "2026-02-27",
        "2026-03-31",
        "2026-04-30",
        "2026-05-29",
        "2026-06-30",
        "2026-07-31",
    ];
    let candidates: Vec<serde_json::Value> = periods
        .iter()
        .enumerate()
        .map(|(i, pay)| {
            serde_json::json!({
                "amountPerShareMinor": 10 + i as i64,
                "amountScale": 2,
                "paymentPeriod": pay,
                "source": "amplify"
            })
        })
        .collect();
    let seed = must_ok(
        &platform,
        "PositionResearchSeed",
        serde_json::json!({
            "symbol": "HAKY",
            "sourceUrl": url,
            "candidates": candidates,
            "suggestedFrequency": "Monthly"
        }),
    )
    .await;
    let security_id = seed["securityId"].as_str().unwrap();
    let before = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-08-29" }),
    )
    .await;
    assert_eq!(before["declarationCount"], 7);
    assert_eq!(before["rocPct2026EstimateMinor"], serde_json::Value::Null);
    assert_eq!(before["template"]["sourceUrl"], url);

    // Same stored identity + URL x no re-ask. Injected 19a-1 stands in for live notice.
    let retrieved = must_ok(
        &platform,
        "RocResearchRetrieve",
        serde_json::json!({
            "symbol": "HAKY",
            "securityId": security_id,
            "declarationSource": "amplify",
            "sourceUrl": url,
            "asOfDate": "2026-08-29",
            "candidates": [{
                "rocPctMinor": 8750,
                "scale": 2,
                "taxYear": "2026",
                "source": "19a-1",
                "sourceUrl": notice,
                "method": "19a-1-current-year",
                "asOf": "2026-04-30",
                "kind": "estimate",
                "establishedHow": "current distribution 19a-1 estimate",
                "ownerOverride": false
            }]
        }),
    )
    .await;
    assert_eq!(retrieved["posted"], false);
    assert_eq!(retrieved["candidates"][0]["rocPctMinor"], 8750);

    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-08-29" }),
    )
    .await;
    assert_eq!(inv["rocPct2026EstimateMinor"], 8750);
    assert_eq!(inv["rocPct2026ActualMinor"], serde_json::Value::Null);
    assert_eq!(inv["rocPct2025ActualMinor"], serde_json::Value::Null);
    assert_eq!(inv["rocEstimateSourceUrl"], notice);
    assert_eq!(inv["rocResearchCompletedAt"], "2026-08-29");
    assert_eq!(inv["rocResearchStatus"], "estimate-only · N/A 2025");
    assert_eq!(inv["needsRocResearch"], true);
    assert_eq!(inv["planKnown"], false);
    assert!(inv["lots"].as_array().unwrap().is_empty());
    assert_eq!(inv["riskTier"], "");

    let research = query_json(
        &platform,
        "RocResearchGet",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-08-29"
        }),
    )
    .await;
    assert_eq!(research["complete"], false);
    assert_eq!(research["systemRocPctMinor"], 8750);
    assert_eq!(research["sourceUrl"], notice);
    assert_eq!(research["kind"], "estimate");
}

/// PositionResearchRefresh fills holes on HAKY with template + 7 decls + lots.
/// Never wipes lots or declaration periods. Provider Amplify; ROC estimate or visible probes.
#[tokio::test]
async fn position_research_refresh_fills_haky_holes_preserves_lots_and_decls() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let url = "https://amplifyetfs.com/haky/#distributions";
    let notice = "https://amplifyetfs.com/wp-content/uploads/files/19a-1_Notice_04-30-26_HAKY.pdf";
    let periods = [
        "2026-01-30",
        "2026-02-27",
        "2026-03-31",
        "2026-04-30",
        "2026-05-29",
        "2026-06-30",
        "2026-07-31",
    ];
    let candidates: Vec<serde_json::Value> = periods
        .iter()
        .enumerate()
        .map(|(i, pay)| {
            serde_json::json!({
                "amountPerShareMinor": 10 + i as i64,
                "amountScale": 2,
                "paymentPeriod": pay,
                "source": "amplify"
            })
        })
        .collect();
    let seed = must_ok(
        &platform,
        "PositionResearchSeed",
        serde_json::json!({
            "symbol": "HAKY",
            "sourceUrl": url,
            "candidates": candidates.clone(),
            "suggestedFrequency": "Monthly",
            "provider": "",
            "name": "HAKY"
        }),
    )
    .await;
    let security_id = seed["securityId"].as_str().unwrap().to_string();
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "provider": "",
            "underlying": "",
            "needsRocResearch": true,
            "isActive": true,
            "divType": "DIV-1"
        }),
    )
    .await;
    ready_first_lot(&platform, &security_id, "HAKY").await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account["accountId"],
            "securityId": security_id,
            "openedOn": "2026-08-15",
            "quantityMinor": 100,
            "quantityScale": 0,
            "performanceCostMinor": 250000,
            "taxCostMinor": 250000,
            "scale": 2
        }),
    )
    .await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "provider": "",
            "underlying": "",
            "needsRocResearch": true,
            "isActive": true,
            "divType": "DIV-1",
            "rocPct2026EstimateMinor": null
        }),
    )
    .await;

    let before = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-08-29" }),
    )
    .await;
    assert_eq!(before["declarationCount"], 7);
    assert_eq!(before["lots"].as_array().unwrap().len(), 1);
    assert_ne!(
        before["rocResearchStatus"],
        "missing-1099",
        "2026-08 lots are not a 2025 1099 miss: {}",
        before["rocResearchStatus"]
    );
    assert_eq!(before["rocResearchStatus"], "N/A 2025");
    let before_periods: Vec<String> = before["declarations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["paymentPeriod"].as_str().unwrap().to_string())
        .collect();

    let gaps_before = query_json(&platform, "ResearchGapsGet", serde_json::json!({})).await;
    let gap_syms: Vec<_> = gaps_before["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["symbol"].as_str().unwrap().to_string())
        .collect();
    assert!(
        gap_syms.iter().any(|s| Some(s.as_str()) == before["symbol"].as_str()),
        "open-lot payer missing provider or ROC estimate is a fill-gaps hole: {gaps_before}"
    );

    let refresh = must_ok(
        &platform,
        "PositionResearchRefresh",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "HAKY",
            "provider": "Amplify",
            "underlying": "HACK",
            "suggestedFrequency": "Monthly",
            "candidates": candidates,
            "rocCandidates": [{
                "rocPctMinor": 9200,
                "scale": 2,
                "taxYear": "2026",
                "source": "19a-1",
                "sourceUrl": notice,
                "method": "19a-1-current-year",
                "asOf": "2026-04-30",
                "kind": "estimate",
                "establishedHow": "current distribution 19a-1 estimate",
                "ownerOverride": false
            }],
            "rocProbes": [{
                "url": notice,
                "status": 200
            }]
        }),
    )
    .await;
    assert_eq!(refresh["provider"], "Amplify");
    assert_eq!(refresh["underlying"], "HACK");
    assert_eq!(refresh["rocPctMinor"], 9200);
    assert_eq!(refresh["rocSourceUrl"], notice);
    assert_eq!(refresh["needsRocResearch"], true);
    assert_eq!(refresh["rocComplete"], false);

    let after = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-08-29" }),
    )
    .await;
    assert_eq!(after["provider"], "Amplify");
    assert_eq!(after["underlying"], "HACK");
    assert_eq!(after["declarationCount"], 7);
    assert_eq!(after["lots"].as_array().unwrap().len(), 1);
    assert_eq!(after["rocPct2026EstimateMinor"], 9200);
    assert_eq!(after["needsRocResearch"], true);
    assert_eq!(after["rocResearchStatus"], "estimate-only · N/A 2025");
    assert_ne!(after["rocResearchStatus"].as_str().unwrap_or(""), "missing-1099");
    let after_periods: Vec<String> = after["declarations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["paymentPeriod"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(before_periods, after_periods);

    let gaps_after = query_json(&platform, "ResearchGapsGet", serde_json::json!({})).await;
    let gap_after: Vec<_> = gaps_after["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["symbol"].as_str().unwrap().to_string())
        .collect();
    assert!(
        !gap_after.iter().any(|s| Some(s.as_str()) == after["symbol"].as_str()),
        "provider + frequency + DIV-1 + ROC estimate clears the fill-gaps hole: {gaps_after}"
    );
}

#[tokio::test]
async fn position_research_refresh_19a1_miss_lists_probes_never_zero() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let url = "https://amplifyetfs.com/haky/#distributions";
    let tried = "https://amplifyetfs.com/wp-content/uploads/files/19a-1_Notice_04-30-26_HAKY.pdf";
    let periods = ["2026-06-30", "2026-07-31"];
    let candidates: Vec<serde_json::Value> = periods
        .iter()
        .enumerate()
        .map(|(i, pay)| {
            serde_json::json!({
                "amountPerShareMinor": 11 + i as i64,
                "amountScale": 2,
                "paymentPeriod": pay,
                "source": "amplify"
            })
        })
        .collect();
    let seed = must_ok(
        &platform,
        "PositionResearchSeed",
        serde_json::json!({
            "symbol": "HAKY",
            "sourceUrl": url,
            "candidates": candidates,
            "suggestedFrequency": "Monthly",
            "provider": "Amplify",
            "underlying": "HACK"
        }),
    )
    .await;
    let security_id = seed["securityId"].as_str().unwrap();
    let refresh = must_ok(
        &platform,
        "PositionResearchRefresh",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "HAKY",
            "rocProbes": [
                {"url": tried, "status": 404},
                {"url": "https://amplifyetfs.com/wp-content/uploads/files/19a-1_Notice_05-29-26_HAKY.pdf", "status": 404}
            ]
        }),
    )
    .await;
    assert!(
        refresh["rocPctMinor"].is_null(),
        "miss must not invent 0%: {refresh}"
    );
    let probes = refresh["rocProbes"].as_array().expect("probes");
    assert!(probes.len() >= 2, "tried URLs must be visible: {refresh}");
    assert_eq!(refresh["needsRocResearch"], true);
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-08-29" }),
    )
    .await;
    assert!(inv["rocPct2026EstimateMinor"].is_null());
    assert_eq!(inv["needsRocResearch"], true);
}

/// Complete research must replace a stored 0% / stale seed estimate with live 19a-1.
/// 0% is unknown, not an estimate. Seed needsRocResearch=false is not owner-lock.
#[tokio::test]
async fn complete_research_replaces_zero_and_stale_unconfirmed_estimate() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let url = "https://amplifyetfs.com/haky/#distributions";
    let notice = "https://amplifyetfs.com/wp-content/uploads/files/19a-1_Notice_04-30-26_HAKY.pdf";
    let candidates = vec![serde_json::json!({
        "amountPerShareMinor": 1000,
        "amountScale": 4,
        "paymentPeriod": "2026-07-31",
        "source": "amplify"
    })];
    let seed = must_ok(
        &platform,
        "PositionResearchSeed",
        serde_json::json!({
            "symbol": "HAKY",
            "sourceUrl": url,
            "candidates": candidates,
            "suggestedFrequency": "Monthly"
        }),
    )
    .await;
    let security_id = seed["securityId"].as_str().unwrap();
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "rocPct2026EstimateMinor": 0,
            "rocScale": 2,
            "needsRocResearch": false,
            "isActive": true
        }),
    )
    .await;
    let zero_cand = must_ok(
        &platform,
        "PositionResearchRefresh",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "HAKY",
            "candidates": candidates,
            "rocCandidates": [{
                "rocPctMinor": 0,
                "scale": 2,
                "taxYear": "2026",
                "source": "19a-1",
                "sourceUrl": notice,
                "method": "table-roc-current",
                "asOf": "2026-08-21",
                "kind": "estimate",
                "establishedHow": "latest distribution table ROC percent",
                "ownerOverride": false
            }]
        }),
    )
    .await;
    assert!(
        zero_cand["rocPctMinor"].is_null(),
        "0% table row is not an estimate: {zero_cand}"
    );

    let stale = must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "rocPct2026EstimateMinor": 1000,
            "rocScale": 2,
            "needsRocResearch": false,
            "isActive": true
        }),
    )
    .await;
    let _ = stale;
    let refresh = must_ok(
        &platform,
        "PositionResearchRefresh",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "HAKY",
            "candidates": candidates,
            "rocCandidates": [{
                "rocPctMinor": 9874,
                "scale": 2,
                "taxYear": "2026",
                "source": "19a-1",
                "sourceUrl": notice,
                "method": "table-roc-current",
                "asOf": "2026-08-14",
                "kind": "estimate",
                "establishedHow": "latest distribution table ROC percent",
                "ownerOverride": false
            }]
        }),
    )
    .await;
    assert_eq!(refresh["rocPctMinor"], 9874);
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-08-29" }),
    )
    .await;
    assert_eq!(inv["rocPct2026EstimateMinor"], 9874);
    assert_eq!(inv["needsRocResearch"], true);
}

/// Imported Calculator plans are owner data. Adapter research must not replace any of them.
#[tokio::test]
async fn adapter_research_does_not_replace_imported_plan_amounts() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let imported = [
        ("AMDW", "Weekly", 52, 55, 2, "roundhill", "https://www.roundhillinvestments.com/etf/amdw"),
        ("CRF", "Monthly", 12, 1168, 4, "cornerstone", "https://www.cornerstonetotalreturn.com/"),
        ("EPD", "Quarterly", 4, 55, 2, "enterprise", "https://www.enterpriseproducts.com/"),
    ];
    for (symbol, freq, periods, minor, scale, source, url) in imported {
        let security = must_ok(
            &platform,
            "SecurityRegister",
            serde_json::json!({"symbol": symbol, "name": symbol}),
        )
        .await;
        let security_id = security["securityId"].as_str().unwrap();
        must_ok(
            &platform,
            "PositionCharacteristicUpsert",
            serde_json::json!({
                "securityId": security_id,
                "paymentFrequency": freq,
                "riskTier": "Risk On",
                "provider": source,
                "divType": "DIV-1",
                "needsRocResearch": true,
                "isActive": true
            }),
        )
        .await;
        must_ok(
            &platform,
            "IssuerDeclarationRecord",
            serde_json::json!({
                "securityId": security_id,
                "amountPerShareMinor": minor,
                "amountScale": scale,
                "paymentPeriod": "2026-08-12",
                "source": "import",
                "enteredAt": "2026-08-12"
            }),
        )
        .await;
        must_ok(
            &platform,
            "PlanHistoryConfirm",
            serde_json::json!({
                "securityId": security_id,
                "amountPerShareMinor": minor,
                "amountScale": scale,
                "planningPeriodsPerYear": periods,
                "effectiveFrom": "2026-08-12",
                "decisionReason": "Locked Calculator from initial load on 2026-08-12",
                "incompleteAnalysisReason": "Fewer than 6 observations"
            }),
        )
        .await;
        must_ok(
            &platform,
            "PositionResearchRefresh",
            serde_json::json!({
                "securityId": security_id,
                "symbol": symbol,
                "declarationSource": source,
                "sourceUrl": url,
                "candidates": [{
                    "amountPerShareMinor": 9999,
                    "amountScale": 4,
                    "paymentPeriod": "2026-09-04",
                    "source": source
                }]
            }),
        )
        .await;
        let after = query_json(
            &platform,
            "InvestmentGet",
            serde_json::json!({ "securityId": security_id, "asOfDate": "2026-09-02" }),
        )
        .await;
        assert_eq!(after["planKnown"], true, "{symbol}");
        assert_eq!(after["planPerShareMinor"], minor, "{symbol}");
        assert_eq!(after["planScale"], scale, "{symbol}");
    }
}

fn seven_paid_amplify() -> Vec<serde_json::Value> {
    [
        "2026-01-30",
        "2026-02-27",
        "2026-03-31",
        "2026-04-30",
        "2026-05-29",
        "2026-06-30",
        "2026-07-31",
    ]
    .iter()
    .enumerate()
    .map(|(i, pay)| {
        serde_json::json!({
            "amountPerShareMinor": 10 + i as i64,
            "amountScale": 2,
            "paymentPeriod": pay,
            "source": "amplify"
        })
    })
    .collect()
}

/// First history parse fail prompts a second same-adapter URL once, then loud fail.
#[tokio::test]
async fn process_a_second_url_once_then_loud_fail() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let first = "https://amplifyetfs.com/haky/#distributions";
    let second = "https://amplifyetfs.com/haky/distributions/";
    let seed = must_ok(
        &platform,
        "PositionResearchSeed",
        serde_json::json!({
            "symbol": "HAKY",
            "sourceUrl": first,
            "misses": [{
                "symbol": "HAKY",
                "code": "declaration_retrieve_miss",
                "reason": "Issuer page empty."
            }]
        }),
    )
    .await;
    assert_eq!(seed["retrieveOk"], false);
    assert_eq!(seed["needsSecondUrl"], true);
    assert_eq!(seed["adapterFailed"], false);
    let security_id = seed["securityId"].as_str().unwrap();

    let foreign = must_ok(
        &platform,
        "PositionResearchRefresh",
        serde_json::json!({
            "securityId": security_id,
            "sourceUrl": "https://www.roundhillinvestments.com/etf/topw/",
            "secondUrlAttempt": true
        }),
    )
    .await;
    assert_eq!(foreign["retrieveCode"], "adapter_url_mismatch");
    assert_eq!(foreign["sourceUrl"], first);

    let loud = must_ok(
        &platform,
        "PositionResearchRefresh",
        serde_json::json!({
            "securityId": security_id,
            "sourceUrl": second,
            "secondUrlAttempt": true,
            "misses": [{
                "securityId": security_id,
                "symbol": "HAKY",
                "code": "declaration_retrieve_miss",
                "reason": "Second page empty."
            }]
        }),
    )
    .await;
    assert_eq!(loud["retrieveOk"], false);
    assert_eq!(loud["secondUrlTried"], true);
    assert_eq!(loud["adapterFailed"], true);
    assert!(
        loud["retrieveMessage"]
            .as_str()
            .unwrap_or("")
            .contains("Manual adapter is parked")
    );
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-08-29" }),
    )
    .await;
    assert_eq!(inv["collectorComplete"], false);
    assert!(inv["lots"].as_array().unwrap().is_empty());
}

/// After A, paid < 12: Yes stores inception_on; lookback can complete via expected periods.
#[tokio::test]
async fn process_a_inception_yes_stores_date() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let url = "https://amplifyetfs.com/haky/#distributions";
    let seed = must_ok(
        &platform,
        "PositionResearchSeed",
        serde_json::json!({
            "symbol": "HAKY",
            "sourceUrl": url,
            "candidates": seven_paid_amplify(),
            "suggestedFrequency": "Monthly",
            "asOfDate": "2026-09-05",
            "inceptionHits": [{
                "title": "HAKY inception 2026-02-01",
                "snippet": "Fund launched 2026-02-01"
            }]
        }),
    )
    .await;
    assert_eq!(seed["retrieveCode"], "declaration_lookback_short");
    assert_eq!(seed["needsInceptionConfirm"], true);
    assert_eq!(seed["inceptionCandidate"], "2026-02-01");
    let security_id = seed["securityId"].as_str().unwrap();

    let yes = must_ok(
        &platform,
        "PositionResearchRefresh",
        serde_json::json!({
            "securityId": security_id,
            "sourceUrl": url,
            "candidates": seven_paid_amplify(),
            "suggestedFrequency": "Monthly",
            "asOfDate": "2026-09-05",
            "inceptionOn": "2026-02-01",
            "inceptionConfirmed": true
        }),
    )
    .await;
    assert_eq!(yes["expectedPaidSinceInception"], 7);
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-09-05" }),
    )
    .await;
    assert_eq!(inv["template"]["inceptionOn"], "2026-02-01");
    let gaps = inv["collectorGaps"].as_array().unwrap();
    assert!(
        !gaps.iter().any(|g| g == "paid_history"),
        "inception Yes should clear paid_history: {gaps:?}"
    );
    assert_eq!(inv["collectorComplete"], false);
}

/// Owner No or search miss keeps the lookback ticket and collector incomplete.
#[tokio::test]
async fn process_a_inception_no_stays_incomplete() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let url = "https://amplifyetfs.com/haky/#distributions";
    let seed = must_ok(
        &platform,
        "PositionResearchSeed",
        serde_json::json!({
            "symbol": "HAKY",
            "sourceUrl": url,
            "candidates": seven_paid_amplify(),
            "suggestedFrequency": "Monthly",
            "asOfDate": "2026-09-05"
        }),
    )
    .await;
    assert_eq!(seed["inceptionSearchMiss"], true);
    let security_id = seed["securityId"].as_str().unwrap();
    let no = must_ok(
        &platform,
        "PositionResearchRefresh",
        serde_json::json!({
            "securityId": security_id,
            "sourceUrl": url,
            "candidates": seven_paid_amplify(),
            "inceptionConfirmed": false,
            "asOfDate": "2026-09-05"
        }),
    )
    .await;
    assert_eq!(no["inceptionSearchMiss"], true);
    let tickets = query_json(
        &platform,
        "WorkTicketList",
        serde_json::json!({ "securityId": security_id, "status": "open" }),
    )
    .await;
    let items = tickets["items"].as_array().unwrap();
    assert!(
        items.iter().any(|t| t["code"] == "declaration_lookback_short"),
        "{items:?}"
    );
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-09-05" }),
    )
    .await;
    assert_eq!(inv["collectorComplete"], false);
    assert!(inv["collectorGaps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|g| g == "paid_history"));
}

/// Phase I derive uses last calendar day / remaining periods through 31 Dec.
#[tokio::test]
async fn process_a_phase_i_derives_remaining_year_last_calendar_day() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let seed = must_ok(
        &platform,
        "PositionResearchSeed",
        serde_json::json!({
            "symbol": "HAKY",
            "sourceUrl": "https://amplifyetfs.com/haky/#distributions",
            "candidates": seven_paid_amplify(),
            "suggestedFrequency": "Monthly",
            "asOfDate": "2026-08-22"
        }),
    )
    .await;
    let security_id = seed["securityId"].as_str().unwrap();
    let remaining = query_json(
        &platform,
        "RemainingYearIncomeGet",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-08-22"
        }),
    )
    .await;
    let dates: Vec<_> = remaining["payments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["payOn"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        dates,
        ["2026-08-31", "2026-09-30", "2026-10-31", "2026-11-30", "2026-12-31"]
    );
    assert_eq!(remaining["remainingPeriods"].as_i64(), Some(5));
}

/// After C search miss: owner ROC URL is stored as the reusable template and parsed 0% is kept.
#[tokio::test]
async fn process_a_owner_roc_url_parses_zero_when_notice_says_zero() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let url = "https://amplifyetfs.com/haky/#distributions";
    let roc_url = "https://amplifyetfs.com/wp-content/uploads/files/19a-1_Notice_05-29-26_HAKY.pdf";
    let seed = must_ok(
        &platform,
        "PositionResearchSeed",
        serde_json::json!({
            "symbol": "HAKY",
            "sourceUrl": url,
            "candidates": seven_paid_amplify(),
            "suggestedFrequency": "Monthly"
        }),
    )
    .await;
    assert_eq!(seed["rocPctMinor"], serde_json::Value::Null);
    let security_id = seed["securityId"].as_str().unwrap();
    let refresh = must_ok(
        &platform,
        "PositionResearchRefresh",
        serde_json::json!({
            "securityId": security_id,
            "sourceUrl": url,
            "candidates": seven_paid_amplify(),
            "rocSourceUrl": roc_url,
            "rocCandidates": [{
                "rocPctMinor": 0,
                "scale": 2,
                "taxYear": "2026",
                "source": "19a-1",
                "sourceUrl": roc_url,
                "method": "19a-1-current-year",
                "asOf": "2026-05-29",
                "kind": "estimate",
                "establishedHow": "current distribution 19a-1 estimate",
                "ownerOverride": false
            }]
        }),
    )
    .await;
    assert_eq!(refresh["rocPctMinor"], 0);
    assert_eq!(refresh["rocSourceUrl"], roc_url);
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-08-29" }),
    )
    .await;
    assert_eq!(inv["rocPct2026EstimateMinor"], 0);
    assert_eq!(inv["template"]["rocSourceUrl"], roc_url);
    assert_eq!(inv["collectorComplete"], false);
}

/// Required Skip raises a ticket and is not complete. Backtest Skip does not block.
#[tokio::test]
async fn process_a_required_skip_blocks_complete_backtest_does_not() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let seed = must_ok(
        &platform,
        "PositionResearchSeed",
        serde_json::json!({
            "symbol": "HAKY",
            "sourceUrl": "https://amplifyetfs.com/haky/#distributions",
            "candidates": seven_paid_amplify()
        }),
    )
    .await;
    let security_id = seed["securityId"].as_str().unwrap();
    golden_harness::complete_collector_for_first_lot(&platform, security_id, "HAKY")
        .await
        .expect("complete");
    let as_of = chrono::Utc::now().date_naive().to_string();
    let ready = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": as_of }),
    )
    .await;
    assert_eq!(ready["collectorComplete"], true);

    must_ok(
        &platform,
        "CollectorFieldDecisionSet",
        serde_json::json!({
            "securityId": security_id,
            "field": "roc_estimate",
            "decision": "skip"
        }),
    )
    .await;
    must_ok(
        &platform,
        "CollectorFieldDecisionSet",
        serde_json::json!({
            "securityId": security_id,
            "field": "backtest",
            "decision": "skip"
        }),
    )
    .await;
    let skipped = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": as_of }),
    )
    .await;
    assert_eq!(skipped["collectorComplete"], false);
    assert!(skipped["collectorGaps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|g| g == "roc_estimate"));
    let tickets = query_json(
        &platform,
        "WorkTicketList",
        serde_json::json!({ "securityId": security_id, "status": "open" }),
    )
    .await;
    assert!(tickets["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["code"] == "roc_estimate"));

    must_ok(
        &platform,
        "CollectorFieldDecisionSet",
        serde_json::json!({
            "securityId": security_id,
            "field": "roc_estimate",
            "decision": "accept"
        }),
    )
    .await;
    let accepted = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": as_of }),
    )
    .await;
    assert_eq!(accepted["collectorComplete"], true);
}

/// Income payers get DIV-1 on create. Empty div_type is a ticket and not complete.
#[tokio::test]
async fn new1_income_payer_gets_div1_on_create_blank_is_ticket() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let seed = must_ok(
        &platform,
        "PositionResearchSeed",
        serde_json::json!({
            "symbol": "NEW1",
            "sourceUrl": "https://example.test/new1/distributions",
            "skipRefresh": true
        }),
    )
    .await;
    let security_id = seed["securityId"].as_str().unwrap();
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-09-05" }),
    )
    .await;
    assert_eq!(inv["divType"], "DIV-1");
    assert_eq!(inv["collectorComplete"], false);

    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "divType": "",
            "isActive": true
        }),
    )
    .await;
    let after = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-09-05" }),
    )
    .await;
    assert_eq!(after["divType"], "");
    assert_eq!(after["collectorComplete"], false);
    assert!(after["collectorGaps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|g| g == "div_type"));
    let tickets = query_json(
        &platform,
        "WorkTicketList",
        serde_json::json!({ "securityId": security_id, "status": "open" }),
    )
    .await;
    assert!(
        tickets["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["code"] == "div_type"),
        "empty div_type must ticket: {tickets}"
    );
}

/// CASH1 stays CASH. Refresh must not write DIV-1 onto cash. No ROC estimate required.
#[tokio::test]
async fn cash1_stays_cash_and_does_not_need_roc() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let seed = must_ok(
        &platform,
        "PositionResearchSeed",
        serde_json::json!({
            "symbol": "CASH1",
            "sourceUrl": "https://example.test/cash1",
            "divType": "CASH",
            "skipRefresh": true
        }),
    )
    .await;
    let security_id = seed["securityId"].as_str().unwrap();
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-09-05" }),
    )
    .await;
    assert_eq!(inv["divType"], "CASH");
    assert_eq!(inv["needsRocResearch"], false);
    must_ok(
        &platform,
        "PositionResearchRefresh",
        serde_json::json!({
            "securityId": security_id,
            "sourceUrl": "https://example.test/cash1",
            "divType": "DIV-1"
        }),
    )
    .await;
    let after = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-09-05" }),
    )
    .await;
    assert_eq!(after["divType"], "CASH");
    assert_eq!(after["needsRocResearch"], false);
}

/// Checklist characteristics persist. Suggested tier is not auto-applied.
#[tokio::test]
async fn pay1_characteristics_surface_and_risk_is_owner_only() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let seed = must_ok(
        &platform,
        "PositionResearchSeed",
        serde_json::json!({
            "symbol": "PAY1",
            "sourceUrl": "https://example.test/pay1/distributions",
            "underlying": "UNDR",
            "lookthrough": {
                "themeStrategy": "Covered Call + Leveraged",
                "primaryRiskDriver": "Look-through UNDR",
                "coveredCall": true,
                "leveraged": true,
                "riskTierSuggestion": "Risk On",
                "riskTierSuggestionReason": "Covered-call overlay"
            },
            "skipRefresh": true
        }),
    )
    .await;
    let security_id = seed["securityId"].as_str().unwrap();
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-09-05" }),
    )
    .await;
    assert_eq!(inv["divType"], "DIV-1");
    assert_eq!(inv["underlying"], "UNDR");
    assert_eq!(inv["lookthrough"]["coveredCall"], true);
    assert_eq!(inv["lookthrough"]["leveraged"], true);
    assert_eq!(inv["lookthrough"]["riskTierSuggestion"], "Risk On");
    assert_eq!(inv["riskTier"], "");
}

/// Standing ROC URL is stored and reused on the next Complete research / Fill gaps.
#[tokio::test]
async fn pay1_roc_source_url_persists_on_template() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let roc_url = "https://issuer.example/files/19a-1_Notice_05-29-26_PAY1.pdf";
    let seed = must_ok(
        &platform,
        "PositionResearchSeed",
        serde_json::json!({
            "symbol": "PAY1",
            "sourceUrl": "https://example.test/pay1/distributions",
            "rocSourceUrl": roc_url,
            "skipRefresh": true
        }),
    )
    .await;
    let security_id = seed["securityId"].as_str().unwrap();
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-09-05" }),
    )
    .await;
    assert_eq!(inv["template"]["rocSourceUrl"], roc_url);
    let refresh = must_ok(
        &platform,
        "PositionResearchRefresh",
        serde_json::json!({
            "securityId": security_id,
            "sourceUrl": "https://example.test/pay1/distributions"
        }),
    )
    .await;
    let _ = refresh;
    let after = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-09-05" }),
    )
    .await;
    assert_eq!(after["template"]["rocSourceUrl"], roc_url);
}

/// Grandfather is already-has-open-lots, not a named symbol.
#[tokio::test]
async fn lot_open_grandfather_is_already_has_open_lots() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let seed = must_ok(
        &platform,
        "PositionResearchSeed",
        serde_json::json!({
            "symbol": "NEW1",
            "sourceUrl": "https://example.test/new1/distributions",
            "skipRefresh": true
        }),
    )
    .await;
    let new1 = seed["securityId"].as_str().unwrap();
    let blocked = execute_command_on(
        &platform,
        &platform,
        cmd(
            "LotOpen",
            serde_json::json!({
                "accountId": account["accountId"],
                "securityId": new1,
                "openedOn": "2026-01-02",
                "quantityMinor": 10,
                "quantityScale": 0,
                "performanceCostMinor": 1000,
                "taxCostMinor": 1000,
                "scale": 2
            }),
        ),
    )
    .await;
    assert!(!blocked.ok);
    assert_eq!(blocked.error_code.as_deref(), Some("collector_incomplete"));

    let pay = must_ok(
        &platform,
        "PositionResearchSeed",
        serde_json::json!({
            "symbol": "PAY1",
            "sourceUrl": "https://example.test/pay1/distributions",
            "skipRefresh": true
        }),
    )
    .await;
    let pay1 = pay["securityId"].as_str().unwrap().to_string();
    ready_first_lot(&platform, &pay1, "PAY1").await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account["accountId"],
            "securityId": pay1,
            "openedOn": "2026-01-02",
            "quantityMinor": 10,
            "quantityScale": 0,
            "performanceCostMinor": 1000,
            "taxCostMinor": 1000,
            "scale": 2
        }),
    )
    .await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": pay1,
            "riskTier": "",
            "isActive": true
        }),
    )
    .await;
    let incomplete = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": pay1, "asOfDate": "2026-09-05" }),
    )
    .await;
    assert_eq!(incomplete["collectorComplete"], false);
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account["accountId"],
            "securityId": pay1,
            "openedOn": "2026-02-02",
            "quantityMinor": 5,
            "quantityScale": 0,
            "performanceCostMinor": 500,
            "taxCostMinor": 500,
            "scale": 2
        }),
    )
    .await;
}
