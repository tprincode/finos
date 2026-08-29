//! Simplify ETF vendor site. Fund pages load distribution history via Drupal AJAX at
//! `/etfs/{node_id}/distributions` (e.g. SVOL → node 536).

use serde_json::Value;

use crate::retrieve::html::{
    distribution_candidate, html_td_rows, parse_issuer_amount, parse_issuer_date, sort_newest_first,
};

use super::generic::parse_generic_distributions;

pub fn simplify_fund_url(symbol: &str) -> String {
    let sym = symbol.trim().to_ascii_lowercase();
    if sym == "svol" {
        return "https://www.simplify.us/etfs/svol-simplify-volatility-premium-etf".into();
    }
    format!("https://www.simplify.us/etfs/{sym}")
}

pub fn simplify_distributions_url(node_id: &str) -> String {
    format!(
        "https://www.simplify.us/etfs/{}/distributions",
        node_id.trim()
    )
}

/// Drupal node id from fund page (`data-history-node-id`, distributions link, or settings JSON).
pub fn extract_simplify_node_id(html: &str) -> Option<String> {
    if let Some(start) = html.find("data-history-node-id=\"") {
        let rest = html.get(start + 22..)?;
        let end = rest.find('"')?;
        let id = rest.get(..end)?.trim();
        if node_id_valid(id) {
            return Some(id.to_string());
        }
    }
    let lower = html.to_ascii_lowercase();
    for marker in ["/etfs/", "href=\"/etfs/"] {
        let mut search = 0usize;
        while let Some(rel) = lower.get(search..).and_then(|s| s.find(marker)) {
            let idx = search + rel + marker.len();
            let digits: String = lower
                .get(idx..)?
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if node_id_valid(&digits) {
                return Some(digits);
            }
            search = idx + digits.len().max(1);
        }
    }
    if let Some(start) = html.find(r#""node_id":""#) {
        let rest = html.get(start + 11..)?;
        let end = rest.find('"')?;
        let id = rest.get(..end)?.trim();
        if node_id_valid(id) {
            return Some(id.to_string());
        }
    }
    None
}

fn node_id_valid(id: &str) -> bool {
    !id.is_empty() && id.len() <= 6 && id.chars().all(|c| c.is_ascii_digit())
}

/// Drupal `/distributions` returns JSON commands; extract embedded distribution table HTML.
pub fn extract_simplify_table_html(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.starts_with('[') {
        if let Ok(commands) = serde_json::from_str::<Vec<Value>>(trimmed) {
            for cmd in commands {
                let Some(data) = cmd.get("data").and_then(|d| d.as_str()) else {
                    continue;
                };
                if data.contains("field-payable-date") || data.contains("Total Distribution") {
                    return data.to_string();
                }
            }
        }
    }
    trimmed.to_string()
}

pub fn parse_simplify_distributions(source: &str, body: &str) -> Vec<Value> {
    let html = extract_simplify_table_html(body);
    let mut out = Vec::new();
    for cells in html_td_rows(&html) {
        if cells.len() >= 4 {
            let pay = parse_issuer_date(&cells[2]);
            let amount = parse_issuer_amount(&cells[3]);
            if let Some(pay) = pay {
                out.push(distribution_candidate(source, pay, amount, None));
                continue;
            }
        }
    }
    if out.is_empty() {
        out = parse_generic_distributions(source, &html);
    }
    sort_newest_first(&mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn simplify_fund_page_snippet_finds_calendar_pdf() {
        let html = r#"<a href="/sites/default/files/2026-05/Simplify-Distribution-Calendar-June-2026..pdf" class="distribution-calendar-link">"#;
        let url = crate::retrieve::html::distribution_calendar_url(html, "https://www.simplify.us")
            .expect("calendar url");
        assert!(url.contains("Simplify-Distribution-Calendar-June-2026"));
    }

    #[test]
    fn fixture_svol_ajax_json_parses_full_history() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/simplify_svol_distributions.json");
        let body = std::fs::read_to_string(path).expect("fixture");
        let rows = parse_simplify_distributions("simplify", &body);
        assert!(
            rows.len() >= 12,
            "SVOL distributions should cover 12+ months, got {}",
            rows.len()
        );
        assert_eq!(rows[0]["paymentPeriod"].as_str(), Some("2026-09-30"));
        assert!(rows[0]["amountPerShareMinor"].is_null());
        assert_eq!(rows[1]["paymentPeriod"].as_str(), Some("2026-08-31"));
        assert_eq!(rows[1]["amountPerShareMinor"], 28000);
    }

    #[test]
    fn extract_node_id_from_fund_page_snippet() {
        let html = r#"<section data-history-node-id="536"><a href="/etfs/536/distributions">"#;
        assert_eq!(extract_simplify_node_id(html).as_deref(), Some("536"));
    }
}
