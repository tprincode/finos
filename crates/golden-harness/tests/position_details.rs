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

fn qry(name: &str) -> QueryRequest {
    QueryRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        query_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: None,
    }
}

async fn must_ok(platform: &LocalPlatform, name: &str, body: serde_json::Value) -> serde_json::Value {
    let result = execute_command_on(platform, platform, cmd(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

async fn query_json(platform: &LocalPlatform, name: &str) -> serde_json::Value {
    let result = execute_query_on(platform, platform, qry(name)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

#[tokio::test]
async fn position_details_rollup_does_not_post_dividend_or_change_basis_totals() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let taxable = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Taxable Brokerage", "kind": "taxable"}),
    )
    .await;
    let account_id = taxable["accountId"].as_str().unwrap();
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "AAPL", "name": "Apple"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    research_template(&platform, security_id).await;

    must_ok(
        &platform,
        "DividendActualRecord",
        serde_json::json!({
            "accountId": account_id,
            "occurredOn": "2026-06-15",
            "amountMinor": 50_000,
            "scale": 2,
            "idempotencyKey": "pos-div-1"
        }),
    )
    .await;

    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "openedOn": "2026-01-05",
            "origin": "purchase",
            "quantityMinor": 10,
            "quantityScale": 0,
            "performanceBasisMinor": 100_000,
            "taxBasisMinor": 80_000,
            "scale": 2
        }),
    )
    .await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "openedOn": "2026-02-05",
            "origin": "purchase",
            "quantityMinor": 10,
            "quantityScale": 0,
            "performanceBasisMinor": 40_000,
            "taxBasisMinor": 40_000,
            "scale": 2
        }),
    )
    .await;

    let dividend_before = query_json(&platform, "DividendGet").await;
    let basis_before = query_json(&platform, "BasisGet").await;
    assert_eq!(dividend_before["actualTotalMinor"].as_i64().unwrap(), 50_000);
    assert_eq!(basis_before["openPerformanceMinor"].as_i64().unwrap(), 140_000);
    assert_eq!(basis_before["lots"].as_array().unwrap().len(), 2);

    let details = query_json(&platform, "PositionDetailsGet").await;
    let positions = details["positions"].as_array().unwrap();
    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0]["symbol"].as_str().unwrap(), "AAPL");
    assert_eq!(positions[0]["accountName"].as_str().unwrap(), "Taxable Brokerage");
    assert_eq!(positions[0]["remainingQuantityMinor"].as_i64().unwrap(), 20);
    assert_eq!(positions[0]["remainingPerformanceMinor"].as_i64().unwrap(), 140_000);
    assert_eq!(positions[0]["remainingTaxMinor"].as_i64().unwrap(), 120_000);
    assert_eq!(positions[0]["lotCount"].as_u64().unwrap(), 2);
    assert_eq!(details["openPerformanceMinor"].as_i64().unwrap(), 140_000);
    assert_eq!(details["openTaxMinor"].as_i64().unwrap(), 120_000);

    let dividend_after = query_json(&platform, "DividendGet").await;
    let basis_after = query_json(&platform, "BasisGet").await;
    assert_eq!(
        dividend_after["actualTotalMinor"],
        dividend_before["actualTotalMinor"]
    );
    assert_eq!(
        basis_after["openPerformanceMinor"],
        basis_before["openPerformanceMinor"]
    );
    assert_eq!(basis_after["lots"].as_array().unwrap().len(), 2);
    assert_eq!(details["symbolCount"].as_u64().unwrap(), 1);
    assert_eq!(details["accountCount"].as_u64().unwrap(), 1);
    assert_eq!(details["openLotCount"].as_u64().unwrap(), 2);
}

async fn seed_two_accounts(platform: &LocalPlatform) -> (String, String, String, String) {
    let income = must_ok(
        platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let car = must_ok(
        platform,
        "AccountRegister",
        serde_json::json!({"name": "Car", "kind": "taxable"}),
    )
    .await;
    let haky = must_ok(
        platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "HAKY", "name": "HAKY"}),
    )
    .await;
    let gof = must_ok(
        platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "GOF", "name": "GOF"}),
    )
    .await;
    (
        income["accountId"].as_str().unwrap().to_string(),
        car["accountId"].as_str().unwrap().to_string(),
        haky["securityId"].as_str().unwrap().to_string(),
        gof["securityId"].as_str().unwrap().to_string(),
    )
}

async fn research_template(platform: &LocalPlatform, security_id: &str) {
    must_ok(
        platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "priceSource": "public",
            "sourceSymbol": "RESEARCHED",
            "declarationSource": "issuer",
            "sourceUrl": "https://example.test/distributions",
            "calendarPolicy": "derived_walk",
            "collectorEnabled": true,
            "lookbackCount": 12
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot(platform, security_id, "RESEARCHED")
        .await
        .expect("complete collector");
}

async fn open_lot(
    platform: &LocalPlatform,
    account_id: &str,
    security_id: &str,
    qty: i64,
    cost: i64,
    tax: i64,
) {
    research_template(platform, security_id).await;
    must_ok(
        platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "openedOn": "2026-01-05",
            "origin": "purchase",
            "quantityMinor": qty,
            "quantityScale": 0,
            "performanceBasisMinor": cost,
            "taxBasisMinor": tax,
            "scale": 2
        }),
    )
    .await;
}

