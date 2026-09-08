//! Nasdaq quote dividends JSON parser — fixture-only. Fetch never uses Nasdaq
//! or dividendhistory.org; those are not vendor adapters.

use serde_json::Value;

use crate::retrieve::html::{
    distribution_candidate, json_amount, json_pay_date, sort_newest_first,
};

/// `assetclass` query value for Nasdaq dividends API (fixture parser only).
#[allow(dead_code)]
pub fn nasdaq_asset_class(symbol: &str) -> &'static str {
    match symbol.trim().to_ascii_uppercase().as_str() {
        // Equities / CEFs / BDCs on Nasdaq dividends API as stocks
        "GLAD" | "EFC" | "ORC" | "TRIN" | "CRF" | "CLM" | "MPLX" | "EPD" | "ET" => "stocks",
        _ => "etf",
    }
}

#[allow(dead_code)]
pub fn nasdaq_dividends_url(symbol: &str) -> String {
    let sym = symbol.trim().to_ascii_uppercase();
    format!(
        "https://api.nasdaq.com/api/quote/{}/dividends?assetclass={}",
        sym,
        nasdaq_asset_class(&sym)
    )
}

/// Not fetched. Kept so tests can prove this host is rejected.
#[allow(dead_code)]
pub fn dividendhistory_url(symbol: &str) -> String {
    format!(
        "https://dividendhistory.org/payout/{}/",
        symbol.trim().to_ascii_uppercase()
    )
}

/// Parse `api.nasdaq.com/.../dividends` JSON body. Payment date by field name only.
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
        let Some(pay) = json_pay_date(&row) else {
            continue;
        };
        let amount = json_amount(&row);
        out.push(distribution_candidate(source, pay, amount, None));
    }
    sort_newest_first(&mut out);
    out
}
