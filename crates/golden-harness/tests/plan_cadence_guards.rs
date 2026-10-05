//! Future-plan cadence bands + ticket Steps/Remediation lock.

use application_core::contracts::{CommandRequest, FINANCE_CLIENT_CONTRACT_VERSION};
use application_core::ports::canonical::Canonical;
use application_core::queries::execute_command_on;
use financial_domain::calculator::PaymentCadence;
use financial_domain::collector::INCOME_FLEET_SYMBOLS;
use financial_domain::declaration_post::{
    cadence_count_window, cadence_gap_band, cadence_twin_gap_ceiling_days,
    consecutive_gap_band_issues, count_window_ok, filter_dates_to_gap_band,
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

async fn must_ok(platform: &LocalPlatform, name: &str, body: serde_json::Value) -> serde_json::Value {
    let result = execute_command_on(platform, platform, cmd(name, body)).await;
    assert!(result.ok, "{name} failed: {:?}", result.error_code);
    serde_json::from_str(result.body_json.as_deref().unwrap_or("{}")).unwrap()
}

async fn seed_div1(platform: &LocalPlatform, symbol: &str, freq: &str, source: &str) -> String {
    let account = must_ok(
        platform,
        "AccountRegister",
        serde_json::json!({"name": "Income", "kind": "taxable"}),
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
            "declarationSource": source,
            "sourceSymbol": symbol,
            "sourceUrl": format!("https://example.test/{}/distributions", symbol.to_ascii_lowercase()),
            "rocSourceUrl": format!("https://example.test/{}/19a-1", symbol.to_ascii_lowercase()),
            "calendarPolicy": "issuer_calendar",
            "collectorEnabled": true,
            "lookbackCount": 12
        }),
    )
    .await;
    must_ok(
        platform,
        "PositionCharacteristicUpsert",
        serde_json::json!({
            "securityId": security_id,
            "paymentFrequency": freq,
            "provider": "Test",
            "divType": "DIV-1",
            "isActive": true
        }),
    )
    .await;
    golden_harness::complete_collector_for_first_lot_as(platform, &security_id, symbol, freq, true)
        .await
        .expect("complete collector");
    must_ok(
        platform,
        "LotOpen",
        serde_json::json!({
            "accountId": account["accountId"],
            "securityId": security_id,
            "openedOn": "2026-01-02",
            "quantityMinor": 10,
            "quantityScale": 0,
            "performanceCostMinor": 1000,
            "taxCostMinor": 1000,
            "scale": 2
        }),
    )
    .await;
    security_id
}

/// Stride comes from the cadence's own gap band, so a new cadence cannot silently fall
/// through to a 30-day step and pretend to be tested. Twice monthly fell through before.
fn synthetic_series(freq: &str, last: &str, n: usize) -> Vec<String> {
    let step = match PaymentCadence::parse(freq) {
        Some(PaymentCadence::Weekly) => 7i64,
        Some(PaymentCadence::TwiceMonthly) => 15,
        Some(PaymentCadence::Monthly) => 30,
        Some(PaymentCadence::Quarterly) => 92,
        Some(PaymentCadence::None) | None => {
            panic!("no synthetic stride for {freq:?}; add one with its gap band")
        }
    };
    let start = chrono::NaiveDate::parse_from_str(last, "%Y-%m-%d").unwrap();
    (0..n)
        .map(|i| {
            (start + chrono::Duration::days(step * i as i64))
                .format("%Y-%m-%d")
                .to_string()
        })
        .collect()
}

#[test]
fn plan_gap_band_and_count_window_pure() {
    let et_bad = filter_dates_to_gap_band(
        &["2026-09-30".into(), "2026-11-19".into()],
        "Quarterly",
        Some("2026-08-19"),
    );
    assert_eq!(et_bad, vec!["2026-11-19".to_string()]);
    let good = ["2026-08-19", "2026-11-19", "2027-02-19", "2027-05-20"];
    let g: Vec<&str> = good.iter().copied().collect();
    assert!(consecutive_gap_band_issues(&g, "Quarterly").is_empty());
    // Forward window from last pay ≤ as_of (domain lock); as_of near first keeps full count.
    assert!(count_window_ok(&g, "Quarterly", "2026-10-01"));

    let weekly = synthetic_series("Weekly", "2026-01-02", 52);
    let wrefs: Vec<&str> = weekly.iter().map(String::as_str).collect();
    assert!(consecutive_gap_band_issues(&wrefs, "Weekly").is_empty());
    assert!(count_window_ok(&wrefs, "Weekly", "2026-01-02"));

    let monthly = synthetic_series("Monthly", "2026-01-30", 12);
    let mrefs: Vec<&str> = monthly.iter().map(String::as_str).collect();
    assert!(consecutive_gap_band_issues(&mrefs, "Monthly").is_empty());
    assert!(count_window_ok(&mrefs, "Monthly", "2026-01-30"));
}