#[tokio::test]
async fn data_totals_stay_put_and_account_subtotals_sum() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let (income_id, car_id, haky_id, gof_id) = seed_two_accounts(&platform).await;
    open_lot(&platform, &income_id, &haky_id, 10, 100_000, 90_000).await;
    open_lot(&platform, &car_id, &gof_id, 5, 50_000, 40_000).await;

    let details = query_json(&platform, "PositionDetailsGet").await;
    assert_eq!(details["symbolCount"].as_u64().unwrap(), 2);
    assert_eq!(details["accountCount"].as_u64().unwrap(), 2);
    assert_eq!(details["openLotCount"].as_u64().unwrap(), 2);
    assert_eq!(details["openPerformanceMinor"].as_i64().unwrap(), 150_000);
    assert_eq!(details["openTaxMinor"].as_i64().unwrap(), 130_000);
    let accounts = details["accountTotals"].as_array().unwrap();
    assert_eq!(accounts.len(), 2);
    let acct_cost: i64 = accounts
        .iter()
        .map(|a| a["openPerformanceMinor"].as_i64().unwrap())
        .sum();
    let acct_tax: i64 = accounts
        .iter()
        .map(|a| a["openTaxMinor"].as_i64().unwrap())
        .sum();
    assert_eq!(acct_cost, details["openPerformanceMinor"].as_i64().unwrap());
    assert_eq!(acct_tax, details["openTaxMinor"].as_i64().unwrap());

    let _inv = execute_query_on(
        &platform,
        &platform,
        QueryRequest {
            contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
            query_name: "InvestmentGet".into(),
            correlation_id: Uuid::new_v4(),
            body_json: Some(serde_json::json!({"symbol":"HAKY","asOfDate":"2026-08-22"}).to_string()),
        },
    )
    .await;
    let after = query_json(&platform, "PositionDetailsGet").await;
    assert_eq!(after["openPerformanceMinor"], details["openPerformanceMinor"]);
    assert_eq!(after["openTaxMinor"], details["openTaxMinor"]);
    assert_eq!(after["symbolCount"], details["symbolCount"]);
    assert_eq!(after["accountCount"], details["accountCount"]);
}

#[tokio::test]
async fn wz10_no_dates_means_no_result_and_retrieve_does_not_post_price() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let (income_id, _car_id, haky_id, _gof_id) = seed_two_accounts(&platform).await;
    open_lot(&platform, &income_id, &haky_id, 10, 100_000, 90_000).await;

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

    let inv = execute_query_on(
        &platform,
        &platform,
        QueryRequest {
            contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
            query_name: "InvestmentGet".into(),
            correlation_id: Uuid::new_v4(),
            body_json: Some(serde_json::json!({"symbol":"HAKY","asOfDate":"2026-08-22"}).to_string()),
        },
    )
    .await;
    let body: serde_json::Value =
        serde_json::from_str(inv.body_json.as_deref().unwrap_or("{}")).unwrap();
    assert!(body["results"].as_array().unwrap().is_empty());
    assert!(body["periods"].as_array().unwrap().is_empty());

    let retrieve = must_ok(
        &platform,
        "PeriodSeriesRetrieve",
        serde_json::json!({
            "symbol": "HAKY",
            "startOn": "2026-01-02",
            "endOn": "2026-04-07",
            "candidates": [
                {"asOfAt": "2026-01-02", "priceMinor": 1000},
                {"asOfAt": "2026-04-07", "priceMinor": 800}
            ]
        }),
    )
    .await;
    assert_eq!(retrieve["posted"], false);
    let price = execute_query_on(
        &platform,
        &platform,
        QueryRequest {
            contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
            query_name: "CurrentPriceGet".into(),
            correlation_id: Uuid::new_v4(),
            body_json: Some(
                serde_json::json!({"securityId": haky_id, "asOfDate": "2026-08-22"}).to_string(),
            ),
        },
    )
    .await;
    let price_body: serde_json::Value =
        serde_json::from_str(price.body_json.as_deref().unwrap_or("{}")).unwrap();
    assert!(
        price_body["priceMinor"].is_null(),
        "PeriodSeriesRetrieve must not post CurrentPrice: {price_body}"
    );
}

