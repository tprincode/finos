//! Remaining DIV-1 issuer adapters (one registered source per provider).
//! Table parse when GET exposes dates; otherwise loud miss — not Yahoo, not $0.
//! ProShares fund pages render distributions via `/api/distributionsummary/` (empty HTML tbody).

use serde_json::Value;

use crate::retrieve::html::{
    distribution_candidate, json_amount, json_pay_date, parse_distribution_tables,
    parse_issuer_amount, parse_issuer_date, sort_newest_first, strip_html,
};

use super::generic::{generic_fund_page, parse_generic_distributions};

pub struct Div1Issuer {
    pub source: &'static str,
    pub needles: &'static [&'static str],
    pub urls: fn(symbol: &str) -> Vec<String>,
}

fn urls_for(templates: &[&str], symbol: &str) -> Vec<String> {
    let sym = symbol.trim().to_ascii_lowercase();
    templates
        .iter()
        .map(|t| t.replace("{sym}", &sym))
        .collect()
}

fn proshares_urls(symbol: &str) -> Vec<String> {
    // BITO and peers live under category folders (e.g. /our-etfs/strategic/bito).
    // Bare /our-etfs/{sym} is often 404. Distributions load from the JSON API, not the HTML table.
    urls_for(
        &[
            "https://www.proshares.com/our-etfs/strategic/{sym}",
            "https://www.proshares.com/our-etfs/thematic/{sym}",
            "https://www.proshares.com/our-etfs/inflation/{sym}",
            "https://www.proshares.com/our-etfs/leveraged-and-inverse/{sym}",
            "https://www.proshares.com/our-etfs/{sym}",
            "https://www.proshares.com/{sym}",
        ],
        symbol,
    )
}

fn simplify_urls(symbol: &str) -> Vec<String> {
    let sym = symbol.trim().to_ascii_lowercase();
    // Drupal uses long slugs; distributions load from `/etfs/{node_id}/distributions`.
    let mut out = Vec::new();
    if sym == "svol" {
        out.push(super::simplify::simplify_distributions_url("536"));
        out.push(super::simplify::simplify_fund_url("svol"));
    }
    out.push(format!("https://www.simplify.us/etfs/{sym}"));
    out.push(format!("https://www.simplify.us/{sym}"));
    out
}

/// JPMorgan `#/dividends` tab: `FundsMarketingHandler/historicalData` JSON.
pub fn parse_jpmorgan_distributions(body: &str) -> Vec<Value> {
    let trimmed = body.trim();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
            let rows = match &v {
                Value::Array(a) => a.clone(),
                Value::Object(o) => o
                    .get("dividendsDistributionHistoryList")
                    .or_else(|| o.get("latestAvailableDistributionHistory"))
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
                out.push(distribution_candidate("jpmorgan", pay, amount, None));
            }
            if !out.is_empty() {
                sort_newest_first(&mut out);
                return out;
            }
        }
    }
    parse_distribution_tables("jpmorgan", body)
}

pub fn jpmorgan_cusip_from_seed(seed_url: &str, html: &str) -> Option<String> {
    if let Some(v) = html.split("id=\"cspCode\"").nth(1) {
        if let Some(start) = v.find("value=\"") {
            let rest = v.get(start + 7..).unwrap_or("");
            let end = rest.find('"').unwrap_or(0);
            let cusip = rest.get(..end).unwrap_or("").trim();
            if cusip.len() >= 8 {
                return Some(cusip.to_ascii_uppercase());
            }
        }
    }
    let path = seed_url.split('#').next().unwrap_or(seed_url);
    let tail = path.rsplit('/').next().unwrap_or("");
    for token in tail.split('-').rev() {
        let t = token.trim();
        if t.len() == 9
            && t.chars().take(5).all(|c| c.is_ascii_digit())
            && t.chars().nth(5).is_some_and(|c| c.is_ascii_alphabetic())
            && t.chars().skip(6).all(|c| c.is_ascii_digit())
        {
            return Some(t.to_ascii_uppercase());
        }
    }
    None
}

pub fn jpmorgan_historical_data_url(cusip: &str) -> String {
    format!(
        "https://am.jpmorgan.com/FundsMarketingHandler/historicalData?cusip={}&country=us&role=adv&userLoggedIn=false&language=en",
        cusip.trim().to_ascii_uppercase()
    )
}

/// Parse ProShares `/api/distributionsummary/?fund=…&year=…` JSON (array or wrapped).
pub fn parse_proshares_distribution_summary(source: &str, body: &str) -> Vec<Value> {
    let trimmed = body.trim();
    if !trimmed.starts_with('[') && !trimmed.starts_with('{') {
        return Vec::new();
    }
    let Ok(v) = serde_json::from_str::<Value>(trimmed) else {
        return Vec::new();
    };
    let rows = match v {
        Value::Array(a) => a,
        Value::Object(o) => o
            .get("data")
            .or_else(|| o.get("distributions"))
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
        out.push(distribution_candidate(source, pay, amount, None));
    }
    sort_newest_first(&mut out);
    out
}

