//! Candidate-only retrieval adapters (ADR-0010). They never post facts.
//! Live HTTP is best-effort; tests inject fixtures and do not require a network.

mod adapters;
mod edgar;
mod html;
mod lookthrough;

pub use adapters::{
    amplify_fund_page, parse_amplify_distributions, parse_generic_distributions,
    parse_moneymarket_distributions, parse_nasdaq_dividends, parse_neos_distributions,
    parse_proshares_distribution_summary, parse_saba_distributions, parse_simplify_distributions,
    parse_roundhill_distribution_api, parse_roundhill_distributions, parse_yieldmax_distributions,
    roundhill_fund_page,
};
pub use edgar::{parse_edgar_offering_as_of, parse_edgar_offering_price};
#[allow(unused_imports)]
pub(crate) use adapters::{
    parse_roundhill_csv, parse_roundhill_roc_html, parse_vendor_distributions_with_csv,
};
pub(crate) use edgar::{
    live_edgar_offering_quote, live_energyx_investor_quote, live_offering_snapshot, uses_offering_price,
    ENERGYX_CIK,
};
pub(crate) use html::{
    candidate_amount, strip_html, upcoming_from_candidates,
};
use lookthrough::{empty_lookthrough, extract_lookthrough};

use std::time::Duration;

use chrono::{Datelike, TimeZone, Utc};
use serde_json::{json, Value};

use adapters::{hrefs_matching, neos_fund_page, page_is_not_found, yieldmax_fund_page};
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
        Some(Value::Array(arr)) => arr.clone(),
        Some(v) if v.is_object() => vec![v.clone()],
        _ => Vec::new(),
    }
}

pub fn retrieve_result(candidates: Vec<Value>) -> Value {
    json!({ "candidates": candidates })
}

fn yahoo_symbol(symbol: &str) -> String {
    let s = symbol.trim().to_ascii_uppercase().replace('.', "-");
    match s.as_str() {
        "ETH" | "SOL" => format!("{s}-USD"),
        _ => s,
    }
}

#[derive(Clone)]
pub struct LastPriceTarget {
    pub security_id: String,
    pub symbol: String,
    pub price_source: String,
    pub source_symbol: String,
}


fn live_offering_or_yahoo(target: &LastPriceTarget) -> Option<Value> {
    if uses_offering_price(&target.price_source, &target.symbol) {
        let cik = if target.source_symbol.chars().any(|c| c.is_ascii_digit()) {
            target.source_symbol.trim()
        } else {
            ENERGYX_CIK
        };
        if let Some(q) = live_edgar_offering_quote(cik) {
            return Some(q);
        }
        if target.symbol.trim().eq_ignore_ascii_case("ENERGYX") {
            if let Some(q) = live_energyx_investor_quote() {
                return Some(q);
            }
        }
        return None;
    }
    let symbol = if target.source_symbol.trim().is_empty() {
        target.symbol.as_str()
    } else {
        target.source_symbol.as_str()
    };
    live_price_quote(symbol)
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
    let periods: Vec<&str> = dates_newest_first.iter().map(String::as_str).collect();
    financial_domain::calculator::infer_payment_cadence(&periods, None)
        .map(|c| c.label().to_string())
        .unwrap_or_default()
}

const HTTP_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

pub(crate) fn http_get(url: &str) -> Result<String, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(12))
        .user_agent(HTTP_UA)
        .build();
    agent
        .get(url)
        .set("Accept", "application/json,text/csv,*/*")
        .call()
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())
}

fn http_get_referer(url: &str, referer: &str) -> Result<String, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(12))
        .user_agent(HTTP_UA)
        .build();
    agent
        .get(url)
        .set("Accept", "*/*")
        .set("Referer", referer)
        .set("Origin", "https://www.roundhillinvestments.com")
        .set("X-Requested-With", "XMLHttpRequest")
        .call()
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())
}

fn http_post_form_origin(
    url: &str,
    referer: &str,
    origin: &str,
    form: &[(&str, &str)],
) -> Result<String, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(20))
        .user_agent(HTTP_UA)
        .build();
    agent
        .post(url)
        .set("Accept", "*/*")
        .set("Referer", referer)
        .set("Origin", origin)
        .set("X-Requested-With", "XMLHttpRequest")
        .set("Content-Type", "application/x-www-form-urlencoded")
        .send_form(form)
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())
}

fn http_post_form_body(url: &str, referer: &str, origin: &str, body: &str) -> Result<String, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(20))
        .user_agent(HTTP_UA)
        .build();
    agent
        .post(url)
        .set("Accept", "text/html,*/*")
        .set("Referer", referer)
        .set("Origin", origin)
        .set("Content-Type", "application/x-www-form-urlencoded")
        .send_string(body)
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())
}

fn http_probe(url: &str, status: Option<u16>, error: Option<&str>) -> Value {
    json!({
        "url": url,
        "status": status,
        "error": error,
    })
}

fn http_status_from_err(err: &ureq::Error) -> (Option<u16>, String) {
    match err {
        ureq::Error::Status(code, _) => (Some(*code), format!("http_{code}")),
        other => (None, other.to_string()),
    }
}

#[allow(dead_code)]
fn http_get_bytes(url: &str) -> Result<Vec<u8>, String> {
    http_get_bytes_probed(url).0
}

fn http_get_bytes_probed(url: &str) -> (Result<Vec<u8>, String>, Value) {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(6))
        .user_agent(HTTP_UA)
        .build();
    match agent
        .get(url)
        .set("Accept", "application/pdf,text/html,*/*")
        .call()
    {
        Ok(resp) => {
            let status = resp.status();
            let mut reader = resp.into_reader();
            let mut buf = Vec::new();
            match std::io::Read::read_to_end(&mut reader, &mut buf) {
                Ok(_) => (Ok(buf), http_probe(url, Some(status), None)),
                Err(e) => (
                    Err(e.to_string()),
                    http_probe(url, Some(status), Some(&e.to_string())),
                ),
            }
        }
        Err(e) => {
            let (status, msg) = http_status_from_err(&e);
            (Err(msg.clone()), http_probe(url, status, Some(&msg)))
        }
    }
}

fn http_get_probed(url: &str) -> (Result<String, String>, Value) {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(12))
        .user_agent(HTTP_UA)
        .build();
    match agent
        .get(url)
        .set("Accept", "application/json,text/csv,*/*")
        .call()
    {
        Ok(resp) => {
            let status = resp.status();
            match resp.into_string() {
                Ok(body) => (Ok(body), http_probe(url, Some(status), None)),
                Err(e) => (
                    Err(e.to_string()),
                    http_probe(url, Some(status), Some(&e.to_string())),
                ),
            }
        }
        Err(e) => {
            let (status, msg) = http_status_from_err(&e);
            (Err(msg.clone()), http_probe(url, status, Some(&msg)))
        }
    }
}

fn pdf_ascii(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut cur = String::new();
    for &b in bytes {
        if (32..127).contains(&b) {
            cur.push(b as char);
        } else if !cur.is_empty() {
            if cur.len() >= 3 {
                out.push_str(&cur);
                out.push(' ');
            }
            cur.clear();
        }
    }
    if cur.len() >= 3 {
        out.push_str(&cur);
    }
    out
}

/// Current-year 19a-1 estimate. Network-free. 100% ROC → 10000 at scale 2.
pub fn parse_19a1_notice(text: &str) -> Option<(i64, String)> {
    let lower = text.to_ascii_lowercase();
    let needle = "of such dividend will be a return of capital";
    if let Some(idx) = lower.find(needle) {
        let before = &text[..idx];
        if let Some(pct) = trailing_percent(before) {
            return Some((pct, "current distribution 19a-1 estimate".into()));
        }
    }
    if let Some(idx) = lower.rfind("estimated return of") {
        let after = &text[idx..idx.saturating_add(400).min(text.len())];
        if after.to_ascii_lowercase().contains("capital") {
            if let Some(pct) = first_percent(after) {
                return Some((pct, "19a-1 table current distribution".into()));
            }
        }
    }
    None
}

fn trailing_percent(before: &str) -> Option<i64> {
    let bytes = before.as_bytes();
    let mut i = bytes.len();
    while i > 0 && bytes[i - 1].is_ascii_whitespace() {
        i -= 1;
    }
    if i == 0 || bytes[i - 1] != b'%' {
        return None;
    }
    let mut j = i - 1;
    while j > 0 && (bytes[j - 1].is_ascii_digit() || bytes[j - 1] == b'.') {
        j -= 1;
    }
    percent_to_minor(std::str::from_utf8(&bytes[j..i - 1]).ok()?)
}

pub(crate) fn first_percent(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] == b'%' && i > 0 {
            let mut j = i;
            while j > 0 && (bytes[j - 1].is_ascii_digit() || bytes[j - 1] == b'.') {
                j -= 1;
            }
            if j < i {
                if let Some(pct) = percent_to_minor(std::str::from_utf8(&bytes[j..i]).ok()?) {
                    return Some(pct);
                }
            }
        }
    }
    None
}

pub(crate) fn percent_to_minor(raw: &str) -> Option<i64> {
    let n: f64 = raw.trim().parse().ok()?;
    if !(0.0..=100.0).contains(&n) {
        return None;
    }
    Some((n * 100.0).round() as i64)
}

fn amplify_19a1_urls(symbol: &str) -> Vec<(String, String)> {
    let sym = yahoo_symbol(symbol);
    let stamps = [
        ("2026-08-31", "08-31-26"),
        ("2026-07-31", "07-31-26"),
        ("2026-06-30", "06-30-26"),
        ("2026-05-29", "05-29-26"),
        ("2026-04-30", "04-30-26"),
        ("2026-03-31", "03-31-26"),
        ("2026-02-27", "02-27-26"),
        ("2026-01-30", "01-30-26"),
    ];
    stamps
        .into_iter()
        .map(|(as_of, file)| {
            (
                as_of.to_string(),
                format!(
                    "https://amplifyetfs.com/wp-content/uploads/files/19a-1_Notice_{file}_{sym}.pdf"
                ),
            )
        })
        .collect()
}

/// Live 19a-1 fill: candidates (empty = unknown, never 0%) plus URL/HTTP probes.
#[derive(Debug, Clone, Default)]
pub struct LiveRocFill {
    pub candidates: Vec<Value>,
    pub probes: Vec<Value>,
}

/// Best-effort current-year 19a-1 candidates. Empty means unknown, not 0%.
pub fn live_roc_candidates(symbol: &str) -> Vec<Value> {
    live_roc_candidates_for(symbol, "", "").candidates
}

fn roc_estimate(pct: i64, url: &str, as_of: &str, how: String, method: &str) -> Value {
    json!({
        "rocPctMinor": pct,
        "scale": 2,
        "taxYear": as_of.get(..4).unwrap_or("2026"),
        "source": "19a-1",
        "sourceUrl": url,
        "method": method,
        "asOf": as_of,
        "kind": "estimate",
        "establishedHow": how,
        "ownerOverride": false
    })
}

fn live_amplify_roc(symbol: &str, source_url: &str) -> LiveRocFill {
    let mut probes = Vec::new();
    let sym = yahoo_symbol(symbol);
    if sym.is_empty() {
        return LiveRocFill {
            candidates: Vec::new(),
            probes,
        };
    }
    for (as_of, url) in amplify_19a1_urls(&sym) {
        let (got, probe) = http_get_bytes_probed(&url);
        probes.push(probe);
        if let Ok(bytes) = got {
            let text = pdf_ascii(&bytes);
            if let Some((pct, how)) = parse_19a1_notice(&text) {
                return LiveRocFill {
                    candidates: vec![roc_estimate(pct, &url, &as_of, how, "19a-1-current-year")],
                    probes,
                };
            }
        }
    }
    // Stored issuer URL + tax-center / fund page; follow tax / 19a-1 / Form 19a-1 links.
    let mut pages: Vec<String> = Vec::new();
    let stored = source_url.trim();
    if !stored.is_empty() {
        let base = stored.split('#').next().unwrap_or(stored).trim();
        if !base.is_empty() {
            pages.push(base.to_string());
        }
    }
    pages.push("https://amplifyetfs.com/tax-center/".to_string());
    pages.push(format!("https://amplifyetfs.com/{}/", sym.to_ascii_lowercase()));
    pages.sort();
    pages.dedup();

    let sym_u = sym.to_ascii_uppercase();
    let mut pdf_queue: Vec<String> = Vec::new();
    let mut hub_queue: Vec<String> = pages.clone();
    let mut seen_hubs = pages.iter().cloned().collect::<std::collections::HashSet<_>>();

    while let Some(page) = hub_queue.pop() {
        let (got, probe) = http_get_probed(&page);
        probes.push(probe);
        let Ok(html) = got else {
            continue;
        };
        for needle in ["19a-1_Notice_", "19a-1", "form 19a-1", "tax-center", "tax center"] {
            for href in hrefs_matching(&html, needle, "https://amplifyetfs.com") {
                let lower = href.to_ascii_lowercase();
                if lower.contains(".pdf") && href.to_ascii_uppercase().contains(&sym_u) {
                    if !pdf_queue.iter().any(|u| u == &href) {
                        pdf_queue.push(href);
                    }
                } else if (lower.contains("tax-center") || lower.contains("19a-1") || lower.contains("19a1"))
                    && !lower.contains(".pdf")
                    && seen_hubs.insert(href.clone())
                {
                    hub_queue.push(href);
                }
            }
        }
    }

    for href in pdf_queue {
        let (got, probe) = http_get_bytes_probed(&href);
        probes.push(probe);
        let Ok(bytes) = got else {
            continue;
        };
        let text = pdf_ascii(&bytes);
        if let Some((pct, how)) = parse_19a1_notice(&text) {
            let as_of = Utc::now().date_naive().to_string();
            return LiveRocFill {
                candidates: vec![roc_estimate(pct, &href, &as_of, how, "19a-1-current-year")],
                probes,
            };
        }
    }
    LiveRocFill {
        candidates: Vec::new(),
        probes,
    }
}