/// Driven by the cadence enum, not by a roster of symbols. Adding a `PaymentCadence`
/// variant without bands is a compile error here, and adding an investment cannot turn this
/// red. Twice monthly is in scope: it was the cadence that shipped with no class at all.
#[test]
fn every_paying_cadence_has_bands_and_passes_its_own_series() {
    for cadence in [
        PaymentCadence::Weekly,
        PaymentCadence::TwiceMonthly,
        PaymentCadence::Monthly,
        PaymentCadence::Quarterly,
        PaymentCadence::None,
    ] {
        let label = cadence.label();
        let Some(periods) = cadence.periods() else {
            // Exhaustive by construction: the only variant without periods is None.
            assert!(matches!(cadence, PaymentCadence::None), "{label}");
            assert!(cadence_gap_band(label).is_none(), "{label} must have no band");
            continue;
        };
        let (lo, hi) = cadence_gap_band(label)
            .unwrap_or_else(|| panic!("{label} pays {periods}x but has no gap band"));
        assert!(lo > 0 && lo <= hi, "{label} band {lo}-{hi} is not usable");
        let (need, window) = cadence_count_window(label)
            .unwrap_or_else(|| panic!("{label} pays {periods}x but has no count window"));
        assert_eq!(
            need,
            usize::from(periods),
            "{label} count window must require one pay per period"
        );
        assert!(window >= 365, "{label} window {window} is under a year");
        assert!(
            cadence_twin_gap_ceiling_days(label).is_some(),
            "{label} has no ex/record twin ceiling"
        );

        let start = "2026-01-02";
        let series = synthetic_series(label, start, usize::from(periods));
        let refs: Vec<&str> = series.iter().map(String::as_str).collect();
        assert!(
            consecutive_gap_band_issues(&refs, label).is_empty(),
            "{label} gaps: {:?}",
            consecutive_gap_band_issues(&refs, label)
        );
        assert!(
            count_window_ok(&refs, label, start),
            "{label} count window failed for {series:?}"
        );
    }
}

/// The growth-safe half: every collector the live book has enabled must carry a stored
/// frequency that maps to a cadence class with bands. A symbol added today is covered the
/// moment it is enabled, with no list for anyone to remember to edit.
#[tokio::test]
async fn live_enabled_collectors_all_map_to_a_cadence_class() {
    let dir = golden_harness::profile_a_app_dir();
    let db = dir.join("local.sqlite");
    if !db.is_file() {
        eprintln!("no live book at {}; cadence roster check skipped", db.display());
        return;
    }
    let platform = LocalPlatform::open(&dir).await.expect("open data sqlite");
    let characteristics = platform
        .position_characteristic_list()
        .await
        .expect("characteristics");
    let mut unclassified = Vec::new();
    let mut classified = 0usize;
    for security in platform.security_list().await.expect("securities") {
        let Some(template) = platform
            .retrieval_template_get(security.security_id)
            .await
            .expect("template")
        else {
            continue;
        };
        if !template.collector_enabled {
            continue;
        }
        let frequency = characteristics
            .iter()
            .find(|c| c.security_id == security.security_id)
            .map(|c| c.payment_frequency.clone())
            .unwrap_or_default();
        // Cash pays on a money-market schedule, not an issuer cadence.
        if financial_domain::current_price::uses_cash_par("", &security.symbol) {
            continue;
        }
        match PaymentCadence::parse(&frequency) {
            Some(PaymentCadence::None) | None => {
                unclassified.push(format!("{} ({frequency:?})", security.symbol))
            }
            Some(c) => {
                let label = c.label();
                assert!(
                    cadence_gap_band(label).is_some() && cadence_count_window(label).is_some(),
                    "{} is {label} but that class has no bands",
                    security.symbol
                );
                classified += 1;
            }
        }
    }
    assert!(
        unclassified.is_empty(),
        "enabled collectors with no usable payment frequency: {unclassified:?}"
    );
    assert!(
        classified >= INCOME_FLEET_SYMBOLS.len() - 4,
        "only {classified} enabled payers classified; the locked roster is {}",
        INCOME_FLEET_SYMBOLS.len()
    );
}

#[tokio::test]
async fn et_remaining_keeps_nov_drops_sep_invent() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1(&platform, "ET", "Quarterly", "mlp_sec_8k").await;
    must_ok(
        &platform,
        "IssuerDeclarationRecord",
        serde_json::json!({
            "securityId": security_id,
            "amountPerShareMinor": 3400,
            "amountScale": 4,
            "paymentPeriod": "2026-08-19",
            "source": "sec_8k",
            "enteredAt": "2026-08-19"
        }),
    )
    .await;
    must_ok(
        &platform,
        "IssuerPayDateReplace",
        serde_json::json!({
            "securityId": security_id,
            "asOfDate": "2026-09-01",
            "dates": [
                {"payOn": "2026-09-30", "source": "derived_walk"},
                {"payOn": "2026-11-19", "source": "derived_template"}
            ]
        }),
    )
    .await;
    let sid = Uuid::parse_str(&security_id).unwrap();
    let pays: Vec<String> = platform
        .issuer_pay_date_list(sid)
        .await
        .unwrap()
        .into_iter()
        .map(|p| p.pay_on)
        .collect();
    let filtered = filter_dates_to_gap_band(&pays, "Quarterly", Some("2026-08-19"));
    assert!(
        !filtered.iter().any(|d| d == "2026-09-30"),
        "Sep invent must drop: {filtered:?}"
    );
    assert!(
        filtered.iter().any(|d| d == "2026-11-19"),
        "Nov template must remain: {filtered:?}"
    );
}