fn cornerstone_urls(symbol: &str) -> Vec<String> {
    let sym = symbol.trim().to_ascii_uppercase();
    match sym.as_str() {
        "CLM" => vec![
            "https://www.cornerstonestrategicinvestmentfund.com/press-releases.html".into(),
            "https://cornerstonestrategicinvestmentfund.com/press-releases".into(),
            "https://www.cornerstonestrategicinvestmentfund.com/".into(),
        ],
        _ => vec![
            "https://www.cornerstonetotalreturnfund.com/press-releases.html".into(),
            "https://www.cornerstonetotalreturnfund.com/press-releases".into(),
            "https://www.cornerstonetotalreturnfund.com/".into(),
        ],
    }
}

pub const DIV1_ISSUERS: &[Div1Issuer] = &[
    Div1Issuer {
        source: "cornerstone",
        needles: &["cornerstone"],
        urls: cornerstone_urls,
    },
    Div1Issuer {
        source: "direxion",
        needles: &["direxion"],
        urls: |s| {
            let sym = s.trim().to_ascii_uppercase();
            if sym == "SOXL" {
                return vec![
                    "https://www.direxion.com/product/daily-semiconductor-bull-3x-etf".into(),
                    format!("https://www.direxion.com/etfs/{}", sym.to_ascii_lowercase()),
                ];
            }
            if sym == "TSLL" {
                return vec![
                    "https://www.direxion.com/product/daily-tsla-bull-and-bear-leveraged-single-stock-etfs".into(),
                    format!("https://www.direxion.com/etfs/{}", sym.to_ascii_lowercase()),
                ];
            }
            vec![
                format!("https://www.direxion.com/etfs/{}", sym.to_ascii_lowercase()),
                format!("https://www.direxion.com/product/{}", sym.to_ascii_lowercase()),
                format!("https://www.direxion.com/{}", sym.to_ascii_lowercase()),
            ]
        },
    },
    Div1Issuer {
        source: "proshares",
        needles: &["proshares"],
        urls: proshares_urls,
    },
    Div1Issuer {
        source: "saba",
        needles: &["saba"],
        urls: |s| urls_for(&["https://www.sabaetf.com/{sym}"], s),
    },
    Div1Issuer {
        source: "ellington",
        needles: &["ellington"],
        urls: |_| {
            vec![
        "https://www.ellingtonfinancial.com/dividends-common-stock/".into(),
                "https://ir.ellingtonfinancial.com/dividends".into(),
                "https://www.ellingtonfinancial.com/dividends".into(),
            ]
        },
    },
    Div1Issuer {
        source: "enterprise",
        needles: &["enterprise products", "enterpriseproducts"],
        urls: |_| {
            vec![
                "https://ir.enterpriseproducts.com/distribution-drip".into(),
                "https://www.enterpriseproducts.com/investors/distributions".into(),
            ]
        },
    },
    Div1Issuer {
        source: "energytransfer",
        needles: &["energy transfer", "energytransfer"],
        urls: |_| vec![super::energytransfer::et_sec_atom_url()],
    },
    Div1Issuer {
        source: "mlp_sec_8k",
        needles: &["mlp_sec_8k"],
        urls: |_| vec![super::energytransfer::et_sec_atom_url()],
    },
    Div1Issuer {
        source: "gladstone",
        needles: &["gladstone"],
        urls: |_| {
            vec![
                "https://www.gladstonecapital.com/newsroom/detail/394/gladstone-capital-announces-monthly-cash-distributions-for".into(),
                "https://www.gladstonecapital.com/newsroom".into(),
                "https://www.gladstonecapital.com/investors/stock-data/dividend-history".into(),
                "https://www.gladstonecapital.com/investors".into(),
                "https://www.gladstonecapital.com/".into(),
            ]
        },
    },
    Div1Issuer {
        source: "jpmorgan",
        needles: &["jpmorgan", "jp morgan"],
        urls: |s| {
            let sym = s.trim().to_ascii_uppercase();
            if sym == "JEPQ" {
                return vec![
                    "https://am.jpmorgan.com/us/en/asset-management/adv/products/jpmorgan-nasdaq-equity-premium-income-etf-etf-shares-46654q203#/dividends".into(),
                    "https://am.jpmorgan.com/us/en/asset-management/adv/products/jpmorgan-nasdaq-equity-premium-income-etf-etf-shares-46654q203".into(),
                    "https://am.jpmorgan.com/us/en/asset-management/adv/products/JEPQ".into(),
                ];
            }
            urls_for(
                &[
                    "https://am.jpmorgan.com/us/en/asset-management/adv/products/{sym}",
                    "https://am.jpmorgan.com/us/en/asset-management/liq/products/{sym}",
                ],
                s,
            )
        },
    },
    Div1Issuer {
        source: "mplx",
        needles: &["mplx"],
        urls: |_| {
            vec![
                "https://ir.mplx.com/CorporateProfile/stock-information/stock-information/default.aspx".into(),
                "https://ir.mplx.com/distributions".into(),
            ]
        },
    },
    Div1Issuer {
        source: "orchidisland",
        needles: &["orchid"],
        urls: |_| {
            vec![
                "https://ir.orchidislandcapital.com/stock-information/dividends-splits".into(),
                "https://www.orchidislandcapital.com/".into(),
            ]
        },
    },
    Div1Issuer {
        source: "globalx",
        needles: &["global x", "globalx"],
        urls: |s| {
            urls_for(
                &[
                    "https://www.globalxetfs.com/funds/{sym}",
                    "https://www.globalxetfs.com/funds/{sym}/",
                    "https://www.globalxetfs.com/filings-and-tax-supplements/{sym}",
                ],
                s,
            )
        },
    },
    Div1Issuer {
        source: "simplify",
        needles: &["simplify"],
        urls: simplify_urls,
    },
    Div1Issuer {
        source: "tappalpha",
        needles: &["tappalpha", "tapp alpha"],
        urls: |s| {
            urls_for(
                &[
                    "https://www.tappalphafunds.com/etfs/{sym}",
                    "https://www.tappalpha.com/{sym}",
                    "https://tappalpha.com/etfs/{sym}",
                ],
                s,
            )
        },
    },
    Div1Issuer {
        source: "trinity",
        needles: &["trinity"],
        urls: |_| {
            vec![
                "https://ir.trinitycap.com/stock-information/distributions".into(),
                "https://www.trincapinvestment.com/investors".into(),
            ]
        },
    },
    Div1Issuer {
        source: "ftvest",
        needles: &["ft vest", "first trust", "ftportfolios"],
        urls: |s| {
            urls_for(
                &[
                    "https://www.ftportfolios.com/Retail/Etf/EtfDividHistory.aspx?Ticker={sym}",
                    "https://www.ftportfolios.com/Retail/Etf/EtfSummary.aspx?Ticker={sym}",
                    "https://www.ftvest.com/{sym}",
                ],
                s,
            )
        },
    },
    Div1Issuer {
        source: "trex",
        needles: &["t-rex", "graniteshares", "trex", "rex"],
        urls: |s| {
            let sym = s.trim().to_ascii_lowercase();
            vec![
                format!("https://www.rexshares.com/{sym}/"),
                format!("https://www.rexshares.com/{sym}"),
                format!("https://www.graniteshares.com/{sym}"),
            ]
        },
    },
];

