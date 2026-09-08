//! ET common-unit 8-K on EDGAR. Do not scrape ir.energytransfer.com (WAF).
//! Declared payable only. Remaining quarters keep Plan $ — do not copy this rate forward.

use serde_json::Value;

use super::super::edgar::{edgar_http_get, edgar_http_get_timeout};
use crate::retrieve::html::{
    distribution_candidate, parse_issuer_amount, parse_issuer_date, sort_newest_first, strip_html,
};

pub fn et_sec_atom_url() -> String {
    financial_domain::mlp_sec::atom_url()
}

pub fn parse_energytransfer_distributions(html: &str) -> Vec<Value> {
    let mut out = parse_common_unit_press(&strip_html(html));
    out.retain(|row| !looks_like_preferred_amount(row));
    sort_newest_first(&mut out);
    for row in &mut out {
        row["source"] = serde_json::json!(financial_domain::mlp_sec::SOURCE_SEC_8K);
    }
    out
}

fn looks_like_preferred_amount(row: &Value) -> bool {
    row.get("amountPerShareMinor").and_then(|v| v.as_i64()) == Some(2111)
        && row.get("amountScale").and_then(|v| v.as_u64()) == Some(4)
}

fn parse_common_unit_press(text: &str) -> Vec<Value> {
    let mut out = Vec::new();
    push_common_unit_windows(text, &mut out);
    push_cash_distribution_windows(text, &mut out);
    out
}

fn push_parsed_window(window: &str, out: &mut Vec<Value>) {
    let wlow = window.to_ascii_lowercase();
    if window_is_preferred_only(&wlow) {
        return;
    }
    if !wlow.contains("common unit") || !wlow.contains("distribution") {
        return;
    }
    let Some(amount) = amount_before_common(window).or_else(|| distribution_dollar_amount(window))
    else {
        return;
    };
    let Some(pay) = date_after(
        window,
        &[
            "will be paid on ",
            "paid on ",
            "payable on ",
            "payment date of ",
            "paid ",
            "payable ",
        ],
    ) else {
        return;
    };
    let mut row = distribution_candidate(
        financial_domain::mlp_sec::SOURCE_SEC_8K,
        pay,
        Some(amount),
        None,
    );
    if let Some(rec) = date_after(
        window,
        &[
            "record as of the close of business on ",
            "record as of ",
            "unitholders of record as of the close of business on ",
            "unitholders of record as of ",
        ],
    ) {
        row["recordDate"] = serde_json::json!(rec);
    }
    if !out
        .iter()
        .any(|e: &Value| e["paymentPeriod"] == row["paymentPeriod"])
    {
        out.push(row);
    }
}

fn push_common_unit_windows(text: &str, out: &mut Vec<Value>) {
    let lower = text.to_ascii_lowercase();
    let mut search = 0usize;
    while let Some(rel) = lower.get(search..).and_then(|s| s.find("common unit")) {
        let at = search + rel;
        let start = at.saturating_sub(240);
        let end = (at + 360).min(text.len());
        let window = text.get(start..end).unwrap_or("");
        search = at + 11;
        push_parsed_window(window, out);
    }
}

fn push_cash_distribution_windows(text: &str, out: &mut Vec<Value>) {
    let lower = text.to_ascii_lowercase();
    for needle in ["cash distribution", "quarterly distribution"] {
        let mut search = 0usize;
        while let Some(rel) = lower.get(search..).and_then(|s| s.find(needle)) {
            let at = search + rel;
            let start = at.saturating_sub(80);
            let end = (at + 420).min(text.len());
            let window = text.get(start..end).unwrap_or("");
            search = at + needle.len();
            push_parsed_window(window, out);
        }
    }
}

fn distribution_dollar_amount(window: &str) -> Option<(i64, u8)> {
    let low = window.to_ascii_lowercase();
    for marker in [
        "distribution of $",
        "distribution to $",
        "cash distribution of $",
        "cash distribution to $",
        "quarterly cash distribution to $",
        "quarterly cash distribution of $",
    ] {
        if let Some(i) = low.find(marker) {
            let at = window.get(i..)?.find('$')?;
            return parse_issuer_amount(&window[i + at..]);
        }
    }
    None
}

fn window_is_preferred_only(wlow: &str) -> bool {
    let preferred = wlow.contains("preferred")
        || wlow.contains("series a")
        || wlow.contains("series b")
        || wlow.contains("series g")
        || wlow.contains("series h")
        || wlow.contains("series i");
    if !preferred {
        return false;
    }
    !(wlow.contains("per energy transfer common") || wlow.contains("per common unit"))
}

