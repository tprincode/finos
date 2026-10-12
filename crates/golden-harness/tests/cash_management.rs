use application_core::contracts::{
    CommandRequest, PlannedOccurrenceRecord, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::ports::canonical::Canonical;
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
    assert_eq!(june["tomSsa"]["extraAudit"], false);
    let extra = june["tomSsa"]["recent"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["occurredOn"] == "2026-06-26")
        .unwrap();
    assert_eq!(extra["extraAudit"], false);
    assert_eq!(extra["payee"], "tom");

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
    // Bought and sold on 2026-04-10. IRS calls that short-term; the gain must
    // not vanish into a confident $0.
    let same_day_lot = must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": car["accountId"],
            "securityId": security_id,
            "openedOn": "2026-04-10",
            "quantityMinor": 5,
            "quantityScale": 0,
            "performanceBasisMinor": 50000,
            "taxBasisMinor": 50000,
            "scale": 2
        }),
    )
    .await;
    let same_day_sell = must_ok(
        &platform,
        "ActivityPost",
        serde_json::json!({
            "accountId": car["accountId"],
            "securityId": security_id,
            "activityType": "sell",
            "amountMinor": 42000,
            "scale": 2,
            "occurredOn": "2026-04-10"
        }),
    )
    .await;
    must_ok(
        &platform,
        "LotAssign",
        serde_json::json!({
            "lotId": same_day_lot["lotId"],
            "activityId": same_day_sell["activityId"],
            "quantityMinor": 5,
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
    assert_eq!(
        car_plan["ytdShortTermGainMinor"].as_i64(),
        Some(-8_000),
        "same-day round trip is a short-term sale, not a dropped $0: {car_plan}"
    );
    assert_eq!(car_plan["lotSaleCount"].as_u64(), Some(2), "{car_plan}");
    assert_eq!(
        car_plan["lotSalePlMinor"].as_i64(),
        Some(-28_000),
        "{car_plan}"
    );

    let plan = query_json(
        &platform,
        "TaxPlanningGet",
        serde_json::json!({ "asOfDate": "2026-09-13" }),
    )
    .await;
    let row = |key: &str| {
        plan["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["key"] == key)
            .cloned()
            .unwrap_or_default()
    };
    assert_eq!(row("ltcg")["totalMinor"].as_i64(), Some(-20_000), "{plan}");
    assert_eq!(row("stcg")["totalMinor"].as_i64(), Some(-8_000), "{plan}");
    assert_eq!(plan["netCapitalGainMinor"].as_i64(), Some(-28_000), "{plan}");
    assert_eq!(
        plan["capitalGainMagiMinor"].as_i64(),
        Some(-28_000),
        "a $280.00 loss is under the $3,000.00 limit: {plan}"
    );
    assert_eq!(
        plan["capitalLossCarryforwardMinor"].as_i64(),
        Some(0),
        "{plan}"
    );
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
    assert!(cm.contains("aria-label=\"Cash Management Tax Planning\""));
    assert!(cm.contains("aria-label=\"Car account tax planning\""));
    assert!(cm.contains("HouseholdIncomeReport"));
    let report = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/features/cash/HouseholdIncomeReport.tsx"),
    )
    .unwrap();
    let forecast = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/features/cash/magiForecast.ts"),
    )
    .unwrap();
    let src = format!("{report}\n{forecast}");
    assert!(report.contains("aria-label=\"Tax Planning income\""));
    assert!(report.contains("aria-label=\"Tax Planning MAGI\""));
    assert!(
        report.contains("Barbara LTCG")
            && report.contains("Long-term capital gains")
            && report.contains("Short-term capital gains"),
        "household income lists Barbara and Car gains"
    );
    assert!(
        report.contains("Traditional IRA contribution")
            && src.contains("plan.iraContributionMinor")
            && !src.contains("IRA_CONTRIBUTION_MINOR"),
        "IRA contribution is the sum of ira-contribution activities"
    );
    assert!(
        !report.contains("770_000")
            && !report.contains("7,700")
            && !report.contains("23,000")
            && !report.contains("23000"),
        "do not invent a $7,700 IRA contribution or a $23k premium"
    );
    assert!(
        report.contains("APTC_ACCRUAL_DAY = 25")
            && report.contains("aptcYtdMinor")
            && report.contains("13,540"),
        "APTC YTD accrues on the 25th; 9 months of $18,054 is $13,540"
    );
    assert!(!report.contains("divorce") && !report.contains("Divorce"));
    assert!(report.contains("1095-A"));
    // The heading carries the nav anchor now, so match its text, not the bare tag.
    let income_at = report.find(">Income</h4>").expect("Income heading");
    let aptc_at = report
        .find("Marketplace application and 1095-A")
        .expect("1095-A heading");
    assert!(
        aptc_at > income_at,
        "Marketplace application and 1095-A sit below Income"
    );
    assert!(
        report.contains("Application versus current MAGI")
            && report.contains("APPLICATION_MAGI_MINOR = 8_084_400")
            && src.contains("APPLICATION_APTC_MINOR = 1_805_400"),
        "application MAGI $80,844 and APTC $18,054 are the awarded facts"
    );
    assert!(
        report.contains("aria-label=\"MAGI forecast\"")
            && report.contains("MAGI after estimates vs")
            && report.contains("aria-label=\"MAGI confidence\"")
            && src.contains("isIndeterminate")
            && src.contains("Book the missing fact")
            && report.contains("Estimate")
            && !src.contains("if (isIndeterminate) return null")
            && !src.contains("decisionState === \"INDETERMINATE\" ? null"),
        "INDETERMINATE still fills the hero and labels Estimate"
    );
    assert!(
        report.contains("aria-label=\"MAGI suggestions\"")
            && src.contains("Cut remaining Traditional IRA draws by")
            && src.contains("Or book a Traditional IRA contribution of")
            && src.contains("Or both, split")
            && !src.contains("Roth contribution")
            && report.contains("Medical expenses")
            && report.contains("No effect"),
        "over-cliff suggestions are cut-IRA / contribute-IRA; medical is not a MAGI cut"
    );
    assert!(
        report.contains("aria-label=\"MAGI estimates\"")
            && report.contains("household-estimate-tag")
            && report.contains("Estimate rows feed the forecast tile"),
        "scratch HSA / computer / SE / IRA / Barbara stay labeled Estimate"
    );
    assert!(
        cm.contains("This week confirmed transactions")
            && cm.contains("Add cash activity")
            && !cm.contains("Cash management month")
            && !cm.contains("weekGrossMinor"),
        "Week desk lists confirmed transactions; Add cash activity stays on weekly"
    );
    assert!(
        cm.contains("if (desk === \"car\")"),
        "Car Account Tax Planning is display-only"
    );
    assert!(cm.contains("YTD"));
    assert!(cm.contains("Planned"));
    assert!(cm.contains("Total YTD + Planned"));
    assert!(cm.contains("Ordinary"));
    assert!(cm.contains("Long Term Capital Gains"));
    assert!(cm.contains("Short Term Capital Gains"));
    assert!(cm.contains("formatCarUsd"));
    let car_usd = cm
        .split("function formatCarUsd(")
        .nth(1)
        .expect("formatCarUsd")
        .split("function formatRocCell(")
        .next()
        .expect("formatCarUsd body");
    assert!(
        !car_usd.contains("?? 0") && car_usd.contains("\"unknown\""),
        "a null car amount shows the reason or unknown, not $0"
    );
    assert!(!cm.contains("ytdOrdinaryMinor ?? 0"));
    assert!(!cm.contains("remainingOrdinaryMinor ?? 0"));
    assert!(!cm.contains("ytdLongTermGainMinor ?? 0"));
    assert!(!cm.contains("ytdShortTermGainMinor ?? 0"));
    assert!(cm.contains("addKnown(plan.ytdOrdinaryMinor, plan.remainingOrdinaryMinor)"));
    assert!(cm.contains("addKnown(longYtd, longPlanned)"));
    assert!(cm.contains("addKnown(shortYtd, shortPlanned)"));
    assert!(cm.contains("longYtd == null ? null : 0"));
    assert!(cm.contains("shortYtd == null ? null : 0"));
    assert!(cm.contains("ytdRocUnknownReason"));
    assert!(!cm.contains("ytdRocMinor ?? 0"));
    assert!(!cm.contains("remainingRocMinor ?? 0"));
    assert!(!cm.contains("YTD ROC unknown reason"));
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
    assert!(
        cm.contains("This week confirmed transactions"),
        "confirmed week table stays on the System update week desk"
    );
}

/// A missing ordinary, long-term, or short-term cell stays unknown. A real zero still prints.
#[test]
fn car_tax_null_is_unknown_not_zero() {
    let cm = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/CashManagement.tsx"),
    )
    .unwrap();
    let car_usd = cm
        .split("function formatCarUsd(")
        .nth(1)
        .expect("formatCarUsd")
        .split("function formatRocCell(")
        .next()
        .expect("formatCarUsd body");
    assert!(
        !car_usd.contains("?? 0") && car_usd.contains("\"unknown\""),
        "a null car amount shows the reason or unknown, not $0"
    );
    assert!(cm.contains("function formatRocCell("));
    assert!(!cm.contains("ytdOrdinaryMinor ?? 0"));
    assert!(!cm.contains("remainingOrdinaryMinor ?? 0"));
    assert!(!cm.contains("ytdLongTermGainMinor ?? 0"));
    assert!(!cm.contains("ytdShortTermGainMinor ?? 0"));
    assert!(cm.contains("addKnown(plan.ytdOrdinaryMinor, plan.remainingOrdinaryMinor)"));
    assert!(cm.contains("addKnown(longYtd, longPlanned)"));
    assert!(cm.contains("addKnown(shortYtd, shortPlanned)"));
    assert!(cm.contains("longYtd == null ? null : 0"));
    assert!(cm.contains("shortYtd == null ? null : 0"));
    assert!(cm.contains("plan.lotSaleNote"));
}

#[test]
fn tax_lot_sale_gain_converts_assignment_scale_to_cents() {
    let queries = std::fs::read_to_string(
        golden_harness::repo_root().join("crates/application-core/src/queries.rs"),
    )
    .unwrap();
    let lot = std::fs::read_to_string(
        golden_harness::repo_root().join("crates/financial-domain/src/lot.rs"),
    )
    .unwrap();
    assert!(
        queries.contains("assignment_tax_gain_cents("),
        "Car/Tax Planning must convert lot-scale cost before proceeds − cost"
    );
    assert!(
        lot.contains("fn assignment_tax_gain_cents"),
        "scale-6 TSLW cost is $350.81, not $3.5M"
    );
}

#[test]
fn cct_open_table_drops_duplicate_bill_pay_column() {
    let register = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/features/cash/ExternalRegister.tsx"),
    )
    .unwrap();
    assert!(
        register.contains("Bill Pay Deposit") && register.contains("Bill Pay Withdrawal"),
        "keep deposit and withdrawal"
    );
    assert!(
        !register.contains("sortStepHead(\"Bill\", \"Pay\", \"billpay\")")
            && !register.contains("stepTotal(\"Bill Pay\", \"billpay\""),
        "Bill Pay column is a duplicate of Bill Pay Deposit"
    );
}

