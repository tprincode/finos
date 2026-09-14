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

#[tokio::test]
async fn roc_unknown_is_not_zero_and_car_magi_plans_then_actuals() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    must_ok(
        &platform,
        "MagiRuleSet",
        serde_json::json!({
            "thresholdMinor": 50_000_000,
            "safetyReserveMinor": 1_000_000,
            "scale": 2
        }),
    )
    .await;
    let car = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Car", "kind": "taxable"}),
    )
    .await;
    let account_id = car["accountId"].as_str().unwrap();
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "HAKY", "name": "HAKY"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "riskTier": "Risk On",
            "rocPct2025ActualMinor": 7000,
            "rocScale": 2
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
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "priceSource": "public",
            "sourceSymbol": "HAKY",
            "declarationSource": "amplify",
            "sourceUrl": "https://amplifyetfs.com/haky/#distributions",
            "calendarPolicy": "derived_walk",
            "collectorEnabled": true,
            "lookbackCount": 12
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
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "riskTier": "Risk On",
            "rocPct2025ActualMinor": 7000,
            "rocPct2026EstimateMinor": null,
            "rocScale": 2
        }),
    )
    .await;

    let miss = must_ok(
        &platform,
        "RocResearchRetrieve",
        serde_json::json!({"symbol": "HAKY", "candidates": []}),
    )
    .await;
    assert_eq!(miss["posted"], false);
    assert!(miss["candidates"].as_array().unwrap().is_empty());

    let research = query_json(
        &platform,
        "RocResearchGet",
        serde_json::json!({
            "securityId": security_id,
            "accountId": account_id,
            "asOfDate": "2026-08-22"
        }),
    )
    .await;
    assert_eq!(research["rocPctMinor"].as_i64(), Some(7000));
    assert_eq!(research["complete"], true);
    assert_eq!(research["magiEligible"], true);
    assert!(research["remainingOrdinaryMinor"].as_i64().unwrap() > 0);
    let planned_ordinary = research["remainingOrdinaryMinor"].as_i64().unwrap();
    let planned_roc = research["remainingRocMinor"].as_i64().unwrap();
    assert_eq!(planned_ordinary + planned_roc, research["remainingTotalMinor"].as_i64().unwrap());
    let remaining = query_json(
        &platform,
        "RemainingYearIncomeGet",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-08-22"
        }),
    )
    .await;
    assert_eq!(
        remaining["remainingPeriods"], research["remainingPeriods"],
        "MAGI remaining periods must match remaining-year payment dates"
    );
    assert_eq!(remaining["remainingPeriods"].as_i64(), Some(5));
    assert!(remaining["known"].as_bool().unwrap());
    assert!(!remaining["months"].as_array().unwrap().is_empty());

    must_ok(
        &platform,
        "RocPlanConfirm",
        serde_json::json!({
            "securityId": security_id,
            "accountId": account_id,
            "rocPctMinor": 7000,
            "rocScale": 2,
            "source": "prior-year-actual",
            "asOfDate": "2026-08-22"
        }),
    )
    .await;

    let before = query_json(&platform, "MagiProjectionGet", serde_json::json!({})).await;
    assert!(
        before["uncertainAmount"]["amountMinor"].as_i64().unwrap() >= planned_ordinary,
        "planned ordinary must be MAGI uncertain, not ROC: {before}"
    );
    let basis_before = query_json(&platform, "BasisGet", serde_json::json!({})).await;

    must_ok(
        &platform,
        "DividendActualRecord",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "occurredOn": "2026-08-22",
            "amountMinor": 10_000,
            "scale": 2,
            "idempotencyKey": "haky-div-1"
        }),
    )
    .await;
    let after = query_json(&platform, "MagiProjectionGet", serde_json::json!({})).await;
    assert_eq!(
        after["actualIncludedYtd"]["amountMinor"].as_i64().unwrap(),
        3_000,
        "70% ROC means 30% ordinary MAGI actual: {after}"
    );
    let basis_after = query_json(&platform, "BasisGet", serde_json::json!({})).await;
    assert_eq!(
        basis_after["openPerformanceMinor"],
        basis_before["openPerformanceMinor"],
        "ROC must not reduce original economic cost"
    );
    let car_plan = query_json(
        &platform,
        "CarRocPlanGet",
        serde_json::json!({ "asOfDate": "2026-08-22" }),
    )
    .await;
    assert!(
        car_plan["estimateNote"]
            .as_str()
            .unwrap_or("")
            .contains("prior-year ROC guidance"),
        "{car_plan}"
    );
    assert!(
        car_plan["taxNote"]
            .as_str()
            .unwrap_or("")
            .contains("April 2027"),
        "{car_plan}"
    );
    let rem_ord = car_plan["remainingOrdinaryMinor"].as_i64().unwrap();
    let rem_roc = car_plan["remainingRocMinor"].as_i64().unwrap();
    assert_eq!(
        rem_ord + rem_roc,
        car_plan["remainingTotalMinor"].as_i64().unwrap(),
        "Car remaining ordinary + ROC is the remaining cash: {car_plan}"
    );
    assert_eq!(car_plan["ytdPaidMinor"].as_i64(), Some(10_000));
    assert_eq!(car_plan["ytdOrdinaryMinor"].as_i64(), Some(3_000));
    assert_eq!(car_plan["ytdRocMinor"].as_i64(), Some(7_000));
    assert!(car_plan["ytdLongTermGainMinor"].is_null(), "{car_plan}");
    assert!(car_plan["ytdShortTermGainMinor"].is_null(), "{car_plan}");
    assert!(car_plan["lotSalePlMinor"].is_null(), "{car_plan}");
    assert!(
        car_plan["lotSaleNote"]
            .as_str()
            .unwrap_or("")
            .contains("Long-term and short-term stay unknown"),
        "{car_plan}"
    );
}