fn assert_ticket_diagnostics(reason: &str) {
    assert!(
        reason.contains("Steps ("),
        "missing Steps block: {reason}"
    );
    assert!(reason.contains("pass"), "missing pass step: {reason}");
    assert!(reason.contains("FAIL"), "missing FAIL step: {reason}");
    assert!(
        reason.contains("Remediation:"),
        "missing Remediation action plan: {reason}"
    );
    let after = reason.split("Remediation:").nth(1).unwrap_or("");
    assert!(
        after.lines().any(|l| l.trim().starts_with("1.")),
        "Remediation needs numbered action: {reason}"
    );
}

#[tokio::test]
async fn open_ticket_reason_includes_steps_pass_fail_and_remediation() {
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1(&platform, "EPD", "Quarterly", "enterprise").await;

    must_ok(
        &platform,
        "CollectorRetrieve",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "EPD",
            "declarationSource": "enterprise",
            "paymentFrequency": "Quarterly",
            "asOfDate": "2026-09-20",
            "candidates": [],
            "pagePaid": [],
            "misses": [{
                "securityId": security_id,
                "symbol": "EPD",
                "code": "declaration_retrieve_miss",
                "reason": "Issuer page empty"
            }]
        }),
    )
    .await;

    let sid = Uuid::parse_str(&security_id).unwrap();
    let tickets = platform
        .work_ticket_list(Some(sid), Some("open".into()))
        .await
        .unwrap();
    let miss = tickets
        .iter()
        .find(|t| t.code == "declaration_retrieve_miss")
        .expect("miss ticket");
    assert!(!miss.retrieve_run_id.trim().is_empty());
    assert_ticket_diagnostics(&miss.reason);

    let _ = must_ok(
        &platform,
        "CollectorRecertify",
        serde_json::json!({
            "securityId": security_id,
            "symbol": "EPD",
            "asOfDate": "2026-09-20",
            "trigger": "golden"
        }),
    )
    .await;
    let tickets2 = platform
        .work_ticket_list(Some(sid), Some("open".into()))
        .await
        .unwrap();
    if let Some(t) = tickets2
        .iter()
        .find(|t| t.code == "collector_establish_incomplete")
    {
        assert_ticket_diagnostics(&t.reason);
    }

    let queries = std::fs::read_to_string(
        golden_harness::repo_root().join("crates/application-core/src/queries.rs"),
    )
    .unwrap();
    assert!(
        queries.contains("Remediation:") && queries.contains("format_collector_ticket_reason"),
        "ticket formatter must emit Remediation block"
    );
}

#[tokio::test]
async fn sync_misses_rewrites_opaque_open_ticket_reasons() {
    use application_core::contracts::WorkTicketRecord;
    let dir = tempfile::tempdir().unwrap();
    let platform = LocalPlatform::open(dir.path().join("app-data")).await.unwrap();
    let security_id = seed_div1(&platform, "BITO", "Monthly", "issuer").await;
    let sid = Uuid::parse_str(&security_id).unwrap();
    // Simulate pre-Remediation opaque ticket still open in owner book.
    platform
        .work_ticket_raise(WorkTicketRecord {
            ticket_id: Uuid::new_v4(),
            security_id: sid,
            symbol: "BITO".into(),
            field: "remaining_year".into(),
            code: "remaining_year".into(),
            tool: "fix_remaining_year".into(),
            reason: "Remaining-year stored dates disagree with the issuer list through 31 Dec."
                .into(),
            urls_tried: "[]".into(),
            opened_on: "2026-10-01".into(),
            last_seen_on: "2026-10-01".into(),
            status: "open".into(),
            filed_on: String::new(),
            completed_how: String::new(),
            owner_note: String::new(),
            retrieve_run_id: String::new(),
        })
        .await
        .unwrap();
    let before = platform
        .work_ticket_list(Some(sid), Some("open".into()))
        .await
        .unwrap();
    assert_eq!(before.len(), 1);
    assert!(!before[0].reason.contains("Remediation:"));
    must_ok(&platform, "WorkTicketSyncMisses", serde_json::json!({})).await;
    let after = platform
        .work_ticket_list(Some(sid), Some("open".into()))
        .await
        .unwrap();
    assert_eq!(after.len(), 1);
    assert_ticket_diagnostics(&after[0].reason);
}

#[test]
fn ticket_formatter_sentinel_cannot_be_removed() {
    let queries = std::fs::read_to_string(
        golden_harness::repo_root().join("crates/application-core/src/queries.rs"),
    )
    .unwrap();
    assert!(queries.contains("fn format_collector_ticket_reason"));
    assert!(
        queries.contains(r#"push_str("\nRemediation:")"#),
        "formatter must still emit Remediation block"
    );
    assert!(queries.contains("fn remediation_lines_for_code"));
    assert!(queries.contains("fn ensure_ticket_reason_diagnostics"));
}
