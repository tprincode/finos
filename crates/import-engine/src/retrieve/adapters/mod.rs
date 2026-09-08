//! Per-issuer distribution adapters. Yahoo is never a declaration source.

pub mod amplify;
pub mod div1;
pub mod energytransfer;
pub mod dividendinvestor;
pub mod generic;
pub mod moneymarket;
pub mod nasdaq;
pub mod neos;
pub mod roundhill;
pub mod saba;
pub mod simplify;
pub mod yieldmax;

pub use amplify::{
    amplify_distributions_pack_url, amplify_fund_page, amplify_fund_page_url,
    parse_amplify_distribution_pack, parse_amplify_distributions,
};
pub use neos::parse_neos_distributions;
pub(crate) use neos::neos_fund_page;
pub use roundhill::{
    parse_roundhill_distribution_api, parse_roundhill_distributions, roundhill_api_ticker,
    roundhill_fund_page,
};
pub(crate) use roundhill::{
    hrefs_matching, https_urls_matching, parse_roundhill_csv, parse_roundhill_roc_html,
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
    cornerstone_candidates_table, div1_fund_page, div1_probe_urls, ftvest_history_form,
    ftvest_history_url, ftvest_history_years, globalx_19a_notice_urls, globalx_filings_hub_url,
    globalx_fund_url, globalx_tax_supplements_url, jpmorgan_cusip_from_seed,
    jpmorgan_historical_data_url, parse_cornerstone_press, parse_div1_distributions,
    parse_gladstone_press, parse_globalx_distribution_history,
    parse_jpmorgan_distributions, parse_proshares_distribution_summary, parse_return_of_capital_pct,
    parse_tappalpha_distributions, rexshares_calendar_covers_inception, tappalpha_distributions_url,
    tappalpha_fund_page_url,
};
pub(crate) use dividendinvestor::dividendinvestor_fund_page;
#[allow(unused_imports)]
pub use dividendinvestor::parse_dividendinvestor_distributions;
pub use generic::parse_generic_distributions;
pub use moneymarket::parse_moneymarket_distributions;
pub(crate) use moneymarket::{
    fidelity_mm_cusip, fidelity_mm_header_url, is_cash_rate_candidate, moneymarket_fund_page,
    moneymarket_probe_urls, parse_moneymarket_distributions_for, standing_mm_url,
};
pub use nasdaq::parse_nasdaq_dividends;

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
        "roundhill" => parse_roundhill_distributions(html),
        "fidelity" | "schwab" => parse_moneymarket_distributions(source, html),
        "saba" => parse_saba_distributions(source, html),
        "simplify" => parse_simplify_distributions(source, html),
        "dividendinvestor" => Vec::new(),
        "energytransfer" | "mlp_sec_8k" => energytransfer::parse_energytransfer_distributions(html),
        other => parse_div1_distributions(other, html),
    }
}