fn live_neos_roc(symbol: &str) -> LiveRocFill {
    let Some((page_url, html, _)) = fetch_adapter_page("neos", symbol, None) else {
        return LiveRocFill::default();
    };
    let mut probes = vec![http_probe(&page_url, Some(200), None)];
    for href in hrefs_matching(&html, "19a1", "https://neosfunds.com") {
        let (got, probe) = http_get_bytes_probed(&href);
        probes.push(probe);
        if let Ok(bytes) = got {
            let text = pdf_ascii(&bytes);
            if let Some((pct, how)) = parse_19a1_notice(&text) {
                return LiveRocFill {
                    candidates: vec![roc_estimate(
                        pct,
                        &href,
                        &Utc::now().date_naive().to_string(),
                        how,
                        "19a-1-current-year",
                    )],
                    probes,
                };
            }
        }
    }
    LiveRocFill {
        candidates: Vec::new(),
        probes,
    }
}

fn live_yieldmax_roc(symbol: &str) -> LiveRocFill {
    let Some((url, html, _)) = fetch_adapter_page("yieldmax", symbol, None) else {
        return LiveRocFill::default();
    };
    let probes = vec![http_probe(&url, Some(200), None)];
    let cands = parse_yieldmax_distributions(&html);
    for c in &cands {
        if let Some(pct) = c.get("rocPctMinor").and_then(|x| x.as_i64()) {
            let on = c
                .get("paymentPeriod")
                .and_then(|p| p.as_str())
                .unwrap_or("");
            return LiveRocFill {
                candidates: vec![roc_estimate(
                    pct,
                    &url,
                    on,
                    "latest distribution table ROC percent".into(),
                    "table-roc-current",
                )],
                probes,
            };
        }
    }
    LiveRocFill {
        candidates: Vec::new(),
        probes,
    }
}

fn live_roundhill_roc(symbol: &str) -> LiveRocFill {
    let Some((url, html, _)) = fetch_adapter_page("roundhill", symbol, None) else {
        return LiveRocFill::default();
    };
    let probes = vec![http_probe(&url, Some(200), None)];
    if let Some(pct) = parse_roundhill_roc_html(&html) {
        return LiveRocFill {
            candidates: vec![roc_estimate(
                pct,
                &url,
                &Utc::now().date_naive().to_string(),
                "issuer page 19a-1 sentence".into(),
                "19a-1-current-year",
            )],
            probes,
        };
    }
    LiveRocFill {
        candidates: Vec::new(),
        probes,
    }
}

pub fn live_roc_candidates_for(symbol: &str, declaration_source: &str, source_url: &str) -> LiveRocFill {
    let src = declaration_source.trim().to_ascii_lowercase();
    match src.as_str() {
        "neos" => live_neos_roc(symbol),
        "yieldmax" => live_yieldmax_roc(symbol),
        "roundhill" => live_roundhill_roc(symbol),
        "amplify" => live_amplify_roc(symbol, source_url),
        "" => {
            let amplify = live_amplify_roc(symbol, source_url);
            if !amplify.candidates.is_empty() {
                return amplify;
            }
            live_roundhill_roc(symbol)
        }
        _ => {
            // Unknown vendor: still try Amplify-shaped notices when a stored Amplify URL is present.
            if source_url.to_ascii_lowercase().contains("amplifyetfs.com") {
                live_amplify_roc(symbol, source_url)
            } else {
                LiveRocFill::default()
            }
        }
    }
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
    let suggested_provider = suggested_provider_from_name(&name);
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
        "suggestedProvider": suggested_provider,
        "suggestedFrequency": suggest_frequency(&dates),
        "quote": quote,
        "quoteCandidates": quote.clone().map(|q| vec![q]).unwrap_or_default(),
        "candidates": declarations,
        "source": "yahoo"
    }))
}

/// Issuer/sponsor from a retrieved instrument name. Empty means the owner must type it.
fn suggested_provider_from_name(name: &str) -> String {
    let lower = name.trim().to_lowercase();
    if lower.is_empty() {
        return String::new();
    }
    const KNOWN: &[(&str, &str)] = &[
        ("amplify", "Amplify"),
        ("roundhill", "Roundhill"),
        ("neos", "NEOS"),
        ("yieldmax", "YieldMax"),
        ("yield max", "YieldMax"),
        ("guggenheim", "Guggenheim"),
        ("simplify", "Simplify"),
        ("innovator", "Innovator"),
        ("global x", "Global X"),
        ("first trust", "First Trust"),
        ("ft vest", "FT Vest"),
        ("jpmorgan", "JPMorgan"),
        ("jp morgan", "JPMorgan"),
        ("cornerstone", "Cornerstone"),
        ("direxion", "Direxion"),
        ("proshares", "ProShares"),
        ("dividendinvestor", "DividendInvestor"),
        ("saba", "Saba"),
        ("ellington", "Ellington"),
        ("enterprise", "Enterprise Products"),
        ("energy transfer", "Energy Transfer"),
        ("gladstone", "Gladstone"),
        ("mplx", "MPLX LP"),
        ("orchid", "Orchid Island"),
        ("tappalpha", "TappAlpha"),
        ("tapp alpha", "TappAlpha"),
        ("trinity", "Trinity Capital"),
        ("graniteshares", "T-Rex/GraniteShares"),
        ("t-rex", "T-Rex/GraniteShares"),
    ];
    for (needle, label) in KNOWN {
        if lower == *needle
            || lower.starts_with(&format!("{needle} "))
            || lower.starts_with(&format!("{needle}-"))
        {
            return (*label).to_string();
        }
    }
    String::new()
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
        "suggestedProvider": "",
        "suggestedFrequency": "",
        "quote": quote,
        "quoteCandidates": [quote],
        "candidates": [],
        "source": "stooq"
    }))
}

pub(crate) fn empty_snapshot(symbol: &str) -> Value {
    json!({
        "symbol": yahoo_symbol(symbol),
        "name": "",
        "suggestedProvider": "",
        "suggestedFrequency": "",
        "quote": null,
        "quoteCandidates": [],
        "candidates": [],
        "source": "miss"
    })
}

/// Best-effort public quote + last 12 issuer declarations. Never posts.
pub fn live_market_snapshot(symbol: &str) -> Value {
    if uses_offering_price("", symbol) {
        return live_offering_snapshot(symbol, ENERGYX_CIK);
    }
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

/// Last trade only (short chart window). Missing or nonpositive stays unknown.
pub fn live_price_quote(symbol: &str) -> Option<Value> {
    let snap = live_price_snapshot(symbol);
    let quote = snap.get("quote")?;
    if quote.is_null() {
        return None;
    }
    let price = quote.get("priceMinor")?.as_i64().filter(|p| *p > 0)?;
    Some(json!({
        "priceMinor": price,
        "scale": quote.get("scale").and_then(|s| s.as_u64()).unwrap_or(2),
        "asOfAt": quote.get("asOfAt").and_then(|s| s.as_str()).unwrap_or(""),
        "source": quote.get("source").and_then(|s| s.as_str()).unwrap_or("yahoo"),
    }))
}

fn live_price_snapshot(symbol: &str) -> Value {
    let sym = yahoo_symbol(symbol);
    if sym.is_empty() {
        return empty_snapshot(symbol);
    }
    let yahoo = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{sym}?interval=1d&range=5d"
    );
    let yahoo2 = format!(
        "https://query2.finance.yahoo.com/v8/finance/chart/{sym}?interval=1d&range=5d"
    );
    for url in [yahoo, yahoo2] {
        if let Ok(body) = http_get(&url) {
            if let Some(parsed) = parse_yahoo_chart(&sym, &body) {
                if parsed.get("quote").map(|q| !q.is_null()).unwrap_or(false) {
                    return parsed;
                }
            }
        }
    }
    empty_snapshot(&sym)
}

fn last_price_quote_json(security_id: &str, quote: Value) -> Option<Value> {
    let price = quote.get("priceMinor")?.as_i64().filter(|p| *p > 0)?;
    let as_of = quote
        .get("asOfAt")
        .and_then(|s| s.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Utc::now().date_naive().to_string());
    Some(json!({
        "securityId": security_id,
        "priceMinor": price,
        "scale": quote.get("scale").and_then(|s| s.as_u64()).unwrap_or(2),
        "asOfAt": as_of,
        "source": quote.get("source").and_then(|s| s.as_str()).unwrap_or("yahoo"),
    }))
}

/// Best-effort last prices for the daily set. Fail closed: skip misses, never write $0.
pub fn collect_last_price_quotes(pairs: Vec<(String, String)>) -> Vec<Value> {
    collect_last_price_quotes_for(
        pairs
            .into_iter()
            .map(|(security_id, symbol)| LastPriceTarget {
                security_id,
                symbol,
                price_source: String::new(),
                source_symbol: String::new(),
            })
            .collect(),
    )
}

/// Same as collect_last_price_quotes, honoring retrieval template (EDGAR vs Yahoo).
pub fn collect_last_price_quotes_for(targets: Vec<LastPriceTarget>) -> Vec<Value> {
    let mut out = Vec::new();
    for batch in targets.chunks(8) {
        std::thread::scope(|scope| {
            let handles: Vec<_> = batch
                .iter()
                .map(|target| {
                    let target = target.clone();
                    scope.spawn(move || {
                        live_offering_or_yahoo(&target)
                            .and_then(|quote| last_price_quote_json(&target.security_id, quote))
                    })
                })
                .collect();
            for handle in handles {
                if let Ok(Some(quote)) = handle.join() {
                    out.push(quote);
                }
            }
        });
    }
    out
}

/// Dated daily closes from a Yahoo v8 chart. Network-free. Never a CurrentPrice post.
pub fn parse_yahoo_daily_closes(body: &str, start_on: &str, end_on: &str) -> Vec<Value> {
    let v: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let result = match v.pointer("/chart/result/0") {
        Some(r) => r,
        None => return Vec::new(),
    };
    let timestamps = match result.get("timestamp").and_then(|t| t.as_array()) {
        Some(t) => t,
        None => return Vec::new(),
    };
    let closes = match result.pointer("/indicators/quote/0/close").and_then(|c| c.as_array()) {
        Some(c) => c,
        None => return Vec::new(),
    };
    let start = iso_date_prefix(start_on);
    let end = iso_date_prefix(end_on);
    timestamps
        .iter()
        .zip(closes.iter())
        .filter_map(|(ts, close)| {
            let unix = ts.as_i64()?;
            let on = iso_from_unix(unix);
            let px = close.as_f64().filter(|p| *p > 0.0)?;
            if on.as_str() < start.as_str() || on.as_str() > end.as_str() {
                return None;
            }
            Some(json!({
                "asOfAt": on,
                "priceMinor": dollars_to_minor(px, 2),
                "scale": 2,
                "source": "yahoo"
            }))
        })
        .collect()
}

fn iso_date_prefix(raw: &str) -> String {
    let prefix: String = raw.trim().chars().take(10).collect();
    if prefix.chars().count() == 10 {
        prefix
    } else {
        raw.trim().to_string()
    }
}

