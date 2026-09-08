//! ENERGYX / Form 1-A offering price. Different concern from vendor distributions.

use std::time::Duration;

use serde_json::{json, Value};

use super::html::{parse_leading_dollars, strip_html};
use super::{empty_snapshot, http_get};

pub(crate) const ENERGYX_CIK: &str = "1830166";

pub fn parse_edgar_offering_price(html: &str) -> Option<(i64, u8)> {
    let text = strip_html(html);
    let lower = text.to_ascii_lowercase();
    let cover_end = lower.len().min(80_000);
    let cover = &lower[..cover_end];
    for needle in [
        "the price per share in this offering is $",
        "shares of common stock at $",
        "common stock at $",
        "being sold at $",
        "offered hereby are being sold at $",
    ] {
        let Some(idx) = lower.find(needle) else {
            continue;
        };
        let after = text.get(idx + needle.len()..)?;
        let Some(px) = parse_leading_dollars(after) else {
            continue;
        };
        let window = after.get(..48).unwrap_or(after).to_ascii_lowercase();
        if window.contains("per share")
            || needle.contains("per share")
            || needle.contains("common stock at")
        {
            return Some(px);
        }
        return Some(px);
    }
    let idx = cover.find("at a price of $")?;
    let after = text.get(idx + "at a price of $".len()..)?;
    let px = parse_leading_dollars(after)?;
    let window = after.get(..48).unwrap_or(after).to_ascii_lowercase();
    if window.contains("per share") {
        return Some(px);
    }
    None
}

pub fn parse_edgar_offering_as_of(html: &str) -> Option<String> {
    let text = strip_html(html);
    let lower = text.to_ascii_lowercase();
    let marker = if lower.contains("offering circular dated ") {
        "offering circular dated "
    } else if lower.contains("circular dated ") {
        "circular dated "
    } else {
        return None;
    };
    let idx = lower.find(marker)?;
    let start = idx + marker.len();
    let rest = text.get(start..)?.trim_start();
    let parts: Vec<&str> = rest.split_whitespace().take(3).collect();
    if parts.len() < 3 {
        return None;
    }
    let month = parts[0];
    let day = parts[1].trim_end_matches(',');
    let year = parts[2].trim_end_matches(',');
    let month_num = match month.to_ascii_lowercase().as_str() {
        "january" => 1,
        "february" => 2,
        "march" => 3,
        "april" => 4,
        "may" => 5,
        "june" => 6,
        "july" => 7,
        "august" => 8,
        "september" => 9,
        "october" => 10,
        "november" => 11,
        "december" => 12,
        _ => return None,
    };
    let d: u32 = day.parse().ok()?;
    let y: i32 = year.parse().ok()?;
    chrono::NaiveDate::from_ymd_opt(y, month_num, d).map(|dt| dt.to_string())
}

pub(crate) fn edgar_quote_from_html(html: &str) -> Option<Value> {
    let (price_minor, scale) = parse_edgar_offering_price(html)?;
    let as_of = parse_edgar_offering_as_of(html).unwrap_or_default();
    if price_minor <= 0 {
        return None;
    }
    Some(json!({
        "priceMinor": price_minor,
        "scale": scale,
        "asOfAt": as_of,
        "source": "sec-edgar",
    }))
}

fn pad_cik(cik: &str) -> String {
    let digits: String = cik.chars().filter(|c| c.is_ascii_digit()).collect();
    format!("{digits:0>10}")
}

pub(crate) fn edgar_http_get(url: &str) -> Result<String, String> {
    edgar_http_get_timeout(url, 20)
}

pub(crate) fn edgar_http_get_timeout(url: &str, timeout_secs: u64) -> Result<String, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(45))
        .timeout(Duration::from_secs(timeout_secs))
        .user_agent("FINOS Desktop lastprice@finos.local")
        .build();
    agent
        .get(url)
        .set("Accept", "application/json,text/html,*/*")
        .call()
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())
        .and_then(|body| {
            if body.contains("Undeclared Automated Tool") {
                Err("sec blocked undeclared user-agent".into())
            } else {
                Ok(body)
            }
        })
}

pub(crate) fn live_edgar_offering_quote(cik: &str) -> Option<Value> {
    let padded = pad_cik(cik);
    let bare: String = cik.chars().filter(|c| c.is_ascii_digit()).collect();
    let submissions = format!("https://data.sec.gov/submissions/CIK{padded}.json");
    let body = edgar_http_get(&submissions).ok()?;
    let v: Value = serde_json::from_str(&body).ok()?;
    let recent = v.get("filings")?.get("recent")?;
    let forms = recent.get("form")?.as_array()?;
    let docs = recent.get("primaryDocument")?.as_array()?;
    let accs = recent.get("accessionNumber")?.as_array()?;
    let dates = recent.get("filingDate").and_then(|d| d.as_array());
    for (i, form) in forms.iter().enumerate() {
        let form = form.as_str().unwrap_or("").trim();
        if !matches!(form, "253G2" | "1-A POS" | "1-A" | "1-A/A") {
            continue;
        }
        let doc = docs.get(i).and_then(|d| d.as_str()).unwrap_or("");
        if doc.is_empty() || doc.contains("xsl") {
            continue;
        }
        let acc = accs
            .get(i)
            .and_then(|a| a.as_str())
            .unwrap_or("")
            .replace('-', "");
        if acc.is_empty() {
            continue;
        }
        let url = format!("https://www.sec.gov/Archives/edgar/data/{bare}/{acc}/{doc}");
        if let Ok(html) = edgar_http_get(&url) {
            if let Some(mut quote) = edgar_quote_from_html(&html) {
                let as_of = quote.get("asOfAt").and_then(|s| s.as_str()).unwrap_or("");
                if as_of.is_empty() {
                    if let Some(d) = dates.and_then(|ds| ds.get(i)).and_then(|d| d.as_str()) {
                        quote["asOfAt"] = json!(d);
                    }
                }
                return Some(quote);
            }
        }
    }
    None
}

pub(crate) fn live_energyx_investor_quote() -> Option<Value> {
    let html = http_get("https://invest.energyx.com").ok()?;
    let mut quote = edgar_quote_from_html(&html)?;
    quote["source"] = json!("invest.energyx.com");
    Some(quote)
}

pub(crate) fn uses_offering_price(source: &str, symbol: &str) -> bool {
    let src = source.trim().to_ascii_lowercase();
    matches!(
        src.as_str(),
        "edgar" | "sec" | "sec-edgar" | "offering" | "reg-a" | "rega"
    ) || symbol.trim().eq_ignore_ascii_case("ENERGYX")
}
pub(crate) fn live_offering_snapshot(symbol: &str, cik: &str) -> Value {
    let quote = live_edgar_offering_quote(cik).or_else(|| {
        if symbol.trim().eq_ignore_ascii_case("ENERGYX") {
            live_energyx_investor_quote()
        } else {
            None
        }
    });
    let Some(quote) = quote else {
        return empty_snapshot(symbol);
    };
    let source = quote.get("source").cloned().unwrap_or(json!("sec-edgar"));
    let name = if symbol.trim().eq_ignore_ascii_case("ENERGYX") {
        "Energy Exploration Technologies"
    } else {
        symbol
    };
    json!({
        "symbol": symbol,
        "name": name,
        "suggestedFrequency": "None",
        "quote": quote,
        "quoteCandidates": [quote.clone()],
        "candidates": [],
        "source": source,
    })
}
