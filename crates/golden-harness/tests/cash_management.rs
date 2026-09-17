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
    let ira_section = trends["distributions"]["sections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == "ira")
        .expect("ira tax section");
    assert!(ira_section["taxNote"].as_str().unwrap_or("").contains("Speculation"));
    assert!(
        trends["distributions"]["sections"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["id"] != "taxable" && s["label"] != "Taxable brokerage"),
        "prior-year 1099 must not create a taxable brokerage bucket: {trends}"
    );
    assert!(
        trends["distributions"]["accountTotals"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["accountName"] == "Income" && a["grossMinor"] == 100000)
    );
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
    assert_eq!(unknown.error_code.as_deref(), Some("cash_account_kind"));
}

#[tokio::test]
async fn cash_post_refuses_a_type_the_account_cannot_use() {
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
    let car = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Car", "kind": "taxable"}),
    )
    .await;
    let external = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "External", "kind": "taxable"}),
    )
    .await;
    let fi_roth = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "FI Roth", "kind": "fi_roth"}),
    )
    .await;

    let withdraw_ira = execute_command_on(
        &platform,
        &platform,
        cmd(
            "CashDistributionPost",
            serde_json::json!({
                "accountId": income["accountId"],
                "activityType": "Withdrawal",
                "occurredOn": "2026-09-12",
                "grossMinor": 10000,
                "federalWithholdingMinor": 0,
                "stateWithholdingMinor": 0,
                "scale": 2
            }),
        ),
    )
    .await;
    assert!(!withdraw_ira.ok, "owner must not withdraw from an IRA");
    assert_eq!(withdraw_ira.error_code.as_deref(), Some("cash_account_kind"));

    let ira_from_car = execute_command_on(
        &platform,
        &platform,
        cmd(
            "CashDistributionPost",
            serde_json::json!({
                "accountId": car["accountId"],
                "activityType": "IRA_Distribution",
                "occurredOn": "2026-09-12",
                "grossMinor": 10000,
                "federalWithholdingMinor": 0,
                "stateWithholdingMinor": 0,
                "scale": 2
            }),
        ),
    )
    .await;
    assert!(!ira_from_car.ok, "Car is taxable brokerage, not IRA");
    assert_eq!(ira_from_car.error_code.as_deref(), Some("cash_account_kind"));

    let ssa_on_car = execute_command_on(
        &platform,
        &platform,
        cmd(
            "SsaConfirm",
            serde_json::json!({
                "accountId": car["accountId"],
                "occurredOn": "2026-09-12",
                "receivedMinor": 286500,
                "scale": 2,
                "payee": "tom"
            }),
        ),
    )
    .await;
    assert!(!ssa_on_car.ok, "SSA only posts to External");
    assert_eq!(ssa_on_car.error_code.as_deref(), Some("cash_account_kind"));

    let withdraw_car = must_ok(
        &platform,
        "CashDistributionPost",
        serde_json::json!({
            "accountId": car["accountId"],
            "activityType": "Withdrawal",
            "occurredOn": "2026-09-12",
            "grossMinor": 10000,
            "federalWithholdingMinor": 0,
            "stateWithholdingMinor": 0,
            "scale": 2,
            "idempotencyKey": "cm-e-car-wd"
        }),
    )
    .await;
    assert_eq!(withdraw_car["amountMinor"].as_i64(), Some(10000));

    let roth_ok = must_ok(
        &platform,
        "CashDistributionPost",
        serde_json::json!({
            "accountId": fi_roth["accountId"],
            "activityType": "Roth_Distribution",
            "occurredOn": "2026-09-12",
            "grossMinor": 5000,
            "federalWithholdingMinor": 0,
            "stateWithholdingMinor": 0,
            "scale": 2,
            "idempotencyKey": "cm-e-roth"
        }),
    )
    .await;
    assert_eq!(roth_ok["amountMinor"].as_i64(), Some(5000));

    let ssa_ok = must_ok(
        &platform,
        "SsaConfirm",
        serde_json::json!({
            "accountId": external["accountId"],
            "occurredOn": "2026-09-12",
            "receivedMinor": 133100,
            "scale": 2,
            "payee": "barbara"
        }),
    )
    .await;
    assert_eq!(ssa_ok["amountMinor"].as_i64(), Some(133100));
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
    let payees = july_ok["ssaPayees"].as_array().expect("ssaPayees");
    assert!(
        payees.iter().any(|p| p["payee"] == "barbara" && p["status"] == "unconfirmed"),
        "Barbara stays unconfirmed when only Tom posted"
    );
}

