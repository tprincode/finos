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
    assert_eq!(default_window["startOn"], "2025-09-02");
    assert_eq!(default_window["weekEnds"].as_array().unwrap().len(), 52);
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
async fn calculator_lists_div1_only_keeps_removed_plans() {
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

#[test]
fn market_impact_planner_holds_the_bull_and_bear_windows() {
    let root = golden_harness::repo_root();
    let ui = std::fs::read_to_string(root.join("packages/ui-components/src/index.tsx")).unwrap();
    let sheet = ui
        .split("export function CalculatorReturnSheet")
        .nth(1)
        .expect("calculator sheet");
    let sheet = sheet
        .split("function MonthPerThousandChart")
        .next()
        .unwrap();
    for header in [
        "Bear price",
        "Bear cushion",
        "Bear total",
        "Bull price",
        "Bull cushion",
        "Bull cash cushion",
        "Bear cash cushion",
        "Bull total",
    ] {
        assert!(
            !sheet.contains(header),
            "Calculator sheet still has {header}"
        );
    }
    for header in [
        "Income reliability",
        "Downside",
        "Recovery",
        "NAV persistence",
        "Diversification",
        "Data confidence",
    ] {
        assert!(
            !sheet.contains(&format!("sortHead(sort, \"{header}\"")),
            "Calculator sheet still has evidence header {header}"
        );
    }
    let screen = std::fs::read_to_string(
        root.join("apps/desktop/src/features/market-impact/MarketImpactPlanner.tsx"),
    )
    .unwrap();
    let row = std::fs::read_to_string(
        root.join("apps/desktop/src/features/market-impact/SymbolWindowRow.tsx"),
    )
    .unwrap();
    let app = std::fs::read_to_string(root.join("apps/desktop/src/App.tsx")).unwrap();
    let owner = std::fs::read_to_string(
        root.join("apps/desktop/src/features/position-details/PositionDetailsScreen.tsx"),
    )
    .unwrap();
    let css = std::fs::read_to_string(root.join("apps/desktop/src/App.css")).unwrap();
    assert!(
        screen.contains("<SymbolWindowTable")
            && owner.contains("<SymbolWindowTable")
            && owner.contains("<h3>Owner period</h3>")
            && owner.contains("<h3>Evidence</h3>")
            && owner.contains("aria-label=\"Evidence dimensions\"")
            && owner.contains("Plan achieved % in period")
            && owner.contains("incomeReliabilityBps")
            && owner.contains(".toFixed(2)}%")
            && owner.contains("Downside resilience")
            && owner.contains("Recovery / upside")
            && owner.contains("NAV persistence")
            && owner.contains("Cash cushion")
            && owner.contains("showCashCushion={false}")
            && owner.contains("Days")
            && owner.contains("Underlying")
            && owner.contains("SPY")
            && owner.contains("Nasdaq-100")
            && owner.contains("Versus underlying")
            && !owner.contains("Data confidence")
            && !owner.contains("Tier suggestion")
            && !owner.contains("Fleet compare")
            && !owner.contains("(suggestion)")
            && owner.contains("Cash received in this window divided by the cash the plan expected. 120% means the window paid 1.20 for each 1.00 the plan expected.")
            && !owner.contains("score stops at 100")
            && !owner.contains("Income reliability")
            && owner.contains("The deepest fall in the share price during this window, from a high point to a later lower point. 100 means the price never fell. A lower score means a deeper fall. 0 means the price fell by its whole value or more.")
            && owner.contains("After the low, how much of the drop was gained back. Unknown when the low is the last price, so there is no bounce to measure. Unknown is not written as 0.")
            && owner.contains("Whether the share price held up once this window’s cash cushion is included. The score runs from 0 to 100.")
            && owner.contains("Price change from the first day to the last day of this window. This is one episode. It does not say what the next downturn will be.")
            && owner.contains("How many days this window covers, including the first and last day. A longer window is still one episode.")
            && owner.contains("This window's price change minus the underlying's price change for the same days. A negative number means the position fell further than the underlying.")
            && owner.contains("${period.kind} ${period.startOn}")
            && !owner.contains("Diversification")
            && !owner.contains("legacy text only")
            && !owner.contains("Evidence{windowName")
            && !owner.contains("aria-label=\"Save owner period\"")
            && !owner.contains("aria-label=\"Calculate window\"")
            && !owner.contains("aria-label=\"Window results\"")
            && row.contains("<th>Bull start</th>")
            && row.contains("<th>Bull stop</th>")
            && row.contains("<th>Bear start</th>")
            && row.contains("<th>Bear stop</th>")
            && row.contains("Bull return")
            && row.contains("Bull cash cushion")
            && row.contains("Bull total")
            && row.contains("Bear return")
            && row.contains("Bear cash cushion")
            && row.contains("Bear total")
            && row.contains("Change in the share price from the first day to the last day of this window. Distributions are not included.")
            && row.contains("Cash the broker paid on one share during this window, divided by that share's price on the first day. 5% means 5 cents of cash for each dollar of the starting price. It is not the whole holding's cash, not yield on cost, and not a Plan figure. A large cushion does not cancel a price drop; read it next to price return.")
            && row.contains("Price return plus cash cushion for the same days, both per share. This is not a reinvested total-return index. If either input is unknown, the total is unknown.")
            && ui.split("export function CalculatorReturnSheet").next().unwrap().contains("Bull cash cushion")
            && ui.split("export function CalculatorReturnSheet").next().unwrap().contains("Bear cash cushion")
            && row.contains("type=\"date\"")
            && row.contains("needs the other date.")
            && row.contains("Save ${row.symbol} windows")
            && row.contains("Save/Update")
            && row.contains("name: `${kind} ${row.symbol}`")
            && row.contains("PeriodSeriesRetrieve")
            && row.contains("PositionBacktestCalculate")
            && row.contains("\"SPY\"")
            && row.contains("\"QQQ\"")
            && !screen.contains("Name this bull window. The system does not pick dates.")
            && !screen.contains("Name this bear window. The system does not pick dates.")
            && app.contains("name: storedName")
            && app.contains("const storedName = `${pdPeriod.kind} ${investment.symbol}`")
            && app.contains("navButton(\"market-impact\", \"Market impact planner\")")
            && app.contains("<MarketImpactPlanner")
            && row.contains("className=\"market-impact-windows\"")
            && row.contains("className=\"market-impact-return\"")
            && row.contains("className={dirty ? \"is-unsaved\" : undefined}")
            && css_has_rule(&css, ".hub-panel > h3:has(+ .table-wrap)", "margin-bottom: 0.15rem")
            && css_has_rule(&css, ".hub-panel > h3 + .table-wrap > table", "margin-top: 0")
            && css_has_rule(&css, "table.market-impact-windows", "width: max-content")
            && css_has_rule(
                &css,
                "table.market-impact-windows .market-impact-date",
                "width: 9rem",
            )
            && css_has_rule(&css, ".market-impact-return", "white-space: nowrap"),
        "Market impact planner and Owner period share one window row"
    );
    assert!(
        app.contains("Optional advisory thesis — does not post facts or apply tier")
            && app.contains("AiAnalyze")
            && row.contains("name: `${kind} ${row.symbol}`"),
        "advisory is AiAnalyze only; windows stay per-symbol, not a fleet NVDY/NVDW share"
    );
}

#[tokio::test]
async fn market_impact_save_updates_the_stop_and_ranks_amdy_against_amdw() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let (amdy_id, _) = open_div1(&platform, "AMDY").await;
    let (amdw_id, _) = open_div1(&platform, "AMDW").await;

    let first = must_ok(
        &platform,
        "BacktestPeriodRecord",
        serde_json::json!({
            "kind": "Bull",
            "name": "Bull AMDY",
            "startOn": "2022-01-03",
            "endOn": "2022-06-30"
        }),
    )
    .await;
    let second = must_ok(
        &platform,
        "BacktestPeriodRecord",
        serde_json::json!({
            "kind": "Bull",
            "name": "Bull AMDY",
            "startOn": "2022-01-03",
            "endOn": "2022-03-31"
        }),
    )
    .await;
    assert_eq!(first["periodId"], second["periodId"]);
    assert_eq!(second["endOn"], "2022-03-31");

    must_ok(
        &platform,
        "BacktestPeriodRecord",
        serde_json::json!({
            "kind": "Bear",
            "name": "Bear AMDY",
            "startOn": "2022-01-03",
            "endOn": "2022-03-31"
        }),
    )
    .await;
    must_ok(
        &platform,
        "BacktestPeriodRecord",
        serde_json::json!({
            "kind": "Bull",
            "name": "Bull AMDW",
            "startOn": "2022-01-03",
            "endOn": "2022-03-31"
        }),
    )
    .await;
    must_ok(
        &platform,
        "BacktestPeriodRecord",
        serde_json::json!({
            "kind": "Bear",
            "name": "Bear AMDW",
            "startOn": "2022-01-03",
            "endOn": "2022-03-31"
        }),
    )
    .await;

    let impact = query_json(&platform, "MarketImpactGet", serde_json::json!({})).await;
    let amdy = impact_row(&impact, "AMDY");
    assert_eq!(amdy["bullStart"], "2022-01-03");
    assert_eq!(amdy["bullEnd"], "2022-03-31");
    assert!(amdy["bullPriceReturnBps"].is_null());
    assert!(amdy["bearPriceReturnBps"].is_null());

    calculate_window(&platform, &amdy_id, "Bull", 10_000, 15_000).await;
    calculate_window(&platform, &amdy_id, "Bear", 10_000, 8_000).await;
    calculate_window(&platform, &amdw_id, "Bull", 10_000, 12_000).await;
    calculate_window(&platform, &amdw_id, "Bear", 10_000, 9_000).await;

    let ranked = query_json(&platform, "MarketImpactGet", serde_json::json!({})).await;
    let amdy = impact_row(&ranked, "AMDY");
    let amdw = impact_row(&ranked, "AMDW");
    let amdy_bear = amdy["bearPriceReturnBps"].as_i64().unwrap();
    let amdw_bear = amdw["bearPriceReturnBps"].as_i64().unwrap();
    let amdy_bull = amdy["bullPriceReturnBps"].as_i64().unwrap();
    let amdw_bull = amdw["bullPriceReturnBps"].as_i64().unwrap();
    assert!(
        amdw_bear > amdy_bear,
        "AMDW lost less in the bear: AMDW {amdw_bear} AMDY {amdy_bear}"
    );
    assert!(
        amdy_bull > amdw_bull,
        "AMDY made more in the bull: AMDY {amdy_bull} AMDW {amdw_bull}"
    );
    assert_eq!(amdy_bear, -2_000);
    assert_eq!(amdw_bear, -1_000);
    assert_eq!(amdy_bull, 5_000);
    assert_eq!(amdw_bull, 2_000);

    let details = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "symbol": "AMDY", "asOfDate": "2026-08-22" }),
    )
    .await;
    let names: Vec<&str> = details["periods"]
        .as_array()
        .unwrap()
        .iter()
        .map(|period| period["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"Bull AMDY") && names.contains(&"Bear AMDY"), "{names:?}");
    assert!(names.iter().all(|name| !name.ends_with("AMDW")), "{names:?}");
}

#[tokio::test]
async fn cash_cushion_is_broker_cash_per_share() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let (security_id, account_id) = open_div1(&platform, "CSHQ").await;
    let before = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "symbol": "CSHQ", "asOfDate": "2026-08-22" }),
    )
    .await;
    assert_eq!(before["remainingQuantityMinor"].as_i64(), Some(10));
    assert_eq!(before["quantityScale"].as_u64(), Some(0));
    assert_eq!(before["riskTier"], "Risk On");
    assert_eq!(before["planKnown"], false);

    must_ok(
        &platform,
        "BacktestPeriodRecord",
        serde_json::json!({
            "kind": "Bull",
            "name": "Bull CSHQ",
            "startOn": "2026-03-03",
            "endOn": "2026-06-30"
        }),
    )
    .await;
    must_ok(
        &platform,
        "BacktestPeriodRecord",
        serde_json::json!({
            "kind": "Bear",
            "name": "Bear CSHQ",
            "startOn": "2026-06-30",
            "endOn": "2026-07-29"
        }),
    )
    .await;
    must_ok(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "activityType": "dividend",
            "amountMinor": 4_000,
            "scale": 2,
            "occurredOn": "2026-04-01",
            "idempotencyKey": "cshq-div"
        }),
    )
    .await;
    must_ok(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "activityType": "interest",
            "amountMinor": 1_000,
            "scale": 2,
            "occurredOn": "2026-05-01",
            "idempotencyKey": "cshq-int"
        }),
    )
    .await;
    let replaced = must_ok(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "activityType": "dividend",
            "amountMinor": 50_000,
            "scale": 2,
            "occurredOn": "2026-04-10",
            "idempotencyKey": "cshq-replaced"
        }),
    )
    .await;
    must_ok(
        &platform,
        "ActivityCorrect",
        serde_json::json!({
            "activityId": replaced["activityId"],
            "amountMinor": 200,
            "scale": 2,
            "occurredOn": "2026-04-10"
        }),
    )
    .await;
    must_ok(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "activityType": "deposit",
            "amountMinor": 999_999,
            "scale": 2,
            "occurredOn": "2026-04-20",
            "idempotencyKey": "cshq-deposit"
        }),
    )
    .await;
    must_ok(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": account_id,
            "securityId": security_id,
            "activityType": "dividend",
            "amountMinor": 8_000,
            "scale": 2,
            "occurredOn": "2026-01-15",
            "idempotencyKey": "cshq-outside"
        }),
    )
    .await;
    must_ok(
        &platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 99_999,
            "amountScale": 2,
            "paymentPeriod": "2026-07-10",
            "source": "issuer",
            "enteredAt": "2026-07-10"
        }),
    )
    .await;

    calculate_ending_on_the_low(&platform, &security_id, "Bear", "2026-07-15").await;
    calculate_ending_on_the_low(&platform, &security_id, "Bull", "2026-05-01").await;

    let after = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "symbol": "CSHQ", "asOfDate": "2026-08-22" }),
    )
    .await;
    let bear = window_result(&after, "Bear");
    assert_eq!(bear["priceReturnBps"].as_i64(), Some(-2_000));
    assert!(bear["cushionBps"].is_null(), "no broker rows stay unknown: {bear}");
    assert!(bear["totalReturnBps"].is_null(), "total stays unknown without cushion: {bear}");
    assert!(bear["recoveryRatioBps"].is_null(), "a window that ends on the low stays unknown");

    let bull = window_result(&after, "Bull");
    assert_eq!(bull["priceReturnBps"].as_i64(), Some(-2_000));
    assert_eq!(bull["cushionBps"].as_i64(), Some(520));
    assert_ne!(bull["cushionBps"].as_i64(), Some(5_200));
    assert_eq!(bull["totalReturnBps"].as_i64(), Some(-1_480));
    assert!(bull["recoveryRatioBps"].is_null());
    assert_eq!(after["evidence"]["navPersistence"].as_i64(), Some(47));
    assert_ne!(after["evidence"]["navPersistence"].as_i64(), Some(0));
    assert!(after["evidence"]["recoveryUpside"].is_null());
    let windows = after["windowEvidence"].as_array().unwrap();
    let bear_evidence = windows
        .iter()
        .find(|row| row["periodId"] == bear["periodId"])
        .unwrap();
    let bull_evidence = windows
        .iter()
        .find(|row| row["periodId"] == bull["periodId"])
        .unwrap();
    assert!(bear_evidence.get("diversification").is_none());
    assert!(bull_evidence.get("diversification").is_none());
    assert!(bear_evidence["navPersistence"].is_null());
    assert_eq!(bear_evidence["downsideResilience"].as_i64(), Some(66));
    assert_eq!(bear_evidence["dataConfidence"].as_i64(), Some(16));
    assert_eq!(bull_evidence["navPersistence"].as_i64(), Some(47));
    assert_eq!(bull_evidence["downsideResilience"].as_i64(), Some(66));
    assert_eq!(bull_evidence["dataConfidence"].as_i64(), Some(33));
    assert!(bull_evidence["recoveryUpside"].is_null());

    assert_eq!(after["remainingQuantityMinor"].as_i64(), Some(10));
    assert_eq!(after["riskTier"], "Risk On");
    assert_eq!(after["planKnown"], false);
    assert_eq!(after["planPerShareMinor"].as_i64(), before["planPerShareMinor"].as_i64());
}

