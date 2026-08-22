//! Candidate-only retrieval adapters (ADR-0010). They never post facts.
//! Live HTTP is best-effort; tests inject fixtures and do not require a network.

use std::time::Duration;

use chrono::{TimeZone, Utc};
use serde_json::{json, Value};

/// Echo injected quote candidates. Empty means the live adapter missed — owner must prompt.
pub fn price_quote_candidates(injected: Option<&Value>) -> Vec<Value> {
    match injected {
        Some(v) if v.is_object() => vec![v.clone()],
        Some(Value::Array(arr)) => arr.clone(),
        _ => Vec::new(),
    }
}

/// Echo up to 12 injected declaration candidates. Empty means prompt/paste.
pub fn declaration_candidates(injected: Option<&Value>) -> Vec<Value> {
    match injected {
        Some(Value::Array(arr)) => arr.iter().take(12).cloned().collect(),
        Some(v) if v.is_object() => vec![v.clone()],
        _ => Vec::new(),
    }
}

pub fn retrieve_result(candidates: Vec<Value>) -> Value {
    json!({ "candidates": candidates })
}

fn yahoo_symbol(symbol: &str) -> String {
    symbol.trim().to_ascii_uppercase().replace('.', "-")
}

fn dollars_to_minor(amount: f64, scale: u8) -> i64 {
    let factor = 10f64.powi(scale as i32);
    (amount * factor).round() as i64
}

fn iso_from_unix(ts: i64) -> String {
    Utc.timestamp_opt(ts, 0)
        .single()
        .map(|dt| dt.date_naive().to_string())
        .unwrap_or_default()
}

fn suggest_frequency(dates_newest_first: &[String]) -> String {
    if dates_newest_first.len() < 2 {
        return "Monthly".into();
    }
    let parsed: Vec<chrono::NaiveDate> = dates_newest_first
        .iter()
        .filter_map(|d| chrono::NaiveDate::parse_from_str(&d[..d.len().min(10)], "%Y-%m-%d").ok())
        .collect();
    if parsed.len() < 2 {
        return "Monthly".into();
    }
    let mut gaps: Vec<i64> = parsed
        .windows(2)
        .map(|w| (w[0] - w[1]).num_days().abs())
        .collect();
    gaps.sort_unstable();
    let med = gaps[gaps.len() / 2];
    if med <= 10 {
        "Weekly".into()
    } else if med <= 40 {
        "Monthly".into()
    } else {
        "Quarterly".into()
    }
}

fn http_get(url: &str) -> Result<String, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(12))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
        .build();
    agent
        .get(url)
        .set("Accept", "application/json,text/csv,*/*")
        .call()
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())
}

/// Parse a Yahoo v8 chart payload (quote + dividend events). Network-free.
pub fn parse_yahoo_chart(symbol: &str, body: &str) -> Option<Value> {
    let v: Value = serde_json::from_str(body).ok()?;
    let result = v.pointer("/chart/result/0")?;
    let meta = result.get("meta")?;
    let price = meta
        .get("regularMarketPrice")
        .and_then(|p| p.as_f64())
        .filter(|p| *p > 0.0);
    let as_of = meta
        .get("regularMarketTime")
        .and_then(|t| t.as_i64())
        .map(iso_from_unix)
        .filter(|s| !s.is_empty());
    let name = meta
        .get("shortName")
        .or_else(|| meta.get("longName"))
        .and_then(|n| n.as_str())
        .unwrap_or(symbol)
        .to_string();
    let mut divs: Vec<(i64, f64)> = Vec::new();
    if let Some(obj) = result.pointer("/events/dividends").and_then(|d| d.as_object()) {
        for item in obj.values() {
            let amount = item.get("amount").and_then(|a| a.as_f64()).unwrap_or(0.0);
            let date = item.get("date").and_then(|d| d.as_i64()).unwrap_or(0);
            if amount > 0.0 && date > 0 {
                divs.push((date, amount));
            }
        }
    }
    divs.sort_by(|a, b| b.0.cmp(&a.0));
    divs.truncate(12);
    let declarations: Vec<Value> = divs
        .iter()
        .map(|(ts, amt)| {
            json!({
                "amountPerShareMinor": dollars_to_minor(*amt, 4),
                "amountScale": 4,
                "paymentPeriod": iso_from_unix(*ts),
                "source": "yahoo"
            })
        })
        .collect();
    let dates: Vec<String> = declarations
        .iter()
        .filter_map(|d| d.get("paymentPeriod").and_then(|p| p.as_str()).map(|s| s.to_string()))
        .collect();
    let quote = price.map(|p| {
        json!({
            "priceMinor": dollars_to_minor(p, 2),
            "scale": 2,
            "asOfAt": as_of.clone().unwrap_or_default(),
            "source": "yahoo"
        })
    });
    Some(json!({
        "symbol": yahoo_symbol(symbol),
        "name": name,
        "suggestedFrequency": suggest_frequency(&dates),
        "quote": quote,
        "quoteCandidates": quote.clone().map(|q| vec![q]).unwrap_or_default(),
        "candidates": declarations,
        "source": "yahoo"
    }))
}