#[tokio::test]
async fn barbara_and_tom_are_two_confirms_third_is_audit() {
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
        "SsaConfirm",
        serde_json::json!({
            "accountId": external["accountId"],
            "occurredOn": "2026-09-03",
            "receivedMinor": 133100,
            "scale": 2,
            "payee": "barbara"
        }),
    )
    .await;
    must_ok(
        &platform,
        "SsaConfirm",
        serde_json::json!({
            "accountId": external["accountId"],
            "occurredOn": "2026-09-10",
            "receivedMinor": 286500,
            "scale": 2,
            "payee": "tom"
        }),
    )
    .await;
    let sept = query_json(
        &platform,
        "CashManagementRemindersGet",
        serde_json::json!({"asOfDate": "2026-09-13"}),
    )
    .await;
    assert_eq!(sept["tomSsa"]["status"], "confirmed");
    assert_eq!(sept["tomSsa"]["extraAudit"], false);
    let payees = sept["ssaPayees"].as_array().unwrap();
    assert!(payees.iter().any(|p| {
        p["payee"] == "barbara"
            && p["status"] == "confirmed"
            && p["postedMinor"] == 133100
    }));
    assert!(payees.iter().any(|p| {
        p["payee"] == "tom" && p["status"] == "confirmed" && p["postedMinor"] == 286500
    }));
    must_ok(
        &platform,
        "CashDistributionPost",
        serde_json::json!({
            "accountId": external["accountId"],
            "activityType": "SSA",
            "occurredOn": "2026-09-15",
            "grossMinor": 10000,
            "federalWithholdingMinor": 0,
            "stateWithholdingMinor": 0,
            "scale": 2,
            "idempotencyKey": "extra-ssa-sept"
        }),
    )
    .await;
    let after = query_json(
        &platform,
        "CashManagementRemindersGet",
        serde_json::json!({"asOfDate": "2026-09-15"}),
    )
    .await;
    assert_eq!(after["tomSsa"]["extraAudit"], true);
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