#[tokio::test]
async fn tr_pd_25_calculate_does_not_write_tier_apply_does() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let (income_id, _car_id, haky_id, _gof_id) = seed_two_accounts(&platform).await;
    open_lot(&platform, &income_id, &haky_id, 10, 100_000, 90_000).await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": haky_id,
            "paymentFrequency": "Monthly",
            "riskTier": "Core",
            "provider": "Amplify"
        }),
    )
    .await;

    let first = must_ok(
        &platform,
        "BacktestPeriodRecord",
        serde_json::json!({
            "kind": "Bear",
            "name": "owner stress",
            "startOn": "2026-01-02",
            "endOn": "2026-04-07",
            "benchmarkSymbol": "SPY",
            "selectionReason": "owner stress window",
            "method": "close"
        }),
    )
    .await;
    let second = must_ok(
        &platform,
        "BacktestPeriodRecord",
        serde_json::json!({
            "kind": "Recovery",
            "name": "owner recovery",
            "startOn": "2026-04-08",
            "endOn": "2026-07-01",
            "benchmarkSymbol": "SPY",
            "selectionReason": "owner recovery window",
            "method": "close"
        }),
    )
    .await;
    assert_ne!(first["periodId"], second["periodId"]);

    let period_id = first["periodId"].as_str().unwrap();
    must_ok(
        &platform,
        "PositionBacktestCalculate",
        serde_json::json!({
            "securityId": haky_id,
            "periodId": period_id,
            "candidates": [
                {"asOfAt": "2026-01-02", "priceMinor": 10000},
                {"asOfAt": "2026-02-01", "priceMinor": 9000},
                {"asOfAt": "2026-04-07", "priceMinor": 8000}
            ],
            "benchmarkCandidates": [
                {"asOfAt": "2026-01-02", "priceMinor": 10000},
                {"asOfAt": "2026-04-07", "priceMinor": 8000}
            ],
            "distributionMinor": 500,
            "plannedIncomeMinor": 1000,
            "observedIncomeMinor": 900
        }),
    )
    .await;

    let after_calc = execute_query_on(
        &platform,
        &platform,
        QueryRequest {
            contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
            query_name: "InvestmentGet".into(),
            correlation_id: Uuid::new_v4(),
            body_json: Some(serde_json::json!({"symbol":"HAKY","asOfDate":"2026-08-22"}).to_string()),
        },
    )
    .await;
    let calc_body: serde_json::Value =
        serde_json::from_str(after_calc.body_json.as_deref().unwrap_or("{}")).unwrap();
    assert_eq!(calc_body["riskTier"], "Core");
    assert_eq!(calc_body["periods"].as_array().unwrap().len(), 2);
    assert_eq!(calc_body["results"].as_array().unwrap().len(), 1);
    assert_eq!(
        calc_body["results"][0]["cushionBps"].as_i64(),
        Some(500)
    );
    assert_eq!(
        calc_body["remainingPerformanceMinor"].as_i64().unwrap(),
        100_000,
        "regime score must not reduce original cost (PD-BR-11)"
    );

    let suggest = execute_query_on(
        &platform,
        &platform,
        QueryRequest {
            contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
            query_name: "ClassificationSuggestGet".into(),
            correlation_id: Uuid::new_v4(),
            body_json: Some(serde_json::json!({"securityId": haky_id}).to_string()),
        },
    )
    .await;
    assert!(suggest.ok, "{:?}", suggest.error_code);

    must_ok(
        &platform,
        "ClassificationApply",
        serde_json::json!({"securityId": haky_id, "riskTier": "Risk On"}),
    )
    .await;
    let after_apply = execute_query_on(
        &platform,
        &platform,
        QueryRequest {
            contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
            query_name: "InvestmentGet".into(),
            correlation_id: Uuid::new_v4(),
            body_json: Some(serde_json::json!({"symbol":"HAKY","asOfDate":"2026-08-22"}).to_string()),
        },
    )
    .await;
    let apply_body: serde_json::Value =
        serde_json::from_str(after_apply.body_json.as_deref().unwrap_or("{}")).unwrap();
    assert_eq!(apply_body["riskTier"], "Risk On");
}

fn qry_body(name: &str, body: serde_json::Value) -> QueryRequest {
    QueryRequest {
        contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
        query_name: name.to_string(),
        correlation_id: Uuid::new_v4(),
        body_json: Some(body.to_string()),
    }
}

async fn query_body(
    platform: &LocalPlatform,
    name: &str,
    body: serde_json::Value,
) -> serde_json::Value {
    let result = execute_query_on(platform, platform, qry_body(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

fn master_row<'a>(master: &'a serde_json::Value, symbol: &str) -> &'a serde_json::Value {
    master["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["symbol"] == symbol)
        .unwrap_or_else(|| panic!("{symbol} missing from PositionMasterGet"))
}

#[tokio::test]
async fn position_master_shows_underlying_without_selecting_a_symbol() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let (income_id, car_id, haky_id, gof_id) = seed_two_accounts(&platform).await;
    open_lot(&platform, &income_id, &haky_id, 10, 100_000, 90_000).await;
    open_lot(&platform, &car_id, &gof_id, 5, 50_000, 40_000).await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": haky_id,
            "paymentFrequency": "Weekly",
            "replaceCadence": true,
            "riskTier": "Core",
            "provider": "Amplify",
            "underlying": "HACK",
            "divType": "ROC",
            "notes": "covered call"
        }),
    )
    .await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": gof_id,
            "paymentFrequency": "Monthly",
            "riskTier": "Foundation",
            "provider": "Guggenheim",
            "underlying": "CEF book"
        }),
    )
    .await;

    let master = query_json(&platform, "PositionMasterGet").await;
    let haky = master_row(&master, "HAKY");
    let gof = master_row(&master, "GOF");
    assert_eq!(haky["underlying"], "HACK");
    assert_eq!(gof["underlying"], "CEF book");
    assert_eq!(haky["divType"], "ROC");
    assert_eq!(haky["remainingPerformanceMinor"].as_i64(), Some(100_000));
    assert_eq!(haky["unitCostMinor"].as_i64(), Some(10_000));
    assert!(
        haky["planFwdYieldBps"].is_null(),
        "FWD yield stays unknown without a valid last price"
    );
    assert!(
        haky["allocationBps"].is_null(),
        "allocation stays unknown without a valid last price"
    );
    assert_eq!(haky["periodDated"], false);
    assert!(haky["bearTotalReturnBps"].is_null());
    assert!(haky["bullTotalReturnBps"].is_null());
    assert!(
        haky["completeness"]
            .as_str()
            .unwrap()
            .contains("period:none"),
        "{}",
        haky["completeness"]
    );
}