fn issuer(source: &str) -> Option<&'static Div1Issuer> {
    DIV1_ISSUERS
        .iter()
        .find(|i| i.source.eq_ignore_ascii_case(source.trim()))
}

pub fn parse_div1_distributions(source: &str, html: &str) -> Vec<Value> {
    if financial_domain::mlp_sec::is_adapter_kind(source) {
        let parsed = super::energytransfer::parse_energytransfer_distributions(html);
        if !parsed.is_empty() {
            return parsed;
        }
    }
    if source.eq_ignore_ascii_case("jpmorgan") {
        let api = parse_jpmorgan_distributions(html);
        if !api.is_empty() {
            return api;
        }
    }
    if source.eq_ignore_ascii_case("proshares") {
        let api = parse_proshares_distribution_summary(source, html);
        if !api.is_empty() {
            return api;
        }
    }
    if source.eq_ignore_ascii_case("tappalpha") {
        let api = parse_tappalpha_distributions(html);
        if !api.is_empty() {
            return api;
        }
    }
    if source.eq_ignore_ascii_case("trinity") {
        let parsed = parse_trinity_distributions(html);
        if !parsed.is_empty() {
            return parsed;
        }
    }
    if source.eq_ignore_ascii_case("gladstone") {
        let press = parse_gladstone_press(html);
        if !press.is_empty() {
            return press;
        }
    }
    if source.eq_ignore_ascii_case("trex") {
        let parsed = parse_rexshares_distributions(html);
        if !parsed.is_empty() {
            return parsed;
        }
    }
    if source.eq_ignore_ascii_case("globalx") {
        let history = parse_globalx_distribution_history(html);
        if !history.is_empty() {
            return history;
        }
        let notices = parse_globalx_19a_notice(html);
        if !notices.is_empty() {
            return notices;
        }
    }
    parse_generic_distributions(source, html)
}

