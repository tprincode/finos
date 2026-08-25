//! NEOS 5-column distribution table.

use serde_json::Value;

use crate::retrieve::html::{
    distribution_candidate, html_td_rows, parse_issuer_amount, parse_issuer_date, sort_newest_first,
};
use super::{html_names_symbol, page_is_not_found};

pub fn parse_neos_distributions(html: &str) -> Vec<Value> {
    let mut out = Vec::new();
    for cells in html_td_rows(html) {
        if cells.len() < 5 {
            continue;
        }
        let Some(pay) = parse_issuer_date(&cells[3]).or_else(|| parse_issuer_date(&cells[1]))
        else {
            continue;
        };
        let amount = parse_issuer_amount(&cells[4]);
        out.push(distribution_candidate("neos", pay, amount, None));
    }
    sort_newest_first(&mut out);
    out
}
pub(crate) fn neos_fund_page(html: &str, symbol: &str) -> bool {
    !page_is_not_found(html)
        && html.to_ascii_lowercase().contains("neos")
        && html_names_symbol(html, symbol)
}