#[tokio::test]
async fn position_master_settings_yields_inactive_and_dated_regime() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let (income_id, car_id, haky_id, gof_id) = seed_two_accounts(&platform).await;
    open_lot(&platform, &income_id, &haky_id, 10, 100_000, 90_000).await;
    open_lot(&platform, &car_id, &gof_id, 5, 50_000, 40_000).await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": haky_id,
            "paymentFrequency": "Weekly",
            "replaceCadence": true,
            "riskTier": "Core",
            "provider": "Amplify",
            "underlying": "HACK",
            "divType": "ROC",
            "needsRocResearch": true,
            "isActive": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": gof_id,
            "paymentFrequency": "Monthly",
            "riskTier": "Foundation",
            "provider": "Guggenheim",
            "underlying": "CEF book",
            "isActive": false
        }),
    )
    .await;
    must_ok(
        &platform,
        "ExpectedPaymentPatternUpsert",
        serde_json::json!({
            "securityId": haky_id,
            "declarationWeekday": "Wednesday",
            "exdateWeekday": "Thursday",
            "paydayWeekday": "Friday"
        }),
    )
    .await;
    must_ok(
        &platform,
        "PositionTaxProfileUpsert",
        serde_json::json!({
            "securityId": haky_id,
            "expectedHandling": "Ordinary"
        }),
    )
    .await;
    must_ok(
        &platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": haky_id,
            "amountPerShareMinor": 60,
            "amountScale": 2,
            "paymentPeriod": "2026-08-01",
            "source": "provider-site",
            "enteredAt": "2026-08-21"
        }),
    )
    .await;
    must_ok(
        &platform,
        "PlanHistoryConfirm",
        serde_json::json!({
            "securityId": haky_id,
            "amountPerShareMinor": 55,
            "amountScale": 2,
            "planningPeriodsPerYear": 52,
            "effectiveFrom": "2026-01-01",
            "decisionReason": "owner",
            "incompleteAnalysisReason": "Fewer than 6 observations"
        }),
    )
    .await;
    must_ok(
        &platform,
        "PriceQuoteRecord",
        serde_json::json!({
            "securityId": haky_id,
            "priceMinor": 10000,
            "scale": 2,
            "asOfAt": "2026-08-21",
            "source": "fixture"
        }),
    )
    .await;
    must_ok(
        &platform,
        "PriceQuoteRecord",
        serde_json::json!({
            "securityId": gof_id,
            "priceMinor": 2000,
            "scale": 2,
            "asOfAt": "2026-08-21",
            "source": "fixture"
        }),
    )
    .await;

    let set = query_json(&platform, "PriceRetrievalSetGet").await;
    let ids = set["securityIds"].as_array().cloned().unwrap_or_default();
    let id_text: Vec<String> = ids
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    assert!(id_text.contains(&haky_id), "active stays in last-price set");
    assert!(
        !id_text.contains(&gof_id),
        "inactive is excluded from last-price set"
    );

    let master = query_json(&platform, "PositionMasterGet").await;
    let haky = master_row(&master, "HAKY");
    let gof = master_row(&master, "GOF");
    assert_eq!(haky["isActive"], true);
    assert_eq!(gof["isActive"], false);
    assert_eq!(haky["needsRocResearch"], true);
    assert_eq!(haky["taxHandling"], "Ordinary");
    assert_eq!(haky["declarationWeekday"], "Wednesday");
    assert_eq!(haky["exdateWeekday"], "Thursday");
    assert_eq!(haky["paydayWeekday"], "Friday");
    assert_eq!(haky["planFwdYieldBps"].as_i64(), Some(2_860));
    assert_eq!(haky["mostCurrentFwdYieldBps"].as_i64(), Some(3_120));
    assert_eq!(haky["marketValueMinor"].as_i64(), Some(100_000));
    assert_eq!(gof["marketValueMinor"].as_i64(), Some(10_000));
    assert_eq!(haky["allocationBps"].as_i64(), Some(9_090));
    assert_eq!(gof["allocationBps"].as_i64(), Some(909));
    assert!(
        haky["completeness"].as_str().unwrap().contains("decl:1"),
        "{}",
        haky["completeness"]
    );

    let remaining = query_body(
        &platform,
        "RemainingYearIncomeGet",
        serde_json::json!({
            "securityId": haky_id,
            "asOfDate": "2026-08-22"
        }),
    )
    .await;
    assert!(remaining.get("known").is_some());

    let first = must_ok(
        &platform,
        "BacktestPeriodRecord",
        serde_json::json!({
            "kind": "Bear",
            "name": "owner stress",
            "startOn": "2026-01-02",
            "endOn": "2026-04-07",
            "benchmarkSymbol": "SPY",
            "selectionReason": "owner stress window",
            "method": "close"
        }),
    )
    .await;
    must_ok(
        &platform,
        "PositionBacktestCalculate",
        serde_json::json!({
            "securityId": haky_id,
            "periodId": first["periodId"],
            "candidates": [
                {"asOfAt": "2026-01-02", "priceMinor": 10000},
                {"asOfAt": "2026-04-07", "priceMinor": 8000}
            ],
            "benchmarkCandidates": [
                {"asOfAt": "2026-01-02", "priceMinor": 10000},
                {"asOfAt": "2026-04-07", "priceMinor": 8000}
            ],
            "distributionMinor": 500,
            "plannedIncomeMinor": 1000,
            "observedIncomeMinor": 900
        }),
    )
    .await;
    let after = query_json(&platform, "PositionMasterGet").await;
    let haky_after = master_row(&after, "HAKY");
    let gof_after = master_row(&after, "GOF");
    assert_eq!(haky_after["periodDated"], true);
    assert!(
        haky_after["bearPriceReturnBps"].as_i64().is_some()
            || haky_after["bearTotalReturnBps"].as_i64().is_some(),
        "dated Bear result missing on HAKY: {haky_after}"
    );
    assert!(
        gof_after["bearTotalReturnBps"].is_null() && gof_after["bearPriceReturnBps"].is_null(),
        "GOF has no dated result — never invent Average(D:K): {gof_after}"
    );

    let inv = query_body(
        &platform,
        "InvestmentGet",
        serde_json::json!({"symbol": "HAKY", "asOfDate": "2026-08-22"}),
    )
    .await;
    assert_eq!(inv["taxHandling"], "Ordinary");
    assert_eq!(inv["declarationWeekday"], "Wednesday");
    assert_eq!(inv["needsRocResearch"], true);
    assert_eq!(inv["isActive"], true);
    assert!(inv["results"][0]["bearRelativeBps"].is_number() || inv["results"][0]["bearRelativeBps"].is_null());
    assert!(inv["results"][0]["downsideCaptureBps"].is_number() || inv["results"][0]["downsideCaptureBps"].is_null());
}