fn unix_date(on: &str) -> Option<i64> {
    let prefix = iso_date_prefix(on);
    let d = chrono::NaiveDate::parse_from_str(&prefix, "%Y-%m-%d").ok()?;
    let ndt = d.and_hms_opt(0, 0, 0)?;
    Some(Utc.from_utc_datetime(&ndt).timestamp())
}

fn live_period_series(symbol: &str, start_on: &str, end_on: &str) -> Vec<Value> {
    let sym = yahoo_symbol(symbol);
    if sym.is_empty() {
        return Vec::new();
    }
    let start = unix_date(start_on).unwrap_or(0);
    let end = unix_date(end_on).unwrap_or(0).saturating_add(86_400);
    if start <= 0 || end <= start {
        return Vec::new();
    }
    let yahoo = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{sym}?interval=1d&period1={start}&period2={end}"
    );
    let yahoo2 = format!(
        "https://query2.finance.yahoo.com/v8/finance/chart/{sym}?interval=1d&period1={start}&period2={end}"
    );
    for url in [yahoo, yahoo2] {
        if let Ok(body) = http_get(&url) {
            let series = parse_yahoo_daily_closes(&body, start_on, end_on);
            if !series.is_empty() {
                return series;
            }
        }
    }
    Vec::new()
}


/// Registered issuer adapters. Yahoo is never a declaration source.
pub fn is_registered_declaration_source(source: &str) -> bool {
    financial_domain::div1::is_registered_declaration_source(source)
}

pub fn registered_declaration_sources() -> &'static [&'static str] {
    financial_domain::div1::REGISTERED_DECLARATION_SOURCES
}

fn adapter_label(source: &str) -> &'static str {
    financial_domain::div1::source_label(source)
}

fn adapter_is_fund_page(source: &str, html: &str, symbol: &str) -> bool {
    let low = html.to_ascii_lowercase();
    if low.contains("payout date") && low.contains("cash amount") {
        return adapters::html_names_symbol(html, symbol)
            || low.contains(&symbol.trim().to_ascii_lowercase());
    }
    match source.trim().to_ascii_lowercase().as_str() {
        "roundhill" => roundhill_fund_page(html, symbol),
        "amplify" => amplify_fund_page(html, symbol),
        "neos" => neos_fund_page(html, symbol),
        "yieldmax" => yieldmax_fund_page(html, symbol),
        "fidelity" | "schwab" => adapters::moneymarket_fund_page(source, html, symbol),
        "saba" => adapters::saba_fund_page(html, symbol),
        "dividendinvestor" => adapters::dividendinvestor_fund_page(html, symbol),
        other => adapters::div1_fund_page(other, html, symbol),
    }
}

fn adapter_probe_urls(source: &str, symbol: &str) -> Vec<String> {
    let sym = symbol.trim().to_ascii_lowercase();
    if sym.is_empty() {
        return Vec::new();
    }
    let mut urls = match source.trim().to_ascii_lowercase().as_str() {
        "roundhill" => vec![
            format!("https://www.roundhillinvestments.com/etf/{sym}"),
            format!("https://www.roundhillinvestments.com/etfs/{sym}"),
        ],
        "amplify" => vec![
            format!("https://amplifyetfs.com/{sym}/"),
            format!("https://amplifyetfs.com/{sym}"),
        ],
        "neos" => vec![
            format!("https://neosfunds.com/{sym}/"),
            format!("https://neosfunds.com/{sym}"),
        ],
        "yieldmax" => vec![
            format!("https://www.yieldmaxetfs.com/our-etfs/{sym}/"),
            format!("https://www.yieldmaxetfs.com/{sym}/"),
        ],
        "fidelity" | "schwab" => adapters::moneymarket_probe_urls(source, symbol),
        other => adapters::div1_probe_urls(other, symbol),
    };
    let dh = adapters::dividendhistory_url(symbol);
    if !urls.iter().any(|u| u == &dh) {
        urls.push(dh);
    }
    urls
}

fn live_proshares_distribution_body(symbol: &str) -> Option<String> {
    let sym = symbol.trim().to_ascii_uppercase();
    if sym.is_empty() {
        return None;
    }
    let year = Utc::now().year();
    let mut all = Vec::new();
    for y in [year, year - 1, year - 2] {
        let url = format!("https://www.proshares.com/api/distributionsummary/?fund={sym}&year={y}");
        let Ok(body) = http_get(&url) else {
            continue;
        };
        let Ok(Value::Array(rows)) = serde_json::from_str::<Value>(&body) else {
            continue;
        };
        all.extend(rows);
    }
    if all.is_empty() {
        return None;
    }
    Some(Value::Array(all).to_string())
}

/// Roundhill fills distribution tables via JS; GET HTML has empty tbody.
/// Flow mirrors site app.js: warm fund page → token from server.php → POST distribution-call.php.
fn live_roundhill_distribution_body(symbol: &str) -> Option<String> {
    let ticker = adapters::roundhill_api_ticker(symbol);
    if ticker.is_empty() {
        return None;
    }
    let page = format!(
        "https://www.roundhillinvestments.com/etf/{}",
        ticker.to_ascii_lowercase()
    );
    let _ = http_get(&page);
    let token = http_get_referer(
        "https://www.roundhillinvestments.com/assets/php/server.php",
        &page,
    )
    .ok()?
    .trim()
    .to_string();
    if token.is_empty() {
        return None;
    }
    let lower = format!("{}/", ticker.to_ascii_lowercase());
    let body = http_post_form_origin(
        "https://www.roundhillinvestments.com/assets/php/distribution-call.php",
        &page,
        "https://www.roundhillinvestments.com",
        &[
            ("upperetf", ticker.as_str()),
            ("loweretf", lower.as_str()),
            ("token", token.as_str()),
            ("is_ajax", "1"),
        ],
    )
    .ok()?;
    if adapters::parse_roundhill_distribution_api(&body).is_empty() {
        return None;
    }
    Some(body)
}

/// Saba fund page embeds all years in Nuxt `__NUXT_DATA__` (year tabs are client-side).
fn live_saba_distribution_body(symbol: &str) -> Option<String> {
    let url = adapters::saba_fund_url(symbol);
    let html = http_get(&url).ok()?;
    if adapters::parse_saba_nuxt_distributions("saba", &html).is_empty() {
        return None;
    }
    Some(html)
}

/// Simplify fund page links to `/etfs/{node_id}/distributions` (Drupal AJAX full history).
fn live_simplify_distribution_body(symbol: &str) -> Option<(String, String, Option<String>)> {
    let fund_url = adapters::simplify_fund_url(symbol);
    let fund_html = http_get(&fund_url).ok()?;
    let calendar_url =
        crate::retrieve::html::distribution_calendar_url(&fund_html, "https://www.simplify.us");
    let node_id = adapters::extract_simplify_node_id(&fund_html)?;
    let dist_url = adapters::simplify_distributions_url(&node_id);
    let body = http_get(&dist_url).ok()?;
    if parse_simplify_distributions("simplify", &body).is_empty() {
        return None;
    }
    Some((dist_url, body, calendar_url))
}

fn site_origin_from_url(url: &str) -> String {
    let url = url.trim();
    let Some(scheme) = url.find("://") else {
        return String::new();
    };
    let rest = &url[scheme + 3..];
    match rest.find('/') {
        Some(0) => url[..scheme + 3].trim_end_matches('/').to_string(),
        Some(slash) => url[..scheme + 3 + slash].to_string(),
        None => url.to_string(),
    }
}

/// DividendInvestor fallback when the issuer page is empty or unreachable.
fn live_dividendinvestor_body(symbol: &str) -> Option<String> {
    let sym = symbol.trim().to_ascii_uppercase();
    if sym.is_empty() {
        return None;
    }
    let url = adapters::DIVIDENDINVESTOR_HISTORY_URL;
    let body = format!("newSymbol={sym}&btnAdd=btnAdd&history_show=1&ord=div_yield&da=1");
    let html = http_post_form_body(url, url, "https://www.dividendinvestor.com", &body).ok()?;
    let parsed = adapters::parse_dividendinvestor_distributions("dividendinvestor", &html);
    if parsed.is_empty() {
        return None;
    }
    Some(html)
}

fn live_nasdaq_dividends_body(symbol: &str) -> Option<String> {
    let url = adapters::nasdaq_dividends_url(symbol);
    let body = http_get(&url).ok()?;
    if adapters::parse_nasdaq_dividends("nasdaq", &body).is_empty() {
        return None;
    }
    Some(body)
}

/// Prefer Nasdaq JSON for ETF issuers where the API reliably has rows. BDCs / mREITs
/// (EFC, ORC, CRF, …) use issuer HTML or dividendhistory — Nasdaq is often empty and slow.
fn prefers_nasdaq_first(source: &str) -> bool {
    matches!(
        source.trim().to_ascii_lowercase().as_str(),
        "jpmorgan" | "tappalpha" | "globalx" | "direxion" | "trex"
    )
}

fn pack_adapter_fetch(url: String, html: String, calendar_html: Option<&str>) -> (String, String, Option<String>) {
    let cal_src = calendar_html.unwrap_or(html.as_str());
    let calendar = crate::retrieve::html::distribution_calendar_url(
        cal_src,
        &site_origin_from_url(&url),
    );
    (url, html, calendar)
}

fn follow_distribution_press_links(
    list_html: &str,
    fund_page_url: &str,
    source: &str,
    symbol: &str,
) -> Option<(String, String, Option<String>)> {
    let calendar = crate::retrieve::html::distribution_calendar_url(
        list_html,
        &site_origin_from_url(fund_page_url),
    );
    let hrefs = adapters::hrefs_matching(list_html, "distribution", "");
    let more = adapters::hrefs_matching(list_html, "dividend", "");
    let mut seen = std::collections::HashSet::new();
    for href in hrefs.into_iter().chain(more) {
        let url = if href.starts_with("http") {
            href
        } else if href.starts_with('/') {
            // absolute path without host — skip unless we know host
            continue;
        } else {
            continue;
        };
        if !seen.insert(url.clone()) {
            continue;
        }
        let Ok(html) = http_get(&url) else {
            continue;
        };
        let cands = parse_vendor_distributions_with_csv(source, &html);
        if !cands.is_empty() {
            return Some((url, html, calendar));
        }
        // Cornerstone press often embeds a declaration table without needing fund-page needles.
        if !parse_generic_distributions_pub(source, &html).is_empty() {
            return Some((url, html, calendar));
        }
        let _ = symbol;
    }
    None
}

fn parse_generic_distributions_pub(source: &str, html: &str) -> Vec<Value> {
    adapters::parse_generic_distributions(source, html)
}

fn declaration_candidates_useful(cands: &[Value], as_of: &str) -> bool {
    cands
        .iter()
        .any(|c| candidate_amount(c).is_some())
        || !upcoming_from_candidates(cands, as_of).is_empty()
}