#[tokio::test]
async fn window_comparison_uses_the_same_dates() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let (security_id, _) = open_div1(&platform, "CMPQ").await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "divType": "DIV-1",
            "paymentFrequency": "Weekly",
            "replaceCadence": true,
            "riskTier": "Risk On",
            "isActive": true,
            "underlying": "UNDR"
        }),
    )
    .await;
    let period = must_ok(
        &platform,
        "BacktestPeriodRecord",
        serde_json::json!({
            "kind": "Bear",
            "name": "Bear CMPQ",
            "startOn": "2026-06-30",
            "endOn": "2026-07-29"
        }),
    )
    .await;
    must_ok(
        &platform,
        "PositionBacktestCalculate",
        serde_json::json!({
            "securityId": security_id,
            "periodId": period["periodId"],
            "candidates": [
                {"asOfAt": "2026-06-30", "priceMinor": 10_000, "scale": 2},
                {"asOfAt": "2026-07-29", "priceMinor": 8_000, "scale": 2}
            ],
            "underlyingCandidates": [
                {"asOfAt": "2026-06-30", "priceMinor": 10_000, "scale": 2},
                {"asOfAt": "2026-07-29", "priceMinor": 9_000, "scale": 2}
            ],
            "spyCandidates": [
                {"asOfAt": "2026-06-30", "priceMinor": 10_000, "scale": 2},
                {"asOfAt": "2026-07-29", "priceMinor": 9_500, "scale": 2}
            ],
            "nasdaqCandidates": [
                {"asOfAt": "2026-06-30", "priceMinor": 10_000, "scale": 2},
                {"asOfAt": "2026-07-29", "priceMinor": 9_700, "scale": 2}
            ]
        }),
    )
    .await;
    let after = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "symbol": "CMPQ", "asOfDate": "2026-08-22" }),
    )
    .await;
    assert_eq!(after["underlying"], "UNDR");
    let bear = window_result(&after, "Bear");
    assert_eq!(bear["priceReturnBps"].as_i64(), Some(-2_000));
    assert_eq!(bear["underlyingSymbol"], "UNDR");
    assert_eq!(bear["underlyingReturnBps"].as_i64(), Some(-1_000));
    assert_eq!(bear["spyReturnBps"].as_i64(), Some(-500));
    assert_eq!(bear["nasdaqReturnBps"].as_i64(), Some(-300));

    let (blank_id, _) = open_div1(&platform, "BLNK").await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": blank_id,
            "divType": "DIV-1",
            "paymentFrequency": "Weekly",
            "replaceCadence": true,
            "riskTier": "Risk On",
            "isActive": true,
            "underlying": ""
        }),
    )
    .await;
    let blank_period = must_ok(
        &platform,
        "BacktestPeriodRecord",
        serde_json::json!({
            "kind": "Bear",
            "name": "Bear BLNK",
            "startOn": "2026-06-30",
            "endOn": "2026-07-29"
        }),
    )
    .await;
    must_ok(
        &platform,
        "PositionBacktestCalculate",
        serde_json::json!({
            "securityId": blank_id,
            "periodId": blank_period["periodId"],
            "candidates": [
                {"asOfAt": "2026-06-30", "priceMinor": 10_000, "scale": 2},
                {"asOfAt": "2026-07-29", "priceMinor": 8_000, "scale": 2}
            ],
            "underlyingCandidates": [
                {"asOfAt": "2026-06-30", "priceMinor": 10_000, "scale": 2},
                {"asOfAt": "2026-07-29", "priceMinor": 9_000, "scale": 2}
            ]
        }),
    )
    .await;
    let blank = query_json(
        &platform,
        "InvestmentGet",
        serde_json::json!({ "symbol": "BLNK", "asOfDate": "2026-08-22" }),
    )
    .await;
    assert_eq!(blank["underlying"], "");
    let blank_bear = window_result(&blank, "Bear");
    assert!(blank_bear["underlyingReturnBps"].is_null());
}

