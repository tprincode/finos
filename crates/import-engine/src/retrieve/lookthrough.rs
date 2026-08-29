//! Look-through research from an issuer fund page. Fail-closed: unknown is not 0%.
//! Does not parse a full holdings book. Top names only when the page labels them as top.

use serde_json::{json, Value};

use super::html::strip_html;

const HOLDING_SKIP: &[&str] = &[
    "ETF", "ETFS", "NAV", "USD", "CASH", "THE", "AND", "FOR", "INC", "LTD", "PLC", "CLASS",
    "WEIGHT", "TICKER", "NAME", "TOP", "HOLDINGS", "CURRENT", "EQUITY", "POSITIONS", "SECTOR",
    "INFORMATION", "TECHNOLOGY", "UNDERLYING", "FUND", "INDEX", "TABLE", "BREAKDOWN",
    "ALLOCATION", "EXPOSURE",
];

pub fn extract_lookthrough(html: &str, symbol: &str, underlying: &str) -> Value {
    let text = strip_html(html);
    let lower = text.to_ascii_lowercase();
    let cyber = lower.contains("cyber");
    let covered = lower.contains("covered call");
    let overlay = symbol.eq_ignore_ascii_case("HAKY")
        || (cyber && covered && underlying.eq_ignore_ascii_case("HACK"));

    let theme_strategy = if cyber && covered {
        "Cybersecurity + Covered Call Equity".to_string()
    } else if cyber {
        "Cybersecurity".to_string()
    } else {
        String::new()
    };

    let primary_risk_driver = if !underlying.is_empty() && (overlay || cyber) {
        format!("Look-through cybersecurity equity basket ({underlying})")
    } else if !underlying.is_empty() {
        underlying.to_string()
    } else {
        String::new()
    };

    let vol_proxy = if overlay || (cyber && covered) {
        "Slightly dampened version of HACK / cyber software basket".to_string()
    } else {
        String::new()
    };

    let tax_character = if covered {
        "Option premium (possible ROC component); ROC unknown until 19a-1".to_string()
    } else {
        String::new()
    };

    let (risk_tier_suggestion, risk_tier_suggestion_reason) = if overlay || (cyber && covered)
    {
        (
            "Risk On",
            "Highest return potential + thematic concentration",
        )
    } else {
        ("", "")
    };

    let top_holdings = extract_top_holdings(html, symbol, underlying);
    let sector_weights = extract_sector_weights(html);
    let concentration_status = if top_holdings.is_empty() && sector_weights.is_empty() {
        "unknown"
    } else {
        "issuer_top_holdings"
    };

    json!({
        "themeStrategy": theme_strategy,
        "primaryRiskDriver": primary_risk_driver,
        "concentrationStatus": concentration_status,
        "topHoldings": top_holdings,
        "sectorWeights": sector_weights,
        "concentrationAsOf": Value::Null,
        "volProxy": vol_proxy,
        "taxCharacter": tax_character,
        "riskTierSuggestion": risk_tier_suggestion,
        "riskTierSuggestionReason": risk_tier_suggestion_reason,
    })
}

pub fn empty_lookthrough() -> Value {
    json!({
        "themeStrategy": "",
        "primaryRiskDriver": "",
        "concentrationStatus": "unknown",
        "topHoldings": [],
        "sectorWeights": [],
        "concentrationAsOf": Value::Null,
        "volProxy": "",
        "taxCharacter": "",
        "riskTierSuggestion": "",
        "riskTierSuggestionReason": ""
    })
}

fn extract_top_holdings(html: &str, symbol: &str, underlying: &str) -> Vec<Value> {
    let stripped = strip_html(html).to_ascii_uppercase();
    let needles = [
        "CURRENT TOP UNDERLYING EQUITY POSITIONS",
        "TOP UNDERLYING EQUITY POSITIONS",
        "TOP UNDERLYING HOLDINGS",
        "TOP HOLDINGS",
    ];
    let Some(start) = needles.iter().find_map(|n| stripped.find(n)) else {
        return Vec::new();
    };
    parse_ticker_percents(&stripped[start..], symbol, underlying)
}

fn extract_sector_weights(html: &str) -> Vec<Value> {
    let stripped = strip_html(html).to_ascii_uppercase();
    let needles = [
        "SECTOR BREAKDOWN",
        "SECTOR ALLOCATION",
        "SECTOR WEIGHT",
        "SECTOR EXPOSURE",
    ];
    let Some(start) = needles.iter().find_map(|n| stripped.find(n)) else {
        return Vec::new();
    };
    let window = &stripped[start..];
    let label = "INFORMATION TECHNOLOGY";
    let Some(idx) = window.find(label) else {
        return Vec::new();
    };
    let after = window.get(idx + label.len()..).unwrap_or("");
    let nearby = after.split_whitespace().take(3).collect::<Vec<_>>().join(" ");
    percent_to_bps(&nearby)
        .map(|bps| {
            vec![json!({
                "label": "Information Technology",
                "weightBps": bps,
            })]
        })
        .unwrap_or_default()
}

fn parse_ticker_percents(text: &str, symbol: &str, underlying: &str) -> Vec<Value> {
    let skip_self = symbol.trim().to_ascii_uppercase();
    let skip_sleeve = underlying.trim().to_ascii_uppercase();
    let tokens: Vec<String> = text
        .split_whitespace()
        .map(|t| t.to_ascii_uppercase())
        .collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < tokens.len() && out.len() < 10 {
        let raw = tokens[i]
            .trim_matches(|c: char| !c.is_ascii_alphanumeric())
            .to_string();
        if is_holding_ticker(&raw) && raw != skip_self && raw != skip_sleeve {
            let mut weight = None;
            for look in 1..=3 {
                if let Some(next) = tokens.get(i + look) {
                    if let Some(bps) = percent_to_bps(next) {
                        weight = Some(bps);
                        break;
                    }
                }
            }
            if weight.is_some() {
                out.push(json!({
                    "ticker": raw,
                    "weightBps": weight,
                }));
            }
        }
        i += 1;
    }
    out
}

fn is_holding_ticker(t: &str) -> bool {
    (2..=5).contains(&t.len())
        && t.chars().all(|c| c.is_ascii_uppercase())
        && !HOLDING_SKIP.contains(&t)
}

fn percent_to_bps(text: &str) -> Option<i64> {
    let idx = text.find('%')?;
    let before = text.get(..idx)?;
    let num: String = before
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    let pct: f64 = num.parse().ok()?;
    if !(0.0..=100.0).contains(&pct) {
        return None;
    }
    Some((pct * 100.0).round() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labeled_amplify_table_fills_top_holdings() {
        let html = r#"<html><title>HAKY – Amplify ETFs</title><body>
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
        let v = extract_lookthrough(html, "HAKY", "HACK");
        assert_eq!(
            v["concentrationStatus"],
            "issuer_top_holdings",
            "lookthrough={v}"
        );
        assert_eq!(v["topHoldings"][0]["ticker"], "PANW");
        assert_eq!(v["topHoldings"][0]["weightBps"], 887);
        assert_eq!(v["sectorWeights"][0]["weightBps"], 9120);
    }
}
