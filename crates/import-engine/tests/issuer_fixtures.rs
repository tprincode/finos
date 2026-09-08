//! Fixture contract tests — no live HTTP. Table-shaped issuer pages only.

use import_engine::{
    collect_from_fetched_page, parse_amplify_distribution_pack, parse_amplify_distributions,
    parse_cornerstone_press, parse_div1_distributions, jpmorgan_cusip_from_seed, parse_globalx_distribution_history, parse_jpmorgan_distributions,
    parse_generic_distributions,
    parse_moneymarket_distributions, parse_nasdaq_dividends, parse_neos_distributions,
    parse_production_templates, parse_proshares_distribution_summary,
    parse_roundhill_distribution_api, parse_roundhill_distributions, parse_saba_distributions,
    parse_yieldmax_distributions, DeclarationTarget,
};
use financial_domain::div1::{
    declaration_source_for_provider, is_div1, is_registered_declaration_source,
};

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(path).expect(name)
}

fn target(symbol: &str, source: &str) -> DeclarationTarget {
    DeclarationTarget {
        security_id: "sec-fix".into(),
        symbol: symbol.into(),
        declaration_source: source.into(),
        source_symbol: symbol.into(),
        ..Default::default()
    }
}

fn target_inception(symbol: &str, source: &str, inception: &str, frequency: &str) -> DeclarationTarget {
    DeclarationTarget {
        security_id: "sec-fix".into(),
        symbol: symbol.into(),
        declaration_source: source.into(),
        source_symbol: symbol.into(),
        inception_on: inception.into(),
        payment_frequency: frequency.into(),
        ..Default::default()
    }
}

#[test]
fn amplify_haky_blank_remaining_year_is_null_not_zero() {
    let html = fixture("amplify_haky.html");
    let parsed = parse_amplify_distributions(&html);
    assert_eq!(parsed.len(), 3);
    assert!(parsed[0]["amountPerShareMinor"].is_null());
    assert_eq!(parsed[2]["amountPerShareMinor"], 38826);
}

#[test]
fn cornerstone_press_keeps_only_requested_symbol() {
    let text = fixture("cornerstone_press.txt");
    let parsed = parse_cornerstone_press(&text, "PAY1");
    assert_eq!(parsed.len(), 3);
    assert_eq!(parsed[0]["paymentPeriod"], "2026-09-30");
    assert_eq!(parsed[0]["amountPerShareMinor"], 1215);
    assert_eq!(parsed[0]["amountScale"], 4);
    assert!(parsed.iter().all(|c| c["source"] == "cornerstone"));
    let other = parse_cornerstone_press(&text, "XXX");
    assert_eq!(other.len(), 1);
    assert_eq!(other[0]["amountPerShareMinor"], 9999);
    let table = import_engine::parse_div1_distributions(
        "cornerstone",
        "<table><tr><th>Payable Date</th><th>Distribution Amount</th></tr><tr><td>2026-09-30</td><td>$0.1215</td></tr></table>",
    );
    assert_eq!(table.len(), 1);
}

#[test]
fn gladstone_press_keeps_common_ignores_series_a() {
    let text = "Released July 14, 2026 Cash Distributions: Common Stock: $0.15 per share \
        Record Date Payment Date Cash Distribution July 24 July 31 $ 0.15 August 18 August 31 $ 0.15 \
        September 21 September 30 $ 0.15 Total for the Quarter : $ 0.45 \
        Series A Preferred Stock: $0.130208 July 28 August 5 $ 0.130208";
    let parsed = parse_div1_distributions("gladstone", text);
    assert_eq!(parsed.len(), 3, "{parsed:?}");
    assert_eq!(parsed[0]["paymentPeriod"], "2026-09-30");
    assert_eq!(parsed[0]["amountPerShareMinor"], 15);
    assert!(
        parsed
            .iter()
            .all(|c| c["amountPerShareMinor"] != 130208),
        "Series A must not post: {parsed:?}"
    );
}

