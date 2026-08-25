//! Fixture contract tests — no live HTTP. Table-shaped issuer pages only.

use import_engine::{
    collect_from_fetched_page, parse_amplify_distributions, parse_neos_distributions,
    parse_roundhill_distributions, parse_yieldmax_distributions, DeclarationTarget,
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
        .contains("not using Yahoo"));
}

#[test]
fn amplify_thirteen_paid_truncates_to_twelve() {
    let html = fixture("amplify_thirteen_paid.html");
    assert_eq!(parse_amplify_distributions(&html).len(), 13);
    let out = collect_from_fetched_page(&target("HAKY", "amplify"), "amplify", Some(&html));
    assert_eq!(out.candidates.len(), 12);
}
