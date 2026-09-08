//! Shared issuer HTML date/amount parsing.

use chrono::{Datelike, NaiveDate};
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
            // `%Y` accepts `08/14/26` as year 26; issuer two-digit years are 20xx.
            if d.year() < 100 {
                return NaiveDate::from_ymd_opt(d.year() + 2000, d.month(), d.day())
                    .map(|dt| dt.format("%Y-%m-%d").to_string());
            }
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

/// Column role from an issuer table or CSV header. Pay date is found by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ColumnRole {
    PayDate,
    Amount,
    DeclarationDate,
    ExDate,
    RecordDate,
    Roc,
    Other,
}

fn normalize_header(raw: &str) -> String {
    strip_html(raw)
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Classify a header. Ex / record / declaration are never the week key.
pub(crate) fn classify_header(raw: &str) -> ColumnRole {
    let n = normalize_header(raw);
    if n.is_empty() {
        return ColumnRole::Other;
    }
    if is_pay_header(&n) {
        return ColumnRole::PayDate;
    }
    if is_ex_header(&n) {
        return ColumnRole::ExDate;
    }
    if n.contains("record") {
        return ColumnRole::RecordDate;
    }
    if n == "roc" || n.starts_with("roc ") || n.contains("return of capital") {
        return ColumnRole::Roc;
    }
    if is_amount_header(&n) {
        return ColumnRole::Amount;
    }
    if n.contains("declar") {
        return ColumnRole::DeclarationDate;
    }
    ColumnRole::Other
}

fn is_pay_header(n: &str) -> bool {
    if n.contains("ex ") || (n.starts_with("ex") && n.contains("div")) {
        return false;
    }
    n.contains("payable")
        || n.contains("payout")
        || n == "pay"
        || n.contains("pay date")
        || n.contains("payment date")
        || n.contains("paid date")
        || n.contains("distribution date")
        || n == "distributiondate"
        || (n.contains("pay") && n.contains("date"))
}

fn is_ex_header(n: &str) -> bool {
    n.contains("ex date")
        || n.contains("ex div")
        || n.contains("ex dividend")
        || n == "ex"
        || n.starts_with("ex ")
}

fn is_amount_header(n: &str) -> bool {
    if n.contains("annualized") {
        return false;
    }
    if n.contains("date") && !n.contains("amount") {
        return false;
    }
    n.contains("amount")
        || n.contains("per share")
        || n.contains("per unit")
        || n.contains("distribution")
        || n.contains("dividend")
        || n.contains("cash")
}

fn parse_roc_cell(raw: &str) -> Option<i64> {
    let s = raw.trim().to_ascii_lowercase();
    if s.is_empty() || s.contains("nan") || s == "—" || s == "-" {
        return None;
    }
    let t = s.trim_end_matches('%').trim();
    let n: f64 = t.parse().ok()?;
    if !(0.0..=100.0).contains(&n) {
        return None;
    }
    Some((n * 100.0).round() as i64)
}

fn extract_tagged_cells(row: &str, tag: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let row_l = row.to_ascii_lowercase();
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut search = 0usize;
    while let Some(rel) = row_l.get(search..).and_then(|s| s.find(open.as_str())) {
        let start = search + rel;
        let after = row.get(start..).unwrap_or("");
        let close_at = after
            .to_ascii_lowercase()
            .find(close.as_str())
            .unwrap_or(after.len().min(800));
        let cell_html = after.get(..close_at).unwrap_or("");
        cells.push(strip_html(cell_html).trim().to_string());
        search = start + close_at + close.len();
    }
    cells
}

struct HtmlTable {
    roles: Vec<ColumnRole>,
    rows: Vec<Vec<String>>,
}

fn parse_one_table(table_html: &str) -> Option<HtmlTable> {
    let lower = table_html.to_ascii_lowercase();
    let mut headers: Option<Vec<String>> = None;
    let mut rows = Vec::new();
    let mut search = 0usize;
    while let Some(rel) = lower.get(search..).and_then(|s| s.find("<tr")) {
        let start = search + rel;
        let rest = table_html.get(start..).unwrap_or("");
        let rest_l = rest.to_ascii_lowercase();
        let after_open = rest_l.find('>').map(|i| i + 1).unwrap_or(0);
        let close = rest_l.find("</tr>");
        let next_tr = rest_l
            .get(after_open..)
            .and_then(|s| s.find("<tr"))
            .map(|i| after_open + i);
        // Q4 IR tables omit </tr> on the header row; the next <tr> implied-closes it.
        let end = match (close, next_tr) {
            (Some(c), Some(n)) => c.min(n),
            (Some(c), None) => c,
            (None, Some(n)) => n,
            (None, None) => rest.len().min(8000),
        };
        let row = rest.get(..end).unwrap_or("");
        search = start
            + if close == Some(end) {
                end + 5
            } else {
                end
            };
        let th = extract_tagged_cells(row, "th");
        if !th.is_empty() {
            headers = Some(th);
            continue;
        }
        let td = extract_tagged_cells(row, "td");
        if td.is_empty() {
            continue;
        }
        if headers.is_none() {
            let looks_like_header = td.iter().any(|c| classify_header(c) == ColumnRole::PayDate)
                && td.iter().all(|c| parse_issuer_date(c).is_none());
            if looks_like_header {
                headers = Some(td);
                continue;
            }
        }
        rows.push(td);
    }
    let headers = headers?;
    let roles: Vec<ColumnRole> = headers.iter().map(|h| classify_header(h)).collect();
    if !roles.iter().any(|r| *r == ColumnRole::PayDate) {
        return None;
    }
    Some(HtmlTable { roles, rows })
}

fn html_distribution_tables(html: &str) -> Vec<HtmlTable> {
    let mut tables = Vec::new();
    let lower = html.to_ascii_lowercase();
    let mut search = 0usize;
    while let Some(rel) = lower.get(search..).and_then(|s| s.find("<table")) {
        let start = search + rel;
        let rest = html.get(start..).unwrap_or("");
        let end = rest
            .to_ascii_lowercase()
            .find("</table>")
            .unwrap_or(rest.len().min(80_000));
        let table_html = rest.get(..end).unwrap_or("");
        search = start + 6;
        let after_open = table_html
            .find('>')
            .and_then(|i| table_html.get(i + 1..))
            .unwrap_or("");
        if after_open.to_ascii_lowercase().contains("<table") {
            continue;
        }
        if let Some(t) = parse_one_table(table_html) {
            tables.push(t);
        }
    }
    tables
}

fn row_is_total(cells: &[String]) -> bool {
    cells
        .first()
        .map(|c| c.to_ascii_lowercase().contains("total"))
        .unwrap_or(false)
}

/// Collector primitive: find the pay-date column by header name, take the
/// declaration amount from the amount column, and key `paymentPeriod` to pay date.
/// Declaration / ex / record dates may be present; they are never the week key.
pub(crate) fn parse_distribution_tables(source: &str, html: &str) -> Vec<Value> {
    let mut out = Vec::new();
    for table in html_distribution_tables(html) {
        let Some(pay_idx) = table.roles.iter().position(|r| *r == ColumnRole::PayDate) else {
            continue;
        };
        let amount_idx = table.roles.iter().position(|r| *r == ColumnRole::Amount);
        let roc_idx = table.roles.iter().position(|r| *r == ColumnRole::Roc);
        let record_idx = table.roles.iter().position(|r| *r == ColumnRole::RecordDate);
        let ex_idx = table.roles.iter().position(|r| *r == ColumnRole::ExDate);
        for cells in table.rows {
            if row_is_total(&cells) {
                continue;
            }
            let Some(pay) = cells.get(pay_idx).and_then(|c| parse_issuer_date(c)) else {
                continue;
            };
            let amount = amount_idx
                .and_then(|i| cells.get(i))
                .and_then(|c| parse_issuer_amount(c));
            let roc = roc_idx
                .and_then(|i| cells.get(i))
                .and_then(|c| parse_roc_cell(c));
            let mut row = distribution_candidate(source, pay, amount, roc);
            if let Some(d) = record_idx
                .and_then(|i| cells.get(i))
                .and_then(|c| parse_issuer_date(c))
            {
                row["recordDate"] = json!(d);
            }
            if let Some(d) = ex_idx
                .and_then(|i| cells.get(i))
                .and_then(|c| parse_issuer_date(c))
            {
                row["exDate"] = json!(d);
            }
            out.push(row);
        }
    }
    sort_newest_first(&mut out);
    out
}

/// Map a row onto documented header names (JSON arrays / vendor schemas).
/// Pay date is the header classified as PayDate — never a positional guess.
pub(crate) fn candidate_from_headers(
    source: &str,
    headers: &[&str],
    cells: &[String],
) -> Option<Value> {
    let roles: Vec<ColumnRole> = headers.iter().map(|h| classify_header(h)).collect();
    let pay_idx = roles.iter().position(|r| *r == ColumnRole::PayDate)?;
    let amount_idx = roles.iter().position(|r| *r == ColumnRole::Amount);
    let roc_idx = roles.iter().position(|r| *r == ColumnRole::Roc);
    if row_is_total(cells) {
        return None;
    }
    let pay = cells.get(pay_idx).and_then(|c| parse_issuer_date(c))?;
    let amount = amount_idx
        .and_then(|i| cells.get(i))
        .and_then(|c| parse_issuer_amount(c));
    let roc = roc_idx
        .and_then(|i| cells.get(i))
        .and_then(|c| parse_roc_cell(c));
    Some(distribution_candidate(source, pay, amount, roc))
}
pub(crate) fn parse_distribution_csv(source: &str, text: &str) -> Vec<Value> {
    let mut lines = text.lines();
    let header = lines.next().unwrap_or("");
    let headers: Vec<String> = header
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .collect();
    let roles: Vec<ColumnRole> = headers.iter().map(|h| classify_header(h)).collect();
    let Some(pay_idx) = roles.iter().position(|r| *r == ColumnRole::PayDate) else {
        return Vec::new();
    };
    let amount_idx = roles.iter().position(|r| *r == ColumnRole::Amount);
    let mut out = Vec::new();
    for line in lines {
        let cols: Vec<String> = line
            .split(',')
            .map(|s| s.trim().trim_matches('"').to_string())
            .collect();
        if row_is_total(&cols) {
            continue;
        }
        let Some(pay) = cols.get(pay_idx).and_then(|c| parse_issuer_date(c)) else {
            continue;
        };
        let amount = amount_idx
            .and_then(|i| cols.get(i))
            .and_then(|c| parse_issuer_amount(c));
        out.push(distribution_candidate(source, pay, amount, None));
    }
    sort_newest_first(&mut out);
    out
}

const JSON_PAY_DATE_KEYS: &[&str] = &[
    "PayableDate",
    "payableDate",
    "paymentDate",
    "pay_date",
    "PayDate",
    "payable_date",
    "payment_date",
    "payDate",
    "Payable",
];

const JSON_AMOUNT_KEYS: &[&str] = &[
    "CashDividendPerShare",
    "dividendAmount",
    "amount",
    "Amount",
    "dividend",
    "Dividend",
    "CashAmount",
    "distribution",
    "Distribution",
];

fn json_cell_string(row: &Value, key: &str) -> Option<String> {
    match row.get(key)? {
        Value::String(s) if !s.trim().is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// JSON object: pay date by field name. Never declarationDate / exOrEffDate / ExDate.
pub(crate) fn json_pay_date(row: &Value) -> Option<String> {
    for key in JSON_PAY_DATE_KEYS {
        if let Some(raw) = json_cell_string(row, key) {
            if let Some(d) = parse_issuer_date(&raw) {
                return Some(d);
            }
        }
    }
    None
}

pub(crate) fn json_amount(row: &Value) -> Option<(i64, u8)> {
    for key in JSON_AMOUNT_KEYS {
        if let Some(raw) = json_cell_string(row, key) {
            let normalized = if raw.starts_with('.') {
                format!("0{raw}")
            } else {
                raw
            };
            if let Some(amt) = parse_issuer_amount(&normalized) {
                return Some(amt);
            }
        }
    }
    None
}

pub(crate) fn parse_issuer_amount(raw: &str) -> Option<(i64, u8)> {
    parse_leading_dollars(raw.trim().trim_start_matches('$'))
        .or_else(|| parse_leading_dollars(&format!("${}", raw.trim())))
}

#[allow(dead_code)]
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
        assert_eq!(
            parse_issuer_date("08/14/26").as_deref(),
            Some("2026-08-14")
        );
        assert_eq!(parse_issuer_date("8/14/26").as_deref(), Some("2026-08-14"));
    }

    #[test]
    fn classify_header_finds_pay_date_by_name() {
        assert_eq!(classify_header("Pay Date"), ColumnRole::PayDate);
        assert_eq!(classify_header("Payable"), ColumnRole::PayDate);
        assert_eq!(classify_header("Payable Date"), ColumnRole::PayDate);
        assert_eq!(classify_header("Payout Date"), ColumnRole::PayDate);
        assert_eq!(classify_header("Ex-Date"), ColumnRole::ExDate);
        assert_eq!(classify_header("Ex-Dividend Date"), ColumnRole::ExDate);
        assert_eq!(classify_header("Declaration Date"), ColumnRole::DeclarationDate);
        assert_eq!(classify_header("Declared"), ColumnRole::DeclarationDate);
        assert_eq!(classify_header("Amount (USD)"), ColumnRole::Amount);
        assert_eq!(classify_header("DISTRIBUTION PER SHARE"), ColumnRole::Amount);
        assert_eq!(classify_header("Total Distribution"), ColumnRole::Amount);
        assert_eq!(classify_header("ROC"), ColumnRole::Roc);
        // Energy Transfer / Nasdaq IR edge cases
        assert_eq!(classify_header("DistributionDate"), ColumnRole::PayDate);
        assert_eq!(classify_header("Quarterly$ Per Unit"), ColumnRole::Amount);
        assert_eq!(classify_header("Annualized $ Per Unit"), ColumnRole::Other);
        assert_eq!(classify_header("Payment Date"), ColumnRole::PayDate);
        assert_eq!(classify_header("Net Amount / Declared"), ColumnRole::Amount);
    }

    #[test]
    fn parse_uses_pay_date_column_not_last_date() {
        // Amount first, pay second, ex third, declaration last — last date is NOT pay date.
        let html = r#"<table>
<tr><th>Amount</th><th>Pay Date</th><th>Ex Date</th><th>Declaration Date</th></tr>
<tr><td>$0.17</td><td>9/30/2026</td><td>9/10/2026</td><td>6/17/2026</td></tr>
</table>"#;
        let rows = parse_distribution_tables("fixture", html);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["paymentPeriod"], "2026-09-30");
        assert_eq!(rows[0]["amountPerShareMinor"], 17);
        assert_ne!(rows[0]["paymentPeriod"], "2026-06-17");
        assert_ne!(rows[0]["paymentPeriod"], "2026-09-10");
    }

    #[test]
    fn parse_skips_table_without_pay_date_header() {
        let html = r#"<table>
<tr><th>Declaration Date</th><th>Ex Date</th><th>Amount</th></tr>
<tr><td>6/17/2026</td><td>9/10/2026</td><td>$0.17</td></tr>
</table>"#;
        assert!(parse_distribution_tables("fixture", html).is_empty());
    }
}
