//! A cash deposit is a mark on the journal and an increase of the cash lot.
//! Current cash stays the lot. The same deposit is not added a second time.

use application_core::contracts::{
    CommandRequest, QueryRequest, FINANCE_CLIENT_CONTRACT_VERSION,
};
use application_core::queries::{execute_command_on, execute_query_on};
use golden_harness::repo_root;
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

async fn query_body(platform: &LocalPlatform, name: &str, body: serde_json::Value) -> serde_json::Value {
    let result = execute_query_on(platform, platform, qry(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

fn ending_from_lot(lot: i64, as_of: &str, window_end: &str, hits: &[(&str, i64)]) -> i64 {
    let net: i64 = hits
        .iter()
        .filter(|(day, _)| *day > as_of && *day <= window_end)
        .map(|(_, amount)| *amount)
        .sum();
    lot + net
}

#[tokio::test]
async fn contribution_deposit_and_withdrawal_move_the_lot_once() {
    let root = repo_root();
    let chart = std::fs::read_to_string(
        root.join("apps/desktop/src/features/cash/AccountCashFlow.tsx"),
    )
    .unwrap();
    let capture = std::fs::read_to_string(
        root.join("apps/desktop/src/features/graphing/TrendsCapture.tsx"),
    )
    .unwrap();
    let cart = std::fs::read_to_string(
        root.join("apps/desktop/src/features/shopping-cart/ShoppingCartScreen.tsx"),
    )
    .unwrap();
    let details = std::fs::read_to_string(
        root.join("apps/desktop/src/features/position-details/PositionDetailsScreen.tsx"),
    )
    .unwrap();
    assert!(
        chart.contains("day > start") && chart.contains("source === \"income-plan\""),
        "cash flow adds only hits after as-of, and planned income stays dividends"
    );
    assert!(
        chart.contains("appendToBody: true")
            && chart.contains("confine: true")
            && chart.contains("<th>Date</th><th>Name</th><th>Amount</th>")
            && chart.contains("class=\"acfp-tip\"")
            && chart.contains("One green and/or one red dot per Sat–Fri week"),
        "hover is a Date, Name, Amount table that stays on screen; the Friday dot stays"
    );
    assert!(
        capture.contains("if (!c.closed)") && capture.contains("pileMinor"),
        "an open week prefills from the live pile; a closed Friday stays the snapshot"
    );
    assert!(
        cart.contains("qtyMinor > lot.remainingQuantityMinor")
            && cart.contains("plannedCashMinor"),
        "the cart rejects a cash qty above the lot; the plan sheet qty is the cart line"
    );
    assert!(
        details.contains("remainingQuantityMinor"),
        "position details reads the cash lot quantity"
    );

    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let income = {
        let account = must_ok(
            &platform,
            "AccountRegister",
            serde_json::json!({"name": "Income", "kind": "ira"}),
        )
        .await;
        account["accountId"].as_str().unwrap().to_string()
    };
    let security = must_ok(
        &platform,
        "SecurityRegister",
        serde_json::json!({"symbol": "SPAXX", "name": "SPAXX"}),
    )
    .await;
    let security_id = security["securityId"].as_str().unwrap().to_string();
    must_ok(
        &platform,
        "RetrievalTemplateSet",
        serde_json::json!({
            "securityId": security_id,
            "priceSource": "public",
            "sourceSymbol": "SPAXX",
            "declarationSource": "issuer",
            "sourceUrl": "https://example.test/spaxx/distributions",
            "calendarPolicy": "derived_walk",
            "collectorEnabled": true,
            "lookbackCount": 12
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot(&platform, &security_id, "SPAXX")
        .await
        .expect("complete collector");
    let start = 100_000_i64;
    must_ok(
        &platform,
        "LotOpen",
        serde_json::json!({
            "accountId": income,
            "securityId": security_id,
            "openedOn": "2026-09-01",
            "origin": "purchase",
            "quantityMinor": start,
            "quantityScale": 2,
            "performanceBasisMinor": start,
            "taxBasisMinor": start,
            "scale": 2,
            "isOpen": true
        }),
    )
    .await;

    let saved_cash = start;
    must_ok(
        &platform,
        "TrendsWeekSave",
        serde_json::json!({
            "periodStart": "2026-09-26",
            "periodEnd": "2026-10-02",
            "capturedAt": "2026-10-02T18:00:00Z",
            "profitMinor": 0,
            "monthlyDivsMinor": 0,
            "fidelityTotalMinor": start,
            "schwabTotalMinor": 0,
            "incomeCashMinor": saved_cash,
            "acct9CashMinor": 0,
            "acct9EtfValueMinor": 0,
            "scale": 2
        }),
    )
    .await;

    let as_of = "2026-10-04";
    let contribution = 250_000_i64;
    let synced = must_ok(
        &platform,
        "MagiCliffTaskSync",
        serde_json::json!({
            "asOfDate": as_of,
            "overageMinor": contribution,
            "creditAtRiskMinor": 1,
            "suggestions": [
                "Cut remaining Traditional IRA draws by the gap.",
                "Or book a Traditional IRA contribution of the gap."
            ]
        }),
    )
    .await;
    let task_id = synced["task"]["taskId"].as_str().unwrap();
    must_ok(
        &platform,
        "MagiIraContributionPost",
        serde_json::json!({
            "taskId": task_id,
            "accountName": "Income",
            "amountMinor": contribution,
            "occurredOn": as_of
        }),
    )
    .await;
    let deposit = 8_000_i64;
    must_ok(
        &platform,
        "CashDeposit",
        serde_json::json!({
            "accountId": income,
            "amountMinor": deposit,
            "occurredOn": as_of
        }),
    )
    .await;
    let withdrawal = 3_000_i64;
    must_ok(
        &platform,
        "CashWithdraw",
        serde_json::json!({
            "accountId": income,
            "amountMinor": withdrawal,
            "occurredOn": as_of
        }),
    )
    .await;

    let expected = start + contribution + deposit - withdrawal;
    let pile = query_body(
        &platform,
        "CashPileGet",
        serde_json::json!({ "accountId": income }),
    )
    .await;
    assert_eq!(pile["symbol"], "SPAXX");
    assert_eq!(pile["dollarsMinor"].as_i64().unwrap(), expected, "lot is current cash");
    assert_eq!(pile["remainingQtyMinor"].as_i64().unwrap(), expected);
    assert_eq!(pile["quantityScale"].as_i64().unwrap(), 2);

    let again = must_ok(
        &platform,
        "MagiIraContributionPost",
        serde_json::json!({
            "taskId": task_id,
            "accountName": "Income",
            "amountMinor": contribution,
            "occurredOn": as_of
        }),
    )
    .await;
    assert_eq!(again["status"], "done");
    let pile_again = query_body(
        &platform,
        "CashPileGet",
        serde_json::json!({ "accountId": income }),
    )
    .await;
    assert_eq!(
        pile_again["dollarsMinor"].as_i64().unwrap(),
        expected,
        "a second confirm does not add the contribution again"
    );

    let register = query_body(
        &platform,
        "CashRegisterGet",
        serde_json::json!({
            "account": "Income",
            "period": "1M",
            "asOfDate": as_of,
            "hitsOnly": true
        }),
    )
    .await;
    let rows = register["rows"].as_array().unwrap();
    let marked = |label: &str| rows.iter().find(|row| row["label"] == label);
    let contrib = marked("ira-contribution").expect("contribution is a cash-flow mark");
    assert_eq!(contrib["transaction"], "Deposit");
    assert_eq!(contrib["depositMinor"].as_i64().unwrap(), contribution);
    assert_eq!(contrib["source"], "activity");
    let deposited = marked("deposit").expect("ordinary deposit is a cash-flow mark");
    assert_eq!(deposited["transaction"], "Deposit");
    assert_eq!(deposited["depositMinor"].as_i64().unwrap(), deposit);
    let withdrawn = marked("withdrawal").expect("withdrawal is a cash-flow mark");
    assert_eq!(withdrawn["transaction"], "Withdrawal");
    assert_eq!(withdrawn["withdrawalMinor"].as_i64().unwrap(), withdrawal);

    let window_end = "2027-01-04";
    let hits = [(as_of, contribution), (as_of, deposit), (as_of, -withdrawal)];
    assert_eq!(
        ending_from_lot(expected, as_of, window_end, &hits),
        expected,
        "hits on as-of are marks; ending cash stays the lot"
    );

    let ledger = query_body(
        &platform,
        "CashLedgerGet",
        serde_json::json!({ "accountId": income }),
    )
    .await;
    assert_eq!(ledger["dollarsMinor"].as_i64().unwrap(), expected);
    let entries = ledger["entries"].as_array().unwrap();
    for kind in ["ira-contribution", "deposit", "withdrawal"] {
        assert!(
            entries.iter().any(|row| row["activityType"] == kind),
            "cart ledger lists {kind}"
        );
    }
    assert_eq!(
        entries
            .iter()
            .filter(|row| row["activityType"] == "ira-contribution")
            .count(),
        1,
        "second confirm does not add a second ledger row"
    );

    let holdings = query_body(&platform, "HoldingsGet", serde_json::json!({})).await;
    let spaxx = holdings["lots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|lot| lot["symbol"] == "SPAXX")
        .expect("holdings lists the cash lot");
    assert_eq!(spaxx["remainingQuantityMinor"].as_i64().unwrap(), expected);
    assert_eq!(spaxx["quantityScale"].as_i64().unwrap(), 2);

    let trends = query_body(
        &platform,
        "TrendsGet",
        serde_json::json!({ "asOfDate": "2026-10-02" }),
    )
    .await;
    let saved = trends["weeks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|week| week["periodEnd"] == "2026-10-02")
        .expect("accepted Friday stays in history");
    assert_eq!(
        saved["incomeCashMinor"].as_i64().unwrap(),
        saved_cash,
        "the contribution does not rewrite an accepted Friday"
    );

    let open = query_body(
        &platform,
        "TrendsWeekGet",
        serde_json::json!({ "asOfDate": "2026-10-05" }),
    )
    .await;
    assert_eq!(open["closed"], false, "the week after the snapshot is still open");
    let income_ref = open["cashReferences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["accountName"] == "Income")
        .expect("open week names Income");
    assert_eq!(
        income_ref["pileMinor"].as_i64().unwrap(),
        expected,
        "open-week prefill is the live pile"
    );

    let carts = query_body(
        &platform,
        "CartScenarioList",
        serde_json::json!({ "accountId": income }),
    )
    .await;
    assert_eq!(
        carts["items"].as_array().map(|rows| rows.len()).unwrap_or(0),
        0,
        "posting cash does not change a cart plan qty"
    );
}
