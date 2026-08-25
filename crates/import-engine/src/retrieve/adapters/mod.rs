//! Per-issuer distribution adapters. Yahoo is never a declaration source.

pub mod amplify;
pub mod neos;
pub mod roundhill;
pub mod yieldmax;

pub use amplify::{amplify_fund_page, parse_amplify_distributions};
pub use neos::parse_neos_distributions;
pub(crate) use neos::neos_fund_page;
pub use roundhill::{parse_roundhill_distributions, roundhill_fund_page};
pub(crate) use roundhill::{
    hrefs_matching, parse_roundhill_csv, parse_roundhill_roc_html,
    parse_vendor_distributions_with_csv,
};
pub use yieldmax::parse_yieldmax_distributions;
pub(crate) use yieldmax::yieldmax_fund_page;

pub(crate) fn page_is_not_found(html: &str) -> bool {
    let l = html.to_ascii_lowercase();
    l.contains("page not found") || l.contains(">404<")
}

pub(crate) fn html_names_symbol(html: &str, symbol: &str) -> bool {
    html.to_ascii_uppercase()
        .contains(&symbol.trim().to_ascii_uppercase())
}

pub(crate) fn parse_vendor_distributions(source: &str, html: &str) -> Vec<serde_json::Value> {
    match source.trim().to_ascii_lowercase().as_str() {
        "amplify" => parse_amplify_distributions(html),
        "neos" => parse_neos_distributions(html),
        "yieldmax" => parse_yieldmax_distributions(html),
        _ => parse_roundhill_distributions(html),
    }
}
