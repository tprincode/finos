//! DIV-1 compliance summary and production symbol adapter map.

use application_core::contracts::{
    CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::queries::{execute_command_on, execute_query_on};
use financial_domain::div1::{
    declaration_source_for_provider, is_div1, is_registered_declaration_source,
};
use import_engine::parse_production_templates;
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

#[tokio::test]
async fn div1_compliance_summary_query_returns_rows_shape() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .expect("open sqlite");
    let account = execute_command_on(
        &platform,
        &platform,
        cmd(
            "AccountRegister",
            serde_json::json!({"name": "Income", "kind": "taxable"}),
        ),
    )
    .await;
    assert!(account.ok);
    let account_body: serde_json::Value =
        serde_json::from_str(account.body_json.as_deref().unwrap_or("{}")).unwrap();
    let security = execute_command_on(
        &platform,
        &platform,
        cmd(
            "SecurityRegister",
            serde_json::json!({"symbol": "NVDW", "name": "NVDW"}),
        ),
    )
    .await;
    assert!(security.ok);
    let body: serde_json::Value =
        serde_json::from_str(security.body_json.as_deref().unwrap_or("{}")).unwrap();
    let security_id = body["securityId"].as_str().unwrap();
    execute_command_on(
        &platform,
        &platform,
        cmd(
            "LotOpen",
            serde_json::json!({
                "accountId": account_body["accountId"],
                "securityId": security_id,
                "openedOn": "2026-01-02",
                "quantityMinor": 100,
                "quantityScale": 0,
                "performanceCostMinor": 10000,
                "taxCostMinor": 10000,
                "scale": 2
            }),
        ),
    )
    .await;
    execute_command_on(
        &platform,
        &platform,
        cmd(
            "PositionCharacteristicUpsert",
            serde_json::json!({
                "securityId": security_id,
                "paymentFrequency": "Weekly",
                "provider": "Roundhill",
                "divType": "DIV-1",
            }),
        ),
    )
    .await;
    execute_command_on(
        &platform,
        &platform,
        cmd(
            "RetrievalTemplateSet",
            serde_json::json!({
                "securityId": security_id,
                "declarationSource": "roundhill",
                "collectorEnabled": true,
            }),
        ),
    )
    .await;
    for (i, period) in ["2026-08-01", "2026-08-08", "2026-08-15"].iter().enumerate() {
        execute_command_on(
            &platform,
            &platform,
            cmd(
                "IssuerDeclarationRecord",
                serde_json::json!({
                    "securityId": security_id,
                    "amountPerShareMinor": 3500 + i as i64,
                    "amountScale": 4,
                    "paymentPeriod": period,
                    "source": "roundhill",
                    "enteredAt": "2026-08-28",
                }),
            ),
        )
        .await;
    }
    execute_command_on(
        &platform,
        &platform,
        cmd(
            "IssuerPayDateReplace",
            serde_json::json!({
                "securityId": security_id,
                "asOfDate": "2026-08-28",
                "payDates": [
                    {"payOn": "2026-09-05", "source": "roundhill"},
                    {"payOn": "2026-09-12", "source": "roundhill"},
                ],
            }),
        ),
    )
    .await;

    let result = execute_query_on(
        &platform,
        &platform,
        qry("Div1ComplianceSummaryGet", serde_json::json!({})),
    )
    .await;
    assert!(result.ok, "{:?}", result.error_code);
    let summary: serde_json::Value =
        serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap();
    let rows = summary["rows"].as_array().expect("rows");
    let nvdw = rows
        .iter()
        .find(|r| r["symbol"] == "NVDW")
        .expect("NVDW row");
    assert_eq!(nvdw["priorDeclarationsQty"], 3);
    assert!(nvdw["futurePayDatesQty"].as_u64().unwrap() >= 2);
    assert_eq!(nvdw["currentDeclarationDate"], "2026-08-15");
}

#[test]
#[ignore = "live network: run with cargo test -p golden-harness div1_live_adapter_fleet_audit -- --ignored --nocapture"]
fn div1_live_adapter_fleet_audit() {
    use std::process::Command;
    let status = Command::new("cargo")
        .args([
            "run",
            "-p",
            "golden-harness",
            "--bin",
            "div1-adapter-audit",
        ])
        .status()
        .expect("spawn div1-adapter-audit");
    assert!(status.success(), "adapter fleet audit reported misses");
}

#[test]
fn production_div1_symbols_map_to_adapters() {
    let production = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../database/seed/production");
    let doc = parse_production_templates(&production).expect("production templates");
    let div1: Vec<_> = doc
        .characteristics
        .iter()
        .filter(|c| is_div1(&c.div_type))
        .collect();
    assert_eq!(div1.len(), 39);
    for row in &div1 {
        let src = declaration_source_for_provider(&row.provider)
            .unwrap_or_else(|| panic!("{} provider {}", row.symbol, row.provider));
        assert!(is_registered_declaration_source(src), "{} -> {src}", row.symbol);
    }
}
