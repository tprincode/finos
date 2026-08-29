//! Fixture contract tests — no live HTTP. Table-shaped issuer pages only.

use import_engine::{
    collect_from_fetched_page, parse_amplify_distributions, parse_generic_distributions,
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
fn nasdaq_jepq_json_parses_paid() {
    let body = fixture("nasdaq_jepq_dividends.json");
    let parsed = parse_nasdaq_dividends("jpmorgan", &body);
    assert!(parsed.len() >= 4);
    assert!(parsed.iter().any(|c| !c["amountPerShareMinor"].is_null()));
    let out = collect_from_fetched_page(
        &target_inception("JEPQ", "jpmorgan", "2026-05-01", "Monthly"),
        "jpmorgan",
        Some(&body),
    );
    assert!(out.misses.is_empty());
    assert!(!out.candidates.is_empty());
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
fn dividendhistory_crf_table_parses() {
    let html = fixture("dividendhistory_crf.html");
    let parsed = parse_generic_distributions("cornerstone", &html);
    assert!(parsed.len() >= 3);
    assert!(parsed.iter().any(|c| !c["amountPerShareMinor"].is_null()));
    let out = collect_from_fetched_page(&target("CRF", "cornerstone"), "cornerstone", Some(&html));
    assert!(out.misses.is_empty());
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
fn roundhill_nvdw_dividendhistory_parses() {
    let html = fixture("dividendhistory_nvdw.html");
    let parsed = parse_generic_distributions("roundhill", &html);
    let paid = parsed
        .iter()
        .filter(|c| !c["amountPerShareMinor"].is_null())
        .count();
    assert!(paid >= 12, "NVDW fixture should have at least 12 paid rows");
    let out = collect_from_fetched_page(&target("NVDW", "roundhill"), "roundhill", Some(&html));
    assert!(out.misses.is_empty());
    assert!(out.candidates.len() >= 12);
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