/// TappAlpha NavStar `view=all` JSON: `distributions[].pay_date` + `amount`.
pub fn parse_tappalpha_distributions(body: &str) -> Vec<Value> {
    let trimmed = body.trim();
    if !trimmed.starts_with('{') && !trimmed.starts_with('[') {
        return Vec::new();
    }
    let Ok(v) = serde_json::from_str::<Value>(trimmed) else {
        return Vec::new();
    };
    let rows = match &v {
        Value::Array(a) => a.clone(),
        Value::Object(o) => o
            .get("distributions")
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
        let roc = row
            .get("roc_estimate_pct")
            .and_then(|x| x.as_f64())
            .filter(|n| (0.0..=100.0).contains(n))
            .map(|n| (n * 100.0).round() as i64);
        out.push(distribution_candidate("tappalpha", pay, amount, roc));
    }
    sort_newest_first(&mut out);
    out
}

pub fn tappalpha_distributions_url(symbol: &str) -> String {
    format!(
        "https://jdkfnvgkfwotjlyovbrk.supabase.co/functions/v1/fund-public-api?ticker={}&view=all",
        symbol.trim().to_ascii_uppercase()
    )
}

pub fn tappalpha_fund_page_url(symbol: &str) -> String {
    format!(
        "https://www.tappalphafunds.com/etfs/{}",
        symbol.trim().to_ascii_lowercase()
    )
}

