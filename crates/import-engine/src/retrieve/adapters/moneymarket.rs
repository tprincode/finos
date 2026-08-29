//! Fidelity / Schwab money-market distribution pages.
//! NAV stays par $1 elsewhere; this adapter only parses rates and pay dates.

use serde_json::Value;

use super::generic::{generic_fund_page, parse_generic_distributions};

pub fn parse_moneymarket_distributions(source: &str, html: &str) -> Vec<Value> {
    parse_generic_distributions(source, html)
}

pub fn moneymarket_fund_page(source: &str, html: &str, symbol: &str) -> bool {
    let needles: &[&str] = match source.trim().to_ascii_lowercase().as_str() {
        "fidelity" => &["fidelity", "government money market", "cash reserves"],
        "schwab" => &["schwab", "value advantage", "money market"],
        _ => return false,
    };
    generic_fund_page(html, symbol, needles)
}

pub fn moneymarket_probe_urls(source: &str, symbol: &str) -> Vec<String> {
    let sym = symbol.trim().to_ascii_uppercase();
    let lower = sym.to_ascii_lowercase();
    match source.trim().to_ascii_lowercase().as_str() {
        "fidelity" => vec![
            format!("https://fundresearch.fidelity.com/mutual-funds/summary/{sym}"),
            format!("https://fundresearch.fidelity.com/mutual-funds/fees-and-distributions/{sym}"),
            format!("https://www.fidelity.com/mutual-funds/summary/{lower}"),
            format!("https://dividendhistory.org/payout/{sym}/"),
        ],
        "schwab" => vec![
            format!("https://www.schwabassetmanagement.com/products/{lower}"),
            format!("https://www.schwab.com/asset-management/products/{lower}"),
            format!("https://dividendhistory.org/payout/{sym}/"),
        ],
        _ => Vec::new(),
    }
}