#[test]
fn rexshares_calendar_keeps_cash_pay_skips_zero() {
    let text = "Distribution Calendar: Distribution Per Share Declaration Date EX Date \
        Record Date Payable$0.027908/24/202608/25/202608/25/202608/26/2026\
        $0.000012/23/202512/24/202512/24/202512/26/2025\
        $0.000012/23/202412/24/202412/24/202412/26/2024";
    let parsed = parse_div1_distributions("trex", text);
    assert_eq!(parsed.len(), 1, "{parsed:?}");
    assert_eq!(parsed[0]["paymentPeriod"], "2026-08-26");
    assert_eq!(parsed[0]["amountPerShareMinor"], 279);
    assert_eq!(parsed[0]["amountScale"], 4);
    let html = r#"<div id="distribution-wrapper"><div class="t-row"><div class="t-col t-header">Distribution Per Share</div><div class="t-col t-header">Declaration Date</div><div class="t-col t-header">EX Date</div><div class="t-col t-header">Record Date</div><div class="t-col t-header">Payable</div></div><div class="t-row"><div class="t-col t-data">$0.0279</div><div class="t-col t-data">08/24/2026</div><div class="t-col t-data">08/25/2026</div><div class="t-col t-data">08/25/2026</div><div class="t-col t-data">08/26/2026</div></div><div class="t-row"><div class="t-col t-data">$0.0000</div><div class="t-col t-data">12/23/2025</div><div class="t-col t-data">12/24/2025</div><div class="t-col t-data">12/24/2025</div><div class="t-col t-data">12/26/2025</div></div><div class="t-row"><div class="t-col t-data">$0.0000</div><div class="t-col t-data">12/23/2024</div><div class="t-col t-data">12/24/2024</div><div class="t-col t-data">12/24/2024</div><div class="t-col t-data">12/26/2024</div></div></div>"#;
    let from_html = parse_div1_distributions("trex", html);
    assert_eq!(from_html.len(), 1, "{from_html:?}");
    assert_eq!(from_html[0]["paymentPeriod"], "2026-08-26");
    assert_eq!(from_html[0]["amountPerShareMinor"], 279);
    assert!(import_engine::rexshares_calendar_covers_inception(
        html,
        "2024-09-18",
        "2026-09-08"
    ));
    let out = collect_from_fetched_page(
        &target_inception("MSTU", "trex", "2024-09-18", "Quarterly"),
        "trex",
        Some(html),
    );
    assert_eq!(out.candidates.len(), 1, "{:?}", out.candidates);
    assert_eq!(out.candidates[0]["paymentPeriod"], "2026-08-26");
    assert!(
        out.misses.is_empty(),
        "issuer calendar covers 2024-2026; do not invent quarterly pays: {:?}",
        out.misses
    );
}

#[test]
fn globalx_19a_notice_reads_pay_amount_and_roc() {
    let text = "Ticker: PAY1 Record Date: August 24, 2026 Pay Date: August 27, 2026 \
        Distribution Amount Per Share: $0.1829 \
        Return of Capital $0.1824 99.72 % Total (per Capital Share) $0.1829 100.00%";
    let parsed = parse_div1_distributions("globalx", text);
    assert_eq!(parsed.len(), 1, "{parsed:?}");
    assert_eq!(parsed[0]["paymentPeriod"], "2026-08-27");
    assert_eq!(
        parsed[0]["amountPerShareMinor"],
        1829,
        "row={:?}",
        parsed[0]
    );
    assert_eq!(parsed[0]["amountScale"], 4);
}

#[test]
fn ftvest_history_years_from_dropdown() {
    let html = r#"<select name="ctl00$ContentPlaceHolder1$DistributionHistory$ddlDistributionHistoryYearSelection" id="x">
<option value="2021">2021</option>
<option selected="selected" value="2026">2026</option>
</select>"#;
    let years = import_engine::ftvest_history_years(html);
    assert_eq!(years, vec!["2021".to_string(), "2026".to_string()]);
}

#[test]
fn tappalpha_view_all_parses_paid_and_blank() {
    let body = fixture("tappalpha_distributions.json");
    let parsed = parse_div1_distributions("tappalpha", &body);
    assert_eq!(parsed.len(), 3);
    assert_eq!(parsed[0]["paymentPeriod"], "2026-10-02");
    assert!(parsed[0]["amountPerShareMinor"].is_null());
    assert_eq!(parsed[1]["paymentPeriod"], "2026-09-02");
    assert_eq!(parsed[1]["amountPerShareMinor"], 29488);
    assert_eq!(parsed[1]["amountScale"], 5);
    let out = collect_from_fetched_page(
        &target_inception("PAY1", "tappalpha", "2026-08-01", "Monthly"),
        "tappalpha",
        Some(&body),
    );
    assert!(out.misses.is_empty(), "{:?}", out.misses);
    assert_eq!(out.candidates.len(), 2);
}

