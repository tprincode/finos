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
async fn calculator_plan_sets_income_plan_week_not_actuals() {
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
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "AMDW", "name": "AMD"}),
    )
    .await;
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security["securityId"],
            "priceSource": "public",
            "sourceSymbol": "AMDW",
            "declarationSource": "issuer",
            "sourceUrl": "https://example.test/amdw/distributions",
            "calendarPolicy": "derived_walk",
            "collectorEnabled": true,
            "lookbackCount": 12
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot(
        &platform,
        security["securityId"].as_str().unwrap(),
        "AMDW",
    )
    .await
    .expect("complete collector");
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": income["accountId"],
            "securityId": security["securityId"],
            "openedOn": "2026-01-15",
            "origin": "purchase",
            "quantityMinor": 69,
            "quantityScale": 0,
            "performanceBasisMinor": 401200,
            "taxBasisMinor": 401200,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "ProductionSeedLoad",
        serde_json::json!({
            "accounts": [],
            "securities": [],
            "lots": [],
            "yieldBatches": [],
            "disbursements": [],
            "plans": [{
                "symbol": "AMDW",
                "amountPerShareMinor": 55,
                "amountScale": 2,
                "planningPeriodsPerYear": 52,
                "effectiveFrom": "2026-08-12",
                "decisionReason": "test"
            }],
            "characteristics": [{
                "symbol": "AMDW",
                "paymentFrequency": "Weekly",
                "riskTier": "HighRisk",
                "provider": "Roundhill",
                "underlying": "AMD",
                "divType": "DIV-1",
                "needsRocResearch": false,
                "notes": ""
            }]
        }),
    )
    .await;

    let calc = query_json(&platform, "CalculatorGet", serde_json::json!({})).await;
    assert_eq!(calc["planCount"].as_u64().unwrap(), 1);
    let row = calc["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["symbol"] == "AMDW")
        .unwrap();
    assert_eq!(row["planKnown"], true);
    assert_eq!(row["planPaymentMinor"].as_i64().unwrap(), 3_795);

    let week = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({"asOfDate": "2026-08-25"}),
    )
    .await;
    let income_line = week["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["accountName"] == "Income")
        .unwrap();
    assert_eq!(income_line["planKnown"], true);
    assert_eq!(income_line["plannedMinor"].as_i64().unwrap(), 3_795);
    assert_eq!(income_line["actualMinor"].as_i64().unwrap(), 0);

    must_ok(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": income["accountId"],
            "securityId": security["securityId"],
            "activityType": "dividend",
            "amountMinor": 100,
            "scale": 2,
            "occurredOn": "2026-08-25",
            "idempotencyKey": "cash-is-not-plan"
        }),
    )
    .await;
    let after = query_json(
        &platform,
        "IncomePlanWeekGet",
        serde_json::json!({"asOfDate": "2026-08-25"}),
    )
    .await;
    let after_line = after["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["accountName"] == "Income")
        .unwrap();
    assert_eq!(after_line["actualMinor"].as_i64().unwrap(), 100);
    assert_eq!(after_line["plannedMinor"].as_i64().unwrap(), 3_795);
}

async fn open_named(
    platform: &LocalPlatform,
    symbol: &str,
    freq: &str,
) -> (String, String) {
    let income = must_ok(
        platform,
        "AccountRegister",
        serde_json::json!({"name": format!("Income-{symbol}"), "kind": "taxable"}),
    )
    .await;
    let security = must_ok(
        platform,
        "SecurityRegister",
        serde_json::json!({"symbol": symbol, "name": symbol}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap().to_string();
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
    golden_harness::complete_collector_for_first_lot(platform, &security_id, symbol)
        .await
        .expect("complete collector");
    must_ok(
        platform,
        "LotOpen",
        serde_json::json!({
            "accountId": income["accountId"],
            "securityId": security_id,
            "openedOn": "2026-01-15",
            "origin": "purchase",
            "quantityMinor": 10,
            "quantityScale": 0,
            "performanceBasisMinor": 1000,
            "taxBasisMinor": 1000,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;
    must_ok(
        platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": freq,
            "replaceCadence": true,
            "riskTier": "Risk On",
            "isActive": true
        }),
    )
    .await;
    (security_id, income["accountId"].as_str().unwrap().to_string())
}

#[tokio::test]
async fn imported_paid_declaration_survives_retrieve_with_different_amount() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let (security_id, _) = open_named(&platform, "AMDW", "Weekly").await;
    must_ok(
        &platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 57296,
            "amountScale": 5,
            "paymentPeriod": "2026-09-04",
            "source": "import",
            "enteredAt": "2026-08-12"
        }),
    )
    .await;
    let _retrieve = execute_command_on(
        &platform,
        &platform,
        cmd(
            "CollectorRetrieve",
            serde_json::json!({
                "securityId": security_id,
                "symbol": "AMDW",
                "declarationSource": "roundhill",
                "candidates": [{
                    "amountPerShareMinor": 1,
                    "amountScale": 2,
                    "paymentPeriod": "2026-09-04",
                    "source": "roundhill"
                }]
            }),
        ),
    )
    .await;
    must_ok(
        &platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 9,
            "amountScale": 2,
            "paymentPeriod": "2026-09-04",
            "source": "roundhill",
            "enteredAt": "2026-09-02"
        }),
    )
    .await;
    let inv = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-09-02" }),
    )
    .await;
    let decls = inv["declarations"].as_array().expect("declarations");
    let cell = decls
        .iter()
        .find(|d| d["paymentPeriod"] == "2026-09-04")
        .expect("period");
    assert_eq!(cell["amountPerShareMinor"].as_i64(), Some(57296));
    assert_eq!(cell["amountScale"].as_u64(), Some(5));
}

