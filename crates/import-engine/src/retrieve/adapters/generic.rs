//! Generic HTML table: any row with a pay date and optional amount.
//! Blank amount stays unknown — never $0.

use serde_json::Value;

use crate::retrieve::html::{
    distribution_candidate, html_td_rows, parse_issuer_amount, parse_issuer_date, sort_newest_first,
    strip_html,
};
use super::{html_names_symbol, page_is_not_found};

pub fn parse_generic_distributions(source: &str, html: &str) -> Vec<Value> {
    let mut out = Vec::new();
    for cells in html_td_rows(html) {
        let pay = cells.iter().find_map(|c| parse_issuer_date(c));
        let Some(pay) = pay else {
            continue;
        };
        let amount = cells.iter().find_map(|c| parse_issuer_amount(c));
        out.push(distribution_candidate(source, pay, amount, None));
    }
    if out.is_empty() {
        let text = strip_html(html);
        let tokens: Vec<&str> = text.split_whitespace().collect();
        let mut i = 0;
        while i < tokens.len() {
            if let Some(pay) = parse_issuer_date(tokens[i]) {
                let amount = tokens.get(i + 1).and_then(|t| parse_issuer_amount(t));
                out.push(distribution_candidate(source, pay, amount, None));
            }
            i += 1;
        }
    }
    sort_newest_first(&mut out);
    out
}

pub fn generic_fund_page(html: &str, symbol: &str, needles: &[&str]) -> bool {
    if page_is_not_found(html) || !html_names_symbol(html, symbol) {
        return false;
    }
    let lower = html.to_ascii_lowercase();
    needles.iter().any(|n| lower.contains(&n.to_ascii_lowercase()))
}
