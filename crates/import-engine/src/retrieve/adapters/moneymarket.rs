//! Fidelity / Schwab money-market rate pages.
//! NAV stays par $1 elsewhere. This adapter parses the published 7-day yield
//! and converts it to monthly Plan $ (yield ÷ 12). Dividend tables are fallback only.

use serde_json::{json, Value};

use super::generic::{generic_fund_page, parse_generic_distributions};
use financial_domain::current_price::seven_day_yield_to_monthly_plan;

pub fn parse_moneymarket_distributions(source: &str, html: &str) -> Vec<Value> {
    parse_moneymarket_distributions_for(source, html, "")
}

pub fn parse_moneymarket_distributions_for(source: &str, html: &str, symbol: &str) -> Vec<Value> {
    if let Some(row) = cash_rate_candidate(source, html, symbol) {
        return vec![row];
    }
    parse_generic_distributions(source, html)
}

pub fn is_cash_rate_candidate(row: &Value) -> bool {
    row.get("kind")
        .and_then(|v| v.as_str())
        .is_some_and(|k| k.eq_ignore_ascii_case("cash_rate"))
        || row.get("planOnly").and_then(|v| v.as_bool()).unwrap_or(false)
}

pub fn moneymarket_fund_page(source: &str, html: &str, symbol: &str) -> bool {
    let low = html.to_ascii_lowercase();
    if low.contains("sevendayyield") || low.contains("7-day yield") || low.contains("7 day yield")
    {
        return html_has_symbol(html, symbol) || source_needles(source, &low);
    }
    let needles: &[&str] = match source.trim().to_ascii_lowercase().as_str() {
        "fidelity" => &["fidelity", "government money market", "cash reserves"],
        "schwab" => &["schwab", "value advantage", "money market"],
        _ => return false,
    };
    generic_fund_page(html, symbol, needles)
}

pub fn moneymarket_probe_urls(source: &str, symbol: &str) -> Vec<String> {
    let sym = symbol.trim().to_ascii_uppercase();
    let lower = sym.to_ascii_lowercase();
    match source.trim().to_ascii_lowercase().as_str() {
        "fidelity" => match sym.as_str() {
            "SPAXX" => vec![
                "https://fundresearch.fidelity.com/mutual-funds/performance-and-risk/31617H102"
                    .into(),
                format!("https://fundresearch.fidelity.com/mutual-funds/summary/{sym}"),
            ],
            "FDRXX" => vec![
                "https://fundresearch.fidelity.com/mutual-funds/summary/316067107".into(),
                format!(
                    "https://fundresearch.fidelity.com/mutual-funds/performance-and-risk/{sym}"
                ),
            ],
            _ => vec![
                format!("https://fundresearch.fidelity.com/mutual-funds/summary/{sym}"),
                format!(
                    "https://fundresearch.fidelity.com/mutual-funds/fees-and-distributions/{sym}"
                ),
            ],
        },
        "schwab" => match sym.as_str() {
            "SWVXX" => vec![
                "https://www.schwab.com/money-market-funds".into(),
                "https://www.schwab.com/research/mutual-funds/quotes/summary/swvxx".into(),
            ],
            _ => vec![
                format!("https://www.schwabassetmanagement.com/products/{lower}"),
                format!("https://www.schwab.com/asset-management/products/{lower}"),
            ],
        },
        _ => Vec::new(),
    }
}

pub fn fidelity_mm_cusip(symbol: &str, source_url: &str) -> Option<String> {
    if let Some(cusip) = cusip_from_url(source_url) {
        return Some(cusip);
    }
    match symbol.trim().to_ascii_uppercase().as_str() {
        "SPAXX" => Some("31617H102".into()),
        "FDRXX" => Some("316067107".into()),
        _ => None,
    }
}

pub fn fidelity_mm_header_url(cusip: &str) -> String {
    format!(
        "https://fundresearch.fidelity.com/mutual-funds/api/v1/investments/{cusip}/header?funduniverse=RETAIL&documentId={cusip}"
    )
}

pub fn standing_mm_url(source: &str, symbol: &str) -> String {
    moneymarket_probe_urls(source, symbol)
        .into_iter()
        .next()
        .unwrap_or_default()
}