#[tokio::test]
async fn energyx_offering_quote_sets_market_value() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let account = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "ENERGYX", "kind": "taxable"}),
    )
    .await;
    let account_id = account["accountId"].as_str().unwrap();
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({
            "symbol": "ENERGYX",
            "name": "Energy Exploration Technologies"
        }),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap().to_string();
    open_lot(&platform, account_id, &security_id, 200, 65_100, 65_100).await;
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "priceSource": "edgar",
            "sourceSymbol": "1830166",
            "declarationSource": "sec-edgar",
            "lookbackCount": 1,
            "paymentSource": "import"
        }),
    )
    .await;

    let before = query_json(&platform, "PositionMasterGet").await;
    let energyx_before = master_row(&before, "ENERGYX");
    assert!(
        energyx_before["marketValueMinor"].is_null(),
        "unknown last price stays unknown, not $0: {energyx_before}"
    );

    let set = query_json(&platform, "PriceRetrievalSetGet").await;
    let item = set["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["symbol"] == "ENERGYX")
        .expect("ENERGYX in last-price set");
    assert_eq!(item["priceSource"], "edgar");
    assert_eq!(item["sourceSymbol"], "1830166");

    let refresh = must_ok(
        &platform,
        "LastPriceRefresh",
        serde_json::json!({
            "quotes": [{
                "securityId": security_id,
                "priceMinor": 1300,
                "scale": 2,
                "asOfAt": "2026-07-13",
                "source": "sec-edgar"
            }]
        }),
    )
    .await;
    assert_eq!(refresh["recorded"].as_u64(), Some(1));

    let master = query_json(&platform, "PositionMasterGet").await;
    let energyx = master_row(&master, "ENERGYX");
    assert_eq!(energyx["lastPriceMinor"].as_i64(), Some(1300));
    assert_eq!(
        energyx["marketValueMinor"].as_i64(),
        Some(260_000),
        "200 shares × $13.00"
    );

    let details = query_json(&platform, "PositionDetailsGet").await;
    let line = details["positions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["symbol"] == "ENERGYX")
        .expect("ENERGYX line");
    assert_eq!(line["marketValueMinor"].as_i64(), Some(260_000));
}

#[tokio::test]
async fn ac_pd_04_as_of_qty_reconciles_to_open_lots() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let (income_id, car_id, haky_id, _) = seed_two_accounts(&platform).await;
    open_lot(&platform, &income_id, &haky_id, 10, 100_000, 90_000).await;
    open_lot(&platform, &car_id, &haky_id, 5, 50_000, 45_000).await;
    let details = query_json(&platform, "PositionDetailsGet").await;
    assert_eq!(details["qtyReconcileOk"], true);
    let lines = details["positions"].as_array().unwrap();
    let haky_qty: i64 = lines
        .iter()
        .filter(|l| l["symbol"] == "HAKY")
        .map(|l| l["remainingQuantityMinor"].as_i64().unwrap())
        .sum();
    assert_eq!(haky_qty, 15);
    let exceptions = query_json(&platform, "ExceptionList").await;
    let arr = exceptions.as_array().cloned().unwrap_or_default();
    assert!(
        !arr.iter()
            .any(|e| e["code"] == "qty_reconcile_mismatch"),
        "{exceptions}"
    );
}