#[tokio::test]
async fn imported_cadence_survives_research_upsert_without_replace() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let (security_id, _) = open_named(&platform, "TRIN", "Quarterly").await;
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
    let kept = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-09-02" }),
    )
    .await;
    assert_eq!(kept["paymentFrequency"], "Quarterly");
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
    let replaced = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-09-02" }),
    )
    .await;
    assert_eq!(replaced["paymentFrequency"], "Monthly");
}

#[tokio::test]
async fn declaration_history_grid_filters_by_cadence_and_friday_columns() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let (amdw_id, _) = open_named(&platform, "AMDW", "Weekly").await;
    let (epd_id, _) = open_named(&platform, "EPD", "Quarterly").await;
    let (hold_id, _) = open_named(&platform, "HOLD", "None").await;
    for (period, minor) in [
        ("2026-09-04", 57296i64),
        ("2026-08-28", 132617),
        ("2026-08-21", 109968),
        ("2026-08-14", 79676),
        ("2026-08-07", 164781),
    ] {
        must_ok(
            &platform,
            "IssuerDeclarationRecord",
            serde_json::json!({
                "securityId": amdw_id,
                "amountPerShareMinor": minor,
                "amountScale": 5,
                "paymentPeriod": period,
                "source": "import",
                "enteredAt": "2026-09-02"
            }),
        )
        .await;
    }
    must_ok(
        &platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": epd_id,
            "amountPerShareMinor": 5500,
            "amountScale": 4,
            "paymentPeriod": "2026-08-31",
            "source": "import",
            "enteredAt": "2026-09-02"
        }),
    )
    .await;
    must_ok(
        &platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": hold_id,
            "amountPerShareMinor": 1,
            "amountScale": 2,
            "paymentPeriod": "2026-09-04",
            "source": "import",
            "enteredAt": "2026-09-02"
        }),
    )
    .await;

    let calc = query_json(&platform, "CalculatorGet", serde_json::json!({})).await;
    let calc_rows = calc["rows"].as_array().unwrap();
    assert!(calc_rows.iter().any(|r| r["symbol"] == "AMDW"));
    assert!(calc_rows.iter().all(|r| r["symbol"] != "HOLD"));

    let weekly = query_json(
        &platform,
        "DeclarationHistoryGet",
        serde_json::json!({
            "asOfDate": "2026-09-02",
            "cadence": "weekly",
            "startOn": "2026-07-04",
            "endOn": "2026-09-02"
        }),
    )
    .await;
    assert_eq!(weekly["startOn"], "2026-07-04");
    assert_eq!(weekly["endOn"], "2026-09-02");
    let week_ends = weekly["weekEnds"].as_array().unwrap();
    assert_eq!(week_ends[0], "2026-09-04");
    assert_eq!(week_ends[1], "2026-08-28");
    assert_eq!(week_ends[4], "2026-08-07");
    assert_eq!(week_ends.last().unwrap(), "2026-07-10");
    assert_eq!(weekly["weekStarts"][0], "2026-08-29");
    assert_eq!(weekly["weekNumbers"][0], 35);
    assert_eq!(weekly["weekYears"][0], 2026);
    let weekly_rows = weekly["rows"].as_array().unwrap();
    assert!(weekly_rows.iter().any(|r| r["symbol"] == "AMDW"));
    assert!(weekly_rows.iter().all(|r| r["symbol"] != "EPD"));
    assert!(weekly_rows.iter().all(|r| r["symbol"] != "HOLD"));
    let amdw = weekly_rows
        .iter()
        .find(|r| r["symbol"] == "AMDW")
        .unwrap();
    assert_eq!(amdw["cells"][0]["amountPerShareMinor"].as_i64(), Some(57296));
    assert_eq!(amdw["cells"][0]["amountScale"].as_u64(), Some(5));
    assert_eq!(amdw["cells"][1]["amountPerShareMinor"].as_i64(), Some(132617));
    assert_eq!(amdw["cells"][2]["amountPerShareMinor"].as_i64(), Some(109968));
    assert_eq!(amdw["cells"][3]["amountPerShareMinor"].as_i64(), Some(79676));
    assert_eq!(amdw["cells"][4]["amountPerShareMinor"].as_i64(), Some(164781));

    let default_window = query_json(
        &platform,
        "DeclarationHistoryGet",
        serde_json::json!({
            "asOfDate": "2026-09-02",
            "cadence": "weekly"
        }),
    )
    .await;
    assert_eq!(default_window["startOn"], "2026-07-04");
    assert_eq!(default_window["weekEnds"][0], "2026-09-04");
    assert_eq!(default_window["weekEnds"][4], "2026-08-07");

    let all = query_json(
        &platform,
        "DeclarationHistoryGet",
        serde_json::json!({
            "asOfDate": "2026-09-02",
            "cadence": "all",
            "startOn": "2026-07-04",
            "endOn": "2026-09-02"
        }),
    )
    .await;
    let all_rows = all["rows"].as_array().unwrap();
    assert!(all_rows.iter().any(|r| r["symbol"] == "AMDW"));
    assert!(all_rows.iter().all(|r| r["symbol"] != "HOLD"));
    let epd = all_rows.iter().find(|r| r["symbol"] == "EPD").unwrap();
    assert_eq!(epd["cells"][0]["amountPerShareMinor"].as_i64(), Some(5500));
}

