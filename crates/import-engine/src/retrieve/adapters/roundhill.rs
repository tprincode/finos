//! Roundhill HTML table, then CSV href. Empty GET stays unknown.

use serde_json::{json, Value};

use crate::retrieve::html::{
    distribution_candidate, parse_issuer_amount, parse_issuer_date, strip_html,
};
use crate::retrieve::{first_percent, http_get};
use super::{html_names_symbol, page_is_not_found};

pub fn parse_roundhill_distributions(html: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let lower = html.to_ascii_lowercase();
    for (idx, _) in lower.match_indices("<tr") {
        let rest = html.get(idx..).unwrap_or("");
        let end = rest.to_ascii_lowercase().find("</tr>").unwrap_or(rest.len().min(4000));
        let row = rest.get(..end).unwrap_or("");
        if row.to_ascii_lowercase().contains("<th") {
            continue;
        }
        let mut cells = Vec::new();
        let row_l = row.to_ascii_lowercase();
        let mut search = 0usize;
        while let Some(rel) = row_l.get(search..).and_then(|s| s.find("<td")) {
            let start = search + rel;
            let after = row.get(start..).unwrap_or("");
            let close = after.to_ascii_lowercase().find("</td>").unwrap_or(after.len().min(400));
            let cell_html = after.get(..close).unwrap_or("");
            cells.push(strip_html(cell_html).trim().to_string());
            search = start + close + 5;
        }
        if cells.len() < 5 {
            continue;
        }
        let pay = parse_issuer_date(&cells[3]).or_else(|| parse_issuer_date(&cells[1]));
        let Some(payment_period) = pay else {
            continue;
        };
        let Some((amount, scale)) = parse_issuer_amount(&cells[4]) else {
            continue;
        };
        if amount <= 0 {
            continue;
        }
        out.push(json!({
            "amountPerShareMinor": amount,
            "amountScale": scale,
            "paymentPeriod": payment_period,
            "source": "roundhill"
        }));
    }
    if out.is_empty() {
        let text = strip_html(html);
        let tokens: Vec<&str> = text.split_whitespace().collect();
        let mut i = 0;
        while i + 4 < tokens.len() {
            let pay = parse_issuer_date(tokens[i + 3]);
            let amt = parse_issuer_amount(tokens[i + 4]);
            if let (Some(payment_period), Some((amount, scale))) = (pay, amt) {
                if amount > 0 {
                    out.push(json!({
                        "amountPerShareMinor": amount,
                        "amountScale": scale,
                        "paymentPeriod": payment_period,
                        "source": "roundhill"
                    }));
                    i += 5;
                    continue;
                }
            }
            i += 1;
        }
    }
    out.truncate(12);
    out
}
pub(crate) fn parse_roundhill_csv(text: &str) -> Vec<Value> {
    let mut out = Vec::new();
    for line in text.lines() {
        let cols: Vec<&str> = line
            .split(',')
            .map(|s| s.trim().trim_matches('"'))
            .collect();
        if cols.len() < 5 {
            continue;
        }
        if cols[0].to_ascii_lowercase().contains("declaration") {
            continue;
        }
        let Some(pay) = parse_issuer_date(cols[3]).or_else(|| parse_issuer_date(cols[1]))
        else {
            continue;
        };
        let amount = parse_issuer_amount(cols[4]);
        if amount.is_none() {
            continue;
        }
        out.push(distribution_candidate("roundhill", pay, amount, None));
    }
    out.truncate(12);
    out
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