#[tokio::test]
async fn ac_pd_06_plan_yoc_uses_52_12_4() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let weekly = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "WKLY", "name": "Weekly"}),
    )
    .await;
    let monthly = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "MNTH", "name": "Monthly"}),
    )
    .await;
    let quarterly = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "QRTL", "name": "Quarterly"}),
    )
    .await;
    let acct = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    let acct_id = acct["accountId"].as_str().unwrap();
    for (sec, periods) in [
        (weekly["securityId"].as_str().unwrap(), 52u8),
        (monthly["securityId"].as_str().unwrap(), 12u8),
        (quarterly["securityId"].as_str().unwrap(), 4u8),
    ] {
        open_lot(&platform, acct_id, sec, 10, 100_000, 100_000).await;
        let freq = match periods {
            12 => "Monthly",
            4 => "Quarterly",
            _ => "Weekly",
        };
        must_ok(
            &platform,
            "PositionCharacteristicUpsert",
            serde_json::json!({
                "securityId": sec,
                "paymentFrequency": freq,
                "replaceCadence": true,
                "riskTier": "Core"
            }),
        )
        .await;
        must_ok(
            &platform,
            "IssuerDeclarationRecord",
            serde_json::json!({
                "securityId": sec,
                "amountPerShareMinor": 100,
                "amountScale": 2,
                "paymentPeriod": "2026-08-01",
                "source": "fixture",
                "enteredAt": "2026-08-21"
            }),
        )
        .await;
        must_ok(
            &platform,
            "PlanHistoryConfirm",
            serde_json::json!({
                "securityId": sec,
                "amountPerShareMinor": 100,
                "amountScale": 2,
                "planningPeriodsPerYear": periods,
                "effectiveFrom": "2026-01-01",
                "decisionReason": "owner",
                "incompleteAnalysisReason": "Fewer than 6 observations"
            }),
        )
        .await;
    }
    let master = query_json(&platform, "PositionMasterGet").await;
    let wk = master_row(&master, "WKLY");
    let mn = master_row(&master, "MNTH");
    let qt = master_row(&master, "QRTL");
    assert_eq!(wk["planYocBps"].as_i64(), Some(5_200));
    assert_eq!(mn["planYocBps"].as_i64(), Some(1_200));
    assert_eq!(qt["planYocBps"].as_i64(), Some(400));
}

#[tokio::test]
async fn ac_pd_07_car_mv_from_car_lots() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let (income_id, car_id, haky_id, _) = seed_two_accounts(&platform).await;
    open_lot(&platform, &income_id, &haky_id, 10, 100_000, 90_000).await;
    open_lot(&platform, &car_id, &haky_id, 5, 50_000, 40_000).await;
    must_ok(
        &platform,
        "PriceQuoteRecord",
        serde_json::json!({
            "securityId": haky_id,
            "priceMinor": 2000,
            "scale": 2,
            "asOfAt": "2026-08-21",
            "source": "fixture"
        }),
    )
    .await;
    let master = query_json(&platform, "PositionMasterGet").await;
    let haky = master_row(&master, "HAKY");
    assert_eq!(haky["marketValueMinor"].as_i64(), Some(30_000));
    assert_eq!(
        haky["carMarketValueMinor"].as_i64(),
        Some(10_000),
        "Car qty 5 × $20, never unit-cost × price"
    );
    assert_eq!(haky["carShareOfSymbolBps"].as_i64(), Some(3_333));
    assert_eq!(haky["carShareOfDataBps"].as_i64(), Some(3_333));
}

