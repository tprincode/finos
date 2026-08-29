//! Shared issuer HTML date/amount parsing.

use chrono::NaiveDate;
use serde_json::{json, Value};

pub(crate) fn strip_html(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&nbsp;", " ")
        .replace("&#160;", " ")
        .replace("&#36;", "$")
        .replace("&dollar;", "$")
        .replace("&amp;", "&")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn parse_leading_dollars(after: &str) -> Option<(i64, u8)> {
    let token = after
        .trim_start()
        .split(|c: char| c.is_whitespace() || c == ',' || c == ';')
        .next()?
        .trim_start_matches('$')
        .trim_end_matches('.');
    if token.is_empty() {
        return None;
    }
    let (whole, frac) = match token.split_once('.') {
        Some((w, f)) => (w, f),
        None => (token, ""),
    };
    if whole.is_empty() || !whole.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if !frac.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut scale = if frac.is_empty() { 0u8 } else { frac.len() as u8 };
    let mut combined = format!("{whole}{frac}");
    if scale < 2 {
        combined.push_str(&"0".repeat((2 - scale) as usize));
        scale = 2;
    }
    scale = scale.min(6);
    let minor = combined.parse::<i64>().ok().filter(|n| *n > 0)?;
    Some((minor, scale))
}
pub(crate) fn parse_issuer_date(raw: &str) -> Option<String> {
    let s = raw.trim();
    // Never byte-slice UTF-8 (™ and similar appear in issuer page cells).
    let iso_prefix: String = s.chars().take(10).collect();
    if iso_prefix.chars().count() == 10
        && NaiveDate::parse_from_str(&iso_prefix, "%Y-%m-%d").is_ok()
    {
        return Some(iso_prefix);
    }
    for fmt in [
        "%m/%d/%Y",
        "%-m/%-d/%Y",
        "%m/%d/%y",
        "%B %d, %Y",
        "%b %d, %Y",
        "%B %-d, %Y",
        "%b %-d, %Y",
        "%b-%d-%Y",
    ] {
        if let Ok(d) = NaiveDate::parse_from_str(s, fmt) {
            return Some(d.format("%Y-%m-%d").to_string());
        }
    }
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() == 3 {
        let m: u32 = parts[0].parse().ok()?;
        let d: u32 = parts[1].parse().ok()?;
        let mut y: i32 = parts[2].parse().ok()?;
        if y < 100 {
            y += 2000;
        }
        return NaiveDate::from_ymd_opt(y, m, d).map(|dt| dt.format("%Y-%m-%d").to_string());
    }
    None
}

pub(crate) fn parse_issuer_amount(raw: &str) -> Option<(i64, u8)> {
    parse_leading_dollars(raw.trim().trim_start_matches('$'))
        .or_else(|| parse_leading_dollars(&format!("${}", raw.trim())))
}

pub(crate) fn html_td_rows(html: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let lower = html.to_ascii_lowercase();
    for (idx, _) in lower.match_indices("<tr") {
        let rest = html.get(idx..).unwrap_or("");
        let end = rest
            .to_ascii_lowercase()
            .find("</tr>")
            .unwrap_or(rest.len().min(4000));
        let row = rest.get(..end).unwrap_or("");
        if row.to_ascii_lowercase().contains("<th") {
            continue;
        }
        let mut cells = Vec::new();
        let row_l = row.to_ascii_lowercase();
        let mut search = 0usize;
        while let Some(rel) = row_l.get(search..).and_then(|s| s.find("<td")) {
            let start = search + rel;
            let after = row.get(start..).unwrap_or("");
            let close = after
                .to_ascii_lowercase()
                .find("</td>")
                .unwrap_or(after.len().min(400));
            let cell_html = after.get(..close).unwrap_or("");
            cells.push(strip_html(cell_html).trim().to_string());
            search = start + close + 5;
        }
        if !cells.is_empty() {
            rows.push(cells);
        }
    }
    rows
}

pub(crate) fn distribution_candidate(
    source: &str,
    pay_on: String,
    amount: Option<(i64, u8)>,
    roc_pct_minor: Option<i64>,
) -> Value {
    let mut row = json!({
        "paymentPeriod": pay_on,
        "source": source,
    });
    match amount {
        Some((amt, scale)) if amt > 0 => {
            row["amountPerShareMinor"] = json!(amt);
            row["amountScale"] = json!(scale);
        }
        _ => {
            row["amountPerShareMinor"] = Value::Null;
        }
    }
    if let Some(roc) = roc_pct_minor {
        row["rocPctMinor"] = json!(roc);
        row["rocScale"] = json!(2);
    }
    row
}

pub(crate) fn sort_newest_first(cands: &mut Vec<Value>) {
    cands.sort_by(|a, b| {
        let da = a
            .get("paymentPeriod")
            .and_then(|p| p.as_str())
            .unwrap_or("");
        let db = b
            .get("paymentPeriod")
            .and_then(|p| p.as_str())
            .unwrap_or("");
        db.cmp(da)
    });
}

pub(crate) fn candidate_amount(c: &Value) -> Option<i64> {
    c.get("amountPerShareMinor")
        .and_then(|x| if x.is_null() { None } else { x.as_i64() })
        .filter(|n| *n > 0)
}

pub(crate) fn upcoming_from_candidates(cands: &[Value], as_of: &str) -> Vec<Value> {
    cands
        .iter()
        .filter(|c| {
            c.get("paymentPeriod")
                .and_then(|p| p.as_str())
                .map(|p| p >= as_of)
                .unwrap_or(false)
        })
        .map(|c| {
            json!({
                "payOn": c.get("paymentPeriod").and_then(|p| p.as_str()).unwrap_or(""),
                "amountPerShareMinor": c.get("amountPerShareMinor").cloned(),
                "amountScale": c.get("amountScale").cloned(),
                "source": c.get("source").cloned()
            })
        })
        .collect()
}

/// Issuer fund-page link to a distribution calendar PDF (e.g. Simplify sidebar calendar).
pub(crate) fn distribution_calendar_url(html: &str, site_origin: &str) -> Option<String> {
    let origin = site_origin.trim().trim_end_matches('/');
    if origin.is_empty() {
        return None;
    }
    let lower = html.to_ascii_lowercase();
    let mut search = 0usize;
    while let Some(rel) = lower.get(search..).and_then(|s| s.find("href=\"")) {
        let start = search + rel + 6;
        let rest = html.get(start..)?;
        let end = rest.find('"')?;
        let href = rest.get(..end)?.trim();
        search = start + end + 1;
        let h = href.to_ascii_lowercase();
        if !(h.contains("distribution") && h.contains("calendar")) && !h.contains("distribution-calendar") {
            continue;
        }
        if !h.contains(".pdf") {
            continue;
        }
        return Some(if href.starts_with("http") {
            href.to_string()
        } else if href.starts_with('/') {
            format!("{origin}{href}")
        } else {
            format!("{origin}/{href}")
        });
    }
    None
}

#[cfg(test)]
mod calendar_url_tests {
    use super::*;

    #[test]
    fn simplify_distribution_calendar_pdf_link() {
        let html = r#"<div class="distribution-calendar-link">
  <a href="/sites/default/files/2026-05/Simplify-Distribution-Calendar-June-2026..pdf">DISTRIBUTION CALENDAR</a>
</div>"#;
        assert_eq!(
            distribution_calendar_url(html, "https://www.simplify.us").as_deref(),
            Some("https://www.simplify.us/sites/default/files/2026-05/Simplify-Distribution-Calendar-June-2026..pdf")
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_issuer_date_tolerates_utf8_trademark() {
        assert!(parse_issuer_date("Product™ Name").is_none());
        assert_eq!(
            parse_issuer_date("2026-08-26™").as_deref(),
            Some("2026-08-26")
        );
        assert_eq!(
            parse_issuer_date("07/31/2026").as_deref(),
            Some("2026-07-31")
        );
    }
}