fn amount_before_common(window: &str) -> Option<(i64, u8)> {
    let low = window.to_ascii_lowercase();
    let idx = low
        .find("per energy transfer common unit")
        .or_else(|| low.find("per common unit"))?;
    let before = window.get(..idx)?;
    let dollar = before.rfind('$')?;
    parse_issuer_amount(&before[dollar..])
}

fn date_after(window: &str, markers: &[&str]) -> Option<String> {
    let low = window.to_ascii_lowercase();
    for marker in markers {
        let Some(i) = low.find(marker) else {
            continue;
        };
        let rest = window.get(i + marker.len()..)?.trim_start();
        let words: Vec<&str> = rest.split_whitespace().take(3).collect();
        if words.len() < 3 {
            continue;
        }
        let chunk = format!(
            "{} {}, {}",
            words[0],
            words[1].trim_end_matches(','),
            words[2].trim_end_matches(',')
        );
        if let Some(d) = parse_issuer_date(&chunk) {
            return Some(d);
        }
    }
    None
}

fn atom_title_is_preferred_only(title: &str) -> bool {
    let t = title.to_ascii_lowercase();
    t.contains("preferred")
        || t.contains("series i")
        || t.contains("series b")
        || t.contains("series g")
        || t.contains("series h")
}

fn atom_title_is_common_distribution(title: &str) -> bool {
    let t = title.to_ascii_lowercase();
    if atom_title_is_preferred_only(&t) {
        return false;
    }
    t.contains("quarterly cash distribution")
        || t.contains("quarterly distribution")
        || t.contains("cash distribution")
        || (t.contains("common unit") && t.contains("distribution"))
        || (t.contains("8-k") && t.contains("current report") && !t.contains("8-k12"))
}

fn atom_entry_is_8k(block: &str) -> bool {
    let filing = tag_text(block, "filing-type")
        .or_else(|| tag_text(block, "category"))
        .unwrap_or_default()
        .to_ascii_lowercase();
    let title = tag_text(block, "title").unwrap_or_default();
    if atom_title_is_preferred_only(&title) {
        return false;
    }
    if filing.contains("12b") || title.to_ascii_lowercase().contains("8-k12") {
        return false;
    }
    atom_title_is_common_distribution(&title)
        || filing.trim() == "8-k"
        || filing.contains("term=\"8-k\"")
}

fn atom_filing_href(block: &str) -> Option<String> {
    if let Some(href) = tag_text(block, "filing-href") {
        let href = href.replace("&amp;", "&");
        if href.contains("sec.gov") && href.contains("-index") {
            return Some(href);
        }
    }
    if let Some(href) = attr_in(block, "href") {
        if href.contains("sec.gov") && (href.contains("-index") || href.contains("/Archives/")) {
            return Some(href);
        }
    }
    let block_l = block.to_ascii_lowercase();
    let mut search = 0usize;
    while let Some(rel) = block_l.get(search..).and_then(|s| s.find("https://www.sec.gov/")) {
        let start = search + rel;
        let url: String = block[start..]
            .chars()
            .take_while(|c| !c.is_whitespace() && *c != '"' && *c != '<' && *c != '\'')
            .collect();
        search = start + 8;
        if url.contains("-index") || url.contains("/Archives/edgar/data/") {
            return Some(url.replace("&amp;", "&"));
        }
    }
    None
}

fn atom_entry_hrefs(atom: &str) -> Vec<String> {
    let mut out = Vec::new();
    let lower = atom.to_ascii_lowercase();
    let mut search = 0usize;
    while let Some(rel) = lower.get(search..).and_then(|s| s.find("<entry")) {
        let start = search + rel;
        let rest = atom.get(start..).unwrap_or("");
        let end = rest
            .to_ascii_lowercase()
            .find("</entry>")
            .unwrap_or(rest.len().min(8_000));
        let block = rest.get(..end).unwrap_or("");
        search = start + end + 8;
        if !atom_entry_is_8k(block) {
            continue;
        }
        if let Some(href) = atom_filing_href(block) {
            out.push(href);
        }
    }
    out
}

fn tag_text(block: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let low = block.to_ascii_lowercase();
    let start = low.find(&open)?;
    let after = block.get(start..)?;
    let gt = after.find('>')?;
    let body = after.get(gt + 1..)?;
    let end = body.to_ascii_lowercase().find(&close)?;
    Some(strip_html(&body[..end]))
}