#[tokio::test]
async fn current_year_19a1_populates_and_override_keeps_system() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let car = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Car", "kind": "taxable"}),
    )
    .await;
    let account_id = car["accountId"].as_str().unwrap();
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "HAKY", "name": "HAKY"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "riskTier": "Risk On",
            "rocPct2025ActualMinor": 7000,
            "rocScale": 2
        }),
    )
    .await;

    let notice = "https://amplifyetfs.com/wp-content/uploads/files/19a-1_Notice_04-30-26_HAKY.pdf";
    let retrieved = must_ok(
        &platform,
        "RocResearchRetrieve",
        serde_json::json!({
            "symbol": "HAKY",
            "securityId": security_id,
            "asOfDate": "2026-08-22",
            "candidates": [{
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
    assert_eq!(retrieved["posted"], false);
    assert_eq!(retrieved["candidates"][0]["rocPctMinor"], 10000);

    let research = query_json(
        &platform,
        "RocResearchGet",
        serde_json::json!({
            "securityId": security_id,
            "accountId": account_id,
            "asOfDate": "2026-08-22"
        }),
    )
    .await;
    assert_eq!(research["rocPctMinor"].as_i64(), Some(10_000));
    assert_eq!(research["systemRocPctMinor"].as_i64(), Some(10_000));
    assert_eq!(research["sourceUrl"].as_str(), Some(notice));
    assert_eq!(research["method"].as_str(), Some("19a-1-current-year"));
    assert_eq!(research["asOf"].as_str(), Some("2026-04-30"));
    assert_eq!(research["ownerOverride"], false);
    assert!(
        research["establishedHow"]
            .as_str()
            .unwrap_or("")
            .contains("19a-1"),
        "must record how the percent was established: {research}"
    );
    let observations = research["observations"].as_array().unwrap();
    assert!(
        observations.iter().any(|o| {
            o["ownerOverride"] == false && o["rocPctMinor"].as_i64() == Some(10_000)
        }),
        "system 19a-1 row must be stored: {research}"
    );

    must_ok(
        &platform,
        "RocPlanConfirm",
        serde_json::json!({
            "securityId": security_id,
            "accountId": account_id,
            "rocPctMinor": 6500,
            "rocScale": 2,
            "source": "owner-override",
            "sourceUrl": notice,
            "method": "19a-1-current-year",
            "asOf": "2026-04-30",
            "kind": "estimate",
            "establishedHow": "current distribution 19a-1 estimate",
            "systemRocPctMinor": 10000,
            "ownerOverride": true,
            "asOfDate": "2026-08-22"
        }),
    )
    .await;

    let after = query_json(
        &platform,
        "RocResearchGet",
        serde_json::json!({
            "securityId": security_id,
            "accountId": account_id,
            "asOfDate": "2026-08-22"
        }),
    )
    .await;
    assert_eq!(after["rocPctMinor"].as_i64(), Some(6_500));
    assert_eq!(after["systemRocPctMinor"].as_i64(), Some(10_000));
    assert_eq!(after["ownerOverride"], true);
    let kept = after["observations"].as_array().unwrap();
    assert!(kept.iter().any(|o| {
        o["ownerOverride"] == false && o["rocPctMinor"].as_i64() == Some(10_000)
    }));
    assert!(kept.iter().any(|o| {
        o["ownerOverride"] == true && o["rocPctMinor"].as_i64() == Some(6_500)
    }));

    let incomplete = execute_command_on(
        &platform,
        &platform,
        cmd(
            "BacktestPeriodRecord",
            serde_json::json!({
                "kind": "Bear",
                "method": "close",
                "selectionReason": "owner stress window"
            }),
        ),
    )
    .await;
    assert!(!incomplete.ok);
    assert_eq!(
        incomplete.error_code.as_deref(),
        Some("regime_period_incomplete")
    );

    must_ok(
        &platform,
        "BacktestPeriodRecord",
        serde_json::json!({
            "kind": "Bull",
            "name": "Bull HAKY",
            "startOn": "2026-02-27",
            "endOn": "2026-04-07",
            "method": "close",
            "selectionReason": "owner-named post-inception rally",
            "recordedAt": "2026-08-22"
        }),
    )
    .await;
    must_ok(
        &platform,
        "BacktestPeriodRecord",
        serde_json::json!({
            "kind": "Bear",
            "name": "Bear HAKY",
            "startOn": "2026-04-08",
            "endOn": "2026-05-15",
            "method": "close",
            "selectionReason": "owner-named drawdown after rally",
            "recordedAt": "2026-08-22"
        }),
    )
    .await;
}

#[tokio::test]
async fn income_account_does_not_write_car_magi() {
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
        serde_json::json!({"symbol": "XYZ", "name": "XYZ"}),
    )
    .await;
    let research = query_json(
        &platform,
        "RocResearchGet",
        serde_json::json!({
            "securityId": security["securityId"],
            "accountId": income["accountId"],
            "asOfDate": "2026-08-22"
        }),
    )
    .await;
    assert_eq!(research["magiEligible"], false);
    assert_eq!(research["complete"], false);
    assert!(research["rocPctMinor"].is_null());
}

#[tokio::test]
async fn owner_lock_zero_roc_is_accepted_not_invented() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "PAY1", "name": "PAY1"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Quarterly",
            "divType": "DIV-1",
            "isActive": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "RocPlanConfirm",
        serde_json::json!({
            "securityId": security_id,
            "rocPctMinor": 0,
            "rocScale": 2,
            "ownerOverride": true,
            "source": "owner-override",
            "sourceUrl": "https://example.test/pay1/not-roc",
            "establishedHow": "owner lock: not ROC — 0%",
            "asOfDate": "2026-09-06"
        }),
    )
    .await;
    let research = query_json(
        &platform,
        "RocResearchGet",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-09-06"
        }),
    )
    .await;
    assert_eq!(research["rocPctMinor"].as_i64(), Some(0));
    assert_eq!(research["ownerOverride"], true);
    let obs = research["observations"].as_array().unwrap();
    assert!(
        obs.iter().any(|o| {
            o["ownerOverride"] == true
                && o["rocPctMinor"].as_i64() == Some(0)
                && o["sourceUrl"] == "https://example.test/pay1/not-roc"
        }),
        "owner 0% must keep the lock URL: {research}"
    );
}

