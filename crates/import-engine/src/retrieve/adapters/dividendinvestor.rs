//! DividendInvestor.com dividend-history tool (POST form, specific dividend dates view).

use serde_json::Value;

use crate::retrieve::html::{
    distribution_candidate, html_td_rows, parse_issuer_amount, parse_issuer_date, sort_newest_first,
};

pub const HISTORY_URL: &str = "https://www.dividendinvestor.com/dividend-history/";

pub fn dividendinvestor_fund_page(html: &str, symbol: &str) -> bool {
    let sym = symbol.trim().to_ascii_uppercase();
    html.to_ascii_uppercase().contains(&sym)
        && html.contains("specific_yt_table")
}

/// Parse nested `specific_yt_table` rows: Year | Dec | Ex | Rec | Pay | Dividend($).
pub fn parse_dividendinvestor_distributions(source: &str, html: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let lower = html.to_ascii_lowercase();
    let marker = "specific_yt_table";
    let mut search = 0usize;
    while let Some(rel) = lower.get(search..).and_then(|s| s.find(marker)) {
        let marker_at = search + rel;
        let table_start = lower[..marker_at].rfind("<table").unwrap_or(marker_at);
        let table_html = html.get(table_start..).unwrap_or("");
        let end = table_html
            .to_ascii_lowercase()
            .find("</table>")
            .unwrap_or(table_html.len().min(8000));
        let table = table_html.get(..end).unwrap_or("");
        for cells in html_td_rows(table) {
            if cells.len() < 6 {
                continue;
            }
            if cells[0].contains("Total") {
                continue;
            }
            let Some(pay) = parse_issuer_date(&cells[4]) else {
                continue;
            };
            let amount = parse_issuer_amount(&cells[5]);
            out.push(distribution_candidate(source, pay, amount, None));
        }
        search = marker_at + marker.len();
    }
    sort_newest_first(&mut out);
    out
}
