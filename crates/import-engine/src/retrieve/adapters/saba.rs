//! Saba ETF vendor site (sabaetf.com). Distributions are embedded in Nuxt `__NUXT_DATA__`
//! on the fund page (year tabs are client-side only).

use serde_json::Value;

use crate::retrieve::html::{
    distribution_candidate, parse_issuer_amount, parse_issuer_date, sort_newest_first,
};

use super::generic::parse_generic_distributions;

pub fn saba_fund_url(symbol: &str) -> String {
    format!(
        "https://www.sabaetf.com/{}",
        symbol.trim().to_ascii_lowercase()
    )
}

pub fn saba_fund_page(html: &str, symbol: &str) -> bool {
    let sym = symbol.trim().to_ascii_uppercase();
    html.to_ascii_uppercase().contains(&sym)
        && (html.contains("__NUXT_DATA__") || html.contains("DISTRIBUTIONS"))
}

fn nuxt_payload(html: &str) -> Option<Vec<Value>> {
    let marker = r#"id="__NUXT_DATA__">"#;
    let start = html.find(marker)? + marker.len();
    let rest = html.get(start..)?;
    let end = rest.find("</script>")?;
    serde_json::from_str(rest.get(..end)?).ok()
}

fn nuxt_resolve<'a>(data: &'a [Value], v: &'a Value, depth: u8) -> Option<&'a Value> {
    if depth > 24 {
        return None;
    }
    match v {
        Value::Number(n) if n.is_i64() => {
            let idx = n.as_i64()? as usize;
            if idx < data.len() {
                nuxt_resolve(data, data.get(idx)?, depth + 1)
            } else {
                Some(v)
            }
        }
        _ => Some(v),
    }
}

fn nuxt_field<'a>(data: &'a [Value], obj: &'a Value, key: &str) -> Option<&'a Value> {
    let raw = obj.get(key)?;
    match raw {
        Value::Number(n) if n.is_i64() => {
            let idx = n.as_i64()? as usize;
            nuxt_resolve(data, data.get(idx)?, 0)
        }
        _ => nuxt_resolve(data, raw, 0),
    }
}

fn nuxt_string(data: &[Value], obj: &Value, key: &str) -> Option<String> {
    let v = nuxt_field(data, obj, key)?;
    match v {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        _ => None,
    }
}

fn nuxt_amount(data: &[Value], obj: &Value, key: &str) -> Option<(i64, u8)> {
    if let Some(s) = nuxt_string(data, obj, key) {
        return parse_issuer_amount(&s);
    }
    let v = nuxt_field(data, obj, key)?;
    match v {
        Value::Number(n) => parse_issuer_amount(&n.to_string()),
        _ => None,
    }
}

/// Parse distribution rows from Nuxt dehydrated state or fallback HTML table.
pub fn parse_saba_distributions(source: &str, html: &str) -> Vec<Value> {
    let mut out = parse_saba_nuxt_distributions(source, html);
    if out.is_empty() {
        out = parse_generic_distributions(source, html);
    }
    out
}

pub fn parse_saba_nuxt_distributions(source: &str, html: &str) -> Vec<Value> {
    let Some(data) = nuxt_payload(html) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for v in &data {
        let Value::Object(obj) = v else {
            continue;
        };
        if !obj.contains_key("pay_date") || !obj.contains_key("dividend") {
            continue;
        }
        let Some(pay_raw) = nuxt_string(&data, v, "pay_date") else {
            continue;
        };
        let Some(pay) = parse_issuer_date(&pay_raw) else {
            continue;
        };
        let amount = nuxt_amount(&data, v, "dividend");
        out.push(distribution_candidate(source, pay, amount, None));
    }
    sort_newest_first(&mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn fixture_cefs_nuxt_rows() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/saba_cefs.html");
        let html = std::fs::read_to_string(path).expect("fixture");
        let rows = parse_saba_nuxt_distributions("saba", &html);
        assert!(rows.len() >= 12, "expected rows, got {}", rows.len());
        assert_eq!(rows[0]["paymentPeriod"].as_str(), Some("2026-08-31"));
        assert_eq!(rows[0]["amountPerShareMinor"], 14);
        for row in &rows {
            let period = row["paymentPeriod"].as_str().unwrap_or("");
            let minor = row["amountPerShareMinor"].as_i64();
            if period.starts_with("2026") {
                assert_eq!(
                    minor,
                    Some(14),
                    "2026 row {period} should be $0.14 (14 minor), got {minor:?}"
                );
            }
            assert_ne!(
                minor,
                Some(100),
                "suspicious $1.00 parse for {period} (Nuxt index leak)"
            );
        }
    }
}
