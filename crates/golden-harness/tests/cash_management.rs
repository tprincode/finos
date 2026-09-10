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
async fn cash_distribution_proves_net_and_week_total() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let income = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "ira"}),
    )
    .await;
    let posted = must_ok(
        &platform,
        "CashDistributionPost",
        serde_json::json!({
            "accountId": income["accountId"],
            "activityType": "IRA_Distribution",
            "occurredOn": "2026-09-12",
            "grossMinor": 100000,
            "federalWithholdingMinor": 18000,
            "stateWithholdingMinor": 4500,
            "scale": 2,
            "idempotencyKey": "cm-1-prove"
        }),
    )
    .await;
    assert_eq!(posted["amountMinor"].as_i64(), Some(100000));
    assert_eq!(posted["federalWithholdingMinor"].as_i64(), Some(18000));
    assert_eq!(posted["stateWithholdingMinor"].as_i64(), Some(4500));

    let week = query_json(
        &platform,
        "CashManagementWeekGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    assert_eq!(week["periodStart"], "2026-09-12");
    assert_eq!(week["periodEnd"], "2026-09-18");
    assert_eq!(week["weekGrossMinor"].as_i64(), Some(100000));
    assert_eq!(week["weekWithholdingMinor"].as_i64(), Some(22500));
    assert_eq!(week["weekNetMinor"].as_i64(), Some(77500));
    let row = week["rows"].as_array().unwrap().iter().find(|r| r["activityType"] == "IRA_Distribution").unwrap();
    assert_eq!(row["netMinor"].as_i64(), Some(77500));

    let trends = query_json(
        &platform,
        "TrendsGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    let lines = trends["distributions"]["lines"].as_array().unwrap();
    assert!(lines.iter().any(|l| l["activityType"] == "IRA_Distribution" && l["amountMinor"] == 100000));
    assert_eq!(trends["overview"]["profitMinor"].as_i64().unwrap_or(0), 0);

    must_ok(
        &platform,
        "MagiRuleSet",
        serde_json::json!({
            "thresholdMinor": 8_460_000,
            "safetyReserveMinor": 500_000,
            "scale": 2
        }),
    )
    .await;
    must_ok(
        &platform,
        "MagiFactRecord",
        serde_json::json!({
            "sourceId": "w2-cm3",
            "treatment": "include",
            "amountMinor": 300_000,
            "scale": 2,
            "category": "wages"
        }),
    )
    .await;
    must_ok(
        &platform,
        "MagiCoverageSet",
        serde_json::json!({
            "completeness": "complete",
            "remainingMinor": 0,
            "withholdingMinor": 0,
            "formTotalMinor": 300_000,
            "warnings": []
        }),
    )
    .await;
    let magi_before = query_json(&platform, "MagiProjectionGet", serde_json::json!({})).await;
    must_ok(
        &platform,
        "CashDistributionPost",
        serde_json::json!({
            "accountId": income["accountId"],
            "activityType": "IRA_Distribution",
            "occurredOn": "2026-09-13",
            "grossMinor": 50000,
            "federalWithholdingMinor": 9000,
            "stateWithholdingMinor": 0,
            "scale": 2,
            "idempotencyKey": "cm-3-magi"
        }),
    )
    .await;
    let magi_after = query_json(&platform, "MagiProjectionGet", serde_json::json!({})).await;
    assert_eq!(magi_before["decisionState"], magi_after["decisionState"]);
    assert_eq!(magi_before["actualIncludedYtd"], magi_after["actualIncludedYtd"]);
}

#[tokio::test]
async fn roth_refuses_withholding_and_unknown_gross() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let roth = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Roth", "kind": "roth"}),
    )
    .await;
    let refused = execute_command_on(
        &platform,
        &platform,
        cmd(
            "CashDistributionPost",
            serde_json::json!({
                "accountId": roth["accountId"],
                "activityType": "Roth_Distribution",
                "occurredOn": "2026-09-12",
                "grossMinor": 10000,
                "federalWithholdingMinor": 1,
                "stateWithholdingMinor": 0,
                "scale": 2
            }),
        ),
    )
    .await;
    assert!(!refused.ok);
    assert_eq!(refused.error_code.as_deref(), Some("roth_withholding_not_allowed"));

    let unknown = execute_command_on(
        &platform,
        &platform,
        cmd(
            "CashDistributionPost",
            serde_json::json!({
                "accountId": roth["accountId"],
                "activityType": "IRA_Distribution",
                "occurredOn": "2026-09-12",
                "scale": 2
            }),
        ),
    )
    .await;
    assert!(!unknown.ok);
    assert_eq!(unknown.error_code.as_deref(), Some("unknown_amount"));
}

