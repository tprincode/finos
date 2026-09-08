//! Generic HTML table: pay-date column by header name, amount from the declaration row.
//! Blank amount stays unknown — never $0. No column-position guess.

use serde_json::Value;

use crate::retrieve::html::parse_distribution_tables;
use super::{html_names_symbol, page_is_not_found};

pub fn parse_generic_distributions(source: &str, html: &str) -> Vec<Value> {
    parse_distribution_tables(source, html)
}

pub fn generic_fund_page(html: &str, symbol: &str, needles: &[&str]) -> bool {
    if page_is_not_found(html) {
        return false;
    }
    let lower = html.to_ascii_lowercase();
    // Nasdaq IR pages name the company, not always the ticker (TRIN / Trinity).
    needles
        .iter()
        .any(|n| lower.contains(&n.to_ascii_lowercase()))
        || html_names_symbol(html, symbol)
}