fn attr_in(block: &str, attr: &str) -> Option<String> {
    let needle = format!("{attr}=\"");
    let low = block.to_ascii_lowercase();
    let start = low.find(&needle)? + needle.len();
    let rest = block.get(start..)?;
    let end = rest.find('"')?;
    Some(rest[..end].replace("&amp;", "&").to_string())
}

fn index_exhibit_urls(index_html: &str, index_url: &str) -> Vec<String> {
    let origin = "https://www.sec.gov";
    let dir = match index_url.rfind('/') {
        Some(i) => &index_url[..=i],
        None => index_url,
    };
    let low = index_html.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut search = 0usize;
    while let Some(rel) = low.get(search..).and_then(|s| s.find("href=\"")) {
        let start = search + rel + 6;
        search = start;
        let rest = index_html.get(start..).unwrap_or("");
        let end = rest.find('"').unwrap_or(0);
        let href = rest.get(..end).unwrap_or("").replace("&amp;", "&");
        let hlow = href.to_ascii_lowercase();
        if !hlow.ends_with(".htm") && !hlow.ends_with(".html") {
            continue;
        }
        if hlow.contains("xsl")
            || hlow.contains("searchedgar")
            || hlow.starts_with("/ix?")
            || hlow == "/index.htm"
            || hlow.ends_with("/index.htm")
        {
            continue;
        }
        let abs = if href.starts_with("http") {
            href
        } else if href.starts_with('/') {
            format!("{origin}{href}")
        } else {
            format!("{dir}{href}")
        };
        let score = hlow.contains("ex99") || hlow.contains("ex-99") || hlow.contains("press");
        if score {
            out.insert(0, abs);
        } else if out.len() < 6 {
            out.push(abs);
        }
    }
    out
}

pub enum MlpSecFetch {
    Page(String, String),
    Sec403,
    Empty,
}

/// ATOM then 8-K exhibit. Declared common payables only. Never GETs IR.
pub fn live_energytransfer_sec_body() -> Option<(String, String)> {
    match live_mlp_sec_8k_fetch() {
        MlpSecFetch::Page(url, body) => Some((url, body)),
        _ => None,
    }
}

const SUBMISSIONS_URL: &str = "https://data.sec.gov/submissions/CIK0001276187.json";

fn submissions_8k_doc_urls(json: &str) -> Vec<String> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    let Some(recent) = v.get("filings").and_then(|f| f.get("recent")) else {
        return Vec::new();
    };
    let Some(forms) = recent.get("form").and_then(|x| x.as_array()) else {
        return Vec::new();
    };
    let Some(accs) = recent.get("accessionNumber").and_then(|x| x.as_array()) else {
        return Vec::new();
    };
    let items = recent.get("items").and_then(|x| x.as_array());
    let mut out = Vec::new();
    for i in 0..forms.len() {
        let form = forms
            .get(i)
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !form.starts_with("8-k") || form.contains("12b") {
            continue;
        }
        let acc = accs.get(i).and_then(|x| x.as_str()).unwrap_or("");
        if acc.is_empty() {
            continue;
        }
        let item = items
            .and_then(|a| a.get(i))
            .and_then(|x| x.as_str())
            .unwrap_or("");
        let debt_deal = item.contains("1.01") && (item.contains("2.03") || item.contains("2.01"));
        if debt_deal {
            continue;
        }
        let useful = item.contains("7.01") || item.contains("2.02") || item.contains("8.01");
        if !useful {
            continue;
        }
        let acc_flat = acc.replace('-', "");
        out.push(format!(
            "https://www.sec.gov/Archives/edgar/data/1276187/{acc_flat}/{acc}-index.htm"
        ));
        if out.len() >= 1 {
            break;
        }
    }
    out
}

fn fetch_first_common_unit(urls: impl IntoIterator<Item = String>) -> Option<(String, String)> {
    for url in urls {
        let Ok(html) = edgar_http_get_timeout(&url, 180) else {
            continue;
        };
        if !parse_energytransfer_distributions(&html).is_empty() {
            return Some((url, html));
        }
        if url.to_ascii_lowercase().contains("-index") {
            for doc in index_exhibit_urls(&html, &url).into_iter().take(1) {
                let Ok(body) = edgar_http_get_timeout(&doc, 180) else {
                    continue;
                };
                if !parse_energytransfer_distributions(&body).is_empty() {
                    return Some((doc, body));
                }
            }
        }
    }
    None
}