#[tokio::test]
async fn owner_confirm_100_roc_keeps_cited_url() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "AMDW", "name": "AMDW"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Weekly",
            "divType": "DIV-1",
            "isActive": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "RocPlanConfirm",
        serde_json::json!({
            "securityId": security_id,
            "rocPctMinor": 10000,
            "rocScale": 2,
            "ownerOverride": true,
            "source": "19a-1",
            "sourceUrl": "https://www.roundhillinvestments.com/social-disclosures",
            "establishedHow": "owner: 100% ROC from Roundhill social-disclosures",
            "asOfDate": "2026-09-06"
        }),
    )
    .await;
    let research = query_json(
        &platform,
        "RocResearchGet",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-09-06"
        }),
    )
    .await;
    assert_eq!(research["rocPctMinor"].as_i64(), Some(10000));
    assert_eq!(research["complete"], true);
    let obs = research["observations"].as_array().unwrap();
    assert!(
        obs.iter().any(|o| {
            o["rocPctMinor"].as_i64() == Some(10000)
                && o["sourceUrl"] == "https://www.roundhillinvestments.com/social-disclosures"
        }),
        "100% must keep the cited URL: {research}"
    );
}

fn roc_19a1_candidate(pct: i64, url: &str) -> serde_json::Value {
    serde_json::json!({
        "rocPctMinor": pct,
        "scale": 2,
        "taxYear": "2026",
        "source": "19a-1",
        "sourceUrl": url,
        "method": "19a-1-current-year",
        "asOf": "2026-05-29",
        "kind": "estimate",
        "ownerOverride": false
    })
}