#[test]
fn amplify_firestore_pack_parses_paid_and_blank() {
    let body = fixture("amplify_distributions_pack.json");
    let parsed = parse_amplify_distribution_pack(&body);
    assert_eq!(parsed.len(), 3);
    assert!(parsed[0]["amountPerShareMinor"].is_null());
    assert_eq!(parsed[0]["paymentPeriod"], "2026-09-30");
    assert_eq!(parsed[1]["paymentPeriod"], "2026-08-31");
    assert_eq!(parsed[1]["amountPerShareMinor"], 38826);
    assert_eq!(parsed[1]["amountScale"], 5);
    let via_html = parse_amplify_distributions(&body);
    assert_eq!(via_html.len(), 3);
    let out = collect_from_fetched_page(
        &target_inception("PAY1", "amplify", "2026-07-01", "Monthly"),
        "amplify",
        Some(&body),
    );
    assert!(out.misses.is_empty(), "{:?}", out.misses);
    assert_eq!(out.candidates.len(), 2);
}

#[test]
fn ftvest_divid_history_table_parses_payable_date() {
    let html = fixture("ftvest_divid_history.html");
    let parsed = parse_generic_distributions("ftvest", &html);
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0]["paymentPeriod"], "2026-09-02");
    assert_eq!(parsed[0]["amountPerShareMinor"], 399100);
    assert_eq!(parsed[1]["paymentPeriod"], "2026-08-04");
    let out = collect_from_fetched_page(
        &target_inception("NEW1", "ftvest", "2026-08-01", "Monthly"),
        "ftvest",
        Some(&html),
    );
    assert!(out.misses.is_empty(), "{:?}", out.misses);
    assert_eq!(out.candidates.len(), 2);
}

#[test]
fn amplify_qdvo_paid_and_upcoming() {
    let html = fixture("amplify_qdvo.html");
    let parsed = parse_amplify_distributions(&html);
    let paid = parsed
        .iter()
        .filter(|c| !c["amountPerShareMinor"].is_null())
        .count();
    assert_eq!(paid, 2);
    assert!(parsed.iter().any(|c| c["amountPerShareMinor"].is_null()));
}

#[test]
fn neos_spyi_blank_future_is_unknown() {
    let html = fixture("neos_spyi.html");
    let parsed = parse_neos_distributions(&html);
    assert_eq!(parsed.len(), 2);
    assert!(parsed[0]["amountPerShareMinor"].is_null());
    assert_eq!(parsed[1]["amountPerShareMinor"], 5423);
}

#[test]
fn yieldmax_msty_dedupes_and_has_roc() {
    let html = fixture("yieldmax_msty.html");
    let parsed = parse_yieldmax_distributions(&html);
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0]["rocPctMinor"], 0);
    assert_eq!(parsed[1]["rocPctMinor"], 9874);
}

#[test]
fn roundhill_empty_calhisdistri_is_unknown() {
    let html = fixture("roundhill_empty_calhisdistri.html");
    assert!(parse_roundhill_distributions(&html).is_empty());
    let out = collect_from_fetched_page(&target("TOPW", "roundhill"), "roundhill", Some(&html));
    assert_eq!(out.misses[0]["code"], "declaration_retrieve_miss");
    assert!(out
        .misses[0]["reason"]
        .as_str()
        .unwrap_or("")
        .contains("Issuer page empty"));
    assert!(!out.misses[0]["reason"]
        .as_str()
        .unwrap_or("")
        .contains("Yahoo"));
}

#[test]
fn roundhill_ybtc_api_json_parses_paid_and_blank() {
    let body = fixture("roundhill_ybtc_distributions.json");
    let parsed = parse_roundhill_distribution_api(&body);
    assert_eq!(parsed.len(), 8);
    assert_eq!(parsed[0]["paymentPeriod"], "2026-12-31");
    assert!(parsed[0]["amountPerShareMinor"].is_null());
    let paid = parsed
        .iter()
        .filter(|c| !c["amountPerShareMinor"].is_null())
        .count();
    assert_eq!(paid, 6);
    assert_eq!(parsed[2]["paymentPeriod"], "2026-08-27");
    assert_eq!(parsed[2]["amountPerShareMinor"], 68023);
    assert_eq!(parsed[2]["amountScale"], 6);
    let out = collect_from_fetched_page(
        &target_inception("YBTC", "roundhill", "2026-07-21", "Weekly"),
        "roundhill",
        Some(&body),
    );
    assert!(out.misses.is_empty());
    assert_eq!(out.candidates.len(), 6);
    assert!(!out.pay_dates.is_empty());
}