pub fn live_mlp_sec_8k_fetch() -> MlpSecFetch {
    match edgar_http_get(SUBMISSIONS_URL) {
        Ok(json) => {
            let urls = submissions_8k_doc_urls(&json);
            if !urls.is_empty() {
                return match fetch_first_common_unit(urls) {
                    Some(hit) => MlpSecFetch::Page(hit.0, hit.1),
                    None => MlpSecFetch::Empty,
                };
            }
        }
        Err(e) => {
            let low = e.to_ascii_lowercase();
            if low.contains("403") || low.contains("undeclared") {
                return MlpSecFetch::Sec403;
            }
        }
    }
    let atom = match edgar_http_get(&et_sec_atom_url()) {
        Ok(body) => body,
        Err(e) => {
            let low = e.to_ascii_lowercase();
            if low.contains("403") || low.contains("undeclared") {
                return MlpSecFetch::Sec403;
            }
            return MlpSecFetch::Empty;
        }
    };
    match fetch_first_common_unit(atom_entry_hrefs(&atom).into_iter().take(4)) {
        Some(hit) => MlpSecFetch::Page(hit.0, hit.1),
        None => MlpSecFetch::Empty,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atom_keeps_common_distribution_titles() {
        assert!(atom_title_is_common_distribution(
            "Energy Transfer Announces Quarterly Cash Distribution"
        ));
        assert!(atom_title_is_common_distribution(
            "8-K - Energy Transfer Announces Increase in Quarterly Cash Distribution"
        ));
        assert!(!atom_title_is_common_distribution(
            "8-K - Energy Transfer Announces Series I Preferred Distribution"
        ));
        assert!(atom_title_is_common_distribution("8-K - Current report"));
        assert!(!atom_title_is_common_distribution(
            "8-K12B - Notification that a class of securities of successor issuer is deemed to be registered pursuant to section 12(b)"
        ));
        let atom = r#"<feed><entry><title>8-K - Energy Transfer Announces Increase in Quarterly Cash Distribution</title>
<link href="https://www.sec.gov/Archives/edgar/data/1276187/000127618726000088/0001276187-26-000088-index.htm"/></entry>
<entry><title>8-K - Energy Transfer Announces Series I Preferred Distribution</title>
<link href="https://www.sec.gov/Archives/edgar/data/1276187/pref-index.htm"/></entry></feed>"#;
        let hrefs = atom_entry_hrefs(atom);
        assert_eq!(hrefs.len(), 1);
        assert!(hrefs[0].contains("000088-index"));
        let live = r#"<feed><entry>
<title>8-K - Current report</title>
<category label="form type" scheme="https://www.sec.gov/" term="8-K" />
<content type="text/xml">
<file-number-href>https://www.sec.gov/cgi-bin/browse-edgar?action=getcompany&amp;filenum=001-32740&amp;owner=exclude&amp;count=20</file-number-href>
<filing-href>https://www.sec.gov/Archives/edgar/data/1276187/000127618726000033/0001276187-26-000033-index.htm</filing-href>
<filing-type>8-K</filing-type>
</content></entry></feed>"#;
        let live_hrefs = atom_entry_hrefs(live);
        assert_eq!(live_hrefs.len(), 1, "{live_hrefs:?}");
        assert!(
            live_hrefs[0].contains("0001276187-26-000033-index"),
            "{live_hrefs:?}"
        );
        let urls = submissions_8k_doc_urls(
            r#"{
              "filings":{"recent":{
                "form":["8-K","8-K","8-K12B","8-K"],
                "accessionNumber":["0001276187-26-000033","0001193125-26-309155","0001193125-26-295529","0001193125-26-298149"],
                "items":["2.02,9.01","1.01,2.03,9.01","3.03,5.03,8.01,9.01","1.01,8.01,9.01"],
                "primaryDocument":["et-20260804.htm","d99342d8k.htm","d109647d8k12b.htm","d86030d8k.htm"]
              }}
            }"#,
        );
        assert!(
            urls.iter().any(|u| u.contains("0001276187-26-000033-index")),
            "{urls:?}"
        );
        assert!(
            urls.iter().all(|u| !u.contains("0001193125-26-309155")),
            "skip debt 8-K: {urls:?}"
        );
    }

    #[test]
    fn earnings_8k_sentence_parses_common_unit_distribution() {
        let html = "For the quarter, net income per common unit (basic) was $0.59. \
            On July 28, 2026, the Partnership announced a quarterly cash distribution of \
            $0.3400 per common unit, which will be paid on August 19, 2026 to unitholders \
            of record as of August 7, 2026.";
        let rows = parse_energytransfer_distributions(html);
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(rows[0]["paymentPeriod"], "2026-08-19");
        assert_eq!(rows[0]["amountPerShareMinor"], 3400);
    }
}