/// Cornerstone press PDF text: `CLM July 15, 2026 July 31, 2026 $0.1215` (record, payable, amount).
pub fn parse_cornerstone_press(text: &str, symbol: &str) -> Vec<Value> {
    let sym = symbol.trim().to_ascii_uppercase();
    if sym.is_empty() {
        return Vec::new();
    }
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < tokens.len() {
        let tok = tokens[i].trim_matches(|c: char| !c.is_ascii_alphanumeric());
        if tok.eq_ignore_ascii_case(&sym) {
            if let Some((record, after_record)) = take_issuer_date(&tokens, i + 1) {
                let _ = record;
                if let Some((pay, after_pay)) = take_issuer_date(&tokens, after_record) {
                    if let Some(amount) = tokens
                        .get(after_pay)
                        .and_then(|t| parse_issuer_amount(t))
                    {
                        out.push(distribution_candidate("cornerstone", pay, Some(amount), None));
                        i = after_pay + 1;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
    sort_newest_first(&mut out);
    out
}

fn take_issuer_date(tokens: &[&str], start: usize) -> Option<(String, usize)> {
    let first = tokens.get(start)?;
    if let Some(d) = parse_issuer_date(first) {
        return Some((d, start + 1));
    }
    if start + 2 < tokens.len() {
        let joined = format!(
            "{} {} {}",
            tokens[start],
            tokens[start + 1],
            tokens[start + 2]
        );
        if let Some(d) = parse_issuer_date(&joined) {
            return Some((d, start + 3));
        }
    }
    None
}

pub fn cornerstone_candidates_table(cands: &[Value]) -> String {
    let mut html = String::from(
        "<table><tr><th>Payable Date</th><th>Distribution Amount</th></tr>",
    );
    for c in cands {
        let pay = c
            .get("paymentPeriod")
            .and_then(|p| p.as_str())
            .unwrap_or("");
        let amount = c
            .get("amountPerShareMinor")
            .and_then(|a| a.as_i64())
            .filter(|n| *n > 0);
        let scale = c.get("amountScale").and_then(|s| s.as_u64()).unwrap_or(2) as u32;
        if pay.is_empty() || amount.is_none() {
            continue;
        }
        let minor = amount.unwrap();
        let pow = 10i64.pow(scale);
        html.push_str(&format!(
            "<tr><td>{pay}</td><td>${}.{:0width$}</td></tr>",
            minor / pow,
            minor % pow,
            width = scale as usize
        ));
    }
    html.push_str("</table>");
    html
}

/// Nasdaq IR dividend table: pay date and amount located by header name.
pub fn parse_trinity_distributions(html: &str) -> Vec<Value> {
    parse_distribution_tables("trinity", html)
}

pub fn ftvest_history_url(symbol: &str) -> String {
    format!(
        "https://www.ftportfolios.com/Retail/Etf/EtfDividHistory.aspx?Ticker={}",
        symbol.trim().to_ascii_uppercase()
    )
}

/// Year values from the FT Vest Distribution History dropdown.
pub fn ftvest_history_years(html: &str) -> Vec<String> {
    let marker = "ddlDistributionHistoryYearSelection";
    let Some(start) = html.find(marker) else {
        return Vec::new();
    };
    let rest = html.get(start..).unwrap_or("");
    let end = rest
        .to_ascii_lowercase()
        .find("</select>")
        .unwrap_or(rest.len().min(4000));
    let block = rest.get(..end).unwrap_or("");
    let mut years = Vec::new();
    let lower = block.to_ascii_lowercase();
    let mut search = 0usize;
    while let Some(rel) = lower.get(search..).and_then(|s| s.find("<option")) {
        let chunk = block.get(search + rel..).unwrap_or("");
        let close = chunk.find('>').unwrap_or(chunk.len().min(200));
        let tag = chunk.get(..close).unwrap_or("");
        if let Some(v) = attr_value(tag, "value") {
            let y = v.trim();
            if y.len() == 4 && y.chars().all(|c| c.is_ascii_digit()) {
                years.push(y.to_string());
            }
        }
        search = search + rel + 7;
    }
    years
}

pub fn html_input_value(html: &str, name: &str) -> Option<String> {
    let mut search = 0usize;
    while let Some(rel) = html.get(search..).and_then(|s| s.find("<input")) {
        let chunk = html.get(search + rel..).unwrap_or("");
        let close = chunk.find('>').unwrap_or(chunk.len().min(800));
        let tag = chunk.get(..close).unwrap_or("");
        if attr_value(tag, "name").as_deref() == Some(name) {
            return attr_value(tag, "value");
        }
        search = search + rel + 6;
    }
    None
}

fn attr_value(tag: &str, attr: &str) -> Option<String> {
    let needle = format!("{attr}=");
    let lower = tag.to_ascii_lowercase();
    let start = lower.find(&needle.to_ascii_lowercase())? + needle.len();
    let rest = tag.get(start..)?;
    let quote = rest.chars().next()?;
    if quote == '"' || quote == '\'' {
        let body = rest.get(1..)?;
        let end = body.find(quote)?;
        return Some(body.get(..end)?.to_string());
    }
    let end = rest
        .find(|c: char| c.is_whitespace() || c == '>')
        .unwrap_or(rest.len());
    Some(rest.get(..end)?.to_string())
}

pub fn ftvest_history_form(html: &str, year: &str) -> Vec<(String, String)> {
    let mut form = Vec::new();
    for name in [
        "__EVENTTARGET",
        "__EVENTARGUMENT",
        "__VIEWSTATE",
        "__VIEWSTATEGENERATOR",
        "__VIEWSTATEENCRYPTED",
        "__PREVIOUSPAGE",
        "__EVENTVALIDATION",
        "ScriptManager1_HiddenField",
        "ctl00$ContentPlaceHolder1$DistributionHistory$txtMinDate",
        "ctl00$ContentPlaceHolder1$DistributionHistory$txtMaxDate",
    ] {
        form.push((
            name.to_string(),
            html_input_value(html, name).unwrap_or_default(),
        ));
    }
    form.push((
        "ctl00$ContentPlaceHolder1$DistributionHistory$ddlDistributionHistoryYearSelection"
            .into(),
        year.trim().to_string(),
    ));
    form.push((
        "ctl00$ContentPlaceHolder1$DistributionHistory$btnSubmit".into(),
        "List Distribution History".into(),
    ));
    form
}

pub fn div1_fund_page(source: &str, html: &str, symbol: &str) -> bool {
    issuer(source).is_some_and(|i| generic_fund_page(html, symbol, i.needles))
}

pub fn div1_probe_urls(source: &str, symbol: &str) -> Vec<String> {
    issuer(source).map(|i| (i.urls)(symbol)).unwrap_or_default()
}

/// Gladstone newsroom common-stock table: month-day pairs + $0.15. Ignore Series A $0.130208.
pub fn parse_gladstone_press(text: &str) -> Vec<Value> {
    let collapsed = collapse_ws(text);
    let lower = collapsed.to_ascii_lowercase();
    let common = if let Some(idx) = lower.find("series a") {
        collapsed.get(..idx).unwrap_or(&collapsed).to_string()
    } else {
        collapsed.clone()
    };
    let year = infer_press_year(&collapsed).unwrap_or(2026);
    let start = common
        .to_ascii_lowercase()
        .find("common stock")
        .unwrap_or(0);
    let section = common.get(start..).unwrap_or(common.as_str());
    let mut out = Vec::new();
    let tokens: Vec<&str> = section.split_whitespace().collect();
    let mut i = 0usize;
    while i + 4 < tokens.len() {
        let Some(pay_month) = month_num(tokens[i + 2]) else {
            i += 1;
            continue;
        };
        let Some(_rec_month) = month_num(tokens[i]) else {
            i += 1;
            continue;
        };
        let rec_day = tokens[i + 1].trim_end_matches(',').parse::<u32>().ok();
        let pay_day = tokens[i + 3].trim_end_matches(',').parse::<u32>().ok();
        if rec_day.is_none() || pay_day.is_none() {
            i += 1;
            continue;
        }
        let Some(amount) = amount_from_tokens(&tokens[i + 4..]) else {
            i += 1;
            continue;
        };
        if is_series_a_amount(amount) {
            i += 1;
            continue;
        }
        let Some(pay) = chrono::NaiveDate::from_ymd_opt(year, pay_month, pay_day.unwrap()) else {
            i += 1;
            continue;
        };
        out.push(distribution_candidate(
            "gladstone",
            pay.format("%Y-%m-%d").to_string(),
            Some(amount),
            None,
        ));
        i += 5;
    }
    sort_newest_first(&mut out);
    out
}

fn is_series_a_amount(amount: (i64, u8)) -> bool {
    amount.0 == 130_208 && amount.1 >= 6
}

fn amount_from_tokens(tokens: &[&str]) -> Option<(i64, u8)> {
    let first = tokens.first()?.trim();
    if first == "$" {
        return parse_issuer_amount(tokens.get(1).copied().unwrap_or(""));
    }
    parse_issuer_amount(first)
}

fn month_num(raw: &str) -> Option<u32> {
    match raw.trim().trim_end_matches(',').to_ascii_lowercase().as_str() {
        "january" | "jan" => Some(1),
        "february" | "feb" => Some(2),
        "march" | "mar" => Some(3),
        "april" | "apr" => Some(4),
        "may" => Some(5),
        "june" | "jun" => Some(6),
        "july" | "jul" => Some(7),
        "august" | "aug" => Some(8),
        "september" | "sep" | "sept" => Some(9),
        "october" | "oct" => Some(10),
        "november" | "nov" => Some(11),
        "december" | "dec" => Some(12),
        _ => None,
    }
}

fn infer_press_year(text: &str) -> Option<i32> {
    let lower = text.to_ascii_lowercase();
    for marker in ["released ", "newswire / ", "20"] {
        if let Some(idx) = lower.find(marker) {
            let rest = text.get(idx..idx.saturating_add(40).min(text.len()))?;
            if let Some(y) = rest
                .split_whitespace()
                .find_map(|t| t.trim_matches(|c: char| !c.is_ascii_digit()).parse::<i32>().ok())
                .filter(|y| (1990..=2100).contains(y))
            {
                return Some(y);
            }
        }
    }
    None
}

fn collapse_ws(text: &str) -> String {
    strip_html(text)
}

/// REX / T-REX fund page Distribution Calendar: amount then Decl/Ex/Record/Payable.
/// Payable is the stored event. Published $0.00 rows are not paid history.
pub fn parse_rexshares_distributions(html: &str) -> Vec<Value> {
    let mut out = Vec::new();
    for (pay, amount) in rexshares_calendar_rows(html) {
        if amount.0 > 0 {
            out.push(distribution_candidate("trex", pay, Some(amount), None));
        }
    }
    sort_newest_first(&mut out);
    out
}

/// True when this page is the issuer Distribution Calendar and we parsed it.
/// Cadence × age is not the series — REX publishes $0 years and skipped quarters.
pub fn rexshares_calendar_covers_inception(html: &str, _inception_on: &str, _as_of: &str) -> bool {
    let lower = collapse_ws(html).to_ascii_lowercase();
    if !lower.contains("distribution per share") && !lower.contains("distribution calendar") {
        return false;
    }
    rexshares_calendar_rows(html).iter().any(|(_, amt)| amt.0 > 0)
}

fn rexshares_calendar_rows(html: &str) -> Vec<(String, (i64, u8))> {
    let text = collapse_ws(html);
    let lower = text.to_ascii_lowercase();
    let start = lower
        .find("distribution per share")
        .or_else(|| lower.find("distribution calendar"))
        .unwrap_or(0);
    let section = text.get(start..).unwrap_or(text.as_str());
    let mut out = Vec::new();
    let mut i = 0usize;
    while let Some(rel) = section.get(i..).and_then(|s| s.find('$')) {
        let at = i + rel;
        let after = section.get(at..).unwrap_or("");
        let Some((pay, amount, consumed)) = rexshares_row_from_dollar(after) else {
            i = at + 1;
            continue;
        };
        out.push((pay, amount));
        i = at + consumed.max(1);
    }
    out
}

fn rexshares_amount_token(raw: &str) -> Option<(i64, u8)> {
    let token = raw
        .trim()
        .trim_start_matches('$')
        .split(|c: char| c.is_whitespace() || c == ',' || c == ';')
        .next()?
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
    let minor = combined.parse::<i64>().ok()?;
    Some((minor, scale))
}

fn next_mdy_index(s: &str, start: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut j = start;
    while j + 10 <= bytes.len() {
        if bytes[j].is_ascii_digit()
            && bytes.get(j + 2) == Some(&b'/')
            && bytes.get(j + 5) == Some(&b'/')
        {
            return Some(j);
        }
        j += 1;
    }
    None
}

fn rexshares_row_from_dollar(after_dollar: &str) -> Option<(String, (i64, u8), usize)> {
    let first_date = next_mdy_index(after_dollar, 0)?;
    let amt_raw = after_dollar.get(..first_date)?;
    let amount = rexshares_amount_token(amt_raw).or_else(|| parse_issuer_amount(amt_raw))?;
    let mut dates = Vec::new();
    let mut pos = first_date;
    for _ in 0..4 {
        pos = next_mdy_index(after_dollar, pos)?;
        let slice = after_dollar.get(pos..pos + 10)?;
        let pay = parse_issuer_date(slice)?;
        dates.push(pay);
        pos += 10;
    }
    let pay = dates.get(3)?.clone();
    Some((pay, amount, pos))
}

/// Global X fund-page Distribution History: Next.js `distributionHistoryData`.
/// Payable date is the stored event. Ex/record are leftovers, not pay.
pub fn parse_globalx_distribution_history(body: &str) -> Vec<Value> {
    let rows = globalx_history_rows(body);
    if rows.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for row in rows {
        let Some(pay) = json_pay_date(&row) else {
            continue;
        };
        if globalx_amount_is_zero(&row) {
            continue;
        }
        let amount = json_amount(&row);
        let mut cand = distribution_candidate("globalx", pay, amount, None);
        if let Some(ex) = row
            .get("ex_date")
            .and_then(|v| v.as_str())
            .and_then(parse_issuer_date)
        {
            cand["exDate"] = serde_json::json!(ex);
        }
        if let Some(rec) = row
            .get("record_date")
            .and_then(|v| v.as_str())
            .and_then(parse_issuer_date)
        {
            cand["recordDate"] = serde_json::json!(rec);
        }
        out.push(cand);
    }
    sort_newest_first(&mut out);
    out
}

fn globalx_amount_is_zero(row: &Value) -> bool {
    match row.get("amount") {
        Some(Value::Number(n)) => n.as_f64() == Some(0.0) || n.as_i64() == Some(0),
        Some(Value::String(s)) => {
            let t = s.trim().trim_start_matches('$');
            t == "0" || t == "0.0" || t == "0.00"
        }
        _ => false,
    }
}

fn globalx_history_rows(body: &str) -> Vec<Value> {
    if let Some(v) = parse_json_value(body) {
        let rows = collect_globalx_history_rows(&v);
        if !rows.is_empty() {
            return rows;
        }
    }
    if let Some(v) = extract_rsc_globalx_history(body) {
        return collect_globalx_history_rows(&v);
    }
    Vec::new()
}

fn parse_json_value(body: &str) -> Option<Value> {
    let trimmed = body.trim();
    if !trimmed.starts_with('{') && !trimmed.starts_with('[') {
        return None;
    }
    serde_json::from_str(trimmed).ok()
}

fn collect_globalx_history_rows(v: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    walk_globalx_history(v, &mut out);
    out
}

fn walk_globalx_history(v: &Value, out: &mut Vec<Value>) {
    match v {
        Value::Array(a) => {
            if a.iter()
                .any(|x| x.get("payable_date").is_some() || x.get("ex_date").is_some())
            {
                out.extend(a.iter().filter(|x| x.is_object()).cloned());
                return;
            }
            for x in a {
                walk_globalx_history(x, out);
            }
        }
        Value::Object(o) => {
            if let Some(h) = o
                .get("DISTRIBUTION_HISTORY")
                .or_else(|| o.get("distributionHistoryData"))
            {
                walk_globalx_history(h, out);
                return;
            }
            for x in o.values() {
                walk_globalx_history(x, out);
            }
        }
        _ => {}
    }
}

fn extract_rsc_globalx_history(body: &str) -> Option<Value> {
    let key = body
        .find("distributionHistoryData")
        .or_else(|| body.find("DISTRIBUTION_HISTORY"))?;
    let rest = body.get(key..)?;
    let start = rest.find('[')?;
    let slice = rest.get(start..)?;
    let raw = take_balanced_array(slice)?;
    let unescaped = raw.replace("\\\"", "\"");
    serde_json::from_str(&unescaped)
        .ok()
        .or_else(|| serde_json::from_str(raw).ok())
}

fn take_balanced_array(s: &str) -> Option<&str> {
    if !s.starts_with('[') {
        return None;
    }
    let mut depth = 0i32;
    for (i, c) in s.char_indices() {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return s.get(..i + 1);
                }
            }
            _ => {}
        }
    }
    None
}

pub fn globalx_filings_hub_url() -> String {
    "https://www.globalxetfs.com/filings-and-tax-supplements".into()
}

pub fn globalx_fund_url(symbol: &str) -> String {
    format!(
        "https://www.globalxetfs.com/funds/{}",
        symbol.trim().to_ascii_lowercase()
    )
}

/// Global X Form 19a body: Pay Date + Distribution Amount Per Share (+ ROC %).
pub fn parse_globalx_19a_notice(text: &str) -> Vec<Value> {
    let collapsed = collapse_spaced_financial(&strip_html(text));
    let lower = collapsed.to_ascii_lowercase();
    let Some(pay_idx) = lower.find("pay date") else {
        return Vec::new();
    };
    let after_pay = collapsed.get(pay_idx..).unwrap_or("");
    let Some(pay) = take_labeled_date(after_pay) else {
        return Vec::new();
    };
    let amount = collapsed
        .find('$')
        .and_then(|i| collapsed.get(i..))
        .and_then(parse_issuer_amount)
        .or_else(|| {
            lower
                .find("distribution amount per share")
                .and_then(|i| collapsed.get(i..))
                .and_then(take_labeled_amount)
        });
    let roc = parse_return_of_capital_pct(&collapsed).map(|(pct, _)| pct);
    vec![distribution_candidate("globalx", pay, amount, roc)]
}

pub fn globalx_tax_supplements_url(symbol: &str) -> String {
    format!(
        "https://www.globalxetfs.com/filings-and-tax-supplements/{}",
        symbol.trim().to_ascii_lowercase()
    )
}

/// `{SYM}_Form-19a_MMDDYYYY.docx|.pdf` on assets.globalxetfs.com, newest filename date first.
pub fn globalx_19a_notice_urls(html: &str, symbol: &str) -> Vec<String> {
    let sym = symbol.trim().to_ascii_uppercase();
    let needle = format!("{sym}_form-19a_");
    let lower = html.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut search = 0usize;
    while let Some(rel) = lower.get(search..).and_then(|s| s.find(&needle.to_ascii_lowercase())) {
        let start = search + rel;
        let rest = html.get(start..).unwrap_or("");
        let end = rest
            .find(|c: char| c == '"' || c == '\'' || c.is_whitespace() || c == '\\' || c == '<')
            .unwrap_or(rest.len().min(80));
        let name = rest.get(..end).unwrap_or("");
        if name.to_ascii_lowercase().contains(".pdf") || name.to_ascii_lowercase().contains(".docx")
        {
            let url = if name.starts_with("http") {
                name.to_string()
            } else {
                format!("https://assets.globalxetfs.com/funds/tax_supplements/{name}")
            };
            if !out.iter().any(|u: &String| u == &url) {
                out.push(url);
            }
        }
        search = start + needle.len();
    }
    out.sort_by(|a, b| globalx_19a_stamp(b).cmp(&globalx_19a_stamp(a)));
    out
}

fn globalx_19a_stamp(url: &str) -> String {
    let lower = url.to_ascii_lowercase();
    let Some(idx) = lower.rfind("form-19a_") else {
        return String::new();
    };
    lower
        .get(idx + 9..)
        .unwrap_or("")
        .chars()
        .take(8)
        .collect()
}

pub fn collapse_spaced_financial(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '$' {
            out.push('$');
            i += 1;
            let mut dots = 0u8;
            let mut frac = 0u8;
            while i < chars.len() {
                if chars[i].is_whitespace() {
                    let mut k = i;
                    while k < chars.len() && chars[k].is_whitespace() {
                        k += 1;
                    }
                    if k >= chars.len()
                        || !(chars[k].is_ascii_digit() || chars[k] == '.')
                        || frac >= 4
                    {
                        break;
                    }
                    i = k;
                    continue;
                }
                if chars[i] == '.' {
                    if dots > 0 {
                        break;
                    }
                    dots += 1;
                    out.push('.');
                    i += 1;
                    continue;
                }
                if chars[i].is_ascii_digit() {
                    out.push(chars[i]);
                    if dots > 0 {
                        frac += 1;
                    }
                    i += 1;
                    continue;
                }
                break;
            }
            out.push(' ');
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out.replace(" ,", ",")
}

pub fn parse_return_of_capital_pct(text: &str) -> Option<(i64, String)> {
    let lower = text.to_ascii_lowercase();
    let mut search = 0usize;
    while let Some(rel) = lower.get(search..).and_then(|s| s.find("return of capital")) {
        let idx = search + rel;
        let after = text.get(idx..idx.saturating_add(80).min(text.len()))?;
        if let Some(pct) = crate::retrieve::first_percent(after) {
            if pct > 0 {
                return Some((pct, "19a-1 return of capital percent".into()));
            }
        }
        search = idx + 16;
    }
    None
}

fn take_labeled_date(after_label: &str) -> Option<String> {
    let rest = after_label
        .split_once(':')
        .map(|(_, r)| r)
        .unwrap_or(after_label);
    let tokens: Vec<&str> = rest.split_whitespace().take(4).collect();
    if tokens.len() >= 3 {
        let joined = format!("{} {} {}", tokens[0], tokens[1], tokens[2]);
        if let Some(d) = parse_issuer_date(&joined) {
            return Some(d);
        }
    }
    tokens.iter().find_map(|t| parse_issuer_date(t))
}

fn take_labeled_amount(after_label: &str) -> Option<(i64, u8)> {
    let rest = after_label
        .split_once(':')
        .map(|(_, r)| r)
        .unwrap_or(after_label);
    if let Some(idx) = rest.find('$') {
        return parse_issuer_amount(rest.get(idx..).unwrap_or(""));
    }
    rest.split_whitespace().find_map(parse_issuer_amount)
}

#[allow(dead_code)]
pub fn gladstone_candidates_table(cands: &[Value]) -> String {
    cornerstone_candidates_table(cands)
}