#[test]
fn cct_mark_and_unmark_steps_confirm_data_effects() {
    let register = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/features/cash/ExternalRegister.tsx"),
    )
    .unwrap();
    let storage = std::fs::read_to_string(
        golden_harness::repo_root().join("crates/storage-sqlite/src/external_register.rs"),
    )
    .unwrap();
    let queries = std::fs::read_to_string(
        golden_harness::repo_root().join("crates/application-core/src/queries.rs"),
    )
    .unwrap();
    // First-mark: free multi-select + Mark pill total + immediate ExternalRegisterMarkStep.
    // Unmark/change: edit row → confirm → stage → Save (protects Estimated balances / Debt planner).
    assert!(
        register.contains("commitMarkStepNow")
            && register.contains("ExternalRegisterMarkStep")
            && register.contains("Marks selected open charges now")
            && register.contains("Select for Mark total, then use the Mark button")
            && register.contains("Select for ${label}")
            && register.contains("toggleId(line.lineId, ids, setIds)")
            && register.contains("done ? rowEditing : allowSelect")
            && register.contains("checked={done || selected}")
            && register.contains("onClick={() => void commitMarkStepNow(ids, step, label)}")
            && register.contains("data-part=\"cct-step-confirm\"")
            && register.contains("requestUnmarkStep")
            && register.contains("ExternalRegisterUnmarkStep")
            && register.contains("Confirm Unmark")
            && !register.contains("Confirm Mark")
            && !register.contains("requestMarkStep")
            && register.contains("stepImpactLines")
            && register.contains("stepLinkageOf")
            && register.contains("Estimated account Balances")
            && register.contains("Reduces Estimated account Balances")
            && register.contains("Increases Estimated account Balances")
            && register.contains("Progress only. Does not change Estimated account Balances.")
            && register.contains("Simple CCT charge")
            && register.contains("Does not change Debt planner.")
            && register.contains("Mom escrow (Debt planner)")
            && register.contains("Linked loan (Debt planner)")
            && register.contains("Edit the row before unmarking a step")
            && register.contains("editingId === line.lineId")
            && register.contains("Save the row to commit. Cancel discards.")
            && register.contains("unmarked (staged) — Save to commit")
            && register.contains("applyStepToLine")
            && register.contains("commitStepDeltas")
            && register.contains("stepsChanged")
            && register.contains("returns to Open")
            && register.contains("aria-label=\"Step data effects\"")
            && register.contains("Unmark ${label}")
            && register.contains("aria-label={`Mark ${label}`}")
            && !register.contains("disabled={done || busy}")
            && !register.contains("Credit / Mom:"),
        "First-mark selects freely and Mark commits now; unmark stays edit+confirm+Save"
    );
    assert!(
        storage.contains("pub async fn unmark_step")
            && storage.contains("unapply_line_if_present")
            && storage.contains("unapply_account_line")
            && queries.contains("ExternalRegisterUnmarkStep"),
        "Unmark must clear steps, reverse apply, and wire the command"
    );
}

#[test]
fn cct_open_shows_pay_type_subtotals_above_mark_steps() {
    let register = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/features/cash/ExternalRegister.tsx"),
    )
    .unwrap();
    let css = std::fs::read_to_string(golden_harness::repo_root().join("apps/desktop/src/App.css"))
        .unwrap();
    assert!(
        register.contains("className=\"external-open-head\"")
            && register.contains("stepTotal(\"Transfer\"")
            && register.contains("aria-label=\"Estimated account Balances\"")
            && register.contains("data-part=\"estimated-account-balances\"")
            && register.contains("Estimated account Balances")
            && register.contains("openPayTypeTotals")
            && register.contains("external-open-paytype-totals")
            && register.contains("Estimated account Balances: open lines without Pay Bill")
            && register.contains("view.stepPay || line.stepPay")
            && register.contains("openCutoff")
            && register.contains("for (const line of completed)")
            && register.contains("view.amountMinor >= 0")
            && register.contains("on < cutoff")
            && register.contains("bump(line, false)")
            && !register.contains("aria-label=\"Open totals by pay type\""),
        "Estimated account Balances excludes Pay Bill lines; CAP can show 0 while still Open"
    );
    assert!(
        css.contains(".external-open-head .external-open-paytype-totals")
            && css.contains("width: max-content")
            && css.contains("border:")
            && css.contains(".external-open-paytype-total"),
        "pay-type subtotals are a fit-content box after the four Mark pills"
    );
    let account = std::fs::read_to_string(
        golden_harness::repo_root().join("crates/application-core/src/external_account.rs"),
    )
    .unwrap();
    assert!(
        account.contains("pub fn line_matches")
            && account.contains("loan_name == key")
            && !register.contains("line_matches"),
        "pay-type account totals stay in the CCT UI; debt match/reconcile is untouched"
    );
}

#[test]
fn completed_section_stamps_and_greens_only_the_completed_date() {
    let register = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/features/cash/ExternalRegister.tsx"),
    )
    .unwrap();
    let css = std::fs::read_to_string(golden_harness::repo_root().join("apps/desktop/src/App.css"))
        .unwrap();
    let storage = std::fs::read_to_string(
        golden_harness::repo_root().join("crates/storage-sqlite/src/external_register.rs"),
    )
    .unwrap();
    assert!(
        register.contains("completedDateOf")
            && register.contains("className=\"external-green-date\"")
            && register.contains("aria-label=\"Completed external charges\""),
        "Completed table must show a dated green Completed cell"
    );
    assert!(
        css.contains(".external-register-wrap tr.is-complete td")
            && css.contains("background: transparent")
            && css.contains("td.external-green-date")
            && css.contains("background: #7dcea0"),
        "only the Completed date cell stays green; the whole row must not"
    );
    assert!(
        !css.contains("tr.is-complete td {\r\n  background: #d9f2df;")
            && !css.contains("tr.is-complete td {\n  background: #d9f2df;"),
        "row-wide pale green on completed lines must stay removed"
    );
    assert!(
        storage.contains("fill_missing_completed_dates")
            && storage.contains("true_up_on = CASE")
            && storage.contains("AND step_withdrawal = 1"),
        "last CCT step and get() must stamp true_up_on (today when unknown)"
    );
}

#[test]
fn debt_planner_our_transactions_export_to_excel_prompts_for_path() {
    let planner = std::fs::read_to_string(
        golden_harness::repo_root()
            .join("apps/desktop/src/features/cash/ExternalAccountManager.tsx"),
    )
    .unwrap();
    let css = std::fs::read_to_string(golden_harness::repo_root().join("apps/desktop/src/App.css"))
        .unwrap();
    let host = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src-tauri/src/lib.rs"),
    )
    .unwrap();
    assert!(
        planner.contains("managed-register-heading")
            && planner.contains("`Our ${selected.name} transactions`")
            && planner.contains("aria-label=\"Export to excel\"")
            && planner.contains("Export to excel")
            && planner.contains("exportOurTransactions")
            && planner.contains("ExternalRegisterExportGet")
            && planner.contains("save_local_bytes")
            && planner.contains("Our ${selected.name} transactions")
            && planner.contains("safeTitle"),
        "Debt planner list is Our {{account}} transactions with Save-file default from that title"
    );
    assert!(
        css.contains(".managed-register-heading")
            && css.contains("justify-content: space-between"),
        "Export button sits on the same row as Our {{account}} transactions, to the right"
    );
    assert!(
        host.contains("fn save_local_bytes")
            && host.contains(".set_directory(&dir)")
            && host.contains(".set_file_name(&default_file_name)")
            && host.contains(".save_file()"),
        "export must prompt for file name and directory via the established save dialog"
    );
}

#[test]
fn cct_completed_export_offers_print_pdf_excel() {
    let register = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/features/cash/ExternalRegister.tsx"),
    )
    .unwrap();
    let core = std::fs::read_to_string(
        golden_harness::repo_root().join("crates/application-core/src/external_register.rs"),
    )
    .unwrap();
    let queries = std::fs::read_to_string(
        golden_harness::repo_root().join("crates/application-core/src/queries.rs"),
    )
    .unwrap();
    assert!(
        register.contains("aria-label=\"Export\"")
            && register.contains("aria-label=\"CCT Completed export preview\"")
            && register.contains("aria-label=\"Print to page\"")
            && register.contains("aria-label=\"Save PDF\"")
            && register.contains("aria-label=\"Export Excel\"")
            && register.contains("ExternalRegisterExportGet"),
        "Completed section must offer the same Print / PDF / Excel export path as Income Plan"
    );
    assert!(
        queries.contains("\"ExternalRegisterExportGet\"")
            && core.contains("export_completed_from_json")
            && core.contains("cct-completed-"),
        "host must format completed CCT rows for html/pdf/xlsx"
    );
    assert!(
        core.contains("write_number_with_format")
            && core.contains("$#,##0")
            && core.contains("SUM(C2:C")
            && core.contains("Formula::new")
            && !core.contains("write_string(row, 2, &fmt_money"),
        "CCT Excel Total spent must be currency numbers with a SUM total row"
    );
}

#[test]
fn tax_planning_forecast_stays_visible_when_indeterminate() {
    let report = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/features/cash/HouseholdIncomeReport.tsx"),
    )
    .unwrap();
    let forecast = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/features/cash/magiForecast.ts"),
    )
    .unwrap();
    let src = format!("{report}\n{forecast}");
    assert!(
        report.contains("aria-label=\"MAGI forecast\"")
            && report.contains("MAGI after estimates vs")
            && report.contains("aria-label=\"MAGI confidence\"")
            && src.contains("isIndeterminate")
            && src.contains("Book the missing fact")
            && report.contains("confidence = isEstimate ? \"Estimate\" : \"Booked\"")
            && !src.contains("if (isIndeterminate) return null")
            && !src.contains("decisionState === \"INDETERMINATE\" ? null"),
        "INDETERMINATE still fills the hero and labels Estimate"
    );
    assert!(
        report.contains("aria-label=\"MAGI suggestions\"")
            && src.contains("Cut remaining Traditional IRA draws by")
            && src.contains("Or book a Traditional IRA contribution of")
            && src.contains("Or both, split")
            && !src.contains("Roth contribution")
            && report.contains("Medical expenses")
            && report.contains("No effect"),
        "over-cliff suggestions are cut-IRA / contribute-IRA; medical is not a MAGI cut"
    );
    assert!(
        report.contains("aria-label=\"MAGI estimates\"")
            && report.contains("household-estimate-tag")
            && report.contains("Estimate rows feed the forecast tile")
            && src.contains("plan.iraContributionMinor"),
        "scratch HSA / computer / SE / IRA / Barbara stay labeled Estimate"
    );
    assert!(
        !report.contains("MagiFactRecord") && !report.contains("magi_fact"),
        "this patch does not write APTC into magi_fact"
    );
    assert!(
        forecast.contains("plan.netCapitalGainMinor")
            && forecast.contains("Math.max(netMinor, -NET_CAPITAL_LOSS_LIMIT_MINOR)")
            && forecast.contains("gains.magiMinor")
            && !forecast.contains("BARBARA_LTCG_MINOR +\n    carLt.eoy"),
        "MAGI takes the capped capital gain, never the raw ltcg + stcg rows"
    );
    assert!(
        report.contains("Capital loss over the 1040 limit")
            && report.contains("carries forward")
            && report.contains("gains.carryforwardMinor")
            && report.contains("NET_CAPITAL_LOSS_LIMIT_MINOR"),
        "the report names the limited amount and the carryforward"
    );
    assert!(
        report.contains("...carLt") && report.contains("...carSt"),
        "the gain rows keep the uncapped net"
    );
}

