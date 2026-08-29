//! Remaining DIV-1 issuer adapters (one registered source per provider).
//! Table parse when GET exposes dates; otherwise loud miss — not Yahoo, not $0.
//! ProShares fund pages render distributions via `/api/distributionsummary/` (empty HTML tbody).

use serde_json::Value;

use crate::retrieve::html::{
    distribution_candidate, parse_issuer_amount, parse_issuer_date, sort_newest_first,
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
        let pay = row
            .get("PayableDate")
            .or_else(|| row.get("payableDate"))
            .and_then(|x| x.as_str())
            .and_then(parse_issuer_date);
        let Some(pay) = pay else {
            continue;
        };
        let amount = row
            .get("CashDividendPerShare")
            .and_then(|x| match x {
                Value::Number(n) => Some(n.to_string()),
                Value::String(s) => Some(s.clone()),
                _ => None,
            })
            .or_else(|| {
                row.get("Dividend")
                    .and_then(|x| x.as_str())
                    .map(|s| {
                        if s.starts_with('.') {
                            format!("0{s}")
                        } else {
                            s.to_string()
                        }
                    })
            })
            .and_then(|s| parse_issuer_amount(&s));
        out.push(distribution_candidate(source, pay, amount, None));
    }
    sort_newest_first(&mut out);
    out
}

fn cornerstone_urls(symbol: &str) -> Vec<String> {
    let sym = symbol.trim().to_ascii_uppercase();
    match sym.as_str() {
        "CLM" => vec![
            "https://cornerstonestrategicinvestmentfund.com/press-releases".into(),
            "https://cornerstonestrategicinvestmentfund.com/".into(),
        ],
        _ => vec![
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
            urls_for(
                &[
                    "https://www.direxion.com/product/{sym}",
                    "https://www.direxion.com/ticker/{sym}",
                ],
                s,
            )
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
                "https://www.ellingtonfinancial.com/dividends-common-stock".into(),
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
        urls: |_| {
            vec![
                "https://ir.energytransfer.com/distribution-history-et".into(),
                "https://www.energytransfer.com/investor-relations".into(),
            ]
        },
    },
    Div1Issuer {
        source: "gladstone",
        needles: &["gladstone"],
        urls: |_| {
            vec![
                "https://www.gladstonecapital.com/investors".into(),
                "https://www.gladstonecapital.com/".into(),
            ]
        },
    },
    Div1Issuer {
        source: "jpmorgan",
        needles: &["jpmorgan", "jp morgan"],
        urls: |s| {
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
                "https://www.mplx.com/Investors/Distributions".into(),
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
        urls: |s| urls_for(&["https://www.globalxetfs.com/funds/{sym}", "https://www.globalxetfs.com/funds/{sym}/"], s),
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
    if source.eq_ignore_ascii_case("proshares") {
        let api = parse_proshares_distribution_summary(source, html);
        if !api.is_empty() {
            return api;
        }
    }
    parse_generic_distributions(source, html)
}

pub fn div1_fund_page(source: &str, html: &str, symbol: &str) -> bool {
    issuer(source).is_some_and(|i| generic_fund_page(html, symbol, i.needles))
}

pub fn div1_probe_urls(source: &str, symbol: &str) -> Vec<String> {
    issuer(source).map(|i| (i.urls)(symbol)).unwrap_or_default()
}
