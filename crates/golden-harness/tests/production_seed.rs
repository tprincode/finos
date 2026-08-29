use golden_harness::{
    load_production_expected, load_production_seed_via_commands, production_seed_actual_counts,
    production_seed_actual_totals, production_template_totals, repo_root,
};
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

async fn must_ok(platform: &LocalPlatform, name: &str, body: serde_json::Value) -> serde_json::Value {
    let result = execute_command_on(platform, platform, cmd(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

async fn investment_template_source(
    platform: &LocalPlatform,
    symbol: &str,
) -> (String, String, bool) {
    let got = execute_query_on(
        platform,
        platform,
        QueryRequest {
            contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
            query_name: "InvestmentGet".into(),
            correlation_id: Uuid::new_v4(),
            body_json: Some(serde_json::json!({"symbol": symbol}).to_string()),
        },
    )
    .await;
    assert!(got.ok, "{symbol} InvestmentGet: {:?}", got.error_code);
    let val: serde_json::Value =
        serde_json::from_str(got.body_json.as_deref().unwrap_or("{}")).unwrap();
    (
        val["template"]["declarationSource"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        val["template"]["calendarPolicy"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        val["template"]["collectorEnabled"].as_bool().unwrap_or(false),
    )
}

#[tokio::test]
async fn production_seed_counts_reconcile() {
    let root = repo_root();
    let production = root.join("database/seed/production");
    assert!(
        production.join("Template_Accounts.xlsx").exists(),
        "unzip the Include Package so templates land at database/seed/production/"
    );

    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .expect("open sqlite");
    load_production_seed_via_commands(&platform, &production)
        .await
        .expect("production seed load via commands");

    let expected = load_production_expected(&production.join("expected-production.yaml"))
        .expect("expected-production.yaml");
    let actual_counts = production_seed_actual_counts(&platform)
        .await
        .expect("actual counts");
    assert_eq!(actual_counts, expected.counts);

    let template_totals =
        production_template_totals(&production).expect("template money from xlsx");
    assert_eq!(
        template_totals, expected.totals,
        "expected-production.yaml money must match the locked templates"
    );

    let actual_totals = production_seed_actual_totals(&platform)
        .await
        .expect("actual money");
    assert_eq!(
        actual_totals.yield_amount_minor, expected.totals.yield_amount_minor,
        "DividendGet actual total must match owner-approved yield"
    );
    assert_eq!(
        actual_totals.disbursement_gross_minor, expected.totals.disbursement_gross_minor,
        "posted disbursement gross must match owner-approved total"
    );
    assert_eq!(
        actual_totals.open_performance_minor, expected.totals.open_performance_minor,
        "open original cost must match the locked lots template (cents)"
    );
    assert_eq!(
        actual_totals.open_tax_minor, expected.totals.open_tax_minor,
        "open tax basis must match the locked lots template (cents)"
    );

    let master = execute_query_on(
        &platform,
        &platform,
        QueryRequest {
            contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
            query_name: "PositionMasterGet".into(),
            correlation_id: Uuid::new_v4(),
            body_json: None,
        },
    )
    .await;
    assert!(master.ok, "PositionMasterGet: {:?}", master.error_code);
    let master_val: serde_json::Value =
        serde_json::from_str(master.body_json.as_deref().unwrap_or("{}")).unwrap();
    let rows = master_val["rows"].as_array().expect("master rows");
    let doc = import_engine::parse_production_templates(&production).expect("templates");
    let with_underlying: Vec<_> = doc
        .characteristics
        .iter()
        .filter(|c| !c.underlying.trim().is_empty())
        .collect();
    assert!(
        !with_underlying.is_empty(),
        "Template_Positions.xlsx must carry underlying"
    );
    for ch in &with_underlying {
        let row = rows
            .iter()
            .find(|r| r["symbol"] == ch.symbol)
            .unwrap_or_else(|| panic!("{} missing from PositionMasterGet", ch.symbol));
        assert_eq!(
            row["underlying"].as_str().unwrap_or(""),
            ch.underlying,
            "{} underlying",
            ch.symbol
        );
    }
    let amdw = rows
        .iter()
        .find(|r| r["symbol"] == "AMDW")
        .expect("AMDW on position master");
    let amdw_ch = doc
        .characteristics
        .iter()
        .find(|c| c.symbol == "AMDW")
        .expect("AMDW in Template_Positions.xlsx");
    assert_eq!(amdw["underlying"], amdw_ch.underlying);

    let coverage = execute_query_on(
        &platform,
        &platform,
        QueryRequest {
            contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
            query_name: "PositionDetailsCoverageGet".into(),
            correlation_id: Uuid::new_v4(),
            body_json: None,
        },
    )
    .await;
    assert!(coverage.ok, "PositionDetailsCoverageGet: {:?}", coverage.error_code);
    let coverage_val: serde_json::Value =
        serde_json::from_str(coverage.body_json.as_deref().unwrap_or("{}")).unwrap();
    let cov_rows = coverage_val["rows"].as_array().expect("coverage rows");
    let target_dir = repo_root().join("target");
    std::fs::create_dir_all(&target_dir).expect("target dir");
    std::fs::write(
        target_dir.join("position-details-coverage.json"),
        serde_json::to_string_pretty(&coverage_val).unwrap(),
    )
    .expect("write coverage json");
    for ch in &doc.characteristics {
        assert!(
            cov_rows.iter().any(|r| r["symbol"] == ch.symbol),
            "{} missing from PositionDetailsCoverageGet",
            ch.symbol
        );
        if ch.roc_pct_2025_actual_minor.is_some() {
            let row = cov_rows
                .iter()
                .find(|r| r["symbol"] == ch.symbol)
                .unwrap();
            let status = row["rocResearchStatus"].as_str().unwrap_or("");
            assert_ne!(status, "not-in-scope", "{} 2025 actual is in-scope", ch.symbol);
        }
        if ch.roc_pct_2026_estimate_minor.is_none() {
            let master_row = rows.iter().find(|r| r["symbol"] == ch.symbol);
            if let Some(row) = master_row {
                assert!(
                    row["rocPct2026EstimateMinor"].is_null(),
                    "{} blank 2026 estimate must stay unknown, not 0",
                    ch.symbol
                );
            }
        }
    }
    let topw = execute_query_on(
        &platform,
        &platform,
        QueryRequest {
            contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
            query_name: "InvestmentGet".into(),
            correlation_id: Uuid::new_v4(),
            body_json: Some(serde_json::json!({"symbol":"TOPW"}).to_string()),
        },
    )
    .await;
    assert!(topw.ok, "TOPW InvestmentGet: {:?}", topw.error_code);
    let topw_val: serde_json::Value =
        serde_json::from_str(topw.body_json.as_deref().unwrap_or("{}")).unwrap();
    assert_eq!(
        topw_val["template"]["declarationSource"].as_str().unwrap_or(""),
        "roundhill"
    );
    assert_eq!(
        topw_val["template"]["calendarPolicy"].as_str().unwrap_or(""),
        "issuer_calendar"
    );
    assert_ne!(
        topw_val["rocResearchStatus"].as_str().unwrap_or(""),
        "",
        "ROC research strip is derived"
    );

    assert_eq!(
        investment_template_source(&platform, "QDVO").await,
        ("amplify".into(), "issuer_calendar".into(), true)
    );
    assert_eq!(
        investment_template_source(&platform, "SPYI").await,
        ("neos".into(), "issuer_calendar".into(), true)
    );
    assert_eq!(
        investment_template_source(&platform, "MSTY").await,
        ("yieldmax".into(), "issuer_calendar".into(), true)
    );
    assert_eq!(
        investment_template_source(&platform, "QYLD").await,
        ("globalx".into(), "issuer_calendar".into(), true)
    );
    assert_eq!(
        investment_template_source(&platform, "CRF").await,
        ("cornerstone".into(), "issuer_calendar".into(), true)
    );
    assert_eq!(
        investment_template_source(&platform, "CLM").await,
        ("cornerstone".into(), "issuer_calendar".into(), true)
    );
    assert_eq!(
        investment_template_source(&platform, "JEPQ").await,
        ("jpmorgan".into(), "issuer_calendar".into(), true)
    );
    assert_eq!(
        investment_template_source(&platform, "TSLA").await,
        ("unassigned".into(), "none".into(), false)
    );
    assert_eq!(
        investment_template_source(&platform, "ENERGYX").await,
        ("sec-edgar".into(), "none".into(), false)
    );

    let qdvo_row = cov_rows.iter().find(|r| r["symbol"] == "QDVO").unwrap();
    assert_eq!(qdvo_row["declarationSource"].as_str().unwrap_or(""), "amplify");
    assert!(qdvo_row.get("lastRunAt").is_some());

    async fn set_declaration_source(platform: &LocalPlatform, symbol: &str, source: &str) {
        let got = execute_query_on(
            platform,
            platform,
            QueryRequest {
                contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
                query_name: "InvestmentGet".into(),
                correlation_id: Uuid::new_v4(),
                body_json: Some(serde_json::json!({"symbol": symbol}).to_string()),
            },
        )
        .await;
        assert!(got.ok, "{symbol} InvestmentGet: {:?}", got.error_code);
        let val: serde_json::Value =
            serde_json::from_str(got.body_json.as_deref().unwrap_or("{}")).unwrap();
        let security_id = val["securityId"].as_str().expect("securityId");
        must_ok(
            platform,
            "RetrievalTemplateSet",
            serde_json::json!({
                "securityId": security_id,
                "declarationSource": source,
                "calendarPolicy": "derived_walk"
            }),
        )
        .await;
    }

    for symbol in ["QDVO", "SPYI", "MSTY", "TOPW"] {
        set_declaration_source(&platform, symbol, "public").await;
    }
    let applied = must_ok(
        &platform,
        "ProviderDeclarationSourcesApply",
        serde_json::json!({}),
    )
    .await;
    assert!(applied["updated"].as_u64().unwrap_or(0) >= 4);
    assert_eq!(
        investment_template_source(&platform, "QDVO").await,
        ("amplify".into(), "issuer_calendar".into(), true)
    );
    assert_eq!(
        investment_template_source(&platform, "SPYI").await,
        ("neos".into(), "issuer_calendar".into(), true)
    );
    assert_eq!(
        investment_template_source(&platform, "MSTY").await,
        ("yieldmax".into(), "issuer_calendar".into(), true)
    );
    assert_eq!(
        investment_template_source(&platform, "TOPW").await,
        ("roundhill".into(), "issuer_calendar".into(), true)
    );
    assert_eq!(
        investment_template_source(&platform, "QYLD").await,
        ("globalx".into(), "issuer_calendar".into(), true)
    );
    assert_eq!(
        investment_template_source(&platform, "ENERGYX").await,
        ("sec-edgar".into(), "none".into(), false)
    );

    set_declaration_source(&platform, "QDVO", "yieldmax").await;
    must_ok(
        &platform,
        "ProviderDeclarationSourcesApply",
        serde_json::json!({}),
    )
    .await;
    assert_eq!(
        investment_template_source(&platform, "QDVO").await.0,
        "yieldmax",
        "Apply must not overwrite a registered vendor"
    );
    assert!(
        investment_template_source(&platform, "QDVO").await.2,
        "registered vendor must stay or become enabled"
    );
    set_declaration_source(&platform, "QDVO", "public").await;
    must_ok(
        &platform,
        "ProviderDeclarationSourcesApply",
        serde_json::json!({}),
    )
    .await;

    load_production_seed_via_commands(&platform, &production)
        .await
        .expect("second load is a no-op");
    let again = production_seed_actual_counts(&platform)
        .await
        .expect("counts after second load");
    assert_eq!(again, expected.counts);
}

#[tokio::test]
async fn robinhood_btc_splits_from_grayscale_btc() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .expect("open sqlite");
    let speculation = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Speculation", "kind": "taxable"}),
    )
    .await;
    let robinhood = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Robinhood", "kind": "taxable"}),
    )
    .await;
    let btc = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "BTC", "name": "Bitcoin"}),
    )
    .await;
    let spec_id = speculation["accountId"].as_str().unwrap();
    let rh_id = robinhood["accountId"].as_str().unwrap();
    let btc_id = btc["securityId"].as_str().unwrap();
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": spec_id,
            "securityId": btc_id,
            "openedOn": "2024-01-15",
            "origin": "purchase",
            "quantityMinor": 13,
            "quantityScale": 0,
            "performanceBasisMinor": 34_596,
            "taxBasisMinor": 34_596,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": rh_id,
            "securityId": btc_id,
            "openedOn": "2024-06-01",
            "origin": "purchase",
            "quantityMinor": 335_000,
            "quantityScale": 8,
            "performanceBasisMinor": 32_900,
            "taxBasisMinor": 32_900,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;
    must_ok(
        &platform,
        "PriceQuoteRecord",
        serde_json::json!({
            "securityId": btc_id,
            "priceMinor": 3408,
            "scale": 2,
            "asOfAt": "2026-08-21",
            "source": "yahoo"
        }),
    )
    .await;
    must_ok(
        &platform,
        "PriceQuoteRecord",
        serde_json::json!({
            "securityId": btc_id,
            "priceMinor": 7_851_293,
            "scale": 2,
            "asOfAt": "2026-08-24",
            "source": "yahoo"
        }),
    )
    .await;

    application_core::production_seed::ensure_btc_usd_split(&platform)
        .await
        .expect("split");
    application_core::production_seed::ensure_btc_usd_split(&platform)
        .await
        .expect("split is idempotent");

    let details = execute_query_on(
        &platform,
        &platform,
        QueryRequest {
            contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
            query_name: "PositionDetailsGet".into(),
            correlation_id: Uuid::new_v4(),
            body_json: None,
        },
    )
    .await;
    assert!(details.ok, "PositionDetailsGet: {:?}", details.error_code);
    let details_val: serde_json::Value =
        serde_json::from_str(details.body_json.as_deref().unwrap_or("{}")).unwrap();
    let positions = details_val["positions"].as_array().expect("positions");
    let spec = positions
        .iter()
        .find(|p| p["accountName"] == "Speculation")
        .expect("Speculation line");
    let rh = positions
        .iter()
        .find(|p| p["accountName"] == "Robinhood")
        .expect("Robinhood line");
    assert_eq!(spec["symbol"].as_str(), Some("BTC"));
    assert_eq!(rh["symbol"].as_str(), Some("BTC-USD"));

    let btc_price = execute_query_on(
        &platform,
        &platform,
        QueryRequest {
            contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
            query_name: "CurrentPriceGet".into(),
            correlation_id: Uuid::new_v4(),
            body_json: Some(
                serde_json::json!({ "securityId": btc_id, "asOfDate": "2026-08-24" }).to_string(),
            ),
        },
    )
    .await;
    assert!(btc_price.ok);
    let btc_price_val: serde_json::Value =
        serde_json::from_str(btc_price.body_json.as_deref().unwrap_or("{}")).unwrap();
    assert_eq!(btc_price_val["priceMinor"].as_i64(), Some(3408));

    let secs = execute_query_on(
        &platform,
        &platform,
        QueryRequest {
            contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
            query_name: "SecurityList".into(),
            correlation_id: Uuid::new_v4(),
            body_json: None,
        },
    )
    .await;
    let secs_val: serde_json::Value =
        serde_json::from_str(secs.body_json.as_deref().unwrap_or("[]")).unwrap();
    let usd = secs_val
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["symbol"] == "BTC-USD")
        .expect("BTC-USD registered");
    let usd_id = usd["securityId"].as_str().unwrap();
    let usd_price = execute_query_on(
        &platform,
        &platform,
        QueryRequest {
            contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
            query_name: "CurrentPriceGet".into(),
            correlation_id: Uuid::new_v4(),
            body_json: Some(
                serde_json::json!({ "securityId": usd_id, "asOfDate": "2026-08-24" }).to_string(),
            ),
        },
    )
    .await;
    let usd_price_val: serde_json::Value =
        serde_json::from_str(usd_price.body_json.as_deref().unwrap_or("{}")).unwrap();
    assert_eq!(usd_price_val["priceMinor"].as_i64(), Some(7_851_293));

    let basis = execute_query_on(
        &platform,
        &platform,
        QueryRequest {
            contract_version: FINANCE_CLIENT_CONTRACT_VERSION.to_string(),
            query_name: "BasisGet".into(),
            correlation_id: Uuid::new_v4(),
            body_json: None,
        },
    )
    .await;
    let basis_val: serde_json::Value =
        serde_json::from_str(basis.body_json.as_deref().unwrap_or("{}")).unwrap();
    assert_eq!(basis_val["lots"].as_array().map(|a| a.len()), Some(2));
}