#[tokio::test]
async fn imported_disbursement_matches_manual_post_identity() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    must_ok(
        &platform,
        "ProductionSeedLoad",
        serde_json::json!({
            "accounts": [{"name": "Income", "kind": "ira"}],
            "securities": [],
            "lots": [],
            "yieldBatches": [],
            "disbursements": [{
                "accountName": "Income",
                "activityType": "IRA_Distribution",
                "amountMinor": 100000,
                "scale": 2,
                "occurredOn": "2026-09-12",
                "idempotencyKey": "production-disb-cm4-import",
                "federalWithholdingMinor": 18000,
                "stateWithholdingMinor": 4500
            }]
        }),
    )
    .await;
    let accounts = query_json(&platform, "AccountList", serde_json::json!({})).await;
    let account_id = accounts
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["name"] == "Income")
        .expect("Income")["accountId"]
        .clone();
    let twin = must_ok(
        &platform,
        "CashDistributionPost",
        serde_json::json!({
            "accountId": account_id,
            "activityType": "IRA_Distribution",
            "occurredOn": "2026-10-03",
            "grossMinor": 100000,
            "federalWithholdingMinor": 18000,
            "stateWithholdingMinor": 4500,
            "scale": 2,
            "idempotencyKey": "cm-4-manual-twin"
        }),
    )
    .await;
    assert_eq!(twin["amountMinor"].as_i64(), Some(100000));
    assert_eq!(twin["federalWithholdingMinor"].as_i64(), Some(18000));
    assert_eq!(twin["stateWithholdingMinor"].as_i64(), Some(4500));

    let imported_week = query_json(
        &platform,
        "CashManagementWeekGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    let imported = imported_week["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["activityType"] == "IRA_Distribution")
        .expect("imported week row");
    let manual_week = query_json(
        &platform,
        "CashManagementWeekGet",
        serde_json::json!({"asOfDate": "2026-10-03"}),
    )
    .await;
    let manual = manual_week["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["activityType"] == "IRA_Distribution")
        .expect("manual week row");
    for key in [
        "accountName",
        "activityType",
        "grossMinor",
        "federalWithholdingMinor",
        "stateWithholdingMinor",
        "netMinor",
    ] {
        assert_eq!(imported[key], manual[key], "{key}");
    }
    assert_eq!(imported["occurredOn"], "2026-09-12");
    assert_eq!(manual["occurredOn"], "2026-10-03");
    assert_eq!(imported_week["weekNetMinor"].as_i64(), Some(77500));
    assert_eq!(manual_week["weekNetMinor"].as_i64(), Some(77500));

    let imported_month = query_json(
        &platform,
        "CashManagementMonthGet",
        serde_json::json!({"asOfDate": "2026-09-12"}),
    )
    .await;
    let manual_month = query_json(
        &platform,
        "CashManagementMonthGet",
        serde_json::json!({"asOfDate": "2026-10-03"}),
    )
    .await;
    assert_eq!(imported_month["monthGrossMinor"], manual_month["monthGrossMinor"]);
    assert_eq!(imported_month["monthWithholdingMinor"], manual_month["monthWithholdingMinor"]);
    assert_eq!(imported_month["monthNetMinor"], manual_month["monthNetMinor"]);
    assert_eq!(imported_month["monthNetMinor"].as_i64(), Some(77500));

    let trends = query_json(
        &platform,
        "TrendsGet",
        serde_json::json!({"asOfDate": "2026-10-03"}),
    )
    .await;
    let lines = trends["distributions"]["lines"].as_array().unwrap();
    assert!(
        lines.iter().any(|l| {
            l["activityType"] == "IRA_Distribution"
                && l["occurredOn"] == "2026-09-12"
                && l["amountMinor"] == 100000
                && l["federalWithholdingMinor"] == 18000
                && l["stateWithholdingMinor"] == 4500
                && l["netMinor"] == 77500
        }),
        "{trends}"
    );
    assert_eq!(trends["taxMonitor"]["federalWithholdingMinor"].as_i64(), Some(36000));
    assert_eq!(trends["distributions"]["netMinor"].as_i64(), Some(155000));
}

#[tokio::test]
async fn prior_year_1099_is_not_current_year_ytd_and_car_splits_holding_term() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let external = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "External", "kind": "taxable"}),
    )
    .await;
    let car = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Car", "kind": "taxable"}),
    )
    .await;
    must_ok(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": external["accountId"],
            "activityType": "Form_1099",
            "amountMinor": 262500,
            "scale": 2,
            "occurredOn": "2026-05-08",
            "idempotencyKey": "ty2025-1099"
        }),
    )
    .await;
    must_ok(
        &platform,
        "CashDistributionPost",
        serde_json::json!({
            "accountId": car["accountId"],
            "activityType": "Withdrawal",
            "occurredOn": "2026-05-15",
            "grossMinor": 85000,
            "federalWithholdingMinor": 0,
            "stateWithholdingMinor": 0,
            "scale": 2,
            "idempotencyKey": "car-wd"
        }),
    )
    .await;
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "HAKY", "name": "HAKY"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap();
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "sourceSymbol": "HAKY",
            "declarationSource": "amplify",
            "sourceUrl": "https://amplifyetfs.com/haky/#distributions",
            "calendarPolicy": "derived_walk",
            "collectorEnabled": true,
            "lookbackCount": 12
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot(&platform, security_id, "HAKY")
        .await
        .expect("complete");
    let lot = must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": car["accountId"],
            "securityId": security_id,
            "openedOn": "2025-03-01",
            "quantityMinor": 10,
            "quantityScale": 0,
            "performanceBasisMinor": 100000,
            "taxBasisMinor": 80000,
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
            "divType": "DIV-1",
            "rocPct2026EstimateMinor": 7000,
            "rocScale": 2,
            "isActive": true
        }),
    )
    .await;
    let sell = must_ok(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": car["accountId"],
            "securityId": security_id,
            "activityType": "sell",
            "amountMinor": 60000,
            "scale": 2,
            "occurredOn": "2026-03-02"
        }),
    )
    .await;
    must_ok(
        &platform,
        "LotAssign",
        serde_json::json!({
            "lotId": lot["lotId"],
            "activityId": sell["activityId"],
            "quantityMinor": 10,
            "quantityScale": 0
        }),
    )
    .await;

    let trends = query_json(
        &platform,
        "TrendsGet",
        serde_json::json!({"asOfDate": "2026-09-13"}),
    )
    .await;
    let lines = trends["distributions"]["lines"].as_array().unwrap();
    assert!(
        lines.iter().all(|l| l["activityType"] != "Form_1099"),
        "2025 1099 never applies to current-year YTD: {trends}"
    );
    assert!(
        trends["distributions"]["sections"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["id"] != "taxable"),
        "{trends}"
    );
    assert!(
        lines.iter().any(|l| l["activityType"] == "Withdrawal" && l["accountName"] == "Car"),
        "{trends}"
    );

    let car_plan = query_json(
        &platform,
        "CarRocPlanGet",
        serde_json::json!({ "asOfDate": "2026-09-13" }),
    )
    .await;
    assert_eq!(
        car_plan["ytdLongTermGainMinor"].as_i64(),
        Some(-20_000),
        "sold after one-year anniversary is long-term tax lot: {car_plan}"
    );
    assert_eq!(car_plan["ytdShortTermGainMinor"].as_i64(), Some(0), "{car_plan}");
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

