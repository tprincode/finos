//! YieldMax amount + ROC% table. Pay date and amount are located by header name.

use std::collections::HashSet;

use serde_json::Value;

use crate::retrieve::html::{candidate_amount, parse_distribution_tables, sort_newest_first};
use super::{html_names_symbol, page_is_not_found};

/// YieldMax: amount, declared, ex, record, payable, ROC%. Duplicate rows are dropped.
pub fn parse_yieldmax_distributions(html: &str) -> Vec<Value> {
    let mut out = parse_distribution_tables("yieldmax", html);
    let mut seen = HashSet::new();
    out.retain(|row| {
        let pay = row
            .get("paymentPeriod")
            .and_then(|p| p.as_str())
            .unwrap_or("");
        let amt = candidate_amount(row).unwrap_or(0);
        seen.insert(format!("{pay}|{amt}"))
    });
    sort_newest_first(&mut out);
    out
}
pub(crate) fn yieldmax_fund_page(html: &str, symbol: &str) -> bool {
    !page_is_not_found(html)
        && html.to_ascii_lowercase().contains("yieldmax")
        && html_names_symbol(html, symbol)
}