#[tokio::test]
async fn saturday_draft_opens_until_income_ira_posts() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let income = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "ira"}),
    )
    .await;
    let reminders = query_json(
        &platform,
        "CashManagementRemindersGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    assert_eq!(reminders["saturdayDraft"]["open"], true);
    assert_eq!(reminders["saturdayDraft"]["activityType"], "IRA_Distribution");
    assert_eq!(reminders["saturdayDraft"]["occurredOn"], "2026-09-12");
    assert_eq!(
        reminders["saturdayDraft"]["suggestedAccountId"],
        income["accountId"]
    );
    must_ok(
        &platform,
        "CashDistributionPost",
        serde_json::json!({
            "accountId": income["accountId"],
            "activityType": "IRA_Distribution",
            "occurredOn": "2026-09-12",
            "grossMinor": 100000,
            "federalWithholdingMinor": 0,
            "stateWithholdingMinor": 0,
            "scale": 2
        }),
    )
    .await;
    let after = query_json(
        &platform,
        "CashManagementRemindersGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    assert_eq!(after["saturdayDraft"]["open"], false);
}

#[tokio::test]
async fn tom_ssa_confirm_miss_variance_and_june_extra() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let external = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "External", "kind": "external"}),
    )
    .await;
    let july = query_json(
        &platform,
        "CashManagementRemindersGet",
        serde_json::json!({"asOfDate": "2026-07-03"}),
    )
    .await;
    assert_eq!(july["tomSsa"]["label"], "Social Security retirement");
    assert_eq!(july["tomSsa"]["expectedMinor"].as_i64(), Some(286500));
    assert_eq!(july["tomSsa"]["status"], "unconfirmed");
    assert!(july["tomSsa"]["postedMinor"].is_null());
    assert_eq!(july["tomSsa"]["extraAudit"], false);

    let unknown = execute_command_on(
        &platform,
        &platform,
        cmd(
            "SsaConfirm",
            serde_json::json!({
                "accountId": external["accountId"],
                "occurredOn": "2026-07-03",
                "scale": 2
            }),
        ),
    )
    .await;
    assert!(!unknown.ok);
    assert_eq!(unknown.error_code.as_deref(), Some("unknown_amount"));
    let still = query_json(
        &platform,
        "CashManagementRemindersGet",
        serde_json::json!({"asOfDate": "2026-07-03"}),
    )
    .await;
    assert_eq!(still["tomSsa"]["status"], "unconfirmed");
    assert!(still["tomSsa"]["postedMinor"].is_null());

    let variance = must_ok(
        &platform,
        "SsaConfirm",
        serde_json::json!({
            "accountId": external["accountId"],
            "occurredOn": "2026-08-07",
            "receivedMinor": 280000,
            "scale": 2
        }),
    )
    .await;
    assert_eq!(variance["amountMinor"].as_i64(), Some(280000));
    let august = query_json(
        &platform,
        "CashManagementRemindersGet",
        serde_json::json!({"asOfDate": "2026-08-07"}),
    )
    .await;
    assert_eq!(august["tomSsa"]["status"], "variance");
    assert_eq!(august["tomSsa"]["postedMinor"].as_i64(), Some(280000));
    let exceptions = query_json(&platform, "ExceptionList", serde_json::json!({})).await;
    let items = exceptions
        .as_array()
        .cloned()
        .or_else(|| exceptions["exceptions"].as_array().cloned())
        .unwrap_or_default();
    assert!(items.iter().any(|e| {
        e["code"] == "ssa_amount_variance"
            && e["message"]
                .as_str()
                .unwrap_or("")
                .contains("Social Security retirement")
    }));

    must_ok(
        &platform,
        "CashDistributionPost",
        serde_json::json!({
            "accountId": external["accountId"],
            "activityType": "SSA",
            "occurredOn": "2026-06-05",
            "grossMinor": 286500,
            "federalWithholdingMinor": 0,
            "stateWithholdingMinor": 0,
            "scale": 2,
            "idempotencyKey": "seed-tom-june-5"
        }),
    )
    .await;
    must_ok(
        &platform,
        "CashDistributionPost",
        serde_json::json!({
            "accountId": external["accountId"],
            "activityType": "SSA",
            "occurredOn": "2026-06-26",
            "grossMinor": 286500,
            "federalWithholdingMinor": 0,
            "stateWithholdingMinor": 0,
            "scale": 2,
            "idempotencyKey": "seed-tom-june-26"
        }),
    )
    .await;
    let june = query_json(
        &platform,
        "CashManagementRemindersGet",
        serde_json::json!({"asOfDate": "2026-06-15"}),
    )
    .await;
    assert_eq!(june["tomSsa"]["status"], "confirmed");
    assert_eq!(june["tomSsa"]["postedMinor"].as_i64(), Some(286500));
    assert_eq!(june["tomSsa"]["extraAudit"], true);
    let extra = june["tomSsa"]["recent"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["occurredOn"] == "2026-06-26")
        .unwrap();
    assert_eq!(extra["extraAudit"], true);

    let july_after = query_json(
        &platform,
        "CashManagementRemindersGet",
        serde_json::json!({"asOfDate": "2026-07-03"}),
    )
    .await;
    assert_eq!(july_after["tomSsa"]["status"], "unconfirmed");
    assert!(july_after["tomSsa"]["postedMinor"].is_null());

    must_ok(
        &platform,
        "SsaConfirm",
        serde_json::json!({
            "accountId": external["accountId"],
            "occurredOn": "2026-07-03",
            "receivedMinor": 286500,
            "scale": 2
        }),
    )
    .await;
    let july_ok = query_json(
        &platform,
        "CashManagementRemindersGet",
        serde_json::json!({"asOfDate": "2026-07-03"}),
    )
    .await;
    assert_eq!(july_ok["tomSsa"]["status"], "confirmed");
    assert_eq!(july_ok["tomSsa"]["postedMinor"].as_i64(), Some(286500));
}