#[test]
fn known_period_keeps_same_amount_but_posts_changed_amount() {
    let body = fixture("roundhill_ybtc_distributions.json");
    let mut same = target_inception("YBTC", "roundhill", "2026-07-21", "Weekly");
    same.paid_count = 12;
    same.known_payment_periods = vec!["2026-08-27".into()];
    same.known_declaration_amounts = vec![("2026-08-27".into(), 68023, 6)];
    let unchanged = collect_from_fetched_page(&same, "roundhill", Some(&body));
    assert!(
        unchanged
            .candidates
            .iter()
            .all(|c| c["paymentPeriod"] != "2026-08-27"),
        "same amount must not re-post: {:?}",
        unchanged.candidates
    );

    let mut changed = same;
    changed.known_declaration_amounts = vec![("2026-08-27".into(), 23303, 5)];
    let out = collect_from_fetched_page(&changed, "roundhill", Some(&body));
    let row = out
        .candidates
        .iter()
        .find(|c| c["paymentPeriod"] == "2026-08-27")
        .expect("changed amount must post");
    assert_eq!(row["amountPerShareMinor"], 68023);
    assert_eq!(row["amountScale"], 6);
}

#[test]
fn amplify_thirteen_paid_keeps_all() {
    let html = fixture("amplify_thirteen_paid.html");
    assert_eq!(parse_amplify_distributions(&html).len(), 13);
    let out = collect_from_fetched_page(&target("HAKY", "amplify"), "amplify", Some(&html));
    assert_eq!(out.candidates.len(), 13);
}

#[test]
fn cornerstone_press_list_is_loud_miss_not_yahoo() {
    let html = fixture("cornerstone_crf.html");
    let parsed = parse_generic_distributions("cornerstone", &html);
    assert!(parsed.is_empty() || parsed.iter().all(|c| c["amountPerShareMinor"].is_null()));
    let out = collect_from_fetched_page(&target("CRF", "cornerstone"), "cornerstone", Some(&html));
    assert_eq!(out.misses[0]["code"], "declaration_retrieve_miss");
    assert!(out
        .misses[0]["reason"]
        .as_str()
        .unwrap_or("")
        .contains("Issuer page empty"));
    assert!(!out.misses[0]["reason"]
        .as_str()
        .unwrap_or("")
        .contains("Yahoo"));
}

#[test]
fn globalx_distribution_history_uses_payable_not_ex() {
    let body = fixture("globalx_qyld_history.json");
    let parsed = parse_globalx_distribution_history(&body);
    assert!(parsed.iter().any(|c| c["paymentPeriod"] == "2014-02-06"
        && c["amountPerShareMinor"] == 2574
        && c["exDate"] == "2014-01-22"));
    assert!(parsed.iter().any(|c| c["paymentPeriod"] == "2026-08-27"
        && c["amountPerShareMinor"] == 1829
        && c["exDate"] == "2026-08-24"));
    assert!(parsed.iter().any(|c| c["paymentPeriod"] == "2025-12-02"
        && c["amountPerShareMinor"] == 1728));
    assert!(parsed.iter().any(|c| c["paymentPeriod"] == "2026-09-24"
        && c["amountPerShareMinor"].is_null()));
    assert!(parsed.iter().all(|c| c["paymentPeriod"] != "2026-08-24"));
    assert!(parsed.iter().all(|c| c["paymentPeriod"] != "2026-01-07"));
    let rsc = r#"self.__next_f.push([1,"29:[\"$\",\"$L3d\",null,{\"distributionHistoryData\":[{\"ETF_TICKER\":\"QYLD\",\"DISTRIBUTION_HISTORY\":[{\"amount\":0.1829,\"ex_date\":\"2026-08-24\",\"payable_date\":\"2026-08-27\",\"record_date\":\"2026-08-24\"}]}]}]"])"#;
    let from_rsc = parse_div1_distributions("globalx", rsc);
    assert_eq!(from_rsc.len(), 1, "{from_rsc:?}");
    assert_eq!(from_rsc[0]["paymentPeriod"], "2026-08-27");
    assert_eq!(from_rsc[0]["amountPerShareMinor"], 1829);
    let flight = r#"1:{"distributionHistoryData":[{"ETF_TICKER":"QYLD","DISTRIBUTION_HISTORY":[{"amount":0.1829,"ex_date":"2026-08-24","payable_date":"2026-08-27","record_date":"2026-08-24"}]}]}"#;
    let from_flight = parse_globalx_distribution_history(flight);
    assert_eq!(from_flight[0]["paymentPeriod"], "2026-08-27");
}