fn parse_stooq_csv(symbol: &str, csv: &str) -> Option<Value> {
    let line = csv.lines().nth(1)?;
    let cols: Vec<&str> = line.split(',').collect();
    if cols.len() < 7 {
        return None;
    }
    let date = cols.get(1)?.trim();
    let close: f64 = cols.get(6)?.trim().parse().ok()?;
    if close <= 0.0 {
        return None;
    }
    let quote = json!({
        "priceMinor": dollars_to_minor(close, 2),
        "scale": 2,
        "asOfAt": date,
        "source": "stooq"
    });
    Some(json!({
        "symbol": yahoo_symbol(symbol),
        "name": yahoo_symbol(symbol),
        "suggestedFrequency": "Monthly",
        "quote": quote,
        "quoteCandidates": [quote],
        "candidates": [],
        "source": "stooq"
    }))
}

fn empty_snapshot(symbol: &str) -> Value {
    json!({
        "symbol": yahoo_symbol(symbol),
        "name": "",
        "suggestedFrequency": "Monthly",
        "quote": null,
        "quoteCandidates": [],
        "candidates": [],
        "source": "miss"
    })
}

/// Best-effort public quote + last 12 issuer declarations. Never posts.
pub fn live_market_snapshot(symbol: &str) -> Value {
    let sym = yahoo_symbol(symbol);
    if sym.is_empty() {
        return empty_snapshot(symbol);
    }
    let yahoo = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{sym}?interval=1d&range=2y&events=div"
    );
    let yahoo2 = format!(
        "https://query2.finance.yahoo.com/v8/finance/chart/{sym}?interval=1d&range=2y&events=div"
    );
    for url in [yahoo, yahoo2] {
        if let Ok(body) = http_get(&url) {
            if let Some(parsed) = parse_yahoo_chart(&sym, &body) {
                let has_quote = parsed.get("quote").map(|q| !q.is_null()).unwrap_or(false);
                let n = parsed
                    .get("candidates")
                    .and_then(|c| c.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                if has_quote || n > 0 {
                    return parsed;
                }
            }
        }
    }
    let stooq = format!("https://stooq.com/q/l/?s={}.us&f=sd2t2ohlcv&h&e=csv", sym.to_ascii_lowercase());
    if let Ok(csv) = http_get(&stooq) {
        if let Some(parsed) = parse_stooq_csv(&sym, &csv) {
            return parsed;
        }
    }
    empty_snapshot(&sym)
}

/// Fill empty candidate lists on retrieve commands. Injected candidates win (CI).
pub fn enrich_retrieve_body(command_name: &str, body: &mut Value) {
    let has_injected = body
        .get("candidates")
        .and_then(|c| c.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false);
    if has_injected {
        return;
    }
    let symbol = body
        .get("symbol")
        .or_else(|| body.get("sourceSymbol"))
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    if symbol.is_empty() {
        return;
    }
    let live = live_market_snapshot(&symbol);
    match command_name {
        "PriceQuoteRetrieve" => {
            body["candidates"] = live["quoteCandidates"].clone();
        }
        "DeclarationRetrieve" => {
            body["candidates"] = live["candidates"].clone();
        }
        "MarketRetrieve" => {
            if let Some(obj) = live.as_object() {
                for (k, v) in obj {
                    body[k] = v.clone();
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_inject_means_prompt() {
        assert!(price_quote_candidates(None).is_empty());
        assert!(declaration_candidates(None).is_empty());
        let one = json!({"priceMinor": 1250, "asOfAt": "2026-08-21", "source": "fixture"});
        assert_eq!(price_quote_candidates(Some(&one)).len(), 1);
    }

    #[test]
    fn yahoo_chart_fixture_fills_quote_and_twelve_divs() {
        let mut divs = String::from("{");
        for i in 0..12 {
            if i > 0 {
                divs.push(',');
            }
            let ts = 1_700_000_000 + i * 86400 * 7;
            divs.push_str(&format!(
                "\"{ts}\":{{\"amount\":0.1{},\"date\":{ts}}}",
                i % 9
            ));
        }
        divs.push('}');
        let body = format!(
            r#"{{"chart":{{"result":[{{"meta":{{"symbol":"GOF","shortName":"Guggenheim Strategic Opportunities","regularMarketPrice":14.85,"regularMarketTime":1755705600}},"events":{{"dividends":{divs}}}}}]}}}}"#
        );
        let parsed = parse_yahoo_chart("GOF", &body).unwrap();
        assert_eq!(parsed["name"], "Guggenheim Strategic Opportunities");
        assert_eq!(parsed["quote"]["priceMinor"], 1485);
        assert_eq!(parsed["candidates"].as_array().unwrap().len(), 12);
        assert_eq!(parsed["suggestedFrequency"], "Weekly");
    }

    #[test]
    fn enrich_keeps_injected_candidates() {
        let mut body = json!({"symbol":"GOF","candidates":[{"priceMinor":1}]});
        enrich_retrieve_body("PriceQuoteRetrieve", &mut body);
        assert_eq!(body["candidates"].as_array().unwrap().len(), 1);
        assert_eq!(body["candidates"][0]["priceMinor"], 1);
    }
}
