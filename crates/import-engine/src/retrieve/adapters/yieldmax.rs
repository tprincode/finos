//! YieldMax amount + ROC% table with duplicate-row drop.

use std::collections::HashSet;

use serde_json::Value;

use crate::retrieve::html::{
    distribution_candidate, html_td_rows, parse_issuer_amount, parse_issuer_date, sort_newest_first,
};
use super::{html_names_symbol, page_is_not_found};
use crate::retrieve::percent_to_minor;

fn parse_roc_cell(raw: &str) -> Option<i64> {
    let s = raw.trim().to_ascii_lowercase();
    if s.is_empty() || s.contains("nan") || s == "—" || s == "-" {
        return None;
    }
    percent_to_minor(s.trim_end_matches('%').trim())
}

/// YieldMax: amount, declared, ex, record, payable, ROC%. Duplicate rows are dropped.
pub fn parse_yieldmax_distributions(html: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for cells in html_td_rows(html) {
        if cells.len() < 5 {
            continue;
        }
        let pay_idx = if cells.len() >= 6 { 4 } else { 3 };
        let roc_idx = if cells.len() >= 6 { Some(5) } else { None };
        let Some(pay) = parse_issuer_date(&cells[pay_idx]) else {
            continue;
        };
        let amount = parse_issuer_amount(&cells[0]);
        let roc = roc_idx.and_then(|i| parse_roc_cell(&cells[i]));
        let key = format!(
            "{pay}|{}",
            amount.map(|(a, _)| a).unwrap_or(0)
        );
        if !seen.insert(key) {
            continue;
        }
        out.push(distribution_candidate("yieldmax", pay, amount, roc));
    }
    sort_newest_first(&mut out);
    out
}
pub(crate) fn yieldmax_fund_page(html: &str, symbol: &str) -> bool {
    !page_is_not_found(html)
        && html.to_ascii_lowercase().contains("yieldmax")
        && html_names_symbol(html, symbol)
}