#[test]
fn globalx_table_parses_paid_and_blank() {
    let html = fixture("globalx_qyld.html");
    let parsed = parse_generic_distributions("globalx", &html);
    assert_eq!(parsed.len(), 2);
    assert!(parsed.iter().any(|c| c["amountPerShareMinor"].is_null()));
    assert!(parsed.iter().any(|c| c["amountPerShareMinor"] == 1700));
}

#[test]
fn fidelity_spaxx_parses_monthly_rates() {
    let html = fixture("fidelity_spaxx.html");
    let parsed = parse_moneymarket_distributions("fidelity", &html);
    assert!(parsed.len() >= 12);
    assert_eq!(parsed[0]["amountPerShareMinor"], 280);
    assert_eq!(parsed[0]["amountScale"], 5);
    let out = collect_from_fetched_page(&target("SPAXX", "fidelity"), "fidelity", Some(&html));
    assert_eq!(out.candidates.len(), 12);
    assert!(out.misses.is_empty());
}

#[test]
fn schwab_swvxx_parses_monthly_rates() {
    let html = fixture("schwab_swvxx.html");
    let parsed = parse_moneymarket_distributions("schwab", &html);
    assert_eq!(parsed.len(), 4);
    assert_eq!(parsed[0]["amountPerShareMinor"], 150);
    assert_eq!(parsed[0]["amountScale"], 5);
}

#[test]
fn fidelity_spaxx_header_json_sets_monthly_plan_from_seven_day_yield() {
    let body = fixture("fidelity_spaxx_header.json");
    let parsed = parse_moneymarket_distributions("fidelity", &body);
    assert_eq!(parsed.len(), 1, "{parsed:?}");
    assert_eq!(parsed[0]["kind"], "cash_rate");
    assert_eq!(parsed[0]["amountPerShareMinor"], 2783);
    assert_eq!(parsed[0]["amountScale"], 6);
    assert_eq!(parsed[0]["annualYieldBps"], 334);
    assert_eq!(parsed[0]["paymentPeriod"], "2026-09-07");
    let mut t = target("SPAXX", "fidelity");
    t.div_type = "CASH".into();
    t.payment_frequency = "Monthly".into();
    let out = collect_from_fetched_page(&t, "fidelity", Some(&body));
    assert!(out.misses.is_empty(), "{:?}", out.misses);
    assert_eq!(out.candidates.len(), 1);
    assert_eq!(out.candidates[0]["amountPerShareMinor"], 2783);
    assert!(!out.pay_dates.is_empty(), "remaining 2026 months should derive");
}

#[test]
fn fidelity_fdrxx_html_seven_day_yield_converts() {
    let html = fixture("fidelity_fdrxx_yield.html");
    let parsed = parse_moneymarket_distributions("fidelity", &html);
    assert_eq!(parsed.len(), 1, "{parsed:?}");
    assert_eq!(parsed[0]["amountPerShareMinor"], 2833);
    assert_eq!(parsed[0]["annualYieldBps"], 340);
}

#[test]
fn schwab_swvxx_table_uses_swvxx_seven_day_not_snaxx() {
    let html = fixture("schwab_swvxx_yield.html");
    let parsed = parse_moneymarket_distributions("schwab", &html);
    assert_eq!(parsed.len(), 1, "{parsed:?}");
    assert_eq!(parsed[0]["amountPerShareMinor"], 2942);
    assert_eq!(parsed[0]["annualYieldBps"], 353);
}

#[test]
fn nasdaq_jepq_json_parses_paid() {
    let body = fixture("nasdaq_jepq_dividends.json");
    let parsed = parse_nasdaq_dividends("jpmorgan", &body);
    assert!(parsed.len() >= 4);
    assert!(parsed.iter().any(|c| !c["amountPerShareMinor"].is_null()));
}