fn cash_rate_candidate(source: &str, html: &str, symbol: &str) -> Option<Value> {
    let (pct, as_of) = seven_day_yield_from_body(html, symbol)?;
    let (minor, scale, bps) = seven_day_yield_to_monthly_plan(&pct)?;
    Some(json!({
        "amountPerShareMinor": minor,
        "amountScale": scale,
        "paymentPeriod": as_of,
        "source": source.trim().to_ascii_lowercase(),
        "kind": "cash_rate",
        "planOnly": true,
        "annualYieldBps": bps,
        "sevenDayYield": pct,
    }))
}

fn seven_day_yield_from_body(html: &str, symbol: &str) -> Option<(String, String)> {
    if let Some(found) = seven_day_yield_from_json(html) {
        return Some(found);
    }
    seven_day_yield_from_html(html, symbol)
}

fn seven_day_yield_from_json(body: &str) -> Option<(String, String)> {
    let trimmed = body.trim_start();
    if !trimmed.starts_with('{') {
        return None;
    }
    let v: Value = serde_json::from_str(trimmed).ok()?;
    let raw = v.pointer("/dailyInfo/subjectAreaData/sevenDayYield")?;
    let pct = if let Some(n) = raw.as_f64() {
        format!("{n:.2}")
    } else {
        raw.as_str()?.trim().to_string()
    };
    let as_of = v
        .pointer("/dailyInfo/subjectAreaData/yieldAsOfDate")
        .and_then(|x| x.as_str())
        .map(mdy_to_iso)
        .unwrap_or_default();
    Some((pct, as_of))
}

fn seven_day_yield_from_html(html: &str, symbol: &str) -> Option<(String, String)> {
    let low = html.to_ascii_lowercase();
    if let Some(idx) = low
        .find("7-day yield")
        .or_else(|| low.find("7 day yield"))
    {
        let end = (idx + 500).min(html.len());
        if let Some(pct) = first_percent(&html[idx..end]) {
            if symbol_scoped_or_single(html, symbol, idx) {
                return Some((pct, String::new()));
            }
        }
    }
    if !symbol.trim().is_empty() {
        if let Some(pct) = percent_after_symbol(html, symbol) {
            return Some((pct, String::new()));
        }
    }
    None
}

fn symbol_scoped_or_single(html: &str, symbol: &str, yield_idx: usize) -> bool {
    let sym = symbol.trim().to_ascii_uppercase();
    if sym.is_empty() {
        return true;
    }
    let up = html.to_ascii_uppercase();
    let Some(sym_idx) = up.find(&sym) else {
        return true;
    };
    sym_idx < yield_idx + 800
}

fn percent_after_symbol(html: &str, symbol: &str) -> Option<String> {
    let up = html.to_ascii_uppercase();
    let sym = symbol.trim().to_ascii_uppercase();
    if sym.is_empty() {
        return None;
    }
    let idx = up.find(&sym)?;
    let end = (idx + 800).min(html.len());
    first_percent(&html[idx..end])
}

fn first_percent(window: &str) -> Option<String> {
    let bytes = window.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'.' {
                i += 1;
                let frac = i;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                if i > frac {
                    let mut j = i;
                    while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                        j += 1;
                    }
                    if j < bytes.len() && bytes[j] == b'%' {
                        return Some(window[start..i].to_string());
                    }
                }
            }
        } else {
            i += 1;
        }
    }
    None
}

fn mdy_to_iso(raw: &str) -> String {
    let p: Vec<&str> = raw.split(['/', '-']).collect();
    if p.len() == 3 {
        if let (Ok(m), Ok(d), Ok(y)) = (
            p[0].parse::<u32>(),
            p[1].parse::<u32>(),
            p[2].parse::<i32>(),
        ) {
            let y = if y < 100 { 2000 + y } else { y };
            return format!("{y:04}-{m:02}-{d:02}");
        }
    }
    String::new()
}

fn cusip_from_url(url: &str) -> Option<String> {
    let last = url
        .trim()
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("")
        .split('?')
        .next()
        .unwrap_or("");
    let up = last.to_ascii_uppercase();
    if (8..=9).contains(&up.len())
        && up.chars().all(|c| c.is_ascii_alphanumeric())
        && up.chars().any(|c| c.is_ascii_digit())
    {
        Some(up)
    } else {
        None
    }
}

fn html_has_symbol(html: &str, symbol: &str) -> bool {
    let sym = symbol.trim().to_ascii_uppercase();
    !sym.is_empty() && html.to_ascii_uppercase().contains(&sym)
}

fn source_needles(source: &str, low: &str) -> bool {
    match source.trim().to_ascii_lowercase().as_str() {
        "fidelity" => low.contains("fidelity"),
        "schwab" => low.contains("schwab"),
        _ => false,
    }
}
