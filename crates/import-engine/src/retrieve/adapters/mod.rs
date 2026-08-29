//! Per-issuer distribution adapters. Yahoo is never a declaration source.

pub mod amplify;
pub mod div1;
pub mod dividendinvestor;
pub mod generic;
pub mod moneymarket;
pub mod nasdaq;
pub mod neos;
pub mod roundhill;
pub mod saba;
pub mod simplify;
pub mod yieldmax;

pub use amplify::{amplify_fund_page, parse_amplify_distributions};
pub use neos::parse_neos_distributions;
pub(crate) use neos::neos_fund_page;
pub use roundhill::{
    parse_roundhill_distribution_api, parse_roundhill_distributions, roundhill_api_ticker,
    roundhill_fund_page,
};
pub(crate) use roundhill::{
    hrefs_matching, parse_roundhill_csv, parse_roundhill_roc_html,
    parse_vendor_distributions_with_csv,
};
pub(crate) use saba::saba_fund_page;
pub use saba::{parse_saba_distributions, parse_saba_nuxt_distributions, saba_fund_url};
pub use simplify::{
    extract_simplify_node_id, parse_simplify_distributions, simplify_distributions_url,
    simplify_fund_url,
};
pub use yieldmax::parse_yieldmax_distributions;
pub(crate) use yieldmax::yieldmax_fund_page;
pub use div1::{
    div1_fund_page, div1_probe_urls, parse_div1_distributions, parse_proshares_distribution_summary,
};
pub(crate) use dividendinvestor::dividendinvestor_fund_page;
pub use dividendinvestor::{
    parse_dividendinvestor_distributions, HISTORY_URL as DIVIDENDINVESTOR_HISTORY_URL,
};
pub use generic::parse_generic_distributions;
pub use moneymarket::parse_moneymarket_distributions;
pub(crate) use moneymarket::{moneymarket_fund_page, moneymarket_probe_urls};
pub use nasdaq::{dividendhistory_url, nasdaq_dividends_url, parse_nasdaq_dividends};

pub(crate) fn page_is_not_found(html: &str) -> bool {
    let l = html.to_ascii_lowercase();
    l.contains("page not found") || l.contains(">404<")
}

pub(crate) fn html_names_symbol(html: &str, symbol: &str) -> bool {
    html.to_ascii_uppercase()
        .contains(&symbol.trim().to_ascii_uppercase())
}

pub(crate) fn parse_vendor_distributions(source: &str, html: &str) -> Vec<serde_json::Value> {
    let nasdaq = parse_nasdaq_dividends(source, html);
    if !nasdaq.is_empty() {
        return nasdaq;
    }
    match source.trim().to_ascii_lowercase().as_str() {
        "amplify" => parse_amplify_distributions(html),
        "neos" => parse_neos_distributions(html),
        "yieldmax" => parse_yieldmax_distributions(html),
        "roundhill" => parse_roundhill_distributions(html),
        "fidelity" | "schwab" => parse_moneymarket_distributions(source, html),
        "saba" => parse_saba_distributions(source, html),
        "simplify" => parse_simplify_distributions(source, html),
        "dividendinvestor" => parse_dividendinvestor_distributions(source, html),
        other => parse_div1_distributions(other, html),
    }
}