/// Mom shopping register_key is Mom. Medical spend does not debit it. Food defaults bucket Food.
#[tokio::test]
async fn register_bucket_medical_and_mom_credit() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let mom_credit_id = "a1000001-0000-4000-8000-000000000008";
    must_ok(
        &platform,
        "ExternalAccountManagerSave",
        serde_json::json!({
            "accounts": [{
                "accountId": mom_credit_id,
                "name": "Mom shopping",
                "startingMinor": 100000,
                "currentMinor": 100000,
                "paymentMinor": null,
                "reductionMinor": null,
                "financeMinor": null,
                "paidThrough": null,
                "payProcess": "register",
                "registerKey": "Mom"
            }]
        }),
    )
    .await;

    let medical_mom = Uuid::new_v4();
    let medical = Uuid::new_v4();
    let mom = Uuid::new_v4();
    let food = Uuid::new_v4();
    must_ok(
        &platform,
        "ExternalRegisterSave",
        serde_json::json!({
            "lines": [
                {
                    "lineId": medical_mom,
                    "payType": "Checking",
                    "occurredOn": "2026-10-05",
                    "amountMinor": 2500,
                    "scale": 2,
                    "category": "medical-mom",
                    "bucket": "",
                    "vendor": "CVS",
                    "description": "visit"
                },
                {
                    "lineId": medical,
                    "payType": "Checking",
                    "occurredOn": "2026-10-05",
                    "amountMinor": 1000,
                    "scale": 2,
                    "category": "Medical",
                    "bucket": "",
                    "vendor": "Paytient",
                    "description": "plain medical"
                },
                {
                    "lineId": mom,
                    "payType": "Checking",
                    "occurredOn": "2026-10-04",
                    "amountMinor": 400,
                    "scale": 2,
                    "category": "Mom",
                    "bucket": "",
                    "vendor": "Walmart",
                    "description": "shopping"
                },
                {
                    "lineId": food,
                    "payType": "Checking",
                    "occurredOn": "2026-10-04",
                    "amountMinor": 500,
                    "scale": 2,
                    "category": "Food",
                    "bucket": "",
                    "vendor": "Weg",
                    "description": "default bucket"
                }
            ]
        }),
    )
    .await;
    let register = query_json(&platform, "ExternalRegisterGet", serde_json::json!({})).await;
    let projected = register["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|line| line["lineId"] == medical_mom.to_string())
        .expect("medical+medical line");
    assert_eq!(projected["category"], "Medical");
    assert_eq!(projected["bucket"], "Medical");
    let food_line = register["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|line| line["lineId"] == food.to_string())
        .expect("food line");
    assert_eq!(food_line["bucket"], "Food", "blank Food category defaults bucket Food");
    let medical_line = register["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|line| line["lineId"] == medical.to_string())
        .expect("medical line");
    assert_eq!(medical_line["bucket"], "Medical");

    must_ok(
        &platform,
        "ExternalRegisterMarkStep",
        serde_json::json!({
            "lineIds": [medical_mom, medical, mom, food],
            "step": "transfer",
            "tickedOn": "2026-10-05"
        }),
    )
    .await;
    let body = query_json(&platform, "ExternalAccountManagerGet", serde_json::json!({})).await;
    let account = body["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["accountId"] == mom_credit_id)
        .expect("Mom shopping");
    assert_eq!(account["registerKey"], "Mom");
    assert_eq!(
        account["currentMinor"], 99600,
        "only category Mom $4 debits Mom shopping; Medical/Food do not: {account}"
    );
}

#[tokio::test]
async fn mom_shopping_repair_keeps_register_key_mom() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let body = query_json(&platform, "ExternalAccountManagerGet", serde_json::json!({})).await;
    let account = body["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["accountId"] == "a1000001-0000-4000-8000-000000000008")
        .expect("Mom shopping");
    assert_eq!(account["registerKey"], "Mom");
}

/// Truist BANK is the car *loan* (Loan book, day 7). Car brokerage withdrawal
/// stays on account Car and must never be treated as that loan or as CCT settle.
#[tokio::test]
async fn truist_car_loan_day_7_isolated_from_car_brokerage_withdrawal() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let managed = query_json(&platform, "ExternalAccountManagerGet", serde_json::json!({})).await;
    let truist = managed["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "Truist BANK")
        .expect("Truist BANK loan");
    assert_eq!(truist["kind"], "debt");
    assert_eq!(
        truist["payProcess"], "week_ahead",
        "interest car loan uses Week Ahead verify, not element→CCT: {truist}"
    );
    assert_eq!(truist["chargesInterest"], true);
    let due = truist["dueOn"].as_str().unwrap_or("");
    assert!(
        due.ends_with("-07"),
        "Truist loan due must be on the 7th, got {due}"
    );

    // WeekAheadGet seeds Car brokerage withdrawal if missing; Loan Truist comes from migrations.
    let ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({ "asOfDate": "2026-10-07" }),
    )
    .await;
    let elements = query_json(
        &platform,
        "CashElementListGet",
        serde_json::json!({ "account": "all", "asOfDate": "2026-10-07" }),
    )
    .await;
    let items = elements["items"].as_array().expect("elements");
    let truist_el = items
        .iter()
        .find(|row| row["elementId"] == "e1000001-0000-4000-8000-000000000002")
        .expect("Truist Loan Element");
    assert_eq!(truist_el["account"], "Loan", "Truist loan element stays on Loan book");
    assert_eq!(truist_el["note"], "Truist BANK");
    assert_eq!(
        truist_el["weekdayOrMonthDay"], "7",
        "0074 seeded day 1; 0082 must keep Truist on the 7th: {truist_el}"
    );
    assert_eq!(truist_el["associationKind"], "loan");

    let car_el = items
        .iter()
        .find(|row| row["account"] == "Car")
        .expect("Car brokerage withdrawal element must exist separately from Truist loan");
    assert_ne!(
        car_el["elementId"], truist_el["elementId"],
        "Car brokerage withdrawal must not share Truist's element id"
    );
    assert_ne!(car_el["account"], "Loan");
    assert_ne!(
        car_el["associationKind"], "loan",
        "Car brokerage withdrawal is not a loan association: {car_el}"
    );

    let rows = ahead["rows"].as_array().expect("week ahead rows");
    assert!(
        rows.iter().all(|row| row["note"] != "Truist BANK"),
        "Truist must not use the element Confirm row — it belongs in Loan payments: {ahead}"
    );
    assert!(
        rows.iter().all(|row| !(row["account"] == "Car" && row["note"] == "Truist BANK")),
        "Truist must never be listed as a Car brokerage row: {ahead}"
    );
    let loan_rows = ahead["loans"].as_array().expect("week ahead loans");
    let truist_loan = loan_rows
        .iter()
        .find(|row| row["name"] == "Truist BANK")
        .expect("Truist in Week Ahead Loan payments");
    assert_eq!(truist_loan["dueOn"], "2026-10-07");
    assert!(
        truist_loan["interestMinor"].as_i64().unwrap_or(0) > 0,
        "Truist must project interest for verify: {truist_loan}"
    );
    assert_eq!(
        truist_loan["principalMinor"].as_i64().unwrap()
            + truist_loan["interestMinor"].as_i64().unwrap(),
        truist_loan["paymentMinor"].as_i64().unwrap(),
        "principal + interest = payment: {truist_loan}"
    );

    // Next Sat–Fri week after the 7th — unpaid loan must still be on the planner.
    let next_week = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({ "asOfDate": "2026-10-11" }),
    )
    .await;
    assert_eq!(next_week["periodStart"], "2026-10-10");
    let past = next_week["loans"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "Truist BANK")
        .unwrap_or_else(|| {
            panic!("unpaid Truist must remain on Week Ahead after its due week: {next_week}")
        });
    assert_eq!(past["dueOn"], "2026-10-07", "{past}");

    let mig = std::fs::read_to_string(
        golden_harness::repo_root()
            .join("crates/storage-sqlite/migrations/0082_truist_loan_day_7_not_car_brokerage.sql"),
    )
    .unwrap();
    assert!(
        mig.contains("weekday_or_month_day = '7'")
            && mig.contains("Truist BANK")
            && mig.contains("account = 'Loan'")
            && mig.contains("do not touch"),
        "0082 must repair Truist Loan day only and leave Car brokerage alone"
    );
    let mig83 = std::fs::read_to_string(
        golden_harness::repo_root()
            .join("crates/storage-sqlite/migrations/0083_restore_interest_loan_week_ahead.sql"),
    )
    .unwrap();
    assert!(
        mig83.contains("pay_process = 'week_ahead'")
            && mig83.contains("charges_interest = 1")
            && mig83.contains("external_element_cct_draft"),
        "0083 restores interest Week Ahead path and undoes Truist element confirm"
    );
    let week_ui = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/features/cash/WeekAhead.tsx"),
    )
    .unwrap();
    assert!(
        week_ui.contains("Week ahead loan payments")
            && week_ui.contains("projectedInterest")
            && week_ui.contains("principalMinor")
            && week_ui.contains("interestMinor"),
        "Week Ahead Loan payments UI must keep interest verify"
    );
}

#[tokio::test]
async fn newrez_week_ahead_interest_stays_under_total_payment() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let managed = query_json(&platform, "ExternalAccountManagerGet", serde_json::json!({})).await;
    let newrez_acct = managed["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "Newrez")
        .expect("Newrez account");
    assert_eq!(newrez_acct["payProcess"], "week_ahead");
    assert_eq!(newrez_acct["frequency"], "monthly");
    assert_eq!(
        newrez_acct["aprPpm"], 59_900,
        "Newrez needs 5.99% APR to project interest: {newrez_acct}"
    );
    assert_eq!(newrez_acct["paymentMinor"], 179_672);
    // Due 2026-10-10 is Saturday — that week's window is 10/10–10/16.
    let ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({ "asOfDate": "2026-10-10" }),
    )
    .await;
    assert_eq!(ahead["periodStart"], "2026-10-10");
    assert_eq!(ahead["periodEnd"], "2026-10-16");
    let newrez = ahead["loans"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "Newrez")
        .unwrap_or_else(|| panic!("Newrez on Week Ahead Loan payments: {ahead}"));
    let payment = newrez["paymentMinor"].as_i64().unwrap();
    let interest = newrez["interestMinor"].as_i64().unwrap();
    let principal = newrez["principalMinor"].as_i64().unwrap();
    assert_eq!(payment, 179_672, "Newrez contractual payment: {newrez}");
    assert!(
        interest > 0 && interest < payment,
        "interest must be positive and under total payment: {newrez}"
    );
    assert_eq!(principal + interest, payment, "P+I must equal total payment: {newrez}");
    // ~$1,488 on ~$298k @ 5.99%/12 — statement history is ~$1,489–$1,497.
    assert!(
        (148_000..=150_000).contains(&interest),
        "Newrez projected interest out of band: {newrez}"
    );
    let week_ui = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/features/cash/WeekAhead.tsx"),
    )
    .unwrap();
    assert!(
        week_ui.contains("total payment")
            && week_ui.contains("Interest cannot exceed total payment")
            && week_ui.contains("interestTooHigh"),
        "Week Ahead must prompt for total payment and block interest > payment"
    );
}