#[test]
fn jpmorgan_seed_cusip_and_historical_json_parse() {
    let seed = "https://am.jpmorgan.com/us/en/asset-management/adv/products/jpmorgan-nasdaq-equity-premium-income-etf-etf-shares-46654q203#/dividends";
    assert_eq!(
        jpmorgan_cusip_from_seed(seed, ""),
        Some("46654Q203".into())
    );
    assert_eq!(
        jpmorgan_cusip_from_seed("", r#"<input type="hidden" id="cspCode" value="46654Q203"/>"#),
        Some("46654Q203".into())
    );
    let body = fixture("jpmorgan_jepq_historical.json");
    let parsed = parse_jpmorgan_distributions(&body);
    assert_eq!(parsed.len(), 12, "{parsed:?}");
    assert_eq!(parsed[0]["paymentPeriod"], "2026-09-03");
    assert_eq!(parsed[0]["amountPerShareMinor"], 68255);
    assert_eq!(parsed[0]["amountScale"], 5);
    assert!(parsed.iter().any(|c| c["paymentPeriod"] == "2025-10-03"
        && c["amountPerShareMinor"] == 44612));
    let via_div1 = parse_div1_distributions("jpmorgan", &body);
    assert_eq!(via_div1.len(), 12);
}

#[test]
fn trinity_ir_table_uses_pay_date_not_ex_date() {
    let html = fixture("trinity_distributions.html");
    let parsed = parse_div1_distributions("trinity", &html);
    assert!(parsed.len() >= 11);
    assert!(parsed.iter().any(|c| c["paymentPeriod"] == "2026-09-30"
        && c["amountPerShareMinor"] == 17));
    assert!(parsed.iter().any(|c| c["paymentPeriod"] == "2026-01-30"
        && c["amountPerShareMinor"] == 17));
    assert!(parsed.iter().any(|c| c["paymentPeriod"] == "2026-01-15"
        && c["amountPerShareMinor"] == 51));
    assert!(parsed.iter().all(|c| c["paymentPeriod"] != "2026-09-10"));
    assert!(parsed.iter().all(|c| c["paymentPeriod"] != "2026-12-31"));
    let out = collect_from_fetched_page(
        &target_inception("TRIN", "trinity", "2026-01-01", "Monthly"),
        "trinity",
        Some(&html),
    );
    assert!(out.candidates.len() >= 9);
    assert!(out.misses.is_empty());
}

#[test]
fn collector_pairs_amount_to_pay_date_column_not_last_date() {
    let html = r#"<table>
<tr><th>Amount</th><th>Pay Date</th><th>Ex Date</th><th>Declaration Date</th></tr>
<tr><td>$0.572959</td><td>9/1/2026</td><td>8/31/2026</td><td>8/28/2026</td></tr>
</table>"#;
    let parsed = parse_generic_distributions("roundhill", html);
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0]["paymentPeriod"], "2026-09-01");
    assert_eq!(parsed[0]["amountPerShareMinor"], 572959);
    assert_eq!(parsed[0]["amountScale"], 6);
}

#[test]
fn collector_refuses_table_with_only_declaration_and_ex_dates() {
    let html = r#"<table>
<tr><th>Declaration Date</th><th>Ex Date</th><th>Amount</th></tr>
<tr><td>8/28/2026</td><td>8/31/2026</td><td>$0.57</td></tr>
</table>"#;
    assert!(parse_generic_distributions("roundhill", html).is_empty());
}

#[test]
fn ellington_efc_dividends_table_parses() {
    let html = fixture("ellington_efc.html");
    let parsed = parse_generic_distributions("ellington", &html);
    assert!(parsed.len() >= 2);
    assert_eq!(parsed[0]["amountPerShareMinor"], 13);
    let out = collect_from_fetched_page(
        &target_inception("EFC", "ellington", "2026-07-01", "Monthly"),
        "ellington",
        Some(&html),
    );
    assert!(out.misses.is_empty());
    assert!(!out.candidates.is_empty());
}