async fn calculate_ending_on_the_low(
    platform: &LocalPlatform,
    security_id: &str,
    kind: &str,
    mid_on: &str,
) {
    let details = query_json(
        platform,
        "InvestmentGet",
        serde_json::json!({ "securityId": security_id, "asOfDate": "2026-08-22" }),
    )
    .await;
    let period = details["periods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|period| period["kind"] == kind)
        .unwrap();
    let start = period["startOn"].as_str().unwrap();
    let end = period["endOn"].as_str().unwrap();
    must_ok(
        platform,
        "PositionBacktestCalculate",
        serde_json::json!({
            "securityId": security_id,
            "periodId": period["periodId"],
            "candidates": [
                {"asOfAt": start, "priceMinor": 10_000, "scale": 2},
                {"asOfAt": mid_on, "priceMinor": 12_000, "scale": 2},
                {"asOfAt": end, "priceMinor": 8_000, "scale": 2}
            ]
        }),
    )
    .await;
}

fn window_result<'a>(body: &'a serde_json::Value, kind: &str) -> &'a serde_json::Value {
    let period_id = body["periods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|period| period["kind"] == kind)
        .unwrap_or_else(|| panic!("{kind} period missing from {body}"))["periodId"]
        .as_str()
        .unwrap();
    body["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|result| result["periodId"] == period_id)
        .unwrap_or_else(|| panic!("{kind} result missing from {body}"))
}

fn css_has_rule(css: &str, selector: &str, decl: &str) -> bool {
    css.split(selector).skip(1).any(|rest| {
        let body = rest.split('}').next().unwrap_or("");
        body.contains(decl)
    })
}

fn impact_row<'a>(body: &'a serde_json::Value, symbol: &str) -> &'a serde_json::Value {
    body["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["symbol"] == symbol)
        .unwrap_or_else(|| panic!("{symbol} missing from {body}"))
}