#[test]
fn trends_distribution_tax_blocks_are_read_only_cm_summaries() {
    let cm = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/CashManagement.tsx"),
    )
    .unwrap();
    let trends = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/features/graphing/TrendsCharts.tsx"),
    )
    .unwrap();
    assert!(cm.contains("Distributions (YTD)"));
    assert!(cm.contains("Tax / ACA monitor"));
    assert!(!cm.contains("Read-only Cash Management summary"));
    assert!(cm.contains("Fed WH"));
    assert!(cm.contains("State WH"));
    assert!(cm.contains("Federal withholding"));
    assert!(cm.contains("aria-label=\"Cash Management distributions YTD\""));
    assert!(cm.contains("aria-label=\"Distribution account totals\""));
    assert!(cm.contains("aria-label=\"Distribution tax sections\""));
    assert!(cm.contains("aria-label=\"Cash Management tax and ACA monitor\""));
    assert!(cm.contains("aria-label=\"Cash Management Car ROC plan\""));
    assert!(cm.contains("<h3>Car ROC plan</h3>"));
    assert!(cm.contains("<dt>Remaining ordinary</dt>"));
    assert!(cm.contains("<dt>Remaining ROC</dt>"));
    assert!(cm.contains("<dt>YTD ordinary (estimate)</dt>"));
    assert!(cm.contains("<dt>YTD ROC (estimate)</dt>"));
    assert!(cm.contains("<dt>Long-term capital gain/loss</dt>"));
    assert!(cm.contains("<dt>Short-term capital gain/loss</dt>"));
    assert!(!cm.contains("taxable brokerage stay"));
    assert!(!cm.contains("rocPct2026Actual"));
    assert!(!cm.contains("Prior-year 1099 is ROC guidance"));
    assert!(!cm.contains("carRocPlan.estimateNote"));
    assert!(!cm.contains("carRocPlan.taxNote"));
    assert!(!cm.contains("carRocPlan.lotSaleNote"));
    assert!(!cm.contains("section.taxNote"));
    assert!(!cm.contains("taxMonitor.note"));
    assert!(!trends.contains("Distributions (YTD)"));
    assert!(!trends.contains("Tax / ACA monitor"));
    assert!(!trends.contains("Read-only Cash Management summary"));
    assert!(!trends.contains("Saved Trends weeks"));
    assert!(!trends.contains("FID+SCH"));
    assert!(cm.contains("CashWeekDesk") || cm.contains("weekDesk"));
    assert!(cm.contains("Cash week follow-up") || cm.contains("cash-follow-up"));
}