#[tokio::test]
async fn ac_pd_08_backtest_requires_dates_method_source() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let incomplete = execute_command_on(
        &platform,
        &platform,
        cmd(
            "BacktestPeriodRecord",
            serde_json::json!({
                "kind": "Bear",
                "name": "blank",
                "startOn": "",
                "endOn": "",
                "method": "",
                "selectionReason": ""
            }),
        ),
    )
    .await;
    assert!(!incomplete.ok);
    assert_eq!(incomplete.error_code.as_deref(), Some("regime_period_incomplete"));
    let dated = must_ok(
        &platform,
        "BacktestPeriodRecord",
        serde_json::json!({
            "kind": "Bear",
            "name": "owner stress",
            "startOn": "2026-01-02",
            "endOn": "2026-04-07",
            "benchmarkSymbol": "SPY"
        }),
    )
    .await;
    assert!(dated.get("periodId").is_some());
    assert_eq!(dated["method"].as_str(), Some("adjusted"));
    assert_eq!(
        dated["selectionReason"].as_str(),
        Some("dates named as Bear period")
    );
}

#[tokio::test]
async fn ac_pd_11_missing_evidence_lowers_confidence() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let (income_id, _, haky_id, gof_id) = seed_two_accounts(&platform).await;
    open_lot(&platform, &income_id, &haky_id, 10, 100_000, 90_000).await;
    open_lot(&platform, &income_id, &gof_id, 5, 50_000, 40_000).await;
    let period = must_ok(
        &platform,
        "BacktestPeriodRecord",
        serde_json::json!({
            "kind": "Bear",
            "name": "owner stress",
            "startOn": "2026-01-02",
            "endOn": "2026-04-07",
            "benchmarkSymbol": "SPY",
            "selectionReason": "owner stress window",
            "method": "close"
        }),
    )
    .await;
    must_ok(
        &platform,
        "PositionBacktestCalculate",
        serde_json::json!({
            "securityId": haky_id,
            "periodId": period["periodId"],
            "candidates": [
                {"asOfAt": "2026-01-02", "priceMinor": 10000},
                {"asOfAt": "2026-02-01", "priceMinor": 9000},
                {"asOfAt": "2026-04-07", "priceMinor": 8000}
            ],
            "benchmarkCandidates": [
                {"asOfAt": "2026-01-02", "priceMinor": 10000},
                {"asOfAt": "2026-04-07", "priceMinor": 8000}
            ],
            "distributionMinor": 500,
            "plannedIncomeMinor": 1000,
            "observedIncomeMinor": 900
        }),
    )
    .await;
    must_ok(
        &platform,
        "PositionBacktestCalculate",
        serde_json::json!({
            "securityId": gof_id,
            "periodId": period["periodId"],
            "candidates": [{"asOfAt": "2026-01-02", "priceMinor": 10000}]
        }),
    )
    .await;
    let master = query_json(&platform, "PositionMasterGet").await;
    let haky = master_row(&master, "HAKY");
    let gof = master_row(&master, "GOF");
    let haky_conf = haky["evidence"]["dataConfidence"].as_i64().unwrap();
    let gof_conf = gof["evidence"]["dataConfidence"].as_i64().unwrap();
    assert!(gof_conf < haky_conf, "incomplete {gof_conf} vs complete {haky_conf}");
    assert!(gof["evidence"]["incomeReliability"].is_null());
}

#[tokio::test]
async fn ac_pd_12_tier_suggest_does_not_apply() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let (income_id, _, haky_id, _) = seed_two_accounts(&platform).await;
    open_lot(&platform, &income_id, &haky_id, 10, 100_000, 90_000).await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": haky_id,
            "paymentFrequency": "Weekly",
            "replaceCadence": true,
            "riskTier": "Core",
            "provider": "Amplify",
            "isActive": true
        }),
    )
    .await;
    let period = must_ok(
        &platform,
        "BacktestPeriodRecord",
        serde_json::json!({
            "kind": "Bear",
            "name": "owner stress",
            "startOn": "2026-01-02",
            "endOn": "2026-04-07",
            "benchmarkSymbol": "SPY",
            "selectionReason": "owner stress window",
            "method": "close"
        }),
    )
    .await;
    must_ok(
        &platform,
        "PositionBacktestCalculate",
        serde_json::json!({
            "securityId": haky_id,
            "periodId": period["periodId"],
            "candidates": [
                {"asOfAt": "2026-01-02", "priceMinor": 10000},
                {"asOfAt": "2026-02-01", "priceMinor": 9000},
                {"asOfAt": "2026-04-07", "priceMinor": 8000}
            ],
            "benchmarkCandidates": [
                {"asOfAt": "2026-01-02", "priceMinor": 10000},
                {"asOfAt": "2026-04-07", "priceMinor": 8000}
            ],
            "distributionMinor": 500,
            "plannedIncomeMinor": 1000,
            "observedIncomeMinor": 300
        }),
    )
    .await;
    let suggest = query_body(
        &platform,
        "ClassificationSuggestGet",
        serde_json::json!({"securityId": haky_id}),
    )
    .await;
    assert!(!suggest["suggestedTier"].as_str().unwrap_or("").is_empty());
    let inv = query_body(
        &platform,
        "InvestmentGet",
        serde_json::json!({"symbol": "HAKY"}),
    )
    .await;
    assert_eq!(inv["riskTier"], "Core");
    must_ok(
        &platform,
        "ClassificationApply",
        serde_json::json!({"securityId": haky_id, "riskTier": "Risk On"}),
    )
    .await;
    let after = query_body(
        &platform,
        "InvestmentGet",
        serde_json::json!({"symbol": "HAKY"}),
    )
    .await;
    assert_eq!(after["riskTier"], "Risk On");
}