async fn open_div1(platform: &LocalPlatform, symbol: &str) -> (String, String) {
    let (security_id, account_id) = open_named(platform, symbol, "Weekly").await;
    must_ok(
        platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "divType": "DIV-1",
            "paymentFrequency": "Weekly",
            "replaceCadence": true,
            "riskTier": "Risk On",
            "isActive": true
        }),
    )
    .await;
    (security_id, account_id)
}

async fn calculate_window(
    platform: &LocalPlatform,
    security_id: &str,
    kind: &str,
    first: i64,
    last: i64,
) {
    let impact = query_json(platform, "MarketImpactGet", serde_json::json!({})).await;
    let symbol = impact["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["securityId"] == security_id)
        .unwrap();
    let (start, end) = if kind == "Bull" {
        (symbol["bullStart"].as_str().unwrap(), symbol["bullEnd"].as_str().unwrap())
    } else {
        (symbol["bearStart"].as_str().unwrap(), symbol["bearEnd"].as_str().unwrap())
    };
    let period_id = query_json(platform, "InvestmentGet", serde_json::json!({
        "securityId": security_id,
        "asOfDate": "2026-08-22"
    }))
    .await["periods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|period| period["kind"] == kind)
        .unwrap()["periodId"]
        .as_str()
        .unwrap()
        .to_string();
    must_ok(
        platform,
        "PositionBacktestCalculate",
        serde_json::json!({
            "securityId": security_id,
            "periodId": period_id,
            "candidates": [
                {"asOfAt": start, "priceMinor": first, "scale": 2},
                {"asOfAt": end, "priceMinor": last, "scale": 2}
            ]
        }),
    )
    .await;
}

/// A short Period still returns the last six stored pays, newest first.
#[tokio::test]
async fn short_period_keeps_the_last_six_stored_pays() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let (security_id, _) = open_named(&platform, "AVGX", "Monthly").await;
    let pays = [
        ("2026-04-01", 100),
        ("2026-05-01", 200),
        ("2026-06-01", 300),
        ("2026-07-01", 400),
        ("2026-08-01", 500),
        ("2026-09-01", 600),
        ("2026-10-01", 700),
    ];
    for (period, minor) in pays {
        must_ok(
            &platform,
            "IssuerDeclarationRecord",
            serde_json::json!({
                "securityId": security_id,
                "amountPerShareMinor": minor,
                "amountScale": 2,
                "paymentPeriod": period,
                "source": "import",
                "enteredAt": "2026-10-01"
            }),
        )
        .await;
    }
    let history = query_json(
        &platform,
        "DeclarationHistoryGet",
        serde_json::json!({
            "asOfDate": "2026-10-05",
            "cadence": "all",
            "startOn": "2026-09-20",
            "endOn": "2026-10-05"
        }),
    )
    .await;
    let row = history["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["symbol"] == "AVGX")
        .unwrap();
    let filled: Vec<i64> = row["cells"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|cell| cell["amountPerShareMinor"].as_i64())
        .collect();
    assert_eq!(filled, vec![700], "the short Period grid holds only the October pay");
    let recent: Vec<i64> = row["recentPays"]
        .as_array()
        .unwrap()
        .iter()
        .map(|pay| pay["amountPerShareMinor"].as_i64().unwrap())
        .collect();
    assert_eq!(
        recent,
        vec![700, 600, 500, 400, 300, 200],
        "Avg 6 reads the last six stored pays, not the Period columns"
    );
    // Pays are scale 2 cents: 7.00 … 2.00 → mean of six = 4.50 → 450000 @ scale 5
    assert_eq!(row["avg6Minor"], 450_000);
    assert_eq!(row["avg6Scale"], 5);
    assert_eq!(row["avg6Count"], 6);
    assert_eq!(row["avg6Complete"], true);
    // Mean of three newest (7+6+5)/3 = 6.00 → 600000 @ scale 5
    assert_eq!(row["avg3Minor"], 600_000);
    assert_eq!(row["avg3Scale"], 5);
    assert_eq!(row["avg3Count"], 3);
    assert_eq!(row["avg3Complete"], true);
    let ui = std::fs::read_to_string(
        golden_harness::repo_root().join("packages/ui-components/src/index.tsx"),
    )
    .unwrap();
    let sheet = ui
        .split("export function CalculatorReturnSheet")
        .nth(1)
        .unwrap()
        .split("function MonthPerThousandChart")
        .next()
        .unwrap();
    assert!(sheet.contains("avg3Display(row)"));
    assert!(sheet.contains("avg6Display(row)"));
    assert!(!sheet.contains("meanNewestPays("));
    assert!(!sheet.contains("avg6Label("));
    assert!(sheet.contains("newestStoredPay(row.recentPays)"));
    assert!(
        sheet.contains("dividendScore(master, row.cells, undefined, row.inForcePays).threeYield"),
        "3-pay yield reads the newest in-force declarations"
    );
    assert!(
        !ui.contains("const three = lastPaidCents(cells, 3);"),
        "3-pay yield does not walk the older end of the week grid"
    );
    assert!(
        ui.contains("export function avg6Display")
            && ui.contains("formatPerShare(row.avg6Minor")
            && ui.contains("formatPerShare(row.avg3Minor"),
        "Avg 3/6 display uses DeclarationHistory scale-5 fields"
    );
    assert!(sheet.contains("planCheck(\n                inForce,"));
    assert!(sheet.contains("paidInViewCents(row.cells)"));
    assert!(ui.contains("if (c === \"weekly\" || c === \"52\") return 52;"));
    let score = ui
        .split("export function dividendScore(")
        .nth(1)
        .expect("dividendScore")
        .split("export function CalculatorReturnSheet")
        .next()
        .expect("dividendScore body");
    assert!(
        !score.contains("lastPaidCents") && !score.contains("cells.length - 1"),
        "TVAL and the other scores do not walk the older end of the week grid"
    );
    assert!(
        sheet.contains("sortHead(sort, \"Calculator blend\", \"tval\", true)"),
        "the plan-yield blend is not labeled TVAL"
    );
    assert!(
        !sheet.contains("sortHead(sort, \"TVAL\", \"tval\", true)"),
        "the old TVAL header is gone"
    );
    assert!(
        score.contains("Math.round((pnl * 2 + master.planFwdYieldBps) / 2)"),
        "Calculator blend stays Gain% × 2 plus plan FWD, divided by 2"
    );
    assert!(
        !financial_domain::collector::calculator_view_includes("CASH", "SPAXX", "Monthly"),
        "cash stays off the Calculator"
    );
    assert!(
        financial_domain::collector::calculator_view_includes("DIV-1", "AMDW", "Weekly"),
        "DIV-1 stays on the Calculator"
    );
    let check = ui
        .split("export function planCheck(")
        .nth(1)
        .expect("planCheck")
        .split("export function parseTypedPlan")
        .next()
        .expect("planCheck body");
    assert!(
        check.contains("atOrAbove: below === 0"),
        "the Plan check cell is green when none of the counted pays is below Plan"
    );
    assert!(check.contains("text: \"unknown\""));
    let filter = ui
        .split("export function matchesCalculatorPerformanceView(")
        .nth(1)
        .expect("performance filter")
        .split("function calculatorEligibleMaster")
        .next()
        .expect("performance filter body");
    assert!(
        filter.contains("check.abovePct == null || check.abovePct < 80"),
        "the Performance filter keeps a row when Above % is 80 or higher"
    );
    assert!(!filter.contains(".atOrAbove"));
}

/// 00:30 UTC on 6 Oct is still 5 Oct in New York. Business today must not roll at 8:00 p.m. Eastern.
#[test]
fn eight_pm_eastern_is_still_today() {
    let utc = chrono::DateTime::parse_from_rfc3339("2026-10-06T00:30:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    assert_eq!(
        application_core::last_price_window::business_date(utc).to_string(),
        "2026-10-05"
    );
    let queries = std::fs::read_to_string(
        golden_harness::repo_root().join("crates/application-core/src/queries.rs"),
    )
    .unwrap();
    let today = queries
        .split("fn today_iso()")
        .nth(1)
        .expect("today_iso")
        .split("fn calculator_row_visible")
        .next()
        .expect("today_iso body");
    assert!(
        !today.contains("Utc::now().date_naive()"),
        "today is the New York calendar date"
    );
    assert!(today.contains("business_date"));
    let history = queries
        .split("async fn declaration_history_view(")
        .nth(1)
        .expect("declaration_history_view")
        .split("async fn plan_review_view(")
        .next()
        .expect("declaration history body");
    assert!(
        !history.contains("2026, 9, 2"),
        "a bad as-of must not invent 2026-09-02"
    );
    assert!(history.contains("bad_as_of"));
    assert!(
        history.contains("issuer_declaration_list(security.security_id)")
            && history.contains(".await?;"),
        "a failed declaration list stays an error"
    );
    assert!(
        !history.contains("unwrap_or_default()"),
        "a failed declaration list must not become an empty pay row"
    );
}

/// A stored zero is a declaration. The short Period grid does not choose Most current.
#[tokio::test]
async fn stored_zero_is_the_newest_in_force_pay() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let (security_id, _) = open_named(&platform, "ZERO", "Monthly").await;
    for (period, minor) in [("2026-08-01", 200), ("2026-09-01", 100), ("2026-10-01", 0)] {
        must_ok(
            &platform,
            "IssuerDeclarationRecord",
            serde_json::json!({
                "securityId": security_id,
                "amountPerShareMinor": minor,
                "amountScale": 2,
                "paymentPeriod": period,
                "source": "import",
                "enteredAt": "2026-10-01"
            }),
        )
        .await;
    }
    let history = query_json(
        &platform,
        "DeclarationHistoryGet",
        serde_json::json!({
            "asOfDate": "2026-10-05",
            "cadence": "all",
            "startOn": "2026-09-20",
            "endOn": "2026-10-05"
        }),
    )
    .await;
    let row = history["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["symbol"] == "ZERO")
        .unwrap();
    let recent: Vec<i64> = row["recentPays"]
        .as_array()
        .unwrap()
        .iter()
        .map(|pay| pay["amountPerShareMinor"].as_i64().unwrap())
        .collect();
    assert_eq!(recent, vec![0, 100, 200]);
    let in_force: Vec<i64> = row["inForcePays"]
        .as_array()
        .unwrap()
        .iter()
        .map(|pay| pay["amountPerShareMinor"].as_i64().unwrap())
        .collect();
    assert_eq!(in_force, vec![0, 100, 200]);
    let filled: Vec<i64> = row["cells"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|cell| cell["amountPerShareMinor"].as_i64())
        .collect();
    assert_eq!(filled, vec![0], "the dated column shows the stored zero");
}

#[test]
fn field_intent_stores_the_calculator_column_contract() {
    let root = golden_harness::repo_root();
    let app = std::fs::read_to_string(root.join("apps/desktop/src/App.tsx")).unwrap();
    assert!(app.contains("navButton(\"field-intent\", \"Field intent\")"));
    assert!(app.contains("<FieldIntentScreen />"));
    assert!(app.contains("useState(\"year\")"));
    assert!(!app.contains("useState(\"60\")"));
    assert!(app.contains("addUtcDays(asOf, -365)"));
    let catalog = std::fs::read_to_string(root.join("docs/architecture/ui-modules.json")).unwrap();
    assert!(
        catalog.contains("\"id\": \"field-intent\"")
            && catalog.contains("apps/desktop/src/features/field-intent/")
            && catalog.contains("\"status\": \"extracted\"")
    );
    let columns = std::fs::read_to_string(
        root.join("apps/desktop/src/features/field-intent/calculatorColumns.ts"),
    )
    .unwrap();
    for name in [
        "Symbol",
        "MC FWD",
        "3-pay yield",
        "Most current",
        "Avg 3",
        "Avg 6",
        "Paid",
        "Plan check",
        "Week columns (dated)",
    ] {
        assert!(columns.contains(&format!("name: \"{name}\"")), "missing {name}");
    }
    let names = columns.matches("name: \"").count();
    let matched = columns.matches("status: \"matches\"").count();
    let still_wrong = columns.matches("status: \"still wrong\"").count();
    assert_eq!(names, matched + still_wrong);
    assert!(names >= 40);
    for name in ["Type", "Calculator blend", "MC TVAL", "TVAL Δ"] {
        let marker = format!("name: \"{name}\"");
        let at = columns
            .find(&marker)
            .unwrap_or_else(|| panic!("missing {name}"));
        let slice = &columns[at..columns.len().min(at + 280)];
        assert!(
            slice.contains("status: \"still wrong\""),
            "{name} stays still wrong (parked TVAL / Type): {slice}"
        );
    }
    assert_eq!(still_wrong, 4, "only Type and the TVAL family are still wrong");
    assert!(
        columns.contains("kind: \"cash\"") && columns.contains("kind: \"percent\""),
        "field-intent names cash and percent kinds for Excel"
    );
    let contract_fields = std::fs::read_to_string(
        root.join("apps/desktop/src/features/field-intent/contractFields.ts"),
    )
    .unwrap();
    assert!(contract_fields.contains("pageId"));
    assert!(contract_fields.contains("componentId"));
    assert!(contract_fields.contains("contract-positions"));
    assert!(contract_fields.contains("\"open premium\""));
    assert!(!contract_fields.contains("Option mid"));
    let ui = std::fs::read_to_string(root.join("packages/ui-components/src/index.tsx")).unwrap();
    assert!(!ui.contains("Default window is the last 60 days"));
    assert!(!ui.contains("Plan check counts weeks on this row"));
    assert!(ui.contains("trailing year"));
    let week = std::fs::read_to_string(root.join("crates/financial-domain/src/week.rs")).unwrap();
    assert!(week.contains("Duration::days(365)"));
    assert!(!week.contains("Duration::days(60)"));
}

/// Cash stays on Position Master (balance + yield) but off the Calculator sheet.
#[tokio::test]
async fn cash_stays_off_the_calculator() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let (div_id, _) = open_named(&platform, "AMDW", "Weekly").await;
    must_ok(
        &platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": div_id,
            "amountPerShareMinor": 50,
            "amountScale": 2,
            "paymentPeriod": "2026-09-04",
            "source": "import",
            "enteredAt": "2026-09-04"
        }),
    )
    .await;

    let income = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income-SPAXX", "kind": "taxable"}),
    )
    .await;
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "SPAXX", "name": "SPAXX"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "priceSource": "public",
            "sourceSymbol": "SPAXX",
            "declarationSource": "fidelity",
            "sourceUrl": "https://fundresearch.fidelity.com/mutual-funds/performance-and-risk/31617H102",
            "calendarPolicy": "issuer_calendar",
            "collectorEnabled": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "replaceCadence": true,
            "divType": "CASH",
            "provider": "Fidelity",
            "isActive": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": income["accountId"],
            "securityId": security_id,
            "openedOn": "2026-01-15",
            "quantityMinor": 10000,
            "quantityScale": 0,
            "performanceCostMinor": 1000000,
            "taxCostMinor": 1000000,
            "scale": 2
        }),
    )
    .await;
    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "SPAXX",
            "declarationSource": "fidelity",
            "divType": "CASH",
            "candidates": [{
                "kind": "cash_rate",
                "planOnly": true,
                "amountPerShareMinor": 2783,
                "amountScale": 6,
                "annualYieldBps": 334,
                "sevenDayYield": "3.34",
                "paymentPeriod": "2026-09-07",
                "source": "fidelity"
            }]
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
            "paymentPeriod": "2026-09-04",
            "source": "import",
            "enteredAt": "2026-09-04"
        }),
    )
    .await;

    let master = query_json(&platform, "PositionMasterGet", serde_json::json!({})).await;
    let spaxx = master["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["symbol"] == "SPAXX")
        .expect("SPAXX stays on the position master");
    assert_eq!(spaxx["cashPar"], true);
    assert_eq!(spaxx["cashAnnualYieldBps"], 334);
    assert_eq!(spaxx["lastPriceMinor"], 100);
    assert_eq!(spaxx["divType"], "CASH");
    let amdw = master["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["symbol"] == "AMDW")
        .unwrap();
    assert_eq!(amdw["cashPar"], false);
    assert!(amdw["cashAnnualYieldBps"].is_null());

    let calc = query_json(&platform, "CalculatorGet", serde_json::json!({})).await;
    assert!(
        calc["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["symbol"] != "SPAXX"),
        "SPAXX stays off CalculatorGet: {calc}"
    );

    let history = query_json(
        &platform,
        "DeclarationHistoryGet",
        serde_json::json!({
            "asOfDate": "2026-10-05",
            "cadence": "all",
            "startOn": "2026-01-01",
            "endOn": "2026-10-05"
        }),
    )
    .await;
    assert!(
        history["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["symbol"] != "SPAXX"),
        "SPAXX stays off the Calculator sheet: {history}"
    );
    let div_row = history["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["symbol"] == "AMDW")
        .unwrap();
    assert!(
        div_row["cells"]
            .as_array()
            .unwrap()
            .iter()
            .any(|cell| cell["amountPerShareMinor"] == 50),
        "a dividend week cell is unchanged"
    );

    let ui = std::fs::read_to_string(
        golden_harness::repo_root().join("packages/ui-components/src/index.tsx"),
    )
    .unwrap();
    let sheet = ui
        .split("export function CalculatorReturnSheet")
        .nth(1)
        .unwrap()
        .split("function MonthPerThousandChart")
        .next()
        .unwrap();
    assert!(!sheet.contains("Cash rate"));
    assert!(!sheet.contains("Dividend columns on that row are N/A"));
    assert!(sheet.contains("Only DIV-1 positions appear here"));
    let eligible = ui
        .split("function calculatorEligibleMaster")
        .nth(1)
        .unwrap()
        .split("export function DeclarationHistoryPanel")
        .next()
        .unwrap();
    assert!(eligible.contains("DIV-1"));
    assert!(!eligible.contains("CASH"));
    assert!(ui.contains("calculatorHouseholdExCashCents"));
    let parser = std::fs::read_to_string(
        golden_harness::repo_root().join("crates/import-engine/src/retrieve/adapters/moneymarket.rs"),
    )
    .unwrap();
    assert!(!parser.contains("parse_generic_distributions"));
    let follow = std::fs::read_to_string(
        golden_harness::repo_root().join("crates/application-core/src/queries.rs"),
    )
    .unwrap();
    let followup = follow
        .split("async fn apply_cash_moneymarket_followups(")
        .nth(1)
        .unwrap()
        .split("fn declarations_for_security(")
        .next()
        .unwrap();
    assert!(!followup.contains("dividend_actual_record"));
}

/// Typed Calculator Excel: cash + percent number formats, blank not $0, SPAXX off the sheet.
#[tokio::test]
async fn calculator_export_uses_excel_cash_and_percent_types() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let (div_id, _) = open_named(&platform, "AMDW", "Weekly").await;
    must_ok(
        &platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": div_id,
            "amountPerShareMinor": 50,
            "amountScale": 2,
            "paymentPeriod": "2026-09-04",
            "source": "import",
            "enteredAt": "2026-09-04"
        }),
    )
    .await;

    let income = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income-SPAXX", "kind": "taxable"}),
    )
    .await;
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "SPAXX", "name": "SPAXX"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "priceSource": "public",
            "sourceSymbol": "SPAXX",
            "declarationSource": "fidelity",
            "sourceUrl": "https://fundresearch.fidelity.com/mutual-funds/performance-and-risk/31617H102",
            "calendarPolicy": "issuer_calendar",
            "collectorEnabled": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": "Monthly",
            "replaceCadence": true,
            "divType": "CASH",
            "provider": "Fidelity",
            "isActive": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": income["accountId"],
            "securityId": security_id,
            "openedOn": "2026-01-15",
            "quantityMinor": 10000,
            "quantityScale": 0,
            "performanceCostMinor": 1000000,
            "taxCostMinor": 1000000,
            "scale": 2
        }),
    )
    .await;

    let exp = query_json(
        &platform,
        "ComponentExportGet",
        serde_json::json!({
            "moduleId": "calculator",
            "partId": "calculator-sheet",
            "asOfDate": "2026-10-05"
        }),
    )
    .await;
    assert_eq!(exp["defaultFileName"], "calculator-calculator-sheet.xlsx");
    let bytes = b64_decode(exp["bytesBase64"].as_str().unwrap());
    assert!(bytes.starts_with(b"PK"), "xlsx is a zip");
    let unzipped = xlsx_text_parts(&bytes);
    assert!(
        unzipped.contains("$#,##0.00") && unzipped.contains("0.00%"),
        "workbook embeds Excel cash and percent formats"
    );
    assert!(
        unzipped.contains("AMDW") && !unzipped.contains("SPAXX"),
        "DIV-1 row is present; cash SPAXX is not a Calculator export row"
    );
    assert!(
        !unzipped.contains(">32.00<") && !unzipped.contains("\"32.00\""),
        "Price/Plan stay numbers, not money strings"
    );
    assert!(
        unzipped.contains("<v>") && !unzipped.contains("<v>unknown</v>"),
        "typed cells write numbers; blank Plan is an empty cell, not the word unknown"
    );

    let core = std::fs::read_to_string(
        golden_harness::repo_root().join("crates/application-core/src/component_export.rs"),
    )
    .unwrap();
    assert!(
        core.contains("calculator_typed_sheet")
            && core.contains("Cell::Money")
            && core.contains("Cell::Percent")
            && core.contains("Cell::Blank")
            && core.contains("write_number_with_format")
            && core.contains("$#,##0.00")
            && core.contains("0.00%"),
        "one typed Calculator arm writes cash/percent cells"
    );
    let typed = core
        .split("async fn calculator_typed_sheet(")
        .nth(1)
        .unwrap()
        .split("async fn trends_sheet(")
        .next()
        .unwrap();
    assert!(
        !typed.contains("money_text("),
        "typed Calculator path must not stringify money"
    );

    let save = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/features/shared/saveWorkbook.ts"),
    )
    .unwrap();
    assert!(
        save.contains("saveComponentWorkbook")
            && save.contains("ComponentExportGet")
            && save.contains("savePageWorkbook")
            && save.contains("ComponentPageExportGet"),
        "Calculator and Components share one download helper"
    );
    let app = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/App.tsx"),
    )
    .unwrap();
    assert!(
        app.contains("saveComponentWorkbook")
            && app.contains("partId: \"calculator-sheet\""),
        "Calculator screen calls the shared ComponentExportGet arm"
    );
    let registry = std::fs::read_to_string(
        golden_harness::repo_root()
            .join("apps/desktop/src/features/components/ComponentRegistry.tsx"),
    )
    .unwrap();
    assert!(
        registry.contains("savePageWorkbook"),
        "Components page Excel uses the same helper"
    );
}

fn b64_decode(s: &str) -> Vec<u8> {
    fn val(c: u8) -> u8 {
        match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => 0,
        }
    }
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    let mut i = 0;
    while i + 3 < bytes.len() {
        let (a, b, c, d) = (val(bytes[i]), val(bytes[i + 1]), val(bytes[i + 2]), val(bytes[i + 3]));
        out.push((a << 2) | (b >> 4));
        if bytes[i + 2] != b'=' {
            out.push((b << 4) | (c >> 2));
        }
        if bytes[i + 3] != b'=' {
            out.push((c << 6) | d);
        }
        i += 4;
    }
    out
}

fn xlsx_text_parts(bytes: &[u8]) -> String {
    use std::io::{Cursor, Read};
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("xlsx zip");
    let mut out = String::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let name = file.name().to_string();
        if !(name.ends_with(".xml") || name.ends_with(".rels")) {
            continue;
        }
        let mut body = String::new();
        file.read_to_string(&mut body).unwrap();
        out.push_str(&body);
        out.push('\n');
    }
    out
}