#[tokio::test]
async fn july_third_ssa_is_july_month_and_crossing_week() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let external = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "External", "kind": "external"}),
    )
    .await;
    must_ok(
        &platform,
        "CashDistributionPost",
        serde_json::json!({
            "accountId": external["accountId"],
            "activityType": "SSA",
            "occurredOn": "2026-07-03",
            "grossMinor": 133100,
            "federalWithholdingMinor": 0,
            "stateWithholdingMinor": 0,
            "scale": 2,
            "idempotencyKey": "barb-jul-3"
        }),
    )
    .await;
    let week = query_json(
        &platform,
        "CashManagementWeekGet",
        serde_json::json!({"asOfDate": "2026-07-03"}),
    )
    .await;
    assert_eq!(week["periodStart"], "2026-06-27");
    assert_eq!(week["periodEnd"], "2026-07-03");
    assert_eq!(week["weekGrossMinor"].as_i64(), Some(133100));
    let june = query_json(
        &platform,
        "CashManagementMonthGet",
        serde_json::json!({"asOfDate": "2026-06-15"}),
    )
    .await;
    assert_eq!(june["yearMonth"], "2026-06");
    assert_eq!(june["monthGrossMinor"].as_i64(), Some(0));
    let july = query_json(
        &platform,
        "CashManagementMonthGet",
        serde_json::json!({"asOfDate": "2026-07-03"}),
    )
    .await;
    assert_eq!(july["yearMonth"], "2026-07");
    assert_eq!(july["monthGrossMinor"].as_i64(), Some(133100));
    assert_eq!(july["rows"][0]["activityType"], "SSA");
    assert_eq!(july["rows"][0]["count"], 1);
}

#[test]
fn cash_management_ui_never_says_ssi() {
    let ui = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/CashManagement.tsx"),
    )
    .unwrap();
    assert!(ui.contains("Social Security retirement"));
    assert!(!ui.contains("SSI"));
    assert!(!ui.contains("ssi"));
    assert!(ui.contains("changes tax-payment, not Marketplace MAGI"));
    assert!(!ui.contains("MagiFactRecord"));
}