#[tokio::test]
async fn truist_week_ahead_confirm_reduces_current_by_principal_only() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let before = query_json(&platform, "ExternalAccountManagerGet", serde_json::json!({})).await;
    let truist = before["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "Truist BANK")
        .unwrap()
        .clone();
    let start = truist["currentMinor"].as_i64().unwrap();
    let ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({ "asOfDate": "2026-10-07" }),
    )
    .await;
    let loan = ahead["loans"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "Truist BANK")
        .expect("Truist loan row");
    let principal = loan["principalMinor"].as_i64().unwrap();
    let interest = loan["interestMinor"].as_i64().unwrap();
    assert!(interest > 0 && principal > 0 && principal + interest == loan["paymentMinor"]);
    must_ok(
        &platform,
        "LoanPaymentConfirm",
        serde_json::json!({
            "accountId": truist["accountId"],
            "dueOn": "2026-10-07",
            "principalMinor": principal,
            "interestMinor": interest,
        }),
    )
    .await;
    let after = query_json(&platform, "ExternalAccountManagerGet", serde_json::json!({})).await;
    let truist_after = after["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "Truist BANK")
        .unwrap();
    assert_eq!(
        truist_after["currentMinor"].as_i64().unwrap(),
        start - principal,
        "Current falls by principal only, not full payment: before={start} principal={principal} after={truist_after}"
    );
    assert!(
        truist_after["dueOn"]
            .as_str()
            .unwrap_or("")
            .ends_with("-07"),
        "next due stays on the 7th: {truist_after}"
    );
}

/// Statement ending balance overrides current − principal when the bank split disagrees.
#[tokio::test]
async fn loan_confirm_balance_after_overrides_principal_math() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let before = query_json(&platform, "ExternalAccountManagerGet", serde_json::json!({})).await;
    let truist = before["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "Truist BANK")
        .unwrap()
        .clone();
    let start = truist["currentMinor"].as_i64().unwrap();
    let ahead = query_json(
        &platform,
        "WeekAheadGet",
        serde_json::json!({ "asOfDate": "2026-10-07" }),
    )
    .await;
    let loan = ahead["loans"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "Truist BANK")
        .expect("Truist loan row");
    let principal = loan["principalMinor"].as_i64().unwrap();
    let interest = loan["interestMinor"].as_i64().unwrap();
    // Bank ending balance differs from current − entered principal (interest split drift).
    let statement_after = start - principal + 477;
    must_ok(
        &platform,
        "LoanPaymentConfirm",
        serde_json::json!({
            "accountId": truist["accountId"],
            "dueOn": "2026-10-07",
            "principalMinor": principal,
            "interestMinor": interest,
            "balanceAfterMinor": statement_after,
        }),
    )
    .await;
    let after = query_json(&platform, "ExternalAccountManagerGet", serde_json::json!({})).await;
    let truist_after = after["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == "Truist BANK")
        .unwrap();
    assert_eq!(
        truist_after["currentMinor"].as_i64().unwrap(),
        statement_after,
        "statement balance after wins over current − principal"
    );
    assert_ne!(
        statement_after,
        start - principal,
        "fixture must disagree so the override is exercised"
    );
}

/// Household Debt planner must never come back empty after migrate.
/// Well-known blank modes this locks:
/// - migration not embedded / SELECT on missing `inactive` → Get fails
/// - seed wiped or all debts marked inactive → zero active loans
/// - UI silent empty state (no Loading / error / No loans copy)
#[tokio::test]
async fn debt_planner_household_active_loans_never_blank() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let body = query_json(&platform, "ExternalAccountManagerGet", serde_json::json!({})).await;
    let accounts = body["accounts"].as_array().expect("accounts array");
    assert!(
        accounts.len() >= 7,
        "seeded household must keep loans+escrow after migrate, got {}: {accounts:?}",
        accounts.len()
    );
    let active_debts: Vec<&serde_json::Value> = accounts
        .iter()
        .filter(|row| row["kind"] == "debt" && row["inactive"] != true)
        .collect();
    assert!(
        active_debts.len() >= 6,
        "Debt planner active debts blanked (want >= 6): {active_debts:?}"
    );
    for name in [
        "Newrez",
        "Truist BANK",
        "myClearbalance",
        "UVA Health 3/22-3/25",
        "Alphaeon Cat",
        "Alphaeon CK",
    ] {
        assert!(
            active_debts.iter().any(|row| row["name"] == name),
            "missing active loan {name} in {active_debts:?}"
        );
    }
    let paytient = accounts
        .iter()
        .find(|row| row["name"] == "Paytient")
        .expect("Paytient must remain (inactive), not deleted");
    assert_eq!(paytient["inactive"], true, "Paytient stays inactive: {paytient}");
    assert_eq!(
        paytient["currentMinor"],
        0,
        "inactive Paytient current held at 0 (reconcile must not move it): {paytient}"
    );
    let mom = accounts
        .iter()
        .find(|row| row["name"] == "Mom shopping")
        .expect("Mom shopping escrow");
    assert_eq!(mom["kind"], "credit", "Mom shopping stays escrow/credit");

    let storage = std::fs::read_to_string(
        golden_harness::repo_root().join("crates/storage-sqlite/src/external_account.rs"),
    )
    .unwrap();
    assert!(
        storage.contains("linked_element_id, inactive,")
            && storage.contains("if account.inactive")
            && storage.contains("continue;"),
        "Get SELECT must include inactive; reconcile must skip inactive loans"
    );
    let mig_inactive = std::fs::read_to_string(
        golden_harness::repo_root()
            .join("crates/storage-sqlite/migrations/0079_loan_inactive.sql"),
    )
    .unwrap();
    let mig_hold = std::fs::read_to_string(
        golden_harness::repo_root()
            .join("crates/storage-sqlite/migrations/0081_paytient_inactive_balance_hold.sql"),
    )
    .unwrap();
    assert!(
        mig_inactive.contains("ADD COLUMN inactive")
            && mig_hold.contains("current_minor = 0")
            && mig_hold.contains("external_managed_applied"),
        "inactive column + Paytient balance hold migrations required"
    );
    let planner = std::fs::read_to_string(
        golden_harness::repo_root()
            .join("apps/desktop/src/features/cash/ExternalAccountManager.tsx"),
    )
    .unwrap();
    assert!(
        planner.contains("Loading loans…")
            && planner.contains("Loans unavailable — see status above")
            && planner.contains("No loans loaded.")
            && planner.contains("Debt planner load failed:")
            && planner.contains("Check migrations / restart")
            && planner.contains("account.inactive !== true")
            && planner.contains("Load independently so one failing query cannot blank"),
        "Debt planner must surface loading/error/empty — not a silent blank table"
    );
    let store = std::fs::read_to_string(
        golden_harness::repo_root().join("crates/storage-sqlite/src/store.rs"),
    )
    .unwrap();
    assert!(
        store.contains("sqlx::migrate!(") && store.contains("re-embeds"),
        "store.rs must remind agents to re-embed migrations (blank planner failure mode)"
    );
}

#[tokio::test]
async fn paytient_inactive_stays_in_db_hidden_from_active_loan_list() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let body = query_json(&platform, "ExternalAccountManagerGet", serde_json::json!({})).await;
    let accounts = body["accounts"].as_array().expect("accounts");
    let paytient = accounts
        .iter()
        .find(|row| row["accountId"] == "a1000001-0000-4000-8000-000000000004")
        .expect("Paytient account must be restored");
    assert_eq!(paytient["inactive"], true, "Paytient is inactive: {paytient}");
    assert_eq!(paytient["currentMinor"], 0, "paid-off current is zero: {paytient}");
    let planner = std::fs::read_to_string(
        golden_harness::repo_root()
            .join("apps/desktop/src/features/cash/ExternalAccountManager.tsx"),
    )
    .unwrap();
    assert!(
        planner.contains("aria-label=\"Inactive loan\"")
            && planner.contains("account.inactive !== true")
            && planner.contains("inactive: credit ? false : draft.inactive"),
        "Debt planner must hide inactive loans and offer Inactive checkbox"
    );
    let ams = std::fs::read_to_string(
        golden_harness::repo_root()
            .join("apps/desktop/src/features/accounts/AccountManagement.tsx"),
    )
    .unwrap();
    assert!(
        ams.contains("aria-label=\"Account Management\"")
            && ams.contains("Active loan ${loan.name}")
            && ams.contains("aria-label=\"Brokerage accounts\"")
            && ams.contains("cashSymbol")
            && ams.contains("brokerAccountNumber")
            && ams.contains("minBalanceTargetMinor"),
        "Account Management lists brokerage fields and loan Active toggle"
    );
}

#[test]
fn account_table_lands_locked_broker_fields() {
    let migration = std::fs::read_to_string(
        golden_harness::repo_root()
            .join("crates/storage-sqlite/migrations/0080_account_broker_fields.sql"),
    )
    .unwrap();
    let contracts = std::fs::read_to_string(
        golden_harness::repo_root().join("crates/application-core/src/contracts.rs"),
    )
    .unwrap();
    assert!(
        migration.contains("cash_symbol")
            && migration.contains("broker_account_number")
            && migration.contains("min_balance_target_minor"),
        "0080 must add the three locked account columns"
    );
    assert!(
        contracts.contains("pub cash_symbol:")
            && contracts.contains("pub broker_account_number:")
            && contracts.contains("pub min_balance_target_minor:"),
        "AccountRecord must expose cash_symbol, broker_account_number, min_balance_target_minor"
    );
}

async fn open_cash_lot(
    platform: &LocalPlatform,
    account_id: &str,
    symbol: &str,
    qty_minor: i64,
) {
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
            "accountId": account_id,
            "securityId": security_id,
            "openedOn": "2026-09-01",
            "origin": "purchase",
            "quantityMinor": qty_minor,
            "quantityScale": 2,
            "performanceBasisMinor": qty_minor,
            "taxBasisMinor": qty_minor,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;
}

#[tokio::test]
async fn cash_pile_prefers_account_cash_symbol_over_name_map() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let health = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Health", "kind": "ira"}),
    )
    .await;
    let account_id = health["accountId"].as_str().unwrap();
    assert_eq!(
        health["cashSymbol"].as_str(),
        Some("FDRXX"),
        "register fills the name-map default"
    );
    open_cash_lot(&platform, account_id, "SPAXX", 4_200).await;
    must_ok(
        &platform,
        "AccountUpdate",
        serde_json::json!({ "accountId": account_id, "cashSymbol": "SPAXX" }),
    )
    .await;
    let pile = query_json(
        &platform,
        "CashPileGet",
        serde_json::json!({ "accountId": account_id }),
    )
    .await;
    assert_eq!(pile["found"], true);
    assert_eq!(
        pile["symbol"].as_str(),
        Some("SPAXX"),
        "stored column wins over Health→FDRXX map"
    );
    assert_eq!(pile["dollarsMinor"].as_i64(), Some(4_200));

    let week = query_json(
        &platform,
        "TrendsWeekGet",
        serde_json::json!({ "asOfDate": "2026-09-11" }),
    )
    .await;
    let health_ref = week["cashReferences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["accountName"] == "Health")
        .expect("Health cash reference");
    assert_eq!(
        health_ref["cashSymbol"].as_str(),
        Some("SPAXX"),
        "Trends cash label follows account.cash_symbol"
    );
}

#[tokio::test]
async fn cash_pile_prefers_column_lot_over_larger_other_mm_lot() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let health = must_ok(
        &platform,
        "AccountRegister",
        serde_json::json!({"name": "Health", "kind": "ira"}),
    )
    .await;
    let account_id = health["accountId"].as_str().unwrap();
    open_cash_lot(&platform, account_id, "FDRXX", 50_000).await;
    open_cash_lot(&platform, account_id, "SPAXX", 1_500).await;
    must_ok(
        &platform,
        "AccountUpdate",
        serde_json::json!({ "accountId": account_id, "cashSymbol": "SPAXX" }),
    )
    .await;
    let pile = query_json(
        &platform,
        "CashPileGet",
        serde_json::json!({ "accountId": account_id }),
    )
    .await;
    assert_eq!(pile["found"], true);
    assert_eq!(
        pile["symbol"].as_str(),
        Some("SPAXX"),
        "preferred column lot wins even when another MM lot is larger"
    );
    assert_eq!(pile["dollarsMinor"].as_i64(), Some(1_500));
}