fn fetch_adapter_page(source: &str, symbol: &str, source_url: Option<&str>) -> Option<(String, String, Option<String>)> {
    let src = source.trim().to_ascii_lowercase();
    if src == "proshares" {
        if let Some(body) = live_proshares_distribution_body(symbol) {
            let url = format!(
                "https://www.proshares.com/api/distributionsummary/?fund={}",
                symbol.trim().to_ascii_uppercase()
            );
            return Some(pack_adapter_fetch(url, body, None));
        }
    }
    if src == "roundhill" {
        if let Some(body) = live_roundhill_distribution_body(symbol) {
            return Some(pack_adapter_fetch(
                "https://www.roundhillinvestments.com/assets/php/distribution-call.php".into(),
                body,
                None,
            ));
        }
        if let Some(body) = live_nasdaq_dividends_body(symbol) {
            return Some(pack_adapter_fetch(adapters::nasdaq_dividends_url(symbol), body, None));
        }
    }
    if src == "dividendinvestor" {
        if let Some(body) = live_dividendinvestor_body(symbol) {
            return Some(pack_adapter_fetch(
                format!(
                    "{url}?symbol={sym}",
                    url = adapters::DIVIDENDINVESTOR_HISTORY_URL,
                    sym = symbol.trim().to_ascii_uppercase()
                ),
                body,
                None,
            ));
        }
    }
    if src == "saba" {
        if let Some(body) = live_saba_distribution_body(symbol) {
            let url = adapters::saba_fund_url(symbol);
            return Some(pack_adapter_fetch(url, body.clone(), Some(&body)));
        }
        if let Some(body) = live_dividendinvestor_body(symbol) {
            return Some(pack_adapter_fetch(
                format!(
                    "{}?symbol={}",
                    adapters::DIVIDENDINVESTOR_HISTORY_URL,
                    symbol.trim().to_ascii_uppercase()
                ),
                body,
                None,
            ));
        }
    }
    if src == "simplify" {
        if let Some(followed) = live_simplify_distribution_body(symbol) {
            return Some(followed);
        }
    }
    if prefers_nasdaq_first(&src) {
        if let Some(body) = live_nasdaq_dividends_body(symbol) {
            return Some(pack_adapter_fetch(adapters::nasdaq_dividends_url(symbol), body, None));
        }
    }
    let mut urls = Vec::new();
    if let Some(stored) = source_url.map(str::trim).filter(|s| !s.is_empty()) {
        urls.push(stored.to_string());
    }
    urls.extend(adapter_probe_urls(source, symbol));
    let mut seen = std::collections::HashSet::new();
    let as_of = Utc::now().date_naive().to_string();
    for url in urls {
        if !seen.insert(url.clone()) {
            continue;
        }
        if let Ok(html) = http_get(&url) {
            let cands = parse_vendor_distributions_with_csv(source, &html);
            if declaration_candidates_useful(&cands, &as_of) {
                return Some(pack_adapter_fetch(url, html, None));
            }
            if adapter_is_fund_page(source, &html, symbol) {
                if let Some(followed) =
                    follow_distribution_press_links(&html, &url, source, symbol)
                {
                    return Some(followed);
                }
                // JS-rendered fund pages (Roundhill) may be empty on GET — try fallbacks.
                continue;
            }
        }
    }
    if !prefers_nasdaq_first(&src) {
        if let Some(body) = live_nasdaq_dividends_body(symbol) {
            return Some(pack_adapter_fetch(adapters::nasdaq_dividends_url(symbol), body, None));
        }
    }
    None
}

fn live_vendor_page(source: &str, symbol: &str, source_url: Option<&str>) -> Option<String> {
    fetch_adapter_page(source, symbol, source_url).map(|(_, html, _)| html)
}

fn live_vendor_declarations(source: &str, symbol: &str, source_url: Option<&str>) -> Vec<Value> {
    let Some(html) = live_vendor_page(source, symbol, source_url) else {
        return Vec::new();
    };
    let mut cands = parse_vendor_distributions_with_csv(source, &html);
    let src = source.trim().to_ascii_lowercase();
    for cand in &mut cands {
        cand["source"] = json!(src.as_str());
    }
    cands
}

fn suggest_calendar_policy(upcoming_count: usize, fund_page: bool) -> String {
    if upcoming_count >= 1 && fund_page {
        "issuer_calendar".into()
    } else if fund_page {
        "issuer_calendar".into()
    } else {
        String::new()
    }
}

fn extract_underlying(html: &str, symbol: &str) -> String {
    let text = strip_html(html);
    let upper = text.to_ascii_uppercase();
    let sym = symbol.trim().to_ascii_uppercase();
    for needle in [
        "UNDERLYING ETF ",
        "UNDERLYING INDEX ",
        "UNDERLYING SECURITY ",
        "UNDERLYING TICKER ",
        "UNDERLYING: ",
        "TRACKS THE ",
        "TRACKS ",
    ] {
        if let Some(idx) = upper.find(needle) {
            let after = text.get(idx + needle.len()..).unwrap_or("").trim();
            let token = after
                .split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
                .next()
                .unwrap_or("");
            let t = token.trim().to_ascii_uppercase();
            if (2..=8).contains(&t.len())
                && t != sym
                && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            {
                return t.replace('-', "");
            }
        }
    }
    if upper.contains("HACK")
        && sym != "HACK"
        && (upper.contains("CYBER") || upper.contains("COVERED CALL"))
    {
        return "HACK".into();
    }
    String::new()
}

fn source_analytics(html: &str, fund_page: bool, table_rows: usize) -> Value {
    let lower = html.to_ascii_lowercase();
    let js_likely = table_rows == 0
        && fund_page
        && (lower.contains("calhisdistri")
            || lower.contains("__next_data__")
            || (lower.contains("distribution") && lower.contains("<script")));
    json!({
        "htmlReturned": !html.trim().is_empty(),
        "fundPage": fund_page,
        "tableOnGet": table_rows > 0,
        "tableRowCount": table_rows,
        "jsLikely": js_likely,
    })
}

fn future_declaration_strategy(vendor: &str, analytics: &Value) -> String {
    let table_on_get = analytics
        .get("tableOnGet")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let js = analytics
        .get("jsLikely")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let fund = analytics
        .get("fundPage")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if table_on_get {
        format!(
            "GET {vendor} distribution table on each refresh. Take last 12 observations for Plan-review. Pay dates on or after as-of become the remaining-year calendar. Never Yahoo dividends."
        )
    } else if js {
        format!(
            "{vendor} fund page found; distribution grid empty on GET (often filled by JavaScript). Keep declarationSource={vendor}. Do not fall back to Yahoo. Owner paste until a JS adapter exists. Future pay dates stay unknown until the table is readable."
        )
    } else if fund {
        format!(
            "{vendor} fund page found with no distribution table on GET. Keep declarationSource={vendor}. Owner paste for history. Do not use Yahoo dividends."
        )
    } else {
        "No issuer fund page. Declaration source stays unassigned. Last price may still use Yahoo after you confirm a source.".into()
    }
}

fn title_from_html(html: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let Some(start_rel) = lower.find("<title") else {
        return String::new();
    };
    let after = html.get(start_rel..).unwrap_or("");
    let Some(gt) = after.find('>') else {
        return String::new();
    };
    let rest = after.get(gt + 1..).unwrap_or("");
    let end_rel = rest.to_ascii_lowercase().find("</title>").unwrap_or(rest.len().min(200));
    let raw = strip_html(rest.get(..end_rel).unwrap_or("")).trim().to_string();
    raw.split('|')
        .next()
        .unwrap_or(&raw)
        .trim()
        .to_string()
}


fn attempt(vendor: &str, url: &str, found: bool, note: &str) -> Value {
    json!({
        "vendor": vendor,
        "url": url,
        "found": found,
        "note": note
    })
}

/// Choose issuer site before any Yahoo declaration retrieve. Yahoo is last price only.
pub fn profile_from_vendor_htmls(
    symbol: &str,
    probes: &[(&str, &str, &str)],
    as_of: &str,
) -> Value {
    let mut attempts = Vec::new();
    let mut chosen: Option<(&str, &str, &str)> = None;
    for (vendor, url, html) in probes {
        let found = adapter_is_fund_page(vendor, html, symbol);
        let note = if html.trim().is_empty() {
            "no HTML returned"
        } else if page_is_not_found(html) {
            "issuer page not found"
        } else if !found {
            "page did not identify this ticker as a listed fund"
        } else {
            "fund page found"
        };
        attempts.push(attempt(vendor, url, found, note));
        if found && chosen.is_none() {
            chosen = Some((vendor, url, html));
        }
    }
    if let Some((vendor, url, html)) = chosen {
        let cands = parse_vendor_distributions_with_csv(vendor, html);
        let upcoming = upcoming_from_candidates(&cands, as_of);
        let paid_n = cands.iter().filter(|c| candidate_amount(c).is_some()).count();
        let analytics = source_analytics(html, true, paid_n);
        let strategy = future_declaration_strategy(vendor, &analytics);
        let table_note = if paid_n == 0 {
            "Fund page found; distribution table was empty on GET. Amounts stay unknown — not Yahoo, not $0."
        } else {
            ""
        };
        let provider = adapter_label(vendor);
        let name = title_from_html(html);
        let underlying = extract_underlying(html, symbol);
        let lookthrough = extract_lookthrough(html, symbol, &underlying);
        let calendar = suggest_calendar_policy(upcoming.len(), true);
        return json!({
            "symbol": yahoo_symbol(symbol),
            "name": name,
            "suggestedProvider": if provider.is_empty() { vendor } else { provider },
            "underlying": underlying,
            "lookthrough": lookthrough,
            "declarationSource": vendor,
            "priceSource": "public",
            "paymentSource": "import",
            "lookbackCount": 12,
            "sourceUrl": url,
            "source": vendor,
            "candidates": cands,
            "upcomingPays": upcoming,
            "suggestedCalendarPolicy": calendar,
            "researchAttempts": attempts,
            "sourceAnalytics": analytics,
            "futureDeclarationStrategy": strategy,
            "missExplanation": table_note,
            "suggestedFrequency": suggest_frequency(
                &cands.iter().filter_map(|c| c.get("paymentPeriod").and_then(|p| p.as_str()).map(|s| s.to_string())).collect::<Vec<_>>()
            )
        });
    }
    let empty_analytics = json!({
        "htmlReturned": false,
        "fundPage": false,
        "tableOnGet": false,
        "tableRowCount": 0,
        "jsLikely": false,
    });
    json!({
        "symbol": yahoo_symbol(symbol),
        "name": "",
        "suggestedProvider": "",
        "underlying": "",
        "lookthrough": empty_lookthrough(),
        "declarationSource": "",
        "priceSource": "public",
            "paymentSource": "import",
        "lookbackCount": 12,
        "sourceUrl": "",
        "source": "miss",
        "candidates": [],
        "upcomingPays": [],
        "suggestedCalendarPolicy": "",
        "researchAttempts": attempts,
        "sourceAnalytics": empty_analytics,
        "futureDeclarationStrategy": future_declaration_strategy("", &empty_analytics),
        "missExplanation": "No issuer fund page matched. Yahoo was not used for declarations or provider. Last price may still use Yahoo after you confirm a source.",
        "suggestedFrequency": ""
    })
}

/// Probe only the hinted registered issuer. Do not spray every vendor site.
pub fn live_research_identity(symbol: &str, source_url: &str, declaration_source: &str) -> Value {
    let hint = {
        let label = financial_domain::div1::source_label(declaration_source);
        if !label.is_empty() {
            label.to_string()
        } else if let Some(src) = financial_domain::div1::declaration_source_from_url(source_url) {
            financial_domain::div1::source_label(src).to_string()
        } else if is_registered_declaration_source(declaration_source) {
            declaration_source.to_string()
        } else {
            String::new()
        }
    };
    live_research_profile(symbol, &hint)
}

/// Probe only the hinted registered issuer. Do not spray every vendor site.
fn live_research_profile(symbol: &str, provider_hint: &str) -> Value {
    let as_of = Utc::now().date_naive().to_string();
    let source = financial_domain::div1::declaration_source_for_provider(provider_hint)
        .map(|s| s.to_string())
        .or_else(|| {
            let raw = provider_hint.trim().to_ascii_lowercase();
            if is_registered_declaration_source(&raw) {
                Some(raw)
            } else {
                None
            }
        });
    let Some(source) = source else {
        return json!({
            "symbol": yahoo_symbol(symbol),
            "name": "",
            "suggestedProvider": "",
            "underlying": "",
            "declarationSource": "",
            "priceSource": "public",
            "paymentSource": "import",
            "lookbackCount": 12,
            "sourceUrl": "",
            "source": "miss",
            "candidates": [],
            "upcomingPays": [],
            "suggestedCalendarPolicy": "",
            "researchAttempts": [],
            "sourceAnalytics": {
                "htmlReturned": false,
                "fundPage": false,
                "tableOnGet": false,
                "tableRowCount": 0,
                "jsLikely": false,
            },
            "futureDeclarationStrategy": "No issuer hint. Choose a registered declaration source. Yahoo was not used for declarations.",
            "missExplanation": "No issuer fund page probed. Yahoo was not used for declarations or provider.",
            "suggestedFrequency": ""
        });
    };
    let mut probes: Vec<(String, String, String)> = Vec::new();
    for url in adapter_probe_urls(&source, symbol) {
        let html = http_get(&url).unwrap_or_default();
        probes.push((source.clone(), url, html));
        if adapter_is_fund_page(
            &source,
            probes.last().map(|p| p.2.as_str()).unwrap_or(""),
            symbol,
        ) {
            break;
        }
    }
    let refs: Vec<(&str, &str, &str)> = probes
        .iter()
        .map(|(v, u, h)| (v.as_str(), u.as_str(), h.as_str()))
        .collect();
    profile_from_vendor_htmls(symbol, &refs, &as_of)
}