#[test]
fn mplx_q4_dividendtable_uses_pay_date_not_year_total() {
    // Live Q4 HTML omits </tr> on the header row and inserts an extra <tr>.
    let html = r#"<table class="dividendtable table table-hover">
<tr class="ccbnBgTblTtl"><td>Type</td><td>Record Date</td><td>Pay Date</td><td>Distribution per Unit as Paid</td><td>Type</td>
<tr>
<tr class="ccbnBgTblTxt"><td>Distribution</td><td>08/07/26</td><td>08/14/26</td><td>$1.0765</td><td>Cash, quarterly</td></tr>
<tr class="ccbnBgTblTxt"><td>Distribution</td><td>05/08/26</td><td>05/15/26</td><td>$1.0765</td><td>Cash, quarterly</td></tr>
<tr><td></td><td></td><td></td><td>$2.153</td><td></td></tr>
</table>"#;
    let parsed = parse_generic_distributions("mplx", html);
    assert_eq!(parsed.len(), 2, "{parsed:?}");
    assert_eq!(parsed[0]["paymentPeriod"], "2026-08-14");
    assert_eq!(parsed[0]["recordDate"], "2026-08-07");
    assert_eq!(parsed[0]["amountPerShareMinor"], 10765);
    assert_eq!(parsed[0]["amountScale"], 4);
    assert_eq!(parsed[1]["paymentPeriod"], "2026-05-15");
    assert!(parsed.iter().all(|c| c["paymentPeriod"] != "2026-08-07"));
    assert!(parsed.iter().all(|c| c["amountPerShareMinor"] != 2153));
}

#[test]
fn mlp_sec_8k_parses_common_unit_not_preferred_and_derives_remaining() {
    let html = fixture("energytransfer_et_8k.html");
    let parsed = parse_div1_distributions("mlp_sec_8k", &html);
    assert_eq!(parsed.len(), 1, "{parsed:?}");
    assert_eq!(parsed[0]["paymentPeriod"], "2026-08-19");
    assert_eq!(parsed[0]["recordDate"], "2026-08-07");
    assert_eq!(parsed[0]["amountPerShareMinor"], 3400);
    assert_eq!(parsed[0]["amountScale"], 4);
    assert_eq!(parsed[0]["source"], "sec_8k");
    assert!(parsed.iter().all(|c| c["amountPerShareMinor"] != 2111));
    let out = collect_from_fetched_page(
        &DeclarationTarget {
            paid_count: 1,
            payment_frequency: "Quarterly".into(),
            ..target("MLP1", "mlp_sec_8k")
        },
        "mlp_sec_8k",
        Some(&html),
    );
    assert!(out.misses.is_empty(), "{:?}", out.misses);
    let paid: Vec<_> = out
        .candidates
        .iter()
        .filter(|c| c["amountPerShareMinor"].as_i64() == Some(3400))
        .collect();
    assert_eq!(paid.len(), 1);
    assert_eq!(paid[0]["source"], "sec_8k");
    assert_eq!(out.pay_dates.len(), 1, "{:?}", out.pay_dates);
    assert_eq!(out.pay_dates[0]["payOn"], "2026-11-19");
    assert_eq!(out.pay_dates[0]["source"], "derived_template");
    assert!(out.pay_dates[0]["amountPerShareMinor"].is_null());
}

#[test]
fn mlp_sec_8k_preferred_only_writes_no_common_row() {
    let html = fixture("mlp1_8k_preferred_only.html");
    let parsed = parse_div1_distributions("mlp_sec_8k", &html);
    assert!(parsed.is_empty(), "{parsed:?}");
    let out = collect_from_fetched_page(
        &target("MLP1", "mlp_sec_8k"),
        "mlp_sec_8k",
        Some(&html),
    );
    assert!(out.candidates.is_empty(), "{:?}", out.candidates);
    assert!(out
        .candidates
        .iter()
        .all(|c| c["amountPerShareMinor"] != 2111));
    assert!(
        out.misses.is_empty(),
        "preferred-only before Nov 7 is waiting, not a miss: {:?}",
        out.misses
    );
}

#[test]
fn dividendhistory_crf_table_parses() {
    // Table-shape coverage only. Fetch never loads dividendhistory.org.
    let html = fixture("dividendhistory_crf.html");
    let parsed = parse_generic_distributions("cornerstone", &html);
    assert!(parsed.len() >= 3);
    assert!(parsed.iter().any(|c| !c["amountPerShareMinor"].is_null()));
}