#[test]
fn desktop_cash_surfaces_pass_stored_cash_symbol() {
    let app = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/App.tsx"),
    )
    .unwrap();
    let cart = std::fs::read_to_string(
        golden_harness::repo_root()
            .join("apps/desktop/src/features/shopping-cart/ShoppingCartScreen.tsx"),
    )
    .unwrap();
    let cash_flow = std::fs::read_to_string(
        golden_harness::repo_root()
            .join("apps/desktop/src/features/cash/AccountCashFlow.tsx"),
    )
    .unwrap();
    let trends = std::fs::read_to_string(
        golden_harness::repo_root()
            .join("apps/desktop/src/features/graphing/TrendsCharts.tsx"),
    )
    .unwrap();
    let contracts = std::fs::read_to_string(
        golden_harness::repo_root().join("packages/app-contracts/src/index.ts"),
    )
    .unwrap();
    assert!(
        contracts.contains("storedCashSymbol")
            && contracts.contains("ACCOUNT_CASH_SYMBOL[accountName.trim()]"),
        "accountCashSymbol takes stored column before the name map"
    );
    assert!(
        app.contains("accountCashSymbol(account, [], stored)")
            && cart.contains("selectedAccount?.cashSymbol")
            && cash_flow.contains("storedCashSymbol")
            && cash_flow.contains("AccountList")
            && trends.contains("accountCashSymbol(account, [], stored)")
            && trends.contains("accounts?: Array<Pick<AccountListItem"),
        "Cart/Home/AccountCashFlow/Trends pass AccountRecord.cashSymbol into the resolve helper"
    );
}

#[tokio::test]
async fn completed_alphaeon_edit_keeps_completed_and_refuses_alpheon_category() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let line_id = Uuid::new_v4();
    must_ok(
        &platform,
        "ExternalRegisterSave",
        serde_json::json!({
            "lines": [{
                "lineId": line_id,
                "payType": "Checking",
                "occurredOn": "2026-09-01",
                "amountMinor": 25000,
                "scale": 2,
                "category": "Mom",
                "bucket": "",
                "vendor": "Alphaeon",
                "description": "ck",
                "completed": true,
                "trueUpOn": "2026-09-02"
            }]
        }),
    )
    .await;
    must_ok(
        &platform,
        "ExternalRegisterSave",
        serde_json::json!({
            "lines": [{
                "lineId": line_id,
                "payType": "Checking",
                "occurredOn": "2026-09-01",
                "amountMinor": 25000,
                "scale": 2,
                "category": "Medical",
                "bucket": "Medical",
                "vendor": "Alpheon",
                "description": "ck",
                "completed": true,
                "trueUpOn": "2026-09-02"
            }]
        }),
    )
    .await;
    let body = query_json(&platform, "ExternalRegisterGet", serde_json::json!({})).await;
    let line = body["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["lineId"] == line_id.to_string())
        .expect("edited line");
    assert_eq!(line["category"], "Medical");
    assert_eq!(line["bucket"], "Medical");
    assert_eq!(line["vendor"], "Alpheon");
    assert_eq!(line["completed"], true);

    let refused = execute_command_on(
        &platform,
        &platform,
        cmd(
            "ExternalRegisterSave",
            serde_json::json!({
                "lines": [{
                    "lineId": Uuid::new_v4(),
                    "payType": "Checking",
                    "occurredOn": "2026-09-03",
                    "amountMinor": 100,
                    "scale": 2,
                    "category": "Alpheon",
                    "bucket": "",
                    "vendor": "CVS",
                    "description": "bad"
                }]
            }),
        ),
    )
    .await;
    assert!(!refused.ok, "Alpheon is a vendor, not a category");
    assert_eq!(refused.error_code.as_deref(), Some("bad_category"));
}

#[test]
fn cct_register_has_bucket_column_and_completed_edit() {
    let register = std::fs::read_to_string(
        golden_harness::repo_root().join("apps/desktop/src/features/cash/ExternalRegister.tsx"),
    )
    .unwrap();
    let css = std::fs::read_to_string(golden_harness::repo_root().join("apps/desktop/src/App.css"))
        .unwrap();
    assert!(
        register.contains("sortHead(\"Bucket\", \"bucket\")")
            && register.contains("ariaLabel=\"New bucket\"")
            && register.contains("ariaLabel={`Completed bucket ${line.lineId}`}")
            && register.contains("ariaLabel={`Completed category ${line.lineId}`}")
            && register.contains("ariaLabel={`Completed description ${line.lineId}`}")
            && register.contains("ariaLabel=\"Bulk bucket\"")
            && register.contains("aria-label=\"Bulk field\"")
            && register.contains("ariaLabel=\"Bulk value\"")
            && register.contains("aria-label=\"Set field\"")
            && register.contains("aria-label=\"Confirm bulk set field\"")
            && register.contains("data-part=\"external-bulk-set-field\"")
            && register.contains("data-part=\"completed-line-edit-actions\"")
            && register.contains("saveCompletedLine")
            && register.contains("saveCompletedEdits")
            && register.contains("completedEditingId")
            && register.contains("startCompletedEdit")
            && register.contains("BUCKET_OPTIONS")
            && register.contains("\"Food\", \"Cash\", \"Bills\", \"Pets\", \"Medical\", \"HSA\"")
            && register.contains("function BucketSelect")
            && register.contains("catalogBuckets")
            && register.contains("aria-label=\"Save completed edits\"")
            && register.contains("aria-label=\"Cancel completed edits\"")
            && register.contains("isBulkEligibleLine")
            && register.contains("Select visible dated completed")
            && register.contains("function BucketYearRing")
            && register.contains("aria-label={`Bucket spend ${year}`}")
            && register.contains("title=\"Dated lines in selected years\"")
            && register.contains("COLUMN_FILTER_BLANK")
            && register.contains(">Blank</option>")
            && register.contains("function amountFilterHit")
            && register.contains("\"Filter total spent\"")
            && register.contains("name=\"cct-completed-amount-filter\"")
            && register.contains("aria-label={aria}")
            && register.contains("autoComplete=\"off\"")
            && register.contains("ExternalRegisterDelete")
            && register.contains("saveOpenLine")
            && register.contains("deleteOpenLine")
            && register.contains("data-part=\"open-line-edit-actions\"")
            && register.contains("className=\"external-open-line-edit\"")
            && register.contains("external-open-edit-actions")
            && register.contains("Save open line")
            && register.contains("Cancel open line")
            && register.contains("Delete open line")
            && register.contains("openEditDirty")
            && register.contains("activeColumnFilter")
            && register.contains("[bulkField]: \"\"")
            && register.contains("data-part=\"completed-filter-empty\"")
            && register.contains("aria-label=\"Clear column filters\"")
            && register.contains("clearColumnFilters")
            && register.contains("filtersHideCompleted")
            && register.contains("data-part=\"completed-year-ticks\"")
            && register.contains("aria-label=\"Active year\"")
            && register.contains("useState<string[]>([\"2026\"])")
            && register.contains("aria-label={`Include ${year}`}")
            && !register.contains("colSpan={4}\n                        className=\"external-open-edit-actions\"")
            && !register.contains("colSpan={4}\r\n                        className=\"external-open-edit-actions\"")
            && !register.contains("cct-open-amount-filter")
            && !register.contains("Filter open total spent")
            && !register.contains("completedChoices.amount")
            && !register.contains("Dated lines in selected years</span>")
            && !register.contains("external-bulk-hint")
            && !register.contains("data-part=\"external-set-bucket\""),
        "CCT must show catalog Bucket, completed per-line Save, bulk field set, and year bucket ring"
    );
    assert!(
        css.contains(".external-open-wrap .external-open-edit-actions")
            && css.contains(".external-open-wrap .external-open-line-edit")
            && css.contains("display: inline-flex"),
        "Open Save/Cancel/Delete sit on the edited row; step ticks stay on the row"
    );
    assert!(
        css.contains(".external-completed-band")
            && css.contains(".external-bucket-year")
            && css.contains(".external-completed-years")
            && css.contains("width: fit-content")
            && !css.contains(".external-bulk-hint"),
        "Completed band must host a fit-content bulk bar, year ticks, and bucket-year donut"
    );
    assert!(
        !register.contains("Medical-mom") && !register.contains("medical-mom"),
        "Medical-mom alias must stay retired in the UI"
    );
    assert!(
        !register.contains("Cash acct") && !register.contains("Bill acct"),
        "Retired bucket spellings must not appear in CCT UI"
    );
    let planner = std::fs::read_to_string(
        golden_harness::repo_root()
            .join("apps/desktop/src/features/cash/ExternalAccountManager.tsx"),
    )
    .unwrap();
    assert!(
        planner.contains("aria-label=\"Bucket manager\"")
            && planner.contains("debt-bucket-panel")
            && planner.contains("Bucket Name")
            && planner.contains(">Budget category<")
            && planner.contains(">Associated bank<")
            && planner.contains("aria-label=\"Add bucket\"")
            && planner.contains("aria-label=\"Save bucket\"")
            && planner.contains("aria-label=\"Budget buckets\"")
            && planner.contains("Capital One")
            && planner.contains("debt-bucket-bank")
            && planner.contains("Add bank…")
            && planner.contains("__add_bank__")
            && planner.contains("aria-label=\"New bank name\"")
            && planner.contains("function BucketBankField")
            && planner.contains("ExternalBucketListGet")
            && planner.contains("ExternalBucketSave")
            && planner.contains("Bucket Transaction Managed")
            && planner.contains("Scheduled element")
            && planner.contains("Week Ahead")
            && planner.contains("\"week_ahead\"")
            && planner.contains("DEBT_PROCESSES")
            && planner.contains("aria-label=\"Loans\"")
            && planner.contains("aria-label=\"Escrow account\"")
            && planner.contains("managed-section")
            && planner.contains("title=\"Account Name\"")
            && planner.contains("title=\"Loan Name\"")
            && planner.contains("title=\"Loan type\"")
            && planner.contains("managed-col-bucket")
            && planner.contains("managed-col-type")
            && planner.contains("\"Loans total\"")
            && planner.contains("\"Escrow total\"")
            && planner.contains("managed-debt-table")
            && planner.contains("aria-label=\"Loan Element\"")
            && planner.contains("aria-label=\"Loan Element details\"")
            && planner.contains("aria-label=\"Element name\"")
            && planner.contains("aria-label=\"Element amount\"")
            && planner.contains("aria-label=\"Element start date\"")
            && planner.contains("aria-label=\"Element stop date\"")
            && planner.contains("aria-label=\"Open exceptions\"")
            && planner.contains("CashElementSave")
            && planner.contains("NEW_LOAN_ELEMENT")
            && planner.contains("CashElementExceptions")
            && !planner.contains(" · \" + money(element.amountMinor)")
            && !planner.contains("title=\"Associated Bucket\"")
            && !planner.contains("Choose the process in Loan setup")
            && !planner.contains("[\"register\", \"Checking and Credit\"]")
            && !planner.contains("aria-label=\"Bucket setup\"")
            && !planner.contains("<th scope=\"row\">Total</th>"),
        "Debt planner sections, dual totals, Scheduled element, Account Name, Loan Name, full Loan Element fields"
    );
    let catalog = std::fs::read_to_string(
        golden_harness::repo_root()
            .join("apps/desktop/src/features/cash/CashElementsCatalog.tsx"),
    )
    .unwrap();
    assert!(
        catalog.contains("\"Loan\""),
        "Element Management account picker must include the Loan book"
    );
    let name_at = planner.find("Bucket Name").expect("Bucket Name header");
    let cat_at = planner.find(">Budget category<").expect("Budget category header");
    let bank_at = planner.find(">Associated bank<").expect("Associated bank header");
    assert!(
        name_at < cat_at && cat_at < bank_at,
        "columns must be Bucket Name, Budget category, Associated bank"
    );
}

