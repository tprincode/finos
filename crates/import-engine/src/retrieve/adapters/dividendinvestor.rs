//! DividendInvestor.com dividend-history tool (POST form, specific dividend dates view).

use serde_json::Value;

use crate::retrieve::html::parse_distribution_tables;

#[allow(dead_code)]
pub const HISTORY_URL: &str = "https://www.dividendinvestor.com/dividend-history/";

pub fn dividendinvestor_fund_page(html: &str, symbol: &str) -> bool {
    let sym = symbol.trim().to_ascii_uppercase();
    html.to_ascii_uppercase().contains(&sym)
        && html.contains("specific_yt_table")
}

/// Nested `specific_yt_table`: Year | Dec | Ex | Rec | Pay | Dividend($) — columns by header name.
/// Compiled for fixture tests; live retrieve must not call this (D4/D5).
#[allow(dead_code)]
pub fn parse_dividendinvestor_distributions(source: &str, html: &str) -> Vec<Value> {
    parse_distribution_tables(source, html)
}