#[tokio::test]
async fn ac_pd_16_roc_does_not_change_original_cost() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let (income_id, _, haky_id, _) = seed_two_accounts(&platform).await;
    open_lot(&platform, &income_id, &haky_id, 10, 100_000, 90_000).await;
    let before = query_json(&platform, "PositionMasterGet").await;
    let cost = master_row(&before, "HAKY")["remainingPerformanceMinor"]
        .as_i64()
        .unwrap();
    must_ok(
        &platform,
        "RocPlanConfirm",
        serde_json::json!({
            "securityId": haky_id,
            "rocPctMinor": 8000,
            "rocScale": 2,
            "source": "19a-1"
        }),
    )
    .await;
    let after = query_json(&platform, "PositionMasterGet").await;
    assert_eq!(
        master_row(&after, "HAKY")["remainingPerformanceMinor"].as_i64(),
        Some(cost)
    );
}

#[tokio::test]
async fn ac_pd_20_lifetime_distributions_use_original_cost() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let (income_id, _, haky_id, gof_id) = seed_two_accounts(&platform).await;
    open_lot(&platform, &income_id, &haky_id, 10, 100_000, 90_000).await;
    open_lot(&platform, &income_id, &gof_id, 5, 50_000, 40_000).await;
    let posted = must_ok(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": income_id,
            "securityId": haky_id,
            "activityType": "dividend",
            "amountMinor": 10_000,
            "scale": 2,
            "occurredOn": "2026-06-15",
            "idempotencyKey": "pd-div-1"
        }),
    )
    .await;
    must_ok(
        &platform,
        "DistributionCharacterize",
        serde_json::json!({
            "activityId": posted["activityId"],
            "category": "roc",
            "amountMinor": 3_000,
            "scale": 2
        }),
    )
    .await;
    let master = query_json(&platform, "PositionMasterGet").await;
    let haky = master_row(&master, "HAKY");
    let gof = master_row(&master, "GOF");
    assert_eq!(haky["distributionsScope"], "complete");
    assert_eq!(haky["totalDistributionsReceivedMinor"].as_i64(), Some(10_000));
    assert_eq!(haky["rocDistributionsMinor"].as_i64(), Some(3_000));
    assert_eq!(haky["costRecoveryBps"].as_i64(), Some(1_000));
    assert_eq!(gof["distributionsScope"], "incomplete");
    assert!(gof["totalDistributionsReceivedMinor"].is_null());
    assert!(gof["costRecoveryBps"].is_null());
    let inv = query_body(
        &platform,
        "InvestmentGet",
        serde_json::json!({"symbol": "HAKY"}),
    )
    .await;
    assert_eq!(inv["remainingPerformanceMinor"].as_i64(), Some(100_000));
    assert_eq!(inv["costRecoveryBps"].as_i64(), Some(1_000));
}

/// Position hub shows parseable characteristics. Suggested tier is not auto-applied.
#[tokio::test]
async fn pay1_characteristics_are_visible_risk_not_auto_applied() {
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
                "themeStrategy": "Covered Call",
                "primaryRiskDriver": "Look-through UNDR",
                "coveredCall": true,
                "leveraged": false,
                "riskTierSuggestion": "Risk On"
            },
            "skipRefresh": true
        }),
    )
    .await;
    let security_id = seed["securityId"].as_str().unwrap();
    let inv = query_body(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-09-05" }),
    )
    .await;
    assert_eq!(inv["symbol"], "PAY1");
    assert_eq!(inv["divType"], "DIV-1");
    assert_eq!(inv["underlying"], "UNDR");
    assert_eq!(inv["lookthrough"]["coveredCall"], true);
    assert_eq!(inv["lookthrough"]["themeStrategy"], "Covered Call");
    assert_eq!(inv["lookthrough"]["riskTierSuggestion"], "Risk On");
    assert_eq!(inv["riskTier"], "");
}

#[test]
fn position_information_table_edits_owner_facts_in_row() {
    let ui = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/App.tsx"),
    )
    .unwrap();
    let identity = ui
        .split("id=\"hub-identity\"")
        .nth(1)
        .expect("hub-identity");
    let identity = identity
        .split("id=\"hub-calculator\"")
        .next()
        .expect("hub-calculator after identity");
    assert!(
        identity.contains("aria-label=\"Position risk\""),
        "Risk must edit in the Position information table, not only a collapsed details block"
    );
    assert!(identity.contains("aria-label=\"Position frequency\""));
    assert!(identity.contains("aria-label=\"Position name\""));
    assert!(identity.contains("aria-label=\"Position provider\""));
    assert!(identity.contains("aria-label=\"Position underlying\""));
    assert!(identity.contains("RISK_TIERS"));
    assert!(
        !identity.contains("Undecided"),
        "Undecided is not a permitted owner risk"
    );
}

