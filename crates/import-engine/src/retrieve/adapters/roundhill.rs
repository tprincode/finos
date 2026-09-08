//! Roundhill: live distributions via PHP API (HTML calHisDistri tbody is empty on GET).
//! Same-host HTML table or Roundhill CSV only. Empty stays unknown — not Yahoo, not a calendar.

use serde_json::Value;

use crate::retrieve::html::{
    candidate_from_headers, json_amount, json_pay_date, parse_distribution_csv,
    parse_distribution_tables, sort_newest_first,
};
use crate::retrieve::{first_percent, http_get};
use super::{html_names_symbol, page_is_not_found};

/// Site page-id aliases → fund ticker used by distribution-call.php.
pub fn roundhill_api_ticker(symbol: &str) -> String {
    match symbol.trim().to_ascii_uppercase().as_str() {
        "BIGT" => "MAGS".into(),
        "TSW" => "TSLW".into(),
        "NVW" => "NVDW".into(),
        "WPAY" => "TOPW".into(),
        "DRAG" => "MAGC".into(),
        other => other.to_string(),
    }
}

/// Vendor documents these column names on `distribution-call.php` array rows.
const ROUNDHILL_API_HEADERS: &[&str] = &[
    "Declaration",
    "Ex Date",
    "Record Date",
    "Pay Date",
    "Amount",
];

fn json_cell_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

/// Parse JSON from `distribution-call.php`. Array rows are mapped through the
/// documented headers so Pay Date is found by name, not by position or Ex Date.
pub fn parse_roundhill_distribution_api(body: &str) -> Vec<Value> {
    let trimmed = body.trim();
    if !trimmed.starts_with('[') {
        return Vec::new();
    }
    let Ok(Value::Array(rows)) = serde_json::from_str::<Value>(trimmed) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for row in rows {
        match row {
            Value::Array(cols) => {
                let cells: Vec<String> = cols.iter().map(json_cell_text).collect();
                if cells
                    .first()
                    .map(|c| c.eq_ignore_ascii_case("declaration"))
                    .unwrap_or(false)
                {
                    continue;
                }
                if let Some(cand) =
                    candidate_from_headers("roundhill", ROUNDHILL_API_HEADERS, &cells)
                {
                    out.push(cand);
                }
            }
            Value::Object(_) => {
                let Some(pay) = json_pay_date(&row) else {
                    continue;
                };
                let amount = json_amount(&row);
                out.push(crate::retrieve::html::distribution_candidate(
                    "roundhill", pay, amount, None,
                ));
            }
            _ => {}
        }
    }
    sort_newest_first(&mut out);
    out
}

pub fn parse_roundhill_distributions(html: &str) -> Vec<Value> {
    let api = parse_roundhill_distribution_api(html);
    if !api.is_empty() {
        return api;
    }
    parse_distribution_tables("roundhill", html)
}

pub(crate) fn parse_roundhill_csv(text: &str) -> Vec<Value> {
    parse_distribution_csv("roundhill", text)
}

fn csv_hrefs(html: &str) -> Vec<String> {
    hrefs_matching(html, "csv", "")
}

pub(crate) fn hrefs_matching(html: &str, needle: &str, base: &str) -> Vec<String> {
    let mut urls = Vec::new();
    let lower = html.to_ascii_lowercase();
    let needle_l = needle.to_ascii_lowercase();
    let mut search = 0usize;
    while let Some(rel) = lower.get(search..).and_then(|s| s.find("href=")) {
        let start = search + rel + 5;
        let rest = html.get(start..).unwrap_or("");
        let quote = rest.chars().next().unwrap_or('"');
        let body = if quote == '"' || quote == '\'' {
            rest.get(1..).unwrap_or("")
        } else {
            rest
        };
        let end = if quote == '"' || quote == '\'' {
            body.find(quote).unwrap_or(body.len().min(400))
        } else {
            body.find(|c: char| c.is_whitespace() || c == '>')
                .unwrap_or(body.len().min(400))
        };
        let href = body.get(..end).unwrap_or("").trim();
        if href.to_ascii_lowercase().contains(&needle_l) {
            if href.starts_with("http://") || href.starts_with("https://") {
                urls.push(href.to_string());
            } else if href.starts_with('/') && !base.is_empty() {
                urls.push(format!("{base}{href}"));
            } else if !href.is_empty() && !base.is_empty() {
                urls.push(format!("{base}/{href}"));
            } else {
                urls.push(href.to_string());
            }
        }
        search = start + 1;
    }
    urls
}

/// Same-host document URLs embedded in JS (not only `href=`).
pub(crate) fn https_urls_matching(html: &str, needle: &str) -> Vec<String> {
    let lower = html.to_ascii_lowercase();
    let needle_l = needle.to_ascii_lowercase();
    let mut urls = Vec::new();
    let mut search = 0usize;
    while let Some(rel) = lower.get(search..).and_then(|s| s.find("https://")) {
        let start = search + rel;
        let rest = html.get(start..).unwrap_or("");
        let end = rest
            .find(|c: char| {
                c.is_whitespace()
                    || c == '"'
                    || c == '\''
                    || c == '<'
                    || c == '>'
                    || c == ')'
                    || c == '\\'
            })
            .unwrap_or(rest.len().min(400));
        let url = rest
            .get(..end)
            .unwrap_or("")
            .trim_end_matches([',', ';', '.', ']']);
        if url.to_ascii_lowercase().contains(&needle_l) {
            urls.push(url.to_string());
        }
        search = start + 8;
    }
    urls
}

pub(crate) fn parse_roundhill_roc_html(html: &str) -> Option<i64> {
    let lower = html.to_ascii_lowercase();
    for needle in [
        "return of capital (roc) of",
        "return of capital of",
        "estimated return of capital of",
    ] {
        if let Some(idx) = lower.find(needle) {
            let after = html.get(idx + needle.len()..).unwrap_or("");
            if let Some(pct) = first_percent(after) {
                return Some(pct);
            }
        }
    }
    None
}

pub(crate) fn parse_vendor_distributions_with_csv(source: &str, html: &str) -> Vec<Value> {
    let mut cands = super::parse_vendor_distributions(source, html);
    if cands.is_empty() && source.eq_ignore_ascii_case("roundhill") {
        for href in csv_hrefs(html) {
            if !financial_domain::div1::declaration_url_matches_source("roundhill", &href) {
                continue;
            }
            if let Ok(csv) = http_get(&href) {
                cands = parse_roundhill_csv(&csv);
                if !cands.is_empty() {
                    break;
                }
            }
        }
    }
    cands
}
pub fn roundhill_fund_page(html: &str, symbol: &str) -> bool {
    !page_is_not_found(html)
        && html.to_ascii_lowercase().contains("roundhill")
        && html_names_symbol(html, symbol)
}
