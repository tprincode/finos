//! Amplify distributions: HTML table when GET exposes it; else the vendor
//! Firestore pack that the fund page loads (`funds/{ticker}/distributions_pack/full`).
//! Pay date is `payableDate`. Blank / "-" amount stays unknown — never $0.

use serde_json::{json, Value};

use crate::retrieve::html::{
    distribution_candidate, json_amount, json_pay_date, parse_distribution_tables,
    sort_newest_first,
};
use super::{html_names_symbol, page_is_not_found};

pub fn parse_amplify_distributions(html: &str) -> Vec<Value> {
    let tables = parse_distribution_tables("amplify", html);
    if !tables.is_empty() {
        return tables;
    }
    parse_amplify_distribution_pack(html)
}

/// Native `{rows:[…]}` or Firestore REST document for `distributions_pack/full`.
pub fn parse_amplify_distribution_pack(body: &str) -> Vec<Value> {
    let trimmed = body.trim();
    if !trimmed.starts_with('{') && !trimmed.starts_with('[') {
        return Vec::new();
    }
    let Ok(v) = serde_json::from_str::<Value>(trimmed) else {
        return Vec::new();
    };
    let plain = unwrap_firestore(&v);
    let rows = match &plain {
        Value::Array(a) => a.clone(),
        Value::Object(o) => o
            .get("rows")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    let mut out = Vec::new();
    for row in rows {
        let Some(pay) = json_pay_date(&row) else {
            continue;
        };
        let amount = json_amount(&row);
        out.push(distribution_candidate("amplify", pay, amount, None));
    }
    sort_newest_first(&mut out);
    out
}

pub fn amplify_fund_page(html: &str, symbol: &str) -> bool {
    !page_is_not_found(html)
        && html.to_ascii_lowercase().contains("amplify")
        && html_names_symbol(html, symbol)
}

/// Amplify fund-page Firestore project (public read; same data the widget uses).
pub fn amplify_distributions_pack_url(symbol: &str) -> String {
    let sym = symbol.trim().to_ascii_uppercase();
    format!(
        "https://firestore.googleapis.com/v1/projects/amplify-etfs-data-feed/databases/(default)/documents/funds/{sym}/distributions_pack/full?key=AIzaSyCibhGo4lu8ZALtBvf_ZT351BDMUPqOYjc"
    )
}

pub fn amplify_fund_page_url(symbol: &str) -> String {
    format!(
        "https://amplifyetfs.com/{}/",
        symbol.trim().to_ascii_lowercase()
    )
}

fn unwrap_firestore(v: &Value) -> Value {
    if let Some(s) = v.get("stringValue") {
        return s.clone();
    }
    if let Some(n) = v.get("integerValue") {
        return match n {
            Value::String(s) => json!(s.parse::<i64>().ok().unwrap_or(0)),
            other => other.clone(),
        };
    }
    if let Some(n) = v.get("doubleValue") {
        if let Some(f) = n.as_f64() {
            // Trim pad zeros so C5 scale matches a prior HTML parse of the same dollars.
            let padded = format!("{f:.6}");
            let trimmed = padded
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string();
            let raw = if trimmed.contains('.') {
                trimmed
            } else {
                format!("{trimmed}.00")
            };
            return json!(raw);
        }
        return n.clone();
    }
    if v.get("nullValue").is_some() {
        return Value::Null;
    }
    if let Some(b) = v.get("booleanValue") {
        return b.clone();
    }
    if let Some(t) = v.get("timestampValue") {
        return t.clone();
    }
    if let Some(arr) = v
        .get("arrayValue")
        .and_then(|a| a.get("values"))
        .and_then(|x| x.as_array())
    {
        return Value::Array(arr.iter().map(unwrap_firestore).collect());
    }
    if let Some(fields) = v
        .get("mapValue")
        .and_then(|m| m.get("fields"))
        .and_then(|f| f.as_object())
    {
        let mut o = serde_json::Map::new();
        for (k, val) in fields {
            o.insert(k.clone(), unwrap_firestore(val));
        }
        return Value::Object(o);
    }
    if let Some(fields) = v.get("fields").and_then(|f| f.as_object()) {
        let mut o = serde_json::Map::new();
        for (k, val) in fields {
            o.insert(k.clone(), unwrap_firestore(val));
        }
        return Value::Object(o);
    }
    v.clone()
}