#[derive(Clone, Default)]
pub struct DeclarationTarget {
    pub security_id: String,
    pub symbol: String,
    pub declaration_source: String,
    pub source_symbol: String,
    pub source_url: String,
    pub last_content_hash: String,
    pub div_type: String,
    /// When false and last run was ok today with a hash, skip network fetch.
    pub force_refresh: bool,
    pub last_run_ok: bool,
    pub last_run_at: String,
    /// ISO distribution inception (optional). Consulted only when paid count is under 12.
    pub inception_on: String,
    /// Cadence label or period count (Weekly/52, Monthly/12, Quarterly/4).
    pub payment_frequency: String,
    /// Paid declarations already stored before this collect.
    pub paid_count: u8,
    /// Payment periods already in DB — skip re-posting on incremental refresh.
    pub known_payment_periods: Vec<String>,
}

#[derive(Clone, Default)]
pub struct DeclarationCollectOutcome {
    pub candidates: Vec<Value>,
    pub pay_dates: Vec<Value>,
    pub misses: Vec<Value>,
    pub unchanged: Vec<Value>,
    /// Winning issuer URL from the last fetch in this batch.
    pub fetched_source_url: String,
    /// Issuer distribution calendar PDF/HTML when linked from the fund page.
    pub fetched_payment_calendar_url: String,
}

fn collector_run_is_fresh_today(target: &DeclarationTarget) -> bool {
    if target.force_refresh {
        return false;
    }
    if !target.last_run_ok || target.last_content_hash.is_empty() {
        return false;
    }
    let as_of = Utc::now().date_naive().to_string();
    use financial_domain::declaration_lookback::{
        validate_paid_lookback, LookbackValidation,
    };
    if !matches!(
        validate_paid_lookback(
            target.paid_count,
            &target.inception_on,
            &as_of,
            &target.payment_frequency,
        ),
        LookbackValidation::Complete | LookbackValidation::CompleteViaInception { .. }
    ) {
        return false;
    }
    let today = as_of;
    target.last_run_at.starts_with(today.as_str())
}

const MISS_EMPTY: &str = "Issuer page empty.";
const MISS_CHANGED: &str = "Issuer page changed; unparseable.";

/// Documented short-history names are obsolete — use retrieval_template.inception_on
/// so expected lookback is derived from cadence × age (capped at 12).

fn fnv1a_hex(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

pub fn page_content_hash(body: &str) -> String {
    fnv1a_hex(body.as_bytes())
}

fn miss_reason(previous_hash: &str, new_hash: &str) -> &'static str {
    if !previous_hash.is_empty() && previous_hash != new_hash {
        MISS_CHANGED
    } else {
        MISS_EMPTY
    }
}

fn attach_hash(row: &mut Value, hash: &str) {
    row["contentHash"] = json!(hash);
}

/// Parse one already-fetched issuer page. Used by collect and fixture tests (no HTTP).
pub fn collect_from_fetched_page(
    target: &DeclarationTarget,
    source: &str,
    html: Option<&str>,
) -> DeclarationCollectOutcome {
    let mut out = DeclarationCollectOutcome::default();
    apply_fetched_page(&mut out, target, source, html, "", None);
    out
}

fn apply_fetched_page(
    out: &mut DeclarationCollectOutcome,
    target: &DeclarationTarget,
    source: &str,
    html: Option<&str>,
    fetched_url: &str,
    payment_calendar_url: Option<&str>,
) {
    let as_of = Utc::now().date_naive().to_string();
    let Some(html) = html else {
        out.misses.push(json!({
            "securityId": target.security_id,
            "symbol": target.symbol,
            "declarationSource": source,
            "reason": MISS_EMPTY,
            "code": "declaration_retrieve_miss",
        }));
        return;
    };
    let hash = page_content_hash(html);
    if !target.last_content_hash.is_empty() && target.last_content_hash == hash {
        out.unchanged.push(json!({
            "securityId": target.security_id,
            "symbol": target.symbol,
            "contentHash": hash,
        }));
        return;
    }
    let mut cands = parse_vendor_distributions_with_csv(source, html);
    let known: std::collections::HashSet<String> = target
        .known_payment_periods
        .iter()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();
    for cand in &mut cands {
        cand["source"] = json!(source);
        attach_hash(cand, &hash);
    }
    let upcoming = upcoming_from_candidates(&cands, &as_of);
    let paid_all: Vec<Value> = cands
        .iter()
        .filter(|c| candidate_amount(c).is_some())
        .cloned()
        .collect();
    let paid_store: Vec<Value> = if known.is_empty() {
        paid_all.clone()
    } else {
        paid_all
            .iter()
            .filter(|c| {
                c.get("paymentPeriod")
                    .and_then(|p| p.as_str())
                    .map(|p| !known.contains(p))
                    .unwrap_or(true)
            })
            .cloned()
            .collect()
    };
    if paid_store.is_empty() && upcoming.is_empty() {
        out.misses.push(json!({
            "securityId": target.security_id,
            "symbol": target.symbol,
            "declarationSource": source,
            "reason": miss_reason(&target.last_content_hash, &hash),
            "contentHash": hash,
            "code": "declaration_retrieve_miss",
        }));
        return;
    }
    for pay in upcoming {
        let mut row = pay;
        row["securityId"] = json!(target.security_id);
        attach_hash(&mut row, &hash);
        if row.get("source").and_then(|s| s.as_str()).unwrap_or("").is_empty() {
            row["source"] = json!(source);
        }
        if let Some(url) = payment_calendar_url.map(str::trim).filter(|u| !u.is_empty()) {
            row["paymentCalendarUrl"] = json!(url);
        }
        out.pay_dates.push(row);
    }
    if !fetched_url.is_empty() {
        out.fetched_source_url = fetched_url.to_string();
    }
    if let Some(url) = payment_calendar_url.map(str::trim).filter(|u| !u.is_empty()) {
        out.fetched_payment_calendar_url = url.to_string();
    }
    let paid_count = if known.is_empty() {
        paid_all.len() as u8
    } else {
        target
            .paid_count
            .saturating_add(paid_store.len() as u8)
    };
    for mut cand in paid_store {
        cand["securityId"] = json!(target.security_id);
        attach_hash(&mut cand, &hash);
        if cand.get("source").and_then(|s| s.as_str()).unwrap_or("").is_empty() {
            cand["source"] = json!(source);
        }
        if !fetched_url.is_empty() {
            cand["fetchedSourceUrl"] = json!(fetched_url);
        }
        out.candidates.push(cand);
    }
    // Retrieve first; only when under 12 paid does optional inception decide completeness.
    use financial_domain::declaration_lookback::{
        validate_paid_lookback, LookbackValidation, DECLARATION_LOOKBACK_TARGET,
    };
    match validate_paid_lookback(
        paid_count as u8,
        &target.inception_on,
        &as_of,
        &target.payment_frequency,
    ) {
        LookbackValidation::Complete | LookbackValidation::CompleteViaInception { .. } => {}
        LookbackValidation::ShortWithoutInception { paid } => {
            out.misses.push(json!({
                "securityId": target.security_id,
                "symbol": target.symbol,
                "declarationSource": source,
                "reason": format!(
                    "Adapter returned {paid} of {DECLARATION_LOOKBACK_TARGET} required paid declarations. Set optional inception on Settings only if this name is too new for a full lookback."
                ),
                "contentHash": hash,
                "code": "declaration_lookback_short",
                "paidCount": paid,
                "requiredPaid": DECLARATION_LOOKBACK_TARGET,
                "inceptionOn": "",
                "inceptionApplied": false,
            }));
        }
        LookbackValidation::ShortWithInception { paid, expected } => {
            out.misses.push(json!({
                "securityId": target.security_id,
                "symbol": target.symbol,
                "declarationSource": source,
                "reason": format!(
                    "Adapter returned {paid} of {expected} paid declarations expected since inception {}.",
                    target.inception_on.trim()
                ),
                "contentHash": hash,
                "code": "declaration_lookback_short",
                "paidCount": paid,
                "requiredPaid": expected,
                "inceptionOn": target.inception_on,
                "inceptionApplied": true,
            }));
        }
    }
}

/// Vendor-templated declaration candidates. Miss stays empty — never Yahoo dividends.
pub fn collect_declaration_candidates_for(targets: Vec<DeclarationTarget>) -> Vec<Value> {
    collect_declarations_for(targets).candidates
}

pub fn collect_declarations_for(targets: Vec<DeclarationTarget>) -> DeclarationCollectOutcome {
    let mut out = DeclarationCollectOutcome::default();
    for target in targets {
        let source = target.declaration_source.trim().to_ascii_lowercase();
        let symbol = if target.source_symbol.trim().is_empty() {
            target.symbol.clone()
        } else {
            target.source_symbol.clone()
        };
        if !is_registered_declaration_source(&source) {
            if financial_domain::div1::div1_adapter_missing(&target.div_type, &source) {
                out.misses.push(json!({
                    "securityId": target.security_id,
                    "symbol": target.symbol,
                    "declarationSource": target.declaration_source,
                    "reason": "DIV-1 has no issuer adapter assigned.",
                    "code": "div1_adapter_missing",
                }));
                continue;
            }
            let reason = if source.is_empty()
                || source == "unassigned"
                || source == "public"
                || source == "import"
                || source == "sec-edgar"
            {
                continue;
            } else {
                format!("no declaration adapter for {source}")
            };
            out.misses.push(json!({
                "securityId": target.security_id,
                "symbol": target.symbol,
                "declarationSource": target.declaration_source,
                "reason": reason,
                "code": "no_declaration_adapter",
            }));
            continue;
        }
        if collector_run_is_fresh_today(&target) {
            out.unchanged.push(json!({
                "securityId": target.security_id,
                "symbol": target.symbol,
                "contentHash": target.last_content_hash,
            }));
            continue;
        }
        match fetch_adapter_page(&source, &symbol, Some(target.source_url.as_str())) {
            Some((url, html, calendar_url)) => {
                apply_fetched_page(
                    &mut out,
                    &target,
                    &source,
                    Some(html.as_str()),
                    &url,
                    calendar_url.as_deref(),
                );
            }
            None => apply_fetched_page(&mut out, &target, &source, None, "", None),
        }
    }
    out
}

/// Attach Yahoo/par quote to a collector payload without re-fetching declarations.
pub fn enrich_collector_quote_only(body: &mut Value) {
    let skip_quote = body
        .get("unchanged")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if skip_quote {
        return;
    }
    let symbol = body
        .get("symbol")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    if symbol.is_empty() {
        return;
    }
    let div_type = body
        .get("divType")
        .and_then(|s| s.as_str())
        .unwrap_or("");
    let sym_upper = symbol.to_ascii_uppercase();
    if div_type.eq_ignore_ascii_case("CASH")
        || matches!(sym_upper.as_str(), "SPAXX" | "FDRXX" | "SWVXX")
    {
        let today = Utc::now().date_naive().to_string();
        body["quote"] = json!({
            "priceMinor": 100,
            "scale": 2,
            "source": "par",
            "asOfAt": today,
        });
        return;
    }
    if let Some(quote) = live_price_quote(&symbol) {
        body["quote"] = quote;
    }
}

fn array_injected(body: &Value, key: &str) -> bool {
    body.get(key)
        .and_then(|c| c.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false)
}

