//! NEOS distribution table. Pay date and amount are located by header name.

use serde_json::Value;

use crate::retrieve::html::parse_distribution_tables;
use super::{html_names_symbol, page_is_not_found};

pub fn parse_neos_distributions(html: &str) -> Vec<Value> {
    parse_distribution_tables("neos", html)
}
pub(crate) fn neos_fund_page(html: &str, symbol: &str) -> bool {
    !page_is_not_found(html)
        && html.to_ascii_lowercase().contains("neos")
        && html_names_symbol(html, symbol)
}
