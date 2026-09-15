//! Simplify ETF vendor site. Fund pages load distribution history via Drupal AJAX at
//! `/etfs/{node_id}/distributions` (e.g. SVOL → node 536).

use serde_json::Value;

use crate::retrieve::html::{parse_distribution_tables, sort_newest_first};

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

/// S19(a) multi-fund table row: `(SVOL)$0.2800$0.0406$0.2394` → ROC / total at scale 2.
/// When the ROC column is glued to the next CUSIP, ROC is total − net income.
pub fn parse_simplify_s19a_roc(text: &str, symbol: &str) -> Option<(i64, String)> {
    let sym = symbol.trim().to_ascii_uppercase();
    if sym.len() < 2 {
        return None;
    }
    let needle = format!("({sym})");
    let idx = text.find(&needle)?;
    let after = text.get(idx + needle.len()..)?;
    let end = after
        .find("82889")
        .or_else(|| after.find('('))
        .unwrap_or(after.len().min(80));
    let amounts = dollar_minors_scale4(&after[..end]);
    if amounts.len() < 2 {
        return None;
    }
    let total = amounts[0];
    let net_income = amounts[1];
    if total <= 0 {
        return None;
    }
    let roc = if amounts.len() >= 3 {
        let listed = amounts[2];
        if (net_income.saturating_add(listed) - total).unsigned_abs() <= 5 {
            listed
        } else if total > net_income {
            total - net_income
        } else {
            return None;
        }
    } else if total >= net_income {
        total - net_income
    } else {
        return None;
    };
    let pct = (roc.saturating_mul(10_000) + total / 2) / total;
    if !(0..=10_000).contains(&pct) {
        return None;
    }
    Some((pct, "s19a current distribution".into()))
}

fn dollar_minors_scale4(s: &str) -> Vec<i64> {
    let mut out = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && out.len() < 3 {
        if bytes[i] != b'$' {
            i += 1;
            continue;
        }
        i += 1;
        let start = i;
        while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
            i += 1;
        }
        if let Some(v) = ascii_dollars_to_scale4(&s[start..i]) {
            out.push(v);
        }
    }
    out
}

fn ascii_dollars_to_scale4(num: &str) -> Option<i64> {
    let (whole, frac) = num.split_once('.').unwrap_or((num, ""));
    if whole.is_empty() && frac.is_empty() {
        return None;
    }
    let w: i64 = if whole.is_empty() {
        0
    } else {
        whole.parse().ok()?
    };
    let mut f = frac.chars().filter(|c| c.is_ascii_digit()).collect::<String>();
    while f.len() < 4 {
        f.push('0');
    }
    let frac4: i64 = f.get(..4)?.parse().ok()?;
    Some(w.saturating_mul(10_000).saturating_add(frac4))
}

/// Drupal AJAX accordion for the fund-page “S19(a) Notices” control (`/etfs/{node}/supplemental-tax-information`).
pub fn simplify_supplemental_tax_url(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let marker = "/supplemental-tax-information";
    let rel = lower.find(marker)?;
    let before = html.get(..rel)?;
    let etfs = before.rfind("/etfs/")?;
    let path = html.get(etfs..rel + marker.len())?.trim();
    if path.contains('<') || path.len() > 80 {
        return None;
    }
    Some(format!("https://www.simplify.us{path}"))
}

/// Drupal command JSON (`data` HTML) or the raw page.
pub fn extract_simplify_ajax_html(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.starts_with('[') {
        if let Ok(commands) = serde_json::from_str::<Vec<Value>>(trimmed) {
            let mut out = String::new();
            for cmd in commands {
                if let Some(data) = cmd.get("data").and_then(|d| d.as_str()) {
                    out.push_str(data);
                }
            }
            if !out.is_empty() {
                return out;
            }
        }
    }
    unescape_jsonish(body)
}

fn unescape_jsonish(s: &str) -> String {
    s.replace("\\u0022", "\"")
        .replace("\\/", "/")
        .replace("\\\"", "\"")
}

