use application_core::contracts::{
    CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::queries::{execute_command_on, execute_query_on};
use golden_harness::repo_root;
use import_engine::{
    dividend_gap, find_broker_header, parse_broker_csv_detail, parse_yield_sheet_csv, ActionClass,
    BrokerLayout, DividendFact, GapKind,
};
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

fn history_and_sheet() -> (String, String) {
    let root = repo_root();
    (
        std::fs::read_to_string(root.join("tests/golden/fixtures/fidelity-history-gap.csv")).unwrap(),
        std::fs::read_to_string(root.join("tests/golden/fixtures/roi-yield-gap.csv")).unwrap(),
    )
}

#[test]
fn cursor_gap_report_history_matches_roi_sheet_nothing_posted() {
    let (history, sheet_csv) = history_and_sheet();
    assert_eq!(
        find_broker_header(&history).map(|(l, _)| l),
        Some(BrokerLayout::Fidelity)
    );
    let parsed = parse_broker_csv_detail(BrokerLayout::Fidelity, &history, None).unwrap();
    assert_eq!(parsed.candidates.len(), 2, "{parsed:?}");
    let reasons: Vec<_> = parsed.dropped.iter().map(|d| d.reason).collect();
    assert!(reasons.contains(&"purchase"), "{reasons:?}");
    assert!(reasons.contains(&"journal"), "{reasons:?}");
    assert!(reasons.contains(&"tax"), "{reasons:?}");
    assert!(reasons.contains(&"reinvestment"), "{reasons:?}");
    let broker: Vec<_> = parsed
        .candidates
        .iter()
        .filter_map(DividendFact::from_candidate)
        .collect();
    let sheet = parse_yield_sheet_csv(&sheet_csv);
    let report = dividend_gap(&broker, &sheet, &[]);
    assert!(
        report.iter().all(|r| r.kind == GapKind::Match),
        "{report:?}"
    );
    assert_eq!(report.len(), 2);
}

#[tokio::test]
async fn cursor_import_posts_gap_and_second_file_does_not_double() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
    )
    .await;
    must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "NVDW", "name": "NVDW"}),
    )
    .await;
    must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "SPAXX", "name": "SPAXX"}),
    )
    .await;
    let (history, sheet_csv) = history_and_sheet();
    let staged = must_ok(
        &platform,
        "ImportStage",
        serde_json::json!({
            "sourceId": "fidelity-gap-1",
            "filename": "fidelity-history-gap.csv",
            "content": history,
        }),
    )
    .await;
    let batch_id = staged["batchId"].as_str().unwrap();
    for name in ["ImportValidate", "ImportApprove", "ImportPost"] {
        must_ok(&platform, name, serde_json::json!({"batchId": batch_id})).await;
    }
    let dividend = query_json(&platform, "DividendGet").await;
    assert_eq!(dividend["actualTotalMinor"].as_i64().unwrap(), 7_637);
    assert_eq!(dividend["actuals"].as_array().unwrap().len(), 2);

    let sheet = parse_yield_sheet_csv(&sheet_csv);
    let parsed = parse_broker_csv_detail(BrokerLayout::Fidelity, &history, None).unwrap();
    let broker: Vec<_> = parsed
        .candidates
        .iter()
        .filter_map(DividendFact::from_candidate)
        .collect();
    let posted: Vec<_> = dividend["actuals"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let amount_minor = row["amountMinor"].as_i64().unwrap();
            let occurred_on = row["occurredOn"].as_str().unwrap().to_string();
            let matched = broker
                .iter()
                .find(|b| b.amount_minor == amount_minor && b.occurred_on == occurred_on)
                .unwrap_or_else(|| panic!("posted {amount_minor} {occurred_on} not in broker"));
            matched.clone()
        })
        .collect();
    let report = dividend_gap(&broker, &sheet, &posted);
    assert!(
        report.iter().all(|r| r.kind == GapKind::AlreadyPosted),
        "{report:?}"
    );

    let staged2 = must_ok(
        &platform,
        "ImportStage",
        serde_json::json!({
            "sourceId": "fidelity-gap-2",
            "filename": "fidelity-history-gap-again.csv",
            "content": history,
        }),
    )
    .await;
    let batch2 = staged2["batchId"].as_str().unwrap();
    must_ok(&platform, "ImportValidate", serde_json::json!({"batchId": batch2})).await;
    must_ok(&platform, "ImportApprove", serde_json::json!({"batchId": batch2})).await;
    let posted2 = must_ok(&platform, "ImportPost", serde_json::json!({"batchId": batch2})).await;
    assert_eq!(posted2["skippedDuplicateCount"].as_u64(), Some(2));
    assert_eq!(posted2["postedCount"].as_u64(), Some(0));
    let lines = posted2["processLines"].as_array().unwrap();
    assert!(
        lines.iter().any(|l| l.as_str().unwrap_or("").contains("skipped duplicate")),
        "{lines:?}"
    );
    let again = query_json(&platform, "DividendGet").await;
    assert_eq!(again["actualTotalMinor"].as_i64().unwrap(), 7_637);
    assert_eq!(again["actuals"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn for_the_car_alias_posts_to_car() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Car", "kind": "taxable"}),
    )
    .await;
    must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "HAKY", "name": "HAKY"}),
    )
    .await;
    let csv = "Run Date,Account,Action,Symbol,Amount\n\
08-31-2026,For the CAR,DIVIDEND RECEIVED HAKY,HAKY,26.60\n";
    let staged = must_ok(
        &platform,
        "ImportStage",
        serde_json::json!({
            "sourceId": "car-alias",
            "filename": "car.csv",
            "content": csv,
        }),
    )
    .await;
    let batch_id = staged["batchId"].as_str().unwrap();
    for name in ["ImportValidate", "ImportApprove", "ImportPost"] {
        must_ok(&platform, name, serde_json::json!({"batchId": batch_id})).await;
    }
    let dividend = query_json(&platform, "DividendGet").await;
    assert_eq!(dividend["actualTotalMinor"].as_i64().unwrap(), 2_660);
}

#[test]
fn reinvestment_is_not_a_dividend_candidate() {
    assert_eq!(
        import_engine::classify_action(
            "REINVESTMENT FIDELITY GOVERNMENT MONEY MARKET (SPAXX) (Cash)"
        ),
        ActionClass::Drop("reinvestment")
    );
}

#[test]
fn fi_roth_crf_drip_is_not_a_cash_gap_fact() {
    let csv = "History: All Accounts\n\
From: 08/01/2026\n\
\n\
\"Date\",\"Account\",\"Symbol\",\"Description\",\"Quantity\",\"Price\",\"Amount\",\"Commission\",\"Fees\",\"Type\"\n\
\"08/06/2026\",\"ROTH IRA (00000)\",\"CRF\",\"DIVIDEND RECEIVED\",\"0\",\"$0.00\",\"$3.18\",\"$0.00\",\"$0.00\",\"Cash\"\n\
\"08/20/2026\",\"Income (00000)\",\"YMAX\",\"DIVIDEND RECEIVED\",\"0\",\"$0.00\",\"$5.21\",\"$0.00\",\"$0.00\",\"Cash\"\n";
    let parsed = parse_broker_csv_detail(BrokerLayout::Fidelity, csv, None).unwrap();
    let facts: Vec<_> = parsed
        .candidates
        .iter()
        .filter_map(DividendFact::from_candidate)
        .collect();
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].symbol, "YMAX");
    assert!(parsed
        .candidates
        .iter()
        .any(|c| c.symbol.as_deref() == Some("CRF") && c.activity_type == "drip"));
}
