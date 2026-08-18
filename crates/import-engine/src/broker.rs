//! Fidelity- and Schwab-shaped CSV capture. Returns staging candidates only (ADR-0010).

use crate::require_candidate_amount;
use application_core::contracts::ImportCandidate;
use financial_domain::error::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrokerLayout {
    Fidelity,
    Schwab,
}

pub fn detect_broker(header_line: &str) -> Option<BrokerLayout> {
    let h = header_line.to_ascii_lowercase();
    if h.contains("run date") {
        Some(BrokerLayout::Fidelity)
    } else if h.contains("action") && h.contains("date") {
        Some(BrokerLayout::Schwab)
    } else {
        None
    }
}

/// Parse a broker CSV into import candidates. Does not post ledger facts.
pub fn parse_broker_csv(
    layout: BrokerLayout,
    content: &str,
    default_account: Option<&str>,
) -> Result<Vec<ImportCandidate>, DomainError> {
    let mut lines = content.lines().filter(|l| !l.trim().is_empty());
    let header = lines.next().unwrap_or("");
    let cols: Vec<String> = split_csv(header).into_iter().map(|s| s.to_ascii_lowercase()).collect();
    let mut out = Vec::new();
    for line in lines {
        let cells = split_csv(line);
        if !is_dividend_row(layout, &cols, &cells) {
            continue;
        }
        let amount_raw = cell(&cols, &cells, "amount").unwrap_or("");
        let amount_minor = parse_usd_minor(amount_raw);
        require_candidate_amount(amount_minor, 2)?;
        let account_name = match layout {
            BrokerLayout::Fidelity => cell(&cols, &cells, "account")
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .or_else(|| default_account.map(|s| s.to_string()))
                .unwrap_or_default(),
            BrokerLayout::Schwab => default_account.unwrap_or("").to_string(),
        };
        let symbol = cell(&cols, &cells, "symbol").map(|s| s.to_string());
        let occurred_on = parse_mdy(match layout {
            BrokerLayout::Fidelity => cell(&cols, &cells, "run date").unwrap_or(""),
            BrokerLayout::Schwab => cell(&cols, &cells, "date").unwrap_or(""),
        })
        .unwrap_or_default();
        out.push(ImportCandidate {
            account_name,
            symbol,
            activity_type: "dividend".to_string(),
            amount_minor,
            scale: 2,
            occurred_on,
        });
    }
    Ok(out)
}

fn is_dividend_row(layout: BrokerLayout, cols: &[String], cells: &[String]) -> bool {
    let action = match layout {
        BrokerLayout::Fidelity | BrokerLayout::Schwab => cell(cols, cells, "action").unwrap_or(""),
    };
    action.to_ascii_lowercase().contains("dividend")
}

fn cell<'a>(cols: &[String], cells: &'a [String], name: &str) -> Option<&'a str> {
    cols.iter()
        .position(|c| c == name)
        .and_then(|i| cells.get(i))
        .map(|s| s.as_str())
}

fn split_csv(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    for c in line.chars() {
        match c {
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    out.push(cur.trim().to_string());
    out
}

fn parse_usd_minor(raw: &str) -> Option<i64> {
    let s: String = raw
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    if s.is_empty() || s == "." || s == "-" {
        return None;
    }
    let neg = s.starts_with('-');
    let s = s.trim_start_matches('-');
    let (whole, frac) = match s.split_once('.') {
        Some((w, f)) => (w, &f[..f.len().min(2)]),
        None => (s, "00"),
    };
    let frac = format!("{frac:0<2}");
    let n: i64 = format!("{whole}{frac}").parse().ok()?;
    Some(if neg { -n } else { n })
}

fn parse_mdy(raw: &str) -> Option<String> {
    let parts: Vec<&str> = raw.trim().split('/').collect();
    if parts.len() != 3 {
        return None;
    }
    let month: u32 = parts[0].parse().ok()?;
    let day: u32 = parts[1].parse().ok()?;
    let year: u32 = parts[2].parse().ok()?;
    Some(format!("{year:04}-{month:02}-{day:02}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIDELITY: &str = "Run Date,Account,Action,Symbol,Security Description,Amount\n\
01/17/2026,Taxable Brokerage,DIVIDEND RECEIVED,CASH,USD Cash,500.00\n\
01/10/2026,Taxable Brokerage,YOU BOUGHT,CASH,USD Cash,-100.00\n";

    const SCHWAB: &str = "Date,Action,Symbol,Description,Amount\n\
01/17/2026,Cash Dividend,CASH,USD Cash,500.00\n\
01/10/2026,Buy,CASH,USD Cash,-100.00\n";

    #[test]
    fn fidelity_dividend_becomes_a_candidate_not_a_post() {
        let rows = parse_broker_csv(BrokerLayout::Fidelity, FIDELITY, None).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].activity_type, "dividend");
        assert_eq!(rows[0].amount_minor, Some(50_000));
        assert_eq!(rows[0].occurred_on, "2026-01-17");
        assert_eq!(rows[0].account_name, "Taxable Brokerage");
    }

    #[test]
    fn schwab_dividend_matches_fidelity_amount() {
        let f = parse_broker_csv(BrokerLayout::Fidelity, FIDELITY, None).unwrap();
        let s = parse_broker_csv(
            BrokerLayout::Schwab,
            SCHWAB,
            Some("Taxable Brokerage"),
        )
        .unwrap();
        assert_eq!(f[0].amount_minor, s[0].amount_minor);
        assert_eq!(detect_broker(FIDELITY.lines().next().unwrap()), Some(BrokerLayout::Fidelity));
        assert_eq!(detect_broker(SCHWAB.lines().next().unwrap()), Some(BrokerLayout::Schwab));
    }
}