/// Newest monthly S19(a) PDF first (`/2026-08/` or `_2026_August_`).
pub fn simplify_s19a_notice_urls(html: &str, origin: &str) -> Vec<String> {
    let decoded = extract_simplify_ajax_html(html);
    let mut out = Vec::new();
    for needle in ["s19a", "s19(a)", "19(a)"] {
        for href in crate::retrieve::adapters::hrefs_matching(&decoded, needle, origin) {
            let lower = href.to_ascii_lowercase();
            if lower.contains(".pdf") && !out.iter().any(|u| u == &href) {
                out.push(href);
            }
        }
    }
    let scan = decoded.to_ascii_lowercase();
    let mut search = 0usize;
    while let Some(rel) = scan.get(search..).and_then(|s| s.find("/sites/default/files/")) {
        let idx = search + rel;
        let rest = decoded.get(idx..).unwrap_or("");
        let end = rest
            .to_ascii_lowercase()
            .find(".pdf")
            .map(|i| i + 4)
            .unwrap_or(0);
        if end > 0 {
            let path = rest.get(..end).unwrap_or("");
            if path.to_ascii_lowercase().contains("s19a") {
                let url = if path.starts_with("http") {
                    path.to_string()
                } else {
                    format!("{origin}{path}")
                };
                if !out.iter().any(|u| u == &url) {
                    out.push(url);
                }
            }
            search = idx + end;
        } else {
            search = idx + 21;
        }
    }
    out.sort_by(|a, b| simplify_s19a_recency(b).cmp(&simplify_s19a_recency(a)));
    out
}

pub(crate) fn simplify_s19a_recency(url: &str) -> (i32, i32) {
    let u = url.to_ascii_lowercase();
    if let Some(idx) = u.find("/20") {
        let y: i32 = u.get(idx + 1..idx + 5).and_then(|s| s.parse().ok()).unwrap_or(0);
        let m: i32 = u.get(idx + 6..idx + 8).and_then(|s| s.parse().ok()).unwrap_or(0);
        if y >= 2020 && (1..=12).contains(&m) {
            return (y, m);
        }
    }
    const MONTHS: [&str; 12] = [
        "january", "february", "march", "april", "may", "june", "july", "august",
        "september", "october", "november", "december",
    ];
    let y = if u.contains("2026") {
        2026
    } else if u.contains("2025") {
        2025
    } else {
        0
    };
    let m = MONTHS
        .iter()
        .position(|n| u.contains(n))
        .map(|i| i as i32 + 1)
        .unwrap_or(0);
    (y, m)
}

pub fn parse_simplify_distributions(source: &str, body: &str) -> Vec<Value> {
    let html = extract_simplify_table_html(body);
    let mut out = parse_distribution_tables(source, &html);
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

    #[test]
    fn june_s19a_row_is_85_50_roc() {
        let text = "CUSIPFund NameTotal DistributionPer ShareNet IncomeReturn of Capital82889N 863Simplify Volatility Premium ETF (SVOL)$0.2800$0.0406$0.239482889N 723Simplify Aggregate Bond ETF (AGGH)$0.1200$0.0690$0.0510";
        let (pct, how) = parse_simplify_s19a_roc(text, "SVOL").expect("SVOL row");
        assert_eq!(pct, 8_550);
        assert!(how.contains("s19a"));
    }

    #[test]
    fn august_s19a_glued_roc_column_uses_total_minus_income() {
        let text = "(SVOL)$0.2800$0.0543$0.280082889N 723Simplify Aggregate Bond ETF (AGGH)$0.1200";
        let (pct, _) = parse_simplify_s19a_roc(text, "SVOL").expect("SVOL row");
        assert_eq!(pct, 8_061);
    }

    #[test]
    fn s19a_hrefs_newest_month_first() {
        let html = r#"
            <a href="/sites/default/files/2026-06/S19a_Notice_2026_June_M.pdf">June</a>
            <a href="/sites/default/files/2026-08/S19a_Notice_2026_August_M.pdf">August</a>
        "#;
        let urls = simplify_s19a_notice_urls(html, "https://www.simplify.us");
        assert!(urls[0].contains("2026-08"), "{urls:?}");
        assert!(urls[1].contains("2026-06"), "{urls:?}");
    }

    #[test]
    fn fund_page_s19a_control_points_at_node_accordion() {
        let html = r#"<a href="/etfs/536/supplemental-tax-information" class="use-ajax"><span>S19(a) Notices</span></a>"#;
        assert_eq!(
            simplify_supplemental_tax_url(html).as_deref(),
            Some("https://www.simplify.us/etfs/536/supplemental-tax-information")
        );
    }

    #[test]
    fn ajax_json_lists_august_s19a_first() {
        let body = r#"[{"command":"insert","data":"<a href=\"/sites/default/files/2026-08/S19a_Notice_2026_August_M.pdf\">August</a><a href=\"/sites/default/files/2026-06/S19a_Notice_2026_June_M.pdf\">June</a>"}]"#;
        let urls = simplify_s19a_notice_urls(body, "https://www.simplify.us");
        assert!(urls[0].contains("2026-08"), "{urls:?}");
        assert!(urls.iter().any(|u| u.contains("2026-06")), "{urls:?}");
    }
}