#[tokio::test]
async fn calculator_lists_div1_and_cash_only_keeps_removed_plans() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    open_named(&platform, "AMDW", "Weekly").await;
    let (soxl_id, _) = open_named(&platform, "SOXL", "Weekly").await;
    let (btc_id, _) = open_named(&platform, "BTC-USD", "Weekly").await;
    must_ok(
        &platform,
        "ProductionSeedLoad",
        serde_json::json!({
            "accounts": [],
            "securities": [],
            "lots": [],
            "yieldBatches": [],
            "disbursements": [],
            "plans": [
                {
                    "symbol": "AMDW",
                    "amountPerShareMinor": 55,
                    "amountScale": 2,
                    "planningPeriodsPerYear": 52,
                    "effectiveFrom": "2026-08-12",
                    "decisionReason": "test"
                },
                {
                    "symbol": "SOXL",
                    "amountPerShareMinor": 10,
                    "amountScale": 2,
                    "planningPeriodsPerYear": 12,
                    "effectiveFrom": "2026-08-12",
                    "decisionReason": "removed-from-collectors"
                }
            ],
            "characteristics": []
        }),
    )
    .await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": soxl_id,
            "paymentFrequency": "None",
            "replaceCadence": true,
            "divType": "",
            "riskTier": "Risk On",
            "isActive": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": btc_id,
            "divType": "",
            "riskTier": "Risk On",
            "isActive": true
        }),
    )
    .await;

    let calc = query_json(&platform, "CalculatorGet", serde_json::json!({})).await;
    let calc_rows = calc["rows"].as_array().unwrap();
    assert!(calc_rows.iter().any(|r| r["symbol"] == "AMDW"));
    assert!(
        calc_rows.iter().all(|r| r["symbol"] != "SOXL"),
        "removed collector stays off Calculator: {calc}"
    );
    assert!(
        calc_rows.iter().all(|r| r["symbol"] != "BTC-USD"),
        "non DIV-1 stays off Calculator: {calc}"
    );
    assert_eq!(calc["planCount"].as_u64().unwrap(), 1);

    let soxl = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": soxl_id, "asOfDate": "2026-09-02" }),
    )
    .await;
    assert_eq!(soxl["planPerShareMinor"].as_i64(), Some(10));
    assert_eq!(soxl["paymentFrequency"], "None");

    let summary = query_json(&platform, "DataSummaryGet", serde_json::json!({})).await;
    assert_eq!(summary["planCount"].as_u64().unwrap(), 1);

    let history = query_json(
        &platform,
        "DeclarationHistoryGet",
        serde_json::json!({
            "asOfDate": "2026-09-02",
            "cadence": "all"
        }),
    )
    .await;
    let hist_rows = history["rows"].as_array().unwrap();
    assert!(hist_rows.iter().any(|r| r["symbol"] == "AMDW"));
    assert!(hist_rows.iter().all(|r| r["symbol"] != "SOXL"));
    assert!(hist_rows.iter().all(|r| r["symbol"] != "BTC-USD"));
}