#[tokio::test]
async fn bulk_set_bucket_keeps_completed_and_skips_prior_year() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let food_a = Uuid::new_v4();
    let food_b = Uuid::new_v4();
    let prior = Uuid::new_v4();
    must_ok(
        &platform,
        "ExternalRegisterSave",
        serde_json::json!({
            "lines": [
                {
                    "lineId": food_a,
                    "payType": "UCARD",
                    "occurredOn": "2026-03-01",
                    "amountMinor": 1200,
                    "scale": 2,
                    "category": "Food",
                    "bucket": "",
                    "vendor": "Weg",
                    "description": "a",
                    "completed": true,
                    "trueUpOn": "2026-03-02"
                },
                {
                    "lineId": food_b,
                    "payType": "UCARD",
                    "occurredOn": "2026-03-08",
                    "amountMinor": 3400,
                    "scale": 2,
                    "category": "Food",
                    "bucket": "",
                    "vendor": "Walmart",
                    "description": "b",
                    "completed": true,
                    "trueUpOn": "2026-03-09"
                },
                {
                    "lineId": prior,
                    "payType": "UCARD",
                    "occurredOn": "2025-11-01",
                    "amountMinor": 900,
                    "scale": 2,
                    "category": "Food",
                    "bucket": "",
                    "vendor": "Weg",
                    "description": "prior",
                    "completed": true,
                    "trueUpOn": "2025-11-02"
                }
            ]
        }),
    )
    .await;
    // Bulk-style save: only current-year Food lines get bucket Cash.
    must_ok(
        &platform,
        "ExternalRegisterSave",
        serde_json::json!({
            "lines": [
                {
                    "lineId": food_a,
                    "payType": "UCARD",
                    "occurredOn": "2026-03-01",
                    "amountMinor": 1200,
                    "scale": 2,
                    "category": "Food",
                    "bucket": "Cash",
                    "vendor": "Weg",
                    "description": "a",
                    "completed": true,
                    "trueUpOn": "2026-03-02"
                },
                {
                    "lineId": food_b,
                    "payType": "UCARD",
                    "occurredOn": "2026-03-08",
                    "amountMinor": 3400,
                    "scale": 2,
                    "category": "Food",
                    "bucket": "Cash",
                    "vendor": "Walmart",
                    "description": "b",
                    "completed": true,
                    "trueUpOn": "2026-03-09"
                }
            ]
        }),
    )
    .await;
    let body = query_json(&platform, "ExternalRegisterGet", serde_json::json!({})).await;
    let lines = body["lines"].as_array().unwrap();
    let a = lines
        .iter()
        .find(|row| row["lineId"] == food_a.to_string())
        .unwrap();
    let b = lines
        .iter()
        .find(|row| row["lineId"] == food_b.to_string())
        .unwrap();
    let old = lines
        .iter()
        .find(|row| row["lineId"] == prior.to_string())
        .unwrap();
    assert_eq!(a["bucket"], "Cash");
    assert_eq!(b["bucket"], "Cash");
    assert_eq!(a["completed"], true);
    assert_eq!(b["completed"], true);
    // Prior year was omitted from the bulk save; category default Food sticks, not Cash.
    assert_eq!(old["bucket"], "Food");
    assert_eq!(old["completed"], true);
}

#[tokio::test]
async fn bulk_set_vendor_keeps_completed_and_skips_prior_year() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let food_a = Uuid::new_v4();
    let food_b = Uuid::new_v4();
    let prior = Uuid::new_v4();
    must_ok(
        &platform,
        "ExternalRegisterSave",
        serde_json::json!({
            "lines": [
                {
                    "lineId": food_a,
                    "payType": "PPMC",
                    "occurredOn": "2026-01-29",
                    "amountMinor": 3780,
                    "scale": 2,
                    "category": "Medical",
                    "bucket": "Medical",
                    "vendor": "Delta",
                    "description": "vision",
                    "completed": true,
                    "trueUpOn": "2026-01-29"
                },
                {
                    "lineId": food_b,
                    "payType": "PPMC",
                    "occurredOn": "2026-02-27",
                    "amountMinor": 3780,
                    "scale": 2,
                    "category": "Medical",
                    "bucket": "Medical",
                    "vendor": "Eye",
                    "description": "vision",
                    "completed": true,
                    "trueUpOn": "2026-02-27"
                },
                {
                    "lineId": prior,
                    "payType": "PPMC",
                    "occurredOn": "2025-11-01",
                    "amountMinor": 3780,
                    "scale": 2,
                    "category": "Medical",
                    "bucket": "Medical",
                    "vendor": "Delta",
                    "description": "prior",
                    "completed": true,
                    "trueUpOn": "2025-11-02"
                }
            ]
        }),
    )
    .await;
    must_ok(
        &platform,
        "ExternalRegisterSave",
        serde_json::json!({
            "lines": [
                {
                    "lineId": food_a,
                    "payType": "PPMC",
                    "occurredOn": "2026-01-29",
                    "amountMinor": 3780,
                    "scale": 2,
                    "category": "Medical",
                    "bucket": "Medical",
                    "vendor": "Delta Vision",
                    "description": "vision",
                    "completed": true,
                    "trueUpOn": "2026-01-29"
                },
                {
                    "lineId": food_b,
                    "payType": "PPMC",
                    "occurredOn": "2026-02-27",
                    "amountMinor": 3780,
                    "scale": 2,
                    "category": "Medical",
                    "bucket": "Medical",
                    "vendor": "Delta Vision",
                    "description": "vision",
                    "completed": true,
                    "trueUpOn": "2026-02-27"
                }
            ]
        }),
    )
    .await;
    let body = query_json(&platform, "ExternalRegisterGet", serde_json::json!({})).await;
    let lines = body["lines"].as_array().unwrap();
    let a = lines
        .iter()
        .find(|row| row["lineId"] == food_a.to_string())
        .unwrap();
    let b = lines
        .iter()
        .find(|row| row["lineId"] == food_b.to_string())
        .unwrap();
    let old = lines
        .iter()
        .find(|row| row["lineId"] == prior.to_string())
        .unwrap();
    assert_eq!(a["vendor"], "Delta Vision");
    assert_eq!(b["vendor"], "Delta Vision");
    assert_eq!(a["completed"], true);
    assert_eq!(b["completed"], true);
    assert_eq!(old["vendor"], "Delta");
    assert_eq!(old["completed"], true);
}

#[tokio::test]
async fn register_refuses_unknown_bucket() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let refused = execute_command_on(
        &platform,
        &platform,
        cmd(
            "ExternalRegisterSave",
            serde_json::json!({
                "lines": [{
                    "lineId": Uuid::new_v4(),
                    "payType": "UCARD",
                    "occurredOn": "2026-09-03",
                    "amountMinor": 100,
                    "scale": 2,
                    "category": "Food",
                    "bucket": "Grocery",
                    "vendor": "Weg",
                    "description": "bad bucket"
                }]
            }),
        ),
    )
    .await;
    assert!(!refused.ok, "Grocery is not a catalog bucket");
    assert_eq!(refused.error_code.as_deref(), Some("bad_bucket"));
}