fn known_payment_periods_from_body(body: &Value) -> std::collections::HashSet<String> {
    body.get("knownPaymentPeriods")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|p| p.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn filter_new_declaration_candidates(
    cands: Vec<Value>,
    known: &std::collections::HashSet<String>,
) -> Vec<Value> {
    if known.is_empty() {
        return cands;
    }
    cands
        .into_iter()
        .filter(|c| {
            c.get("paymentPeriod")
                .and_then(|p| p.as_str())
                .map(|p| !known.contains(p))
                .unwrap_or(true)
        })
        .collect()
}

/// Fill empty candidate lists on retrieve commands. Injected non-empty candidates win (CI).
pub fn enrich_retrieve_body(command_name: &str, body: &mut Value) {
    if command_name == "RocResearchRetrieve" {
        // Missing or empty candidates → live fill. Non-empty arrays stay injected.
        if array_injected(body, "candidates") {
            return;
        }
        let symbol = body
            .get("symbol")
            .or_else(|| body.get("sourceSymbol"))
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();
        let declaration_source = body
            .get("declarationSource")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();
        let source_url = body
            .get("sourceUrl")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();
        let fill = live_roc_candidates_for(&symbol, &declaration_source, &source_url);
        body["candidates"] = json!(fill.candidates);
        body["rocProbes"] = json!(fill.probes);
        return;
    }
    if command_name == "PeriodSeriesRetrieve" {
        let start = body
            .get("startOn")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();
        let end = body
            .get("endOn")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();
        let symbol = body
            .get("symbol")
            .or_else(|| body.get("sourceSymbol"))
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();
        if !array_injected(body, "candidates") && !symbol.is_empty() {
            body["candidates"] = json!(live_period_series(&symbol, &start, &end));
        }
        let bench = body
            .get("benchmarkSymbol")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();
        if !array_injected(body, "benchmarkCandidates") && !bench.is_empty() {
            body["benchmarkCandidates"] = json!(live_period_series(&bench, &start, &end));
        }
        return;
    }
    if array_injected(body, "candidates") {
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
    let price_source = body
        .get("priceSource")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    let source_symbol = body
        .get("sourceSymbol")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    let declaration_source = body
        .get("declarationSource")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    let research_only = body
        .get("researchOnly")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let research_injected = body
        .get("researchAttempts")
        .and_then(|a| a.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false);
    if declaration_source.is_empty() && !research_injected {
        let name = body
            .get("name")
            .and_then(|s| s.as_str())
            .unwrap_or("");
        let hint = body
            .get("suggestedProvider")
            .or_else(|| body.get("provider"))
            .and_then(|s| s.as_str())
            .unwrap_or("");
        let hint = if hint.is_empty() {
            suggested_provider_from_name(name)
        } else {
            hint.to_string()
        };
        let profile = live_research_profile(&symbol, &hint);
        if let Some(obj) = profile.as_object() {
            for (k, v) in obj {
                if k == "quote" || k == "quoteCandidates" {
                    continue;
                }
                body[k] = v.clone();
            }
        }
    }
    if research_only {
        body["quote"] = json!(null);
        body["quoteCandidates"] = json!([]);
        return;
    }
    let declaration_source = body
        .get("declarationSource")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    let source_url = body
        .get("sourceUrl")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    let registered = is_registered_declaration_source(&declaration_source);
    let live = if uses_offering_price(&price_source, &symbol) {
        let cik = if source_symbol.chars().any(|c| c.is_ascii_digit()) {
            source_symbol.as_str()
        } else {
            ENERGYX_CIK
        };
        live_offering_snapshot(&symbol, cik)
    } else if registered {
        let mut cands =
            live_vendor_declarations(&declaration_source, &symbol, Some(source_url.as_str()));
        let known = known_payment_periods_from_body(body);
        cands = filter_new_declaration_candidates(cands, &known);
        let as_of = Utc::now().date_naive().to_string();
        let upcoming = upcoming_from_candidates(&cands, &as_of);
        json!({
            "symbol": yahoo_symbol(&symbol),
            "quote": live_price_quote(&symbol),
            "quoteCandidates": [],
            "candidates": cands,
            "upcomingPays": upcoming,
            "suggestedCalendarPolicy": suggest_calendar_policy(upcoming.len(), true),
            "source": declaration_source,
            "priceSource": "public",
            "missExplanation": if cands.is_empty() {
                "Issuer page empty.".to_string()
            } else {
                String::new()
            }
        })
    } else {
        json!({
            "symbol": yahoo_symbol(&symbol),
            "quote": live_price_quote(&symbol),
            "quoteCandidates": [],
            "candidates": body.get("candidates").cloned().unwrap_or(json!([])),
            "upcomingPays": [],
            "source": if declaration_source.is_empty() { "miss" } else { declaration_source.as_str() },
            "missExplanation": body.get("missExplanation").cloned().unwrap_or(json!(
                "No issuer fund page matched. Yahoo last price only; declarations stay unknown."
            ))
        })
    };
    match command_name {
        "PriceQuoteRetrieve" => {
            body["candidates"] = live["quoteCandidates"].clone();
        }
        "DeclarationRetrieve" => {
            body["candidates"] = live["candidates"].clone();
            if registered && live["candidates"].as_array().map(|a| a.is_empty()).unwrap_or(true) {
                body["missExplanation"] = live["missExplanation"].clone();
            }
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
    fn amplify_short_name_suggests_provider() {
        assert_eq!(
            super::suggested_provider_from_name("Amplify HACK Cybersecurity Cove"),
            "Amplify"
        );
        assert_eq!(super::suggested_provider_from_name("HACK Cybersecurity ETF"), "");
    }

    #[test]
    fn enrich_keeps_injected_candidates() {
        let mut body = json!({"symbol":"GOF","candidates":[{"priceMinor":1}]});
        enrich_retrieve_body("PriceQuoteRetrieve", &mut body);
        assert_eq!(body["candidates"].as_array().unwrap().len(), 1);
        assert_eq!(body["candidates"][0]["priceMinor"], 1);
    }

    #[test]
    fn yahoo_daily_fixture_filters_window() {
        let body = r#"{"chart":{"result":[{"timestamp":[1767225600,1769817600,1772496000],"indicators":{"quote":[{"close":[10.0,9.0,8.0]}]}}]}}"#;
        let series = parse_yahoo_daily_closes(body, "2026-01-01", "2026-02-28");
        assert_eq!(series.len(), 2);
        assert_eq!(series[0]["priceMinor"], 1000);
        assert_eq!(series[1]["priceMinor"], 900);
    }

    #[test]
    fn parse_19a1_current_distribution_percent() {
        let text = "Pursuant to Rule 19a-1, it is anticipated that 100% of such dividend will be a return of capital.";
        let (pct, how) = parse_19a1_notice(text).expect("19a-1 percent");
        assert_eq!(pct, 10_000);
        assert!(how.contains("19a-1"));
    }

    #[test]
    fn collect_last_price_quotes_empty_is_unknown() {
        assert!(collect_last_price_quotes(Vec::new()).is_empty());
        assert!(collect_last_price_quotes_for(Vec::new()).is_empty());
    }

    #[test]
    fn energyx_253g2_cover_parses_thirteen_not_increment_floor() {
        let html = r#"<html><body>
<p>Purchases must be in increments of at least $13.00</p>
<p>FINAL OFFERING CIRCULAR DATED JULY 13, 2026</p>
<p>up to 2,158,865 Shares of Common Stock at $13.00 per Share</p>
<p>The Shares offered hereby are being sold at a price of $13.00 per share.</p>
<p>The price per Share in this Offering is $13.00.</p>
<p>A+ offering of up to 2,700,000 shares of its common stock, par value $0.01 per share, at a price of $10.00 per share.</p>
</body></html>"#;
        let (minor, scale) = parse_edgar_offering_price(html).expect("offering price");
        assert_eq!(minor, 1300);
        assert_eq!(scale, 2);
        assert_eq!(
            parse_edgar_offering_as_of(html).as_deref(),
            Some("2026-07-13")
        );
    }

    #[test]
    fn energyx_injected_quote_skips_live_edgar() {
        let mut body = json!({
            "symbol": "ENERGYX",
            "priceSource": "edgar",
            "sourceSymbol": "1830166",
            "candidates": [{ "priceMinor": 1300, "scale": 2, "source": "fixture" }]
        });
        enrich_retrieve_body("PriceQuoteRetrieve", &mut body);
        assert_eq!(body["candidates"][0]["priceMinor"], 1300);
        assert_eq!(body["candidates"][0]["source"], "fixture");
    }

    #[test]
    #[ignore = "live SEC; not required for CI"]
    fn live_energyx_edgar_thirteen() {
        let quote = live_edgar_offering_quote(ENERGYX_CIK).expect("sec-edgar offering quote");
        assert_eq!(quote["priceMinor"], 1300);
        assert_eq!(quote["scale"], 2);
        assert_eq!(quote["source"], "sec-edgar");
        assert_eq!(quote["asOfAt"], "2026-07-13");
    }

    #[test]
    fn energyx_increment_only_stays_unknown() {
        let html = "<p>Purchases must be in increments of at least $13.00</p>";
        assert!(parse_edgar_offering_price(html).is_none());
    }

    #[test]
    fn yahoo_symbol_keeps_btc_separate_from_usd_pair() {
        assert_eq!(yahoo_symbol("BTC"), "BTC");
        assert_eq!(yahoo_symbol("btc"), "BTC");
        assert_eq!(yahoo_symbol("BTC-USD"), "BTC-USD");
        assert_eq!(yahoo_symbol("ETH"), "ETH-USD");
        assert_eq!(yahoo_symbol("SOL"), "SOL-USD");
        assert_eq!(yahoo_symbol("AAPL"), "AAPL");
        assert_eq!(yahoo_symbol("BRK.B"), "BRK-B");
    }

    #[test]
    fn parse_19a1_does_not_invent_zero_when_silent() {
        assert!(parse_19a1_notice("no percentage language in this filing").is_none());
    }

    #[test]
    fn roc_retrieve_keeps_nonempty_injected() {
        let mut filled = json!({
            "symbol": "HAKY",
            "candidates": [{
                "rocPctMinor": 10000,
                "scale": 2,
                "source": "19a-1",
                "sourceUrl": "https://example.test/19a-1.pdf"
            }]
        });
        enrich_retrieve_body("RocResearchRetrieve", &mut filled);
        assert_eq!(filled["candidates"][0]["rocPctMinor"], 10000);
    }

    #[test]
    fn period_series_keeps_injected_and_does_not_invent() {
        let mut body = json!({
            "symbol": "HAKY",
            "startOn": "2026-01-02",
            "endOn": "2026-04-07",
            "candidates": [{"asOfAt": "2026-01-02", "priceMinor": 1000}]
        });
        enrich_retrieve_body("PeriodSeriesRetrieve", &mut body);
        assert_eq!(body["candidates"].as_array().unwrap().len(), 1);
        assert_eq!(body["candidates"][0]["priceMinor"], 1000);
    }

    #[test]
    fn roundhill_html_table_parses_candidates() {
        let html = r#"<table><tr><th>Declaration</th><th>Ex Date</th><th>Record Date</th><th>Pay Date</th><th>Amount Paid</th></tr>
<tr><td>8/11/2026</td><td>8/12/2026</td><td>8/12/2026</td><td>8/13/2026</td><td>$0.2500</td></tr>
<tr><td>8/4/2026</td><td>8/5/2026</td><td>8/5/2026</td><td>8/6/2026</td><td>$0.2500</td></tr>
</table>"#;
        let parsed = parse_roundhill_distributions(html);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0]["paymentPeriod"], "2026-08-13");
        assert_eq!(parsed[0]["amountPerShareMinor"], 2500);
        assert_eq!(parsed[0]["source"], "roundhill");
    }

    #[test]
    fn roundhill_empty_html_stays_unknown() {
        assert!(parse_roundhill_distributions("<html><tbody id=\"calHisDistri\"></tbody></html>").is_empty());
    }

    #[test]
    fn roundhill_declaration_source_does_not_use_yahoo() {
        let mut body = json!({
            "symbol": "TOPW",
            "declarationSource": "roundhill",
            "candidates": [{"amountPerShareMinor": 2500, "amountScale": 4, "paymentPeriod": "2026-08-13", "source": "fixture"}]
        });
        enrich_retrieve_body("DeclarationRetrieve", &mut body);
        assert_eq!(body["candidates"][0]["source"], "fixture");
    }

    #[test]
    fn roundhill_404_shell_is_not_a_fund_page() {
        assert!(!roundhill_fund_page(
            "Page Not Found | Roundhill Investments # 404",
            "HAKY"
        ));
        assert!(roundhill_fund_page(
            "<html><title>HAKY | Roundhill Investments</title><body>Roundhill HAKY ETF</body></html>",
            "HAKY"
        ));
    }

    #[test]
    fn haky_research_prefers_roundhill_when_listed() {
        let html = r#"<html><title>HAKY | Roundhill Investments</title><body>Roundhill HAKY
<table><tr><th>Declaration</th><th>Ex Date</th><th>Record Date</th><th>Pay Date</th><th>Amount Paid</th></tr>
<tr><td>8/11/2026</td><td>8/12/2026</td><td>8/12/2026</td><td>8/13/2026</td><td>$0.2500</td></tr>
<tr><td>9/8/2026</td><td>9/9/2026</td><td>9/9/2026</td><td>9/10/2026</td><td>$0.2500</td></tr>
</table></body></html>"#;
        let profile = profile_from_vendor_htmls(
            "HAKY",
            &[(
                "roundhill",
                "https://www.roundhillinvestments.com/etf/haky",
                html,
            )],
            "2026-08-24",
        );
        assert_eq!(profile["declarationSource"], "roundhill");
        assert_eq!(profile["suggestedProvider"], "Roundhill");
        assert_eq!(profile["priceSource"], "public");
        assert_eq!(profile["candidates"].as_array().unwrap().len(), 2);
        assert_eq!(profile["upcomingPays"].as_array().unwrap().len(), 1);
        assert_eq!(profile["upcomingPays"][0]["payOn"], "2026-09-10");
        assert_eq!(profile["sourceAnalytics"]["tableOnGet"], true);
        assert_eq!(profile["sourceAnalytics"]["tableRowCount"], 2);
        assert!(profile["futureDeclarationStrategy"]
            .as_str()
            .unwrap_or("")
            .contains("GET roundhill"));
        assert_eq!(profile["suggestedCalendarPolicy"], "issuer_calendar");
    }

    #[test]
    fn research_extracts_underlying_and_empty_table_strategy() {
        let listed = r#"<html><title>HAKY – Amplify ETFs</title><body>Amplify HACK Cybersecurity Covered Call ETF HAKY</body></html>"#;
        let profile = profile_from_vendor_htmls(
            "HAKY",
            &[(
                "amplify",
                "https://amplifyetfs.com/haky/",
                listed,
            )],
            "2026-08-24",
        );
        assert_eq!(profile["underlying"], "HACK");
        assert_eq!(profile["lookthrough"]["themeStrategy"], "Cybersecurity + Covered Call Equity");
        assert_eq!(
            profile["lookthrough"]["primaryRiskDriver"],
            "Look-through cybersecurity equity basket (HACK)"
        );
        assert_eq!(profile["lookthrough"]["riskTierSuggestion"], "Risk On");
        assert_eq!(profile["lookthrough"]["concentrationStatus"], "unknown");
        assert!(profile["lookthrough"]["topHoldings"].as_array().unwrap().is_empty());
        assert_eq!(profile["suggestedProvider"], "Amplify");
        assert_eq!(profile["sourceAnalytics"]["fundPage"], true);
        assert_eq!(profile["sourceAnalytics"]["tableOnGet"], false);
        assert!(profile["futureDeclarationStrategy"]
            .as_str()
            .unwrap_or("")
            .contains("Keep declarationSource=amplify"));
    }

    #[test]
    fn research_lookthrough_parses_labeled_top_holdings_not_a_full_book() {
        let listed = r#"<html><title>HAKY – Amplify ETFs</title><body>
Amplify HACK Cybersecurity Covered Call ETF HAKY
<h3>Current Top Underlying Equity Positions</h3>
<table>
<tr><th>Ticker</th><th>Weight</th></tr>
<tr><td>PANW</td><td>8.87%</td></tr>
<tr><td>CRWD</td><td>8.10%</td></tr>
<tr><td>FTNT</td><td>7.50%</td></tr>
<tr><td>CSCO</td><td>6.20%</td></tr>
<tr><td>AVGO</td><td>5.40%</td></tr>
</table>
<h3>Sector Breakdown</h3>
<p>Information Technology 91.2%</p>
</body></html>"#;
        let profile = profile_from_vendor_htmls(
            "HAKY",
            &[(
                "amplify",
                "https://amplifyetfs.com/haky/",
                listed,
            )],
            "2026-08-24",
        );
        let holdings = profile["lookthrough"]["topHoldings"].as_array().unwrap();
        assert_eq!(profile["lookthrough"]["concentrationStatus"], "issuer_top_holdings");
        assert_eq!(holdings.len(), 5);
        assert_eq!(holdings[0]["ticker"], "PANW");
        assert_eq!(holdings[0]["weightBps"], 887);
        assert_eq!(profile["lookthrough"]["sectorWeights"][0]["label"], "Information Technology");
        assert_eq!(profile["lookthrough"]["sectorWeights"][0]["weightBps"], 9120);
    }

    #[test]
    fn research_lookthrough_ignores_unlabeled_holdings_list() {
        let listed = r#"<html><title>HAKY – Amplify ETFs</title><body>
Amplify HACK Cybersecurity Covered Call ETF HAKY
<table>
<tr><th>Ticker</th><th>Weight</th></tr>
<tr><td>PANW</td><td>8.87%</td></tr>
<tr><td>CRWD</td><td>8.10%</td></tr>
</table>
</body></html>"#;
        let profile = profile_from_vendor_htmls(
            "HAKY",
            &[(
                "amplify",
                "https://amplifyetfs.com/haky/",
                listed,
            )],
            "2026-08-24",
        );
        assert_eq!(profile["lookthrough"]["concentrationStatus"], "unknown");
        assert!(profile["lookthrough"]["topHoldings"].as_array().unwrap().is_empty());
    }

    #[test]
    fn haky_research_does_not_treat_roundhill_404_as_the_source() {
        let profile = profile_from_vendor_htmls(
            "HAKY",
            &[
                (
                    "roundhill",
                    "https://www.roundhillinvestments.com/etf/haky",
                    "Page Not Found | Roundhill Investments",
                ),
                (
                    "amplify",
                    "https://amplifyetfs.com/haky/",
                    "<html><title>HAKY – Amplify ETFs</title><body>Amplify HACK Cybersecurity Covered Call ETF HAKY</body></html>",
                ),
            ],
            "2026-08-24",
        );
        assert_eq!(profile["researchAttempts"][0]["found"], false);
        assert_eq!(profile["declarationSource"], "amplify");
        assert_eq!(profile["suggestedProvider"], "Amplify");
        assert!(profile["missExplanation"].as_str().unwrap_or("").contains("empty") || profile["candidates"].as_array().unwrap().is_empty());
    }

    #[test]
    fn market_retrieve_research_only_skips_yahoo_quote() {
        let mut body = json!({
            "symbol": "HAKY",
            "researchOnly": true,
            "researchAttempts": [{
                "vendor": "roundhill",
                "url": "https://www.roundhillinvestments.com/etf/haky",
                "found": false,
                "note": "issuer page not found"
            }],
            "missExplanation": "Roundhill page not found for HAKY."
        });
        enrich_retrieve_body("MarketRetrieve", &mut body);
        assert!(body["quote"].is_null());
        assert_eq!(body["quoteCandidates"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn registered_vendors_include_all_providers_not_yahoo() {
        assert!(is_registered_declaration_source("roundhill"));
        assert!(is_registered_declaration_source("Amplify"));
        assert!(is_registered_declaration_source("neos"));
        assert!(is_registered_declaration_source("yieldmax"));
        assert!(is_registered_declaration_source("cornerstone"));
        assert!(is_registered_declaration_source("globalx"));
        assert!(is_registered_declaration_source("jpmorgan"));
        assert!(!is_registered_declaration_source("yahoo"));
        assert!(!is_registered_declaration_source("public"));
        assert!(!is_registered_declaration_source(""));
        assert_eq!(registered_declaration_sources().len(), 23);
    }

    #[test]
    fn collect_never_falls_back_to_yahoo_dividends() {
        let yahoo = collect_declarations_for(vec![DeclarationTarget {
            security_id: "sec-1".into(),
            symbol: "HAKY".into(),
            declaration_source: "yahoo".into(),
            source_symbol: "HAKY".into(),
            ..Default::default()
        }]);
        assert!(yahoo.candidates.is_empty());
        assert_eq!(yahoo.misses[0]["code"], "no_declaration_adapter");

        let public = collect_declarations_for(vec![DeclarationTarget {
            security_id: "sec-2".into(),
            symbol: "TSLA".into(),
            declaration_source: "public".into(),
            source_symbol: "TSLA".into(),
            ..Default::default()
        }]);
        assert!(public.candidates.is_empty());
        assert!(public.misses.is_empty());

        let unassigned = collect_declarations_for(vec![DeclarationTarget {
            security_id: "sec-3".into(),
            symbol: "QYLD".into(),
            declaration_source: "unassigned".into(),
            source_symbol: "QYLD".into(),
            ..Default::default()
        }]);
        assert!(unassigned.candidates.is_empty());
        assert!(unassigned.misses.is_empty());

        let div1_gap = collect_declarations_for(vec![DeclarationTarget {
            security_id: "sec-4".into(),
            symbol: "QYLD".into(),
            declaration_source: "unassigned".into(),
            source_symbol: "QYLD".into(),
            div_type: "DIV-1".into(),
            ..Default::default()
        }]);
        assert!(div1_gap.candidates.is_empty());
        assert_eq!(div1_gap.misses[0]["code"], "div1_adapter_missing");
    }

    #[test]
    fn research_without_provider_hint_does_not_spray_issuer_sites() {
        let mut body = json!({
            "symbol": "HAKY",
            "researchOnly": true
        });
        enrich_retrieve_body("MarketRetrieve", &mut body);
        assert_eq!(body["declarationSource"], "");
        assert_eq!(body["source"], "miss");
        assert!(body["researchAttempts"].as_array().unwrap().is_empty());
    }

    #[test]
    fn two_upcoming_pays_suggest_issuer_calendar() {
        let html = r#"<html><title>HAKY | Amplify</title><body>Amplify HAKY
<table><tr><th>Ex-Date</th><th>Record Date</th><th>Payable Date</th><th>Amount (USD)</th></tr>
<tr><td>9/9/2026</td><td>9/9/2026</td><td>9/10/2026</td><td>$0.2500</td></tr>
<tr><td>10/9/2026</td><td>10/9/2026</td><td>10/10/2026</td><td>$0.2500</td></tr>
</table></body></html>"#;
        let profile = profile_from_vendor_htmls(
            "HAKY",
            &[("amplify", "https://amplifyetfs.com/haky/", html)],
            "2026-08-24",
        );
        assert_eq!(profile["declarationSource"], "amplify");
        assert_eq!(profile["upcomingPays"].as_array().unwrap().len(), 2);
        assert_eq!(profile["suggestedCalendarPolicy"], "issuer_calendar");
    }

    #[test]
    fn amplify_parses_paid_and_blank_remaining_year() {
        let html = r#"<table><tr><th>Ex-Date</th><th>Record Date</th><th>Payable Date</th><th>Amount (USD)</th></tr>
<tr><td>07/30/2026</td><td>07/30/2026</td><td>07/31/2026</td><td>$0.38826</td></tr>
<tr><td>08/28/2026</td><td>08/28/2026</td><td>08/31/2026</td><td></td></tr>
<tr><td>09/29/2026</td><td>09/29/2026</td><td>09/30/2026</td><td>—</td></tr>
</table>"#;
        let parsed = parse_amplify_distributions(html);
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0]["paymentPeriod"], "2026-09-30");
        assert!(parsed[0]["amountPerShareMinor"].is_null());
        assert_eq!(parsed[2]["paymentPeriod"], "2026-07-31");
        assert_eq!(parsed[2]["amountPerShareMinor"], 38826);
        let upcoming = upcoming_from_candidates(&parsed, "2026-08-24");
        assert_eq!(upcoming.len(), 2);
        assert_eq!(upcoming[0]["payOn"], "2026-09-30");
        assert!(upcoming[0]["amountPerShareMinor"].is_null());
    }

    #[test]
    fn qdvo_amplify_profile_keeps_blank_remaining_year() {
        let html = r#"<html><title>QDVO | Amplify</title><body>Amplify QDVO
<table><tr><th>Ex-Date</th><th>Record Date</th><th>Payable Date</th><th>Amount (USD)</th></tr>
<tr><td>06/27/2026</td><td>06/27/2026</td><td>06/30/2026</td><td>$0.24000</td></tr>
<tr><td>07/30/2026</td><td>07/30/2026</td><td>07/31/2026</td><td>$0.24110</td></tr>
<tr><td>08/28/2026</td><td>08/28/2026</td><td>08/31/2026</td><td></td></tr>
<tr><td>09/29/2026</td><td>09/29/2026</td><td>09/30/2026</td><td></td></tr>
</table></body></html>"#;
        let profile = profile_from_vendor_htmls(
            "QDVO",
            &[("amplify", "https://amplifyetfs.com/qdvo/", html)],
            "2026-08-24",
        );
        assert_eq!(profile["declarationSource"], "amplify");
        let paid = profile["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| !c["amountPerShareMinor"].is_null())
            .count();
        assert_eq!(paid, 2);
        assert_eq!(profile["upcomingPays"].as_array().unwrap().len(), 2);
        assert!(profile["upcomingPays"][0]["amountPerShareMinor"].is_null());
        assert_eq!(profile["suggestedCalendarPolicy"], "issuer_calendar");
    }

    #[test]
    fn neos_parses_year_table_including_blank_future() {
        let html = r#"<table><tr><th>Declaration Date</th><th>Ex-Div Date</th><th>Record Date</th><th>Payable Date</th><th>Amount ($)</th></tr>
<tr><td>08/18/2026</td><td>08/19/2026</td><td>08/19/2026</td><td>08/21/2026</td><td>$0.5423</td></tr>
<tr><td>09/15/2026</td><td>09/16/2026</td><td>09/16/2026</td><td>09/18/2026</td><td></td></tr>
</table>"#;
        let parsed = parse_neos_distributions(html);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0]["paymentPeriod"], "2026-09-18");
        assert!(parsed[0]["amountPerShareMinor"].is_null());
        assert_eq!(parsed[1]["amountPerShareMinor"], 5423);
        assert_eq!(parsed[1]["source"], "neos");
        let html = r#"<html><title>SPYI | NEOS</title><body>NEOS SPYI
<table><tr><th>Declaration Date</th><th>Ex-Div Date</th><th>Record Date</th><th>Payable Date</th><th>Amount ($)</th></tr>
<tr><td>08/18/2026</td><td>08/19/2026</td><td>08/19/2026</td><td>08/21/2026</td><td>$0.5423</td></tr>
<tr><td>09/15/2026</td><td>09/16/2026</td><td>09/16/2026</td><td>09/18/2026</td><td></td></tr>
</table></body></html>"#;
        let profile = profile_from_vendor_htmls(
            "SPYI",
            &[("neos", "https://neosfunds.com/spyi/", html)],
            "2026-08-24",
        );
        assert_eq!(profile["declarationSource"], "neos");
        assert_eq!(profile["upcomingPays"].as_array().unwrap().len(), 1);
        assert!(profile["upcomingPays"][0]["amountPerShareMinor"].is_null());
    }

    #[test]
    fn yieldmax_dedupes_and_reads_roc_percent() {
        let html = r#"<table><tr><th>DISTRIBUTION PER SHARE</th><th>DECLARED DATE</th><th>EX DATE</th><th>RECORD DATE</th><th>PAYABLE DATE</th><th>ROC</th></tr>
<tr><td>$0.1620</td><td>08/19/2026</td><td>08/20/2026</td><td>08/20/2026</td><td>08/21/2026</td><td>0.00%</td></tr>
<tr><td>$0.1620</td><td>08/19/2026</td><td>08/20/2026</td><td>08/20/2026</td><td>08/21/2026</td><td>0.00%</td></tr>
<tr><td>$0.1809</td><td>08/12/2026</td><td>08/13/2026</td><td>08/13/2026</td><td>08/14/2026</td><td>98.74%</td></tr>
</table>"#;
        let parsed = parse_yieldmax_distributions(html);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0]["paymentPeriod"], "2026-08-21");
        assert_eq!(parsed[0]["amountPerShareMinor"], 1620);
        assert_eq!(parsed[0]["rocPctMinor"], 0);
        assert_eq!(parsed[1]["rocPctMinor"], 9874);
        let html = r#"<html><title>MSTY | YieldMax</title><body>YieldMax MSTY
<table><tr><th>DISTRIBUTION PER SHARE</th><th>DECLARED DATE</th><th>EX DATE</th><th>RECORD DATE</th><th>PAYABLE DATE</th><th>ROC</th></tr>
<tr><td>$0.1620</td><td>08/19/2026</td><td>08/20/2026</td><td>08/20/2026</td><td>08/21/2026</td><td>0.00%</td></tr>
<tr><td>$0.1620</td><td>08/19/2026</td><td>08/20/2026</td><td>08/20/2026</td><td>08/21/2026</td><td>0.00%</td></tr>
<tr><td>$0.1809</td><td>08/12/2026</td><td>08/13/2026</td><td>08/13/2026</td><td>08/14/2026</td><td>98.74%</td></tr>
</table></body></html>"#;
        let profile = profile_from_vendor_htmls(
            "MSTY",
            &[(
                "yieldmax",
                "https://www.yieldmaxetfs.com/our-etfs/msty/",
                html,
            )],
            "2026-08-24",
        );
        assert_eq!(profile["declarationSource"], "yieldmax");
        assert_eq!(profile["candidates"].as_array().unwrap().len(), 2);
        assert_eq!(profile["upcomingPays"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn roundhill_csv_parses_when_html_empty() {
        let csv = "Declaration,Ex Date,Record Date,Pay Date,Amount Paid\n8/11/2026,8/12/2026,8/12/2026,8/13/2026,$0.2500\n";
        let parsed = parse_roundhill_csv(csv);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0]["paymentPeriod"], "2026-08-13");
        assert_eq!(parsed[0]["amountPerShareMinor"], 2500);
        assert!(parse_roundhill_distributions(
            r#"<html><a href="/distributions.csv">Download CSV</a><tbody id="calHisDistri"></tbody></html>"#
        )
        .is_empty());
        assert_eq!(
            parse_roundhill_roc_html(
                "Per the Fund’s most recent 19a-1 notice, the estimated per share composition of the distribution includes return of capital (ROC) of 100%."
            ),
            Some(10000)
        );
    }

    #[test]
    fn neos_19a1_notice_parses_percent() {
        let text = "In connection with the monthly dividend payment of $0.5309 per share payable on January 23rd, 2026 to shareholders of record on January 21st, 2026, it is anticipated that 97% of such dividend will be a return of capital.";
        let (pct, _) = parse_19a1_notice(text).expect("19a-1");
        assert_eq!(pct, 9700);
    }

    fn target(symbol: &str, source: &str, hash: &str) -> DeclarationTarget {
        DeclarationTarget {
            security_id: "sec-hash".into(),
            symbol: symbol.into(),
            declaration_source: source.into(),
            source_symbol: symbol.into(),
            last_content_hash: hash.into(),
            ..Default::default()
        }
    }

    #[test]
    fn same_page_hash_skips_reparse_and_miss() {
        let html = "<html><tbody id=\"calHisDistri\"></tbody></html>";
        let hash = page_content_hash(html);
        let miss = collect_from_fetched_page(&target("TOPW", "roundhill", ""), "roundhill", Some(html));
        assert_eq!(miss.misses[0]["reason"], MISS_EMPTY);
        assert!(miss.unchanged.is_empty());
        let skip = collect_from_fetched_page(&target("TOPW", "roundhill", &hash), "roundhill", Some(html));
        assert!(skip.misses.is_empty());
        assert_eq!(skip.unchanged.len(), 1);
        assert_eq!(skip.unchanged[0]["contentHash"], hash);
        let changed = collect_from_fetched_page(
            &target("TOPW", "roundhill", &hash),
            "roundhill",
            Some("<html>changed empty shell</html>"),
        );
        assert_eq!(changed.misses[0]["reason"], MISS_CHANGED);
        assert!(changed.unchanged.is_empty());
    }

    #[test]
    fn paid_history_keeps_all_rows() {
        let mut rows = String::from(
            "<table><tr><th>Ex-Date</th><th>Record Date</th><th>Payable Date</th><th>Amount (USD)</th></tr>",
        );
        for i in 1..=13 {
            rows.push_str(&format!(
                "<tr><td>01/{i:02}/2025</td><td>01/{i:02}/2025</td><td>01/{i:02}/2025</td><td>$0.10</td></tr>"
            ));
        }
        rows.push_str("</table>");
        let parsed = parse_amplify_distributions(&rows);
        assert_eq!(parsed.len(), 13);
        let out = collect_from_fetched_page(&target("HAKY", "amplify", ""), "amplify", Some(&rows));
        assert_eq!(out.candidates.len(), 13);
        assert!(out.misses.is_empty());
    }

    #[test]
    fn paid_history_under_twelve_is_lookback_miss() {
        let mut rows = String::from(
            "<table><tr><th>Ex-Date</th><th>Record Date</th><th>Payable Date</th><th>Amount (USD)</th></tr>",
        );
        for i in 1..=5 {
            rows.push_str(&format!(
                "<tr><td>01/{i:02}/2025</td><td>01/{i:02}/2025</td><td>01/{i:02}/2025</td><td>$0.10</td></tr>"
            ));
        }
        rows.push_str("</table>");
        let out = collect_from_fetched_page(
            &DeclarationTarget {
                payment_frequency: "Monthly".into(),
                ..target("AMPX", "amplify", "")
            },
            "amplify",
            Some(&rows),
        );
        assert_eq!(out.candidates.len(), 5);
        assert_eq!(out.misses.len(), 1);
        assert_eq!(out.misses[0]["code"], "declaration_lookback_short");
        assert_eq!(out.misses[0]["paidCount"], 5);
        assert_eq!(out.misses[0]["requiredPaid"], 12);
    }

    #[test]
    fn inception_limits_required_paid_lookback() {
        let mut rows = String::from(
            "<table><tr><th>Ex-Date</th><th>Record Date</th><th>Payable Date</th><th>Amount (USD)</th></tr>",
        );
        for i in 1..=6 {
            rows.push_str(&format!(
                "<tr><td>01/{i:02}/2025</td><td>01/{i:02}/2025</td><td>01/{i:02}/2025</td><td>$0.10</td></tr>"
            ));
        }
        rows.push_str("</table>");
        let as_of = Utc::now().date_naive();
        let inception = as_of
            .checked_sub_months(chrono::Months::new(6))
            .expect("inception");
        let out = collect_from_fetched_page(
            &DeclarationTarget {
                inception_on: inception.to_string(),
                payment_frequency: "Monthly".into(),
                ..target("AMPX", "amplify", "")
            },
            "amplify",
            Some(&rows),
        );
        assert_eq!(out.candidates.len(), 6);
        assert!(
            out.misses.is_empty(),
            "expected complete short history, got {:?}",
            out.misses
        );
        assert_eq!(
            financial_domain::declaration_lookback::expected_declaration_lookback(
                &inception.to_string(),
                &as_of.to_string(),
                "Monthly",
            ),
            6
        );
    }

    #[test]
    #[ignore = "live network probe"]
    fn live_nvdw_roundhill_fund_page_does_not_block_fallback() {
        let page = "https://www.roundhillinvestments.com/etf/nvdw";
        let body = http_get(page).expect("roundhill page GET");
        let cands = adapters::parse_vendor_distributions_with_csv("roundhill", &body);
        let paid = cands
            .iter()
            .filter(|c| candidate_amount(c).is_some())
            .count();
        let upcoming = upcoming_from_candidates(&cands, &Utc::now().date_naive().to_string());
        eprintln!(
            "roundhill page len={} cands={} paid={} upcoming={}",
            body.len(),
            cands.len(),
            paid,
            upcoming.len()
        );
        assert_eq!(paid, 0, "fund page should not yield paid rows");
    }

    #[test]
    #[ignore = "live network probe"]
    fn live_nvdw_dividendhistory_fetch_parses() {
        let url = "https://dividendhistory.org/payout/NVDW/";
        let body = http_get(url).expect("dividendhistory GET");
        assert!(body.len() > 10_000, "body too short: {}", body.len());
        let cands = adapters::parse_vendor_distributions_with_csv("roundhill", &body);
        let paid = cands
            .iter()
            .filter(|c| {
                c.get("amountPerShareMinor")
                    .and_then(|v| v.as_i64())
                    .map(|a| a > 0)
                    .unwrap_or(false)
            })
            .count();
        eprintln!("NVDW dividendhistory paid={paid} total={}", cands.len());
        assert!(paid >= 12, "expected >=12 paid, got {paid}");
    }

    #[test]
    fn fresh_same_day_skips_network_collect() {
        let today = Utc::now().date_naive().to_string();
        let out = collect_declarations_for(vec![DeclarationTarget {
            security_id: "sec-fresh".into(),
            symbol: "EFC".into(),
            declaration_source: "ellington".into(),
            last_content_hash: "deadbeef".into(),
            last_run_ok: true,
            last_run_at: today,
            force_refresh: false,
            paid_count: 12,
            ..Default::default()
        }]);
        assert_eq!(out.unchanged.len(), 1);
        assert!(out.candidates.is_empty());
        assert!(out.misses.is_empty());
    }

    #[test]
    fn force_refresh_bypasses_same_day_skip() {
        let today = Utc::now().date_naive().to_string();
        let out = collect_declarations_for(vec![DeclarationTarget {
            security_id: "sec-force".into(),
            symbol: "EFC".into(),
            declaration_source: "ellington".into(),
            last_content_hash: "deadbeef".into(),
            last_run_ok: true,
            last_run_at: today,
            force_refresh: true,
            ..Default::default()
        }]);
        assert!(out.unchanged.is_empty());
    }
}