#[test]
fn simplify_svol_table_parses_paid_and_na() {
    let html = fixture("simplify_svol.html");
    let parsed = parse_generic_distributions("simplify", &html);
    assert!(parsed.len() >= 3);
    assert!(parsed.iter().any(|c| c["amountPerShareMinor"].is_null()));
    assert!(parsed.iter().any(|c| c["amountPerShareMinor"] == 28000));
    let out = collect_from_fetched_page(
        &target_inception("SVOL", "simplify", "2026-06-01", "Monthly"),
        "simplify",
        Some(&html),
    );
    assert!(out.misses.is_empty());
    assert!(!out.candidates.is_empty());
}

#[test]
fn simplify_svol_distributions_ajax_parses_full_history() {
    use import_engine::parse_simplify_distributions;
    let body = fixture("simplify_svol_distributions.json");
    let parsed = parse_simplify_distributions("simplify", &body);
    assert!(
        parsed.len() >= 12,
        "expected 12+ paid months from distributions endpoint, got {}",
        parsed.len()
    );
    let out = collect_from_fetched_page(
        &target_inception("SVOL", "simplify", "2021-09-01", "Monthly"),
        "simplify",
        Some(&body),
    );
    assert!(out.misses.is_empty(), "{:?}", out.misses);
    assert!(out.candidates.len() >= 12);
}

#[test]
fn proshares_bito_api_json_parses_paid() {
    let body = fixture("proshares_bito_distributions.json");
    let parsed = parse_proshares_distribution_summary("proshares", &body);
    assert_eq!(parsed.len(), 7);
    assert_eq!(parsed[0]["paymentPeriod"], "2026-08-07");
    assert_eq!(parsed[0]["amountPerShareMinor"], 13749);
    assert_eq!(parsed[0]["amountScale"], 6);
    let out = collect_from_fetched_page(
        &target_inception("BITO", "proshares", "2026-02-01", "Monthly"),
        "proshares",
        Some(&body),
    );
    assert!(out.misses.is_empty());
    assert_eq!(out.candidates.len(), 7);
}

#[test]
fn saba_cefs_nuxt_distributions_parses() {
    let html = fixture("saba_cefs.html");
    let parsed = parse_saba_distributions("saba", &html);
    assert!(parsed.len() >= 12, "CEFS should have at least 12 rows, got {}", parsed.len());
    assert_eq!(parsed[0]["paymentPeriod"].as_str(), Some("2026-08-31"));
    assert_eq!(parsed[0]["amountPerShareMinor"], 14);
    let out = collect_from_fetched_page(&target("CEFS", "saba"), "saba", Some(&html));
    assert!(out.candidates.len() >= 12);
}

#[test]
fn roundhill_nvdw_dividendhistory_table_shape_parses() {
    // Parser can read a pay-date table. That does not authorize fetch of this host.
    let html = fixture("dividendhistory_nvdw.html");
    let parsed = parse_generic_distributions("roundhill", &html);
    let paid = parsed
        .iter()
        .filter(|c| !c["amountPerShareMinor"].is_null())
        .count();
    assert!(paid >= 12, "NVDW fixture should have at least 12 paid rows");
}

#[test]
fn production_div1_symbols_all_map_to_registered_adapters() {
    let production = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../database/seed/production");
    let doc = parse_production_templates(&production).expect("production templates");
    let div1: Vec<_> = doc
        .characteristics
        .iter()
        .filter(|c| is_div1(&c.div_type))
        .collect();
    assert_eq!(div1.len(), 39, "DIV-1 count drifted from Template_Positions");
    assert!(
        !div1.iter().any(|c| c.symbol.eq_ignore_ascii_case("CES")),
        "CES is not in this book"
    );
    let crf = div1.iter().find(|c| c.symbol.eq_ignore_ascii_case("CRF")).expect("CRF");
    let clm = div1.iter().find(|c| c.symbol.eq_ignore_ascii_case("CLM")).expect("CLM");
    assert_eq!(crf.provider, "Cornerstone");
    assert_eq!(clm.provider, "Cornerstone");
    for row in &div1 {
        let src = declaration_source_for_provider(&row.provider)
            .unwrap_or_else(|| panic!("{} provider {} has no adapter", row.symbol, row.provider));
        assert!(
            is_registered_declaration_source(src),
            "{} mapped to unregistered {src}",
            row.symbol
        );
    }
}