#[tokio::test]
async fn debt_planner_bucket_catalog_save_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let listed = query_json(&platform, "ExternalBucketListGet", serde_json::json!({})).await;
    let seed = listed["buckets"].as_array().unwrap();
    assert!(seed.len() >= 7, "{seed:?}");
    assert!(seed.iter().any(|b| b["name"] == "Food"));
    assert!(seed.iter().any(|b| b["name"] == "Mom"));
    assert!(!seed.iter().any(|b| b["name"] == "Mortgage"));
    assert!(!seed.iter().any(|b| b["name"] == "Alphaeon Cat"));
    assert!(!seed.iter().any(|b| b["name"] == "myClearbalance"));
    let food_id = seed
        .iter()
        .find(|b| b["name"] == "Food")
        .unwrap()["bucketId"]
        .as_str()
        .unwrap()
        .to_string();
    must_ok(
        &platform,
        "ExternalBucketSave",
        serde_json::json!({
            "bucketId": food_id,
            "name": "Food",
            "bank": "UCARD",
            "description": "Groceries",
            "budgetCategory": "Food"
        }),
    )
    .await;
    let after = query_json(&platform, "ExternalBucketListGet", serde_json::json!({})).await;
    let food = after["buckets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["name"] == "Food")
        .expect("Food bucket");
    assert_eq!(food["bank"], "UCARD");
    assert_eq!(food["description"], "Groceries");
    assert_eq!(food["budgetCategory"], "Food");
    // Free-text bank (Add bank… UI writes the same field).
    must_ok(
        &platform,
        "ExternalBucketSave",
        serde_json::json!({
            "bucketId": food_id,
            "name": "Food",
            "bank": "First National",
            "description": "Groceries",
            "budgetCategory": "Food"
        }),
    )
    .await;
    let custom = query_json(&platform, "ExternalBucketListGet", serde_json::json!({})).await;
    let food_custom = custom["buckets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["name"] == "Food")
        .expect("Food bucket");
    assert_eq!(food_custom["bank"], "First National");
}

#[tokio::test]
async fn bucket_rename_updates_register_lines() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let line_id = Uuid::new_v4();
    must_ok(
        &platform,
        "ExternalRegisterSave",
        serde_json::json!({
            "lines": [{
                "lineId": line_id,
                "payType": "UCARD",
                "occurredOn": "2026-09-01",
                "amountMinor": 500,
                "scale": 2,
                "category": "Food",
                "bucket": "Food",
                "vendor": "Weg",
                "description": "rename me"
            }]
        }),
    )
    .await;
    let listed = query_json(&platform, "ExternalBucketListGet", serde_json::json!({})).await;
    let food_id = listed["buckets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["name"] == "Food")
        .unwrap()["bucketId"]
        .as_str()
        .unwrap()
        .to_string();
    must_ok(
        &platform,
        "ExternalBucketSave",
        serde_json::json!({
            "bucketId": food_id,
            "name": "Groceries",
            "bank": "Capital One",
            "description": "renamed",
            "budgetCategory": "Food"
        }),
    )
    .await;
    let register = query_json(&platform, "ExternalRegisterGet", serde_json::json!({})).await;
    let line = register["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["lineId"] == line_id.to_string())
        .expect("line");
    assert_eq!(line["bucket"], "Groceries");
    let catalog = register["buckets"].as_array().unwrap();
    assert!(
        catalog.iter().any(|b| b == "Groceries") && !catalog.iter().any(|b| b == "Food"),
        "catalog follows rename: {catalog:?}"
    );
    let buckets = query_json(&platform, "ExternalBucketListGet", serde_json::json!({})).await;
    let groceries = buckets["buckets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["name"] == "Groceries")
        .expect("Groceries");
    assert_eq!(groceries["bank"], "Capital One");
}

/// Credit balance edit freezes applied history so Get/reconcile cannot rewrite Current.
/// Completed CCT rows are not updated (Mom shopping RCA / R-24).
#[tokio::test]
async fn mom_shopping_credit_balance_edit_holds_after_reconcile() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let mom_id = "a1000001-0000-4000-8000-000000000008";
    must_ok(
        &platform,
        "ExternalAccountManagerSave",
        serde_json::json!({
            "accounts": [{
                "accountId": mom_id,
                "name": "Mom shopping",
                "accountName": "Mom",
                "startingMinor": 196915,
                "currentMinor": 196915,
                "paymentMinor": null,
                "reductionMinor": null,
                "financeMinor": null,
                "paidThrough": null,
                "payProcess": "register",
                "registerKey": "Mom"
            }]
        }),
    )
    .await;
    let spend = Uuid::new_v4();
    must_ok(
        &platform,
        "ExternalRegisterSave",
        serde_json::json!({
            "lines": [{
                "lineId": spend,
                "payType": "Checking",
                "occurredOn": "2026-10-06",
                "amountMinor": 1500,
                "scale": 2,
                "category": "Mom",
                "bucket": "Mom",
                "loanName": "",
                "vendor": "Walmart",
                "description": "escrow spend before owner balance",
                "completed": true,
                "trueUpOn": "2026-10-06"
            }]
        }),
    )
    .await;
    let after_spend = query_json(&platform, "ExternalAccountManagerGet", serde_json::json!({})).await;
    let mom_after_spend = after_spend["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["accountId"] == mom_id)
        .expect("Mom shopping");
    assert_eq!(
        mom_after_spend["currentMinor"].as_i64(),
        Some(195415),
        "reconcile applies completed Mom spend once"
    );
    // Owner corrects Current (R-24 target shape); completed CCT stays $15.00.
    must_ok(
        &platform,
        "ExternalAccountManagerSave",
        serde_json::json!({
            "accounts": [{
                "accountId": mom_id,
                "name": "Mom shopping",
                "accountName": "Mom",
                "startingMinor": 196915,
                "currentMinor": 138293,
                "paymentMinor": null,
                "reductionMinor": null,
                "financeMinor": null,
                "paidThrough": null,
                "payProcess": "register",
                "registerKey": "Mom"
            }]
        }),
    )
    .await;
    let held = query_json(&platform, "ExternalAccountManagerGet", serde_json::json!({})).await;
    let mom = held["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["accountId"] == mom_id)
        .expect("Mom shopping");
    assert_eq!(
        mom["currentMinor"].as_i64(),
        Some(138293),
        "credit balance edit must survive reconcile"
    );
    assert_eq!(mom["startingMinor"].as_i64(), Some(196915));
    let register = query_json(&platform, "ExternalRegisterGet", serde_json::json!({})).await;
    let line = register["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["lineId"] == spend.to_string())
        .expect("completed CCT line");
    assert_eq!(line["amountMinor"].as_i64(), Some(1500));
    assert_eq!(line["completed"], true);
}

/// Loan Name links CCT to debts; Bucket funds escrow (Mom). Positive lowers Current; Mom negative is a deposit.
#[tokio::test]
async fn loan_name_links_debt_bucket_funds_escrow() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let cat_id = "a1000001-0000-4000-8000-000000000006";
    let mom_id = "a1000001-0000-4000-8000-000000000008";
    must_ok(
        &platform,
        "ExternalAccountManagerSave",
        serde_json::json!({
            "accounts": [{
                "accountId": mom_id,
                "name": "Mom shopping",
                "accountName": "Mom",
                "startingMinor": 100000,
                "currentMinor": 100000,
                "paymentMinor": null,
                "reductionMinor": null,
                "financeMinor": null,
                "paidThrough": null,
                "payProcess": "register",
                "registerKey": "Mom"
            }]
        }),
    )
    .await;
    let before = query_json(&platform, "ExternalAccountManagerGet", serde_json::json!({})).await;
    let cat_before = before["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["accountId"] == cat_id)
        .expect("Alphaeon Cat");
    assert_eq!(cat_before["accountName"], "Alphaeon");
    assert_eq!(cat_before["name"], "Alphaeon Cat");
    assert_eq!(cat_before["payProcess"], "element");
    let cat_start = cat_before["currentMinor"].as_i64().unwrap();
    let cat_line = Uuid::new_v4();
    let mom_spend = Uuid::new_v4();
    let mom_deposit = Uuid::new_v4();
    must_ok(
        &platform,
        "ExternalRegisterSave",
        serde_json::json!({
            "lines": [
                {
                    "lineId": cat_line,
                    "payType": "Checking",
                    "occurredOn": "2026-10-05",
                    "amountMinor": 23600,
                    "scale": 2,
                    "category": "Medical",
                    "bucket": "Medical",
                    "loanName": "Alphaeon Cat",
                    "vendor": "Alpheon",
                    "description": "cat payment",
                    "completed": true,
                    "trueUpOn": "2026-10-05"
                },
                {
                    "lineId": mom_spend,
                    "payType": "Checking",
                    "occurredOn": "2026-10-06",
                    "amountMinor": 1500,
                    "scale": 2,
                    "category": "Food",
                    "bucket": "Mom",
                    "loanName": "",
                    "vendor": "Walmart",
                    "description": "escrow spend",
                    "completed": true,
                    "trueUpOn": "2026-10-06"
                },
                {
                    "lineId": mom_deposit,
                    "payType": "Checking",
                    "occurredOn": "2026-10-07",
                    "amountMinor": -5000,
                    "scale": 2,
                    "category": "Other",
                    "bucket": "Mom",
                    "loanName": "",
                    "vendor": "Deposit",
                    "description": "escrow deposit",
                    "completed": true,
                    "trueUpOn": "2026-10-07"
                }
            ]
        }),
    )
    .await;
    let after = query_json(&platform, "ExternalAccountManagerGet", serde_json::json!({})).await;
    let cat = after["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["accountId"] == cat_id)
        .expect("Alphaeon Cat");
    let mom = after["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["accountId"] == mom_id)
        .expect("Mom shopping");
    assert_eq!(cat["currentMinor"].as_i64().unwrap(), cat_start - 23600);
    assert_eq!(cat["paidThrough"], "October");
    // 100000 - 1500 - (-5000) = 103500
    assert_eq!(mom["currentMinor"], 103500);
}

/// Week Ahead confirm on a Loan Element drafts CCT Open; balance drops only after CCT settle.
#[tokio::test]
async fn loan_element_confirm_drafts_cct_settle_moves_balance() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let cat_id = "a1000001-0000-4000-8000-000000000006";
    let element_id = "e1000001-0000-4000-8000-000000000006";
    let before = query_json(&platform, "ExternalAccountManagerGet", serde_json::json!({})).await;
    let cat_before = before["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["accountId"] == cat_id)
        .expect("Alphaeon Cat");
    assert_eq!(cat_before["payProcess"], "element");
    assert_eq!(cat_before["linkedElementId"], element_id);
    let cat_start = cat_before["currentMinor"].as_i64().unwrap();
    let occurrence_id = Uuid::new_v4();
    let element_uuid = Uuid::parse_str(element_id).unwrap();
    platform
        .planned_occurrence_upsert(PlannedOccurrenceRecord {
            occurrence_id,
            element_id: element_uuid,
            account: "Loan".into(),
            kind: "Withdrawal".into(),
            occurred_on: "2026-10-01".into(),
            amount_minor: 23600,
            confirmed_at: None,
            note: "Alphaeon Cat".into(),
            is_exception: false,
            is_cancelled: false,
        })
        .await
        .unwrap();
    must_ok(
        &platform,
        "WeekAheadConfirm",
        serde_json::json!({ "occurrenceId": occurrence_id }),
    )
    .await;
    let mid = query_json(&platform, "ExternalAccountManagerGet", serde_json::json!({})).await;
    let cat_mid = mid["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["accountId"] == cat_id)
        .unwrap();
    assert_eq!(
        cat_mid["currentMinor"].as_i64().unwrap(),
        cat_start,
        "confirm must not move Current"
    );
    let register = query_json(&platform, "ExternalRegisterGet", serde_json::json!({})).await;
    let draft = register["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["loanName"] == "Alphaeon Cat" && l["completed"] == false)
        .expect("CCT Open draft");
    assert_eq!(draft["amountMinor"], 23600);
    assert_eq!(draft["category"], "Medical");
    // Blank funding bucket projects to Medical for Medical category on read.
    assert!(
        draft["bucket"] == "" || draft["bucket"] == "Medical",
        "bucket={:?}",
        draft["bucket"]
    );
    let line_id = draft["lineId"].as_str().unwrap().to_string();
    must_ok(
        &platform,
        "ExternalRegisterSave",
        serde_json::json!({
            "lines": [{
                "lineId": line_id,
                "payType": "Checking",
                "occurredOn": "2026-10-01",
                "amountMinor": 23600,
                "scale": 2,
                "category": "Medical",
                "bucket": "Medical",
                "loanName": "Alphaeon Cat",
                "vendor": "Alpheon",
                "description": "Week Ahead loan · Alphaeon Cat",
                "completed": true,
                "trueUpOn": "2026-10-01"
            }]
        }),
    )
    .await;
    let after = query_json(&platform, "ExternalAccountManagerGet", serde_json::json!({})).await;
    let cat = after["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["accountId"] == cat_id)
        .unwrap();
    assert_eq!(cat["currentMinor"].as_i64().unwrap(), cat_start - 23600);
    assert_eq!(cat["paidThrough"], "October");
}

/// Open CCT duplicate rows delete cleanly; completed rows are refused.
#[tokio::test]
async fn open_cct_line_deletes_and_completed_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let open_id = Uuid::new_v4();
    let done_id = Uuid::new_v4();
    must_ok(
        &platform,
        "ExternalRegisterSave",
        serde_json::json!({
            "lines": [
                {
                    "lineId": open_id,
                    "payType": "UCARD",
                    "occurredOn": "2026-10-11",
                    "amountMinor": 28485,
                    "scale": 2,
                    "category": "Food",
                    "bucket": "Food",
                    "vendor": "Weg",
                    "description": "duplicate open"
                },
                {
                    "lineId": done_id,
                    "payType": "UCARD",
                    "occurredOn": "2026-10-10",
                    "amountMinor": 1000,
                    "scale": 2,
                    "category": "Food",
                    "bucket": "Food",
                    "vendor": "Weg",
                    "description": "completed keep",
                    "completed": true,
                    "trueUpOn": "2026-10-10"
                }
            ]
        }),
    )
    .await;
    must_ok(
        &platform,
        "ExternalRegisterDelete",
        serde_json::json!({ "lineIds": [open_id] }),
    )
    .await;
    let register = query_json(&platform, "ExternalRegisterGet", serde_json::json!({})).await;
    let lines = register["lines"].as_array().unwrap();
    assert!(
        lines.iter().all(|line| line["lineId"] != open_id.to_string()),
        "open duplicate must be gone"
    );
    assert!(
        lines.iter().any(|line| line["lineId"] == done_id.to_string()),
        "completed line must remain"
    );
    let refused = execute_command_on(
        &platform,
        &platform,
        cmd(
            "ExternalRegisterDelete",
            serde_json::json!({ "lineIds": [done_id] }),
        ),
    )
    .await;
    assert!(!refused.ok, "completed delete must fail");
    assert_eq!(refused.error_code.as_deref(), Some("line_completed"));
}

/// Live-clone smoke (CCT_OWNER_SMOKE=1): Add → edit Save → Transfer → Delete on a copy of live DB.
/// Prefer CCT_OWNER_SMOKE_DB (sqlite path); else copy %LOCALAPPDATA%/com.finos.desktop/local.sqlite.
#[tokio::test]
async fn cct_owner_clone_smoke_add_edit_transfer_delete() {
    if std::env::var("CCT_OWNER_SMOKE").ok().as_deref() != Some("1") {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let app = dir.path().join("app-data");
    std::fs::create_dir_all(&app).unwrap();
    let dest = app.join("local.sqlite");
    if let Ok(smoke_db) = std::env::var("CCT_OWNER_SMOKE_DB") {
        std::fs::copy(&smoke_db, &dest).unwrap();
    } else {
        let live = std::path::PathBuf::from(std::env::var("LOCALAPPDATA").expect("LOCALAPPDATA"))
            .join("com.finos.desktop")
            .join("local.sqlite");
        assert!(live.is_file(), "live db missing: {}", live.display());
        std::fs::copy(&live, &dest).unwrap();
    }
    let platform = LocalPlatform::open(&app).await.unwrap();

    let before = query_json(&platform, "ExternalAccountManagerGet", serde_json::json!({})).await;
    let mom = before["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["registerKey"] == "Mom")
        .expect("Mom registerKey on live clone");
    let mom_id = mom["accountId"].as_str().unwrap().to_string();
    let start = mom["currentMinor"].as_i64().expect("Mom currentMinor");

    let line_id = Uuid::new_v4();
    must_ok(
        &platform,
        "ExternalRegisterSave",
        serde_json::json!({
            "lines": [{
                "lineId": line_id,
                "payType": "UCARD",
                "occurredOn": "2026-10-11",
                "amountMinor": 1234,
                "scale": 2,
                "category": "Food",
                "bucket": "Mom",
                "vendor": "Smoke",
                "description": "cct-owner-smoke-open"
            }]
        }),
    )
    .await;
    must_ok(
        &platform,
        "ExternalRegisterSave",
        serde_json::json!({
            "lines": [{
                "lineId": line_id,
                "payType": "UCARD",
                "occurredOn": "2026-10-11",
                "amountMinor": 1234,
                "scale": 2,
                "category": "Food",
                "bucket": "Mom",
                "vendor": "Smoke",
                "description": "cct-owner-smoke-edited"
            }]
        }),
    )
    .await;
    let edited = query_json(&platform, "ExternalRegisterGet", serde_json::json!({})).await;
    let line = edited["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["lineId"] == line_id.to_string())
        .expect("smoke line");
    assert_eq!(line["description"], "cct-owner-smoke-edited");
    assert_eq!(line["bucket"], "Mom");

    must_ok(
        &platform,
        "ExternalRegisterMarkStep",
        serde_json::json!({
            "lineIds": [line_id],
            "step": "transfer",
            "tickedOn": "2026-10-11"
        }),
    )
    .await;
    let mid = query_json(&platform, "ExternalAccountManagerGet", serde_json::json!({})).await;
    let mom_mid = mid["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["accountId"] == mom_id)
        .unwrap();
    assert_eq!(
        mom_mid["currentMinor"].as_i64().unwrap(),
        start - 1234,
        "Transfer must debit Mom on live clone"
    );

    must_ok(
        &platform,
        "ExternalRegisterDelete",
        serde_json::json!({ "lineIds": [line_id] }),
    )
    .await;
    let after = query_json(&platform, "ExternalAccountManagerGet", serde_json::json!({})).await;
    let mom_after = after["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["accountId"] == mom_id)
        .unwrap();
    assert_eq!(
        mom_after["currentMinor"].as_i64().unwrap(),
        start,
        "Delete must restore Mom on live clone"
    );
    let gone = query_json(&platform, "ExternalRegisterGet", serde_json::json!({})).await;
    assert!(gone["lines"]
        .as_array()
        .unwrap()
        .iter()
        .all(|row| row["lineId"] != line_id.to_string()));
}

/// Mom Transfer apply then Open delete restores currentMinor.
#[tokio::test]
async fn mom_transfer_then_open_delete_restores_current() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let mom_credit_id = "a1000001-0000-4000-8000-000000000008";
    must_ok(
        &platform,
        "ExternalAccountManagerSave",
        serde_json::json!({
            "accounts": [{
                "accountId": mom_credit_id,
                "name": "Mom shopping",
                "startingMinor": 100000,
                "currentMinor": 100000,
                "paymentMinor": null,
                "reductionMinor": null,
                "financeMinor": null,
                "paidThrough": null,
                "payProcess": "register",
                "registerKey": "Mom"
            }]
        }),
    )
    .await;
    let line_id = Uuid::new_v4();
    must_ok(
        &platform,
        "ExternalRegisterSave",
        serde_json::json!({
            "lines": [{
                "lineId": line_id,
                "payType": "Checking",
                "occurredOn": "2026-10-11",
                "amountMinor": 2500,
                "scale": 2,
                "category": "Food",
                "bucket": "Mom",
                "vendor": "Walmart",
                "description": "mom escrow charge"
            }]
        }),
    )
    .await;
    must_ok(
        &platform,
        "ExternalRegisterMarkStep",
        serde_json::json!({
            "lineIds": [line_id],
            "step": "transfer",
            "tickedOn": "2026-10-11"
        }),
    )
    .await;
    let after_transfer = query_json(&platform, "ExternalAccountManagerGet", serde_json::json!({})).await;
    let mom_after_transfer = after_transfer["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["accountId"] == mom_credit_id)
        .expect("Mom shopping");
    assert_eq!(
        mom_after_transfer["currentMinor"], 97500,
        "Bucket Mom + Transfer debits escrow $25: {mom_after_transfer}"
    );
    must_ok(
        &platform,
        "ExternalRegisterDelete",
        serde_json::json!({ "lineIds": [line_id] }),
    )
    .await;
    let after_delete = query_json(&platform, "ExternalAccountManagerGet", serde_json::json!({})).await;
    let mom_after_delete = after_delete["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["accountId"] == mom_credit_id)
        .expect("Mom shopping");
    assert_eq!(
        mom_after_delete["currentMinor"], 100000,
        "delete after Transfer must restore Mom current: {mom_after_delete}"
    );
    let register = query_json(&platform, "ExternalRegisterGet", serde_json::json!({})).await;
    assert!(
        register["lines"]
            .as_array()
            .unwrap()
            .iter()
            .all(|line| line["lineId"] != line_id.to_string()),
        "deleted open line must be gone"
    );
}

/// Mom Transfer apply then Unmark Transfer restores currentMinor and clears the step.
#[tokio::test]
async fn mom_transfer_then_unmark_restores_current() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let mom_credit_id = "a1000001-0000-4000-8000-000000000008";
    must_ok(
        &platform,
        "ExternalAccountManagerSave",
        serde_json::json!({
            "accounts": [{
                "accountId": mom_credit_id,
                "name": "Mom shopping",
                "startingMinor": 100000,
                "currentMinor": 100000,
                "paymentMinor": null,
                "reductionMinor": null,
                "financeMinor": null,
                "paidThrough": null,
                "payProcess": "register",
                "registerKey": "Mom"
            }]
        }),
    )
    .await;
    let line_id = Uuid::new_v4();
    must_ok(
        &platform,
        "ExternalRegisterSave",
        serde_json::json!({
            "lines": [{
                "lineId": line_id,
                "payType": "Checking",
                "occurredOn": "2026-10-11",
                "amountMinor": 2500,
                "scale": 2,
                "category": "Food",
                "bucket": "Mom",
                "vendor": "Walmart",
                "description": "mom escrow charge unmark"
            }]
        }),
    )
    .await;
    must_ok(
        &platform,
        "ExternalRegisterMarkStep",
        serde_json::json!({
            "lineIds": [line_id],
            "step": "transfer",
            "tickedOn": "2026-10-11"
        }),
    )
    .await;
    let after_transfer = query_json(&platform, "ExternalAccountManagerGet", serde_json::json!({})).await;
    let mom_after_transfer = after_transfer["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["accountId"] == mom_credit_id)
        .expect("Mom shopping");
    assert_eq!(mom_after_transfer["currentMinor"], 97500);
    must_ok(
        &platform,
        "ExternalRegisterUnmarkStep",
        serde_json::json!({
            "lineIds": [line_id],
            "step": "transfer"
        }),
    )
    .await;
    let after_unmark = query_json(&platform, "ExternalAccountManagerGet", serde_json::json!({})).await;
    let mom_after_unmark = after_unmark["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["accountId"] == mom_credit_id)
        .expect("Mom shopping");
    assert_eq!(
        mom_after_unmark["currentMinor"], 100000,
        "Unmark Transfer must restore Mom current: {mom_after_unmark}"
    );
    let register = query_json(&platform, "ExternalRegisterGet", serde_json::json!({})).await;
    let line = register["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["lineId"] == line_id.to_string())
        .expect("line still open");
    assert_eq!(line["stepTransfer"], false);
    assert_eq!(line["completed"], false);
}

/// Debt: mark all four CCT steps → apply; Unmark Pay Bill → reopen and restore Current.
#[tokio::test]
async fn debt_all_steps_then_unmark_pay_reopens_and_restores() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data"))
        .await
        .unwrap();
    let cat_id = "a1000001-0000-4000-8000-000000000006";
    let before = query_json(&platform, "ExternalAccountManagerGet", serde_json::json!({})).await;
    let cat_before = before["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["accountId"] == cat_id)
        .expect("Alphaeon Cat");
    let cat_start = cat_before["currentMinor"].as_i64().unwrap();
    let line_id = Uuid::new_v4();
    must_ok(
        &platform,
        "ExternalRegisterSave",
        serde_json::json!({
            "lines": [{
                "lineId": line_id,
                "payType": "Checking",
                "occurredOn": "2026-10-12",
                "amountMinor": 16461,
                "scale": 2,
                "category": "Medical",
                "bucket": "Medical",
                "loanName": "Alphaeon Cat",
                "vendor": "CAP",
                "description": "unmark pay reopen"
            }]
        }),
    )
    .await;
    for step in ["transfer", "billpay_deposit", "pay", "withdrawal"] {
        must_ok(
            &platform,
            "ExternalRegisterMarkStep",
            serde_json::json!({
                "lineIds": [line_id],
                "step": step,
                "tickedOn": "2026-10-12"
            }),
        )
        .await;
    }
    let after_mark = query_json(&platform, "ExternalAccountManagerGet", serde_json::json!({})).await;
    let cat_marked = after_mark["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["accountId"] == cat_id)
        .expect("Alphaeon Cat");
    assert_eq!(
        cat_marked["currentMinor"].as_i64().unwrap(),
        cat_start - 16461,
        "all four steps must apply debt Current"
    );
    let completed = query_json(&platform, "ExternalRegisterGet", serde_json::json!({})).await;
    let done_line = completed["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["lineId"] == line_id.to_string())
        .expect("line");
    assert_eq!(done_line["completed"], true);
    must_ok(
        &platform,
        "ExternalRegisterUnmarkStep",
        serde_json::json!({
            "lineIds": [line_id],
            "step": "pay"
        }),
    )
    .await;
    let after_unmark = query_json(&platform, "ExternalAccountManagerGet", serde_json::json!({})).await;
    let cat_restored = after_unmark["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["accountId"] == cat_id)
        .expect("Alphaeon Cat");
    assert_eq!(
        cat_restored["currentMinor"].as_i64().unwrap(),
        cat_start,
        "Unmark Pay Bill must restore debt Current: {cat_restored}"
    );
    let register = query_json(&platform, "ExternalRegisterGet", serde_json::json!({})).await;
    let line = register["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["lineId"] == line_id.to_string())
        .expect("line");
    assert_eq!(line["stepPay"], false);
    assert_eq!(line["completed"], false);
    assert!(line["trueUpOn"].is_null() || line["trueUpOn"] == "");
}