#[tokio::test]
async fn live_roc_change_tickets_without_overwriting_projection() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "QYLD", "name": "QYLD"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "divType": "DIV-1",
            "isActive": true,
            "rocPct2026EstimateMinor": 8000,
            "rocPct2025ActualMinor": 7200,
            "rocScale": 2,
            "needsRocResearch": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "RocResearchRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "QYLD",
            "asOfDate": "2026-09-14",
            "candidates": [roc_19a1_candidate(
                7500,
                "https://www.globalxetfs.com/filings-and-tax-supplements/QYLD"
            )]
        }),
    )
    .await;
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-09-14" }),
    )
    .await;
    assert_eq!(
        inv["rocPct2026EstimateMinor"].as_i64(),
        Some(8000),
        "live change must not overwrite the projection: {inv}"
    );
    let tickets = query_json(
        &platform,
        "WorkTicketList",
        serde_json::json!({ "securityId": security_id, "status": "open" }),
    )
    .await;
    let change = tickets["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["code"] == "roc_pct_change")
        .expect("roc_pct_change ticket");
    let reason = change["reason"].as_str().unwrap_or("");
    assert!(
        reason.contains("ROC % was 80.00 and now ROC % should be 75.00"),
        "{reason}"
    );
    assert!(
        reason.contains("Last year 1099 was 72.00 (informational)"),
        "{reason}"
    );
    assert_eq!(change["tool"], "roc_confirm");
}

#[tokio::test]
async fn accept_roc_change_updates_current_year_projection() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "QDVO", "name": "QDVO"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "divType": "DIV-1",
            "isActive": true,
            "rocPct2026EstimateMinor": 9800,
            "rocScale": 2,
            "needsRocResearch": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "RocResearchRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "QDVO",
            "asOfDate": "2026-09-14",
            "candidates": [roc_19a1_candidate(
                9000,
                "https://amplifyetfs.com/wp-content/uploads/files/19a-1_Notice_05-29-26_QDVO.pdf"
            )]
        }),
    )
    .await;
    let tickets = query_json(
        &platform,
        "WorkTicketList",
        serde_json::json!({ "securityId": security_id, "status": "open" }),
    )
    .await;
    let ticket_id = tickets["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["code"] == "roc_pct_change")
        .expect("ticket")["ticketId"]
        .as_str()
        .unwrap()
        .to_string();
    must_ok(
        &platform,
        "WorkTicketResolve",
        serde_json::json!({
            "ticketId": ticket_id,
            "tool": "roc_confirm",
            "action": "accept"
        }),
    )
    .await;
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-09-14" }),
    )
    .await;
    assert_eq!(inv["rocPct2026EstimateMinor"].as_i64(), Some(9000));
    assert_eq!(
        inv["needsRocResearch"].as_bool(),
        Some(false),
        "ticket Accept locks the plan % the same way Collectors Accept ROC does"
    );
    let after = query_json(
        &platform,
        "WorkTicketList",
        serde_json::json!({ "securityId": security_id, "status": "open" }),
    )
    .await;
    assert!(
        after["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["code"] != "roc_pct_change"),
        "{after}"
    );
}

#[tokio::test]
async fn reject_roc_change_keeps_previous_projection() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "SVOL", "name": "SVOL"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "divType": "DIV-1",
            "isActive": true,
            "rocPct2026EstimateMinor": 2500,
            "rocScale": 2,
            "needsRocResearch": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "RocResearchRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "SVOL",
            "asOfDate": "2026-09-14",
            "candidates": [roc_19a1_candidate(
                3000,
                "https://www.simplify.us/etfs/1721/supplemental-tax-information"
            )]
        }),
    )
    .await;
    let tickets = query_json(
        &platform,
        "WorkTicketList",
        serde_json::json!({ "securityId": security_id, "status": "open" }),
    )
    .await;
    let ticket_id = tickets["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["code"] == "roc_pct_change")
        .expect("ticket")["ticketId"]
        .as_str()
        .unwrap()
        .to_string();
    must_ok(
        &platform,
        "WorkTicketResolve",
        serde_json::json!({
            "ticketId": ticket_id,
            "tool": "roc_confirm",
            "action": "reject"
        }),
    )
    .await;
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-09-14" }),
    )
    .await;
    assert_eq!(inv["rocPct2026EstimateMinor"].as_i64(), Some(2500));
}
