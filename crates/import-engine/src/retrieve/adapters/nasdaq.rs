//! Nasdaq quote dividends JSON — used when issuer pages are SPA/Cloudflare empty.
//! Not Yahoo. Empty rows stay a loud miss.

use serde_json::Value;

use crate::retrieve::html::{
    distribution_candidate, parse_issuer_amount, parse_issuer_date, sort_newest_first,
};

/// `assetclass` query value for Nasdaq dividends API.
pub fn nasdaq_asset_class(symbol: &str) -> &'static str {
    match symbol.trim().to_ascii_uppercase().as_str() {
        // Equities / CEFs / BDCs on Nasdaq dividends API as stocks
        "GLAD" | "EFC" | "ORC" | "TRIN" | "CRF" | "CLM" | "MPLX" | "EPD" | "ET" => "stocks",
        _ => "etf",
    }
}

pub fn nasdaq_dividends_url(symbol: &str) -> String {
    let sym = symbol.trim().to_ascii_uppercase();
    format!(
        "https://api.nasdaq.com/api/quote/{}/dividends?assetclass={}",
        sym,
        nasdaq_asset_class(&sym)
    )
}

/// Third-party HTML calendar when issuer/Nasdaq have no plain history (not Yahoo).
pub fn dividendhistory_url(symbol: &str) -> String {
    format!(
        "https://dividendhistory.org/payout/{}/",
        symbol.trim().to_ascii_uppercase()
    )
}

/// Parse `api.nasdaq.com/.../dividends` JSON body.
pub fn parse_nasdaq_dividends(source: &str, body: &str) -> Vec<Value> {
    let trimmed = body.trim();
    if !trimmed.starts_with('{') {
        return Vec::new();
    }
    let Ok(v) = serde_json::from_str::<Value>(trimmed) else {
        return Vec::new();
    };
    let rows = v
        .pointer("/data/dividends/rows")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    let mut out = Vec::new();
    for row in rows {
        let pay = row
            .get("paymentDate")
            .and_then(|x| x.as_str())
            .and_then(parse_issuer_date)
            .or_else(|| {
                row.get("exOrEffDate")
                    .and_then(|x| x.as_str())
                    .and_then(parse_issuer_date)
            });
        let Some(pay) = pay else {
            continue;
        };
        let amount = row
            .get("amount")
            .and_then(|x| x.as_str())
            .and_then(parse_issuer_amount);
        out.push(distribution_candidate(source, pay, amount, None));
    }
    sort_newest_first(&mut out);
    out
}
