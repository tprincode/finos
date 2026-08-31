//! Read-only match of broker dividend candidates vs spreadsheet yield vs already-posted cash.

use std::path::Path;

use application_core::contracts::ImportCandidate;
use calamine::{open_workbook, Data, Reader, Xlsx};
use chrono::{Duration, NaiveDate};

use crate::broker::{parse_broker_day, parse_usd_minor, resolve_account_name};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DividendFact {
    pub account_name: String,
    pub symbol: String,
    pub occurred_on: String,
    pub amount_minor: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapKind {
    Match,
    OnlyBroker,
    OnlySheet,
    AlreadyPosted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GapRow {
    pub fact: DividendFact,
    pub kind: GapKind,
}

impl DividendFact {
    pub fn from_candidate(c: &ImportCandidate) -> Option<Self> {
        if !c.activity_type.eq_ignore_ascii_case("dividend") {
            return None;
        }
        Some(Self {
            account_name: c.account_name.clone(),
            symbol: c.symbol.clone()?,
            occurred_on: c.occurred_on.clone(),
            amount_minor: c.amount_minor?,
        })
    }

    pub fn key(&self) -> (String, String, String, i64) {
        (
            self.account_name.to_ascii_lowercase(),
            self.symbol.to_ascii_uppercase(),
            self.occurred_on.clone(),
            self.amount_minor,
        )
    }
}

pub fn is_account_9(name: &str) -> bool {
    let n = resolve_account_name(name, None);
    financial_domain::income_plan::map_control_account(&n) == Some("Account 9")
}

fn cell_string(v: &Data) -> String {
    match v {
        Data::Empty | Data::Error(_) => String::new(),
        Data::String(s) => s.trim().to_string(),
        Data::Float(n) => {
            if n.fract().abs() < 1e-9 {
                format!("{}", *n as i64)
            } else {
                format!("{n}")
            }
        }
        Data::Int(n) => n.to_string(),
        Data::Bool(b) => {
            if *b {
                "1".into()
            } else {
                "0".into()
            }
        }
        Data::DateTime(dt) => excel_serial_to_iso(dt.as_f64()),
        Data::DateTimeIso(s) => s.chars().take(10).collect(),
        other => other.to_string().trim().to_string(),
    }
}

fn excel_serial_to_iso(serial: f64) -> String {
    let days = serial.trunc() as i64;
    let epoch = NaiveDate::from_ymd_opt(1899, 12, 30).expect("epoch");
    (epoch + Duration::days(days)).format("%Y-%m-%d").to_string()
}

fn as_iso_date(v: &Data) -> String {
    match v {
        Data::DateTime(dt) => excel_serial_to_iso(dt.as_f64()),
        Data::DateTimeIso(s) => s.chars().take(10).collect(),
        Data::Float(n) if (20_000.0..=60_000.0).contains(n) && n.fract().abs() < 1e-9 => {
            excel_serial_to_iso(*n)
        }
        other => {
            let s = cell_string(other);
            parse_broker_day(&s).unwrap_or(s.chars().take(10).collect())
        }
    }
}

/// ROI update workbook: Flag/Notes = Div. Cash is Unit Sale Price on Date Sold. Skips Account 9.
pub fn parse_roi_dividend_update(path: &Path) -> Result<Vec<DividendFact>, String> {
    let mut workbook: Xlsx<_> =
        open_workbook(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let sheet = workbook
        .sheet_names()
        .into_iter()
        .next()
        .ok_or_else(|| format!("{} has no sheets", path.display()))?;
    let range = workbook
        .worksheet_range(&sheet)
        .map_err(|e| format!("{} {sheet}: {e}", path.display()))?;
    let mut rows = range.rows();
    let header = rows
        .next()
        .ok_or_else(|| format!("{} empty", path.display()))?;
    let keys: Vec<String> = header
        .iter()
        .map(|c| cell_string(c).trim().to_ascii_lowercase())
        .collect();
    let idx = |name: &str| keys.iter().position(|k| k == name);
    let i_acct = idx("account").ok_or("ROI sheet missing Account")?;
    let i_sym = idx("symbol").ok_or("ROI sheet missing Symbol")?;
    let i_notes = idx("notes");
    let i_flag = idx("flag");
    let i_date = idx("date sold").or_else(|| idx("date purchased"));
    let i_amt = idx("unit sale price").ok_or("ROI sheet missing Unit Sale Price")?;
    let mut out = Vec::new();
    for row in rows {
        let get = |i: usize| row.get(i).map(cell_string).unwrap_or_default();
        let notes = i_notes.map(|i| get(i)).unwrap_or_default();
        let flag = i_flag.map(|i| get(i)).unwrap_or_default();
        let is_div = notes.to_ascii_lowercase().contains("div")
            || flag.eq_ignore_ascii_case("div")
            || notes.to_ascii_lowercase().contains("return of capital");
        if !is_div {
            continue;
        }
        let raw_acct = get(i_acct);
        if is_account_9(&raw_acct) {
            continue;
        }
        let Some(amount_minor) = row.get(i_amt).and_then(|v| match v {
            Data::Float(n) => Some((*n * 100.0).round() as i64),
            Data::Int(n) => Some(n * 100),
            other => parse_usd_minor(&cell_string(other)),
        }) else {
            continue;
        };
        let occurred = i_date
            .and_then(|i| row.get(i))
            .map(as_iso_date)
            .unwrap_or_default();
        out.push(DividendFact {
            account_name: resolve_account_name(&raw_acct, None),
            symbol: get(i_sym).to_ascii_uppercase(),
            occurred_on: occurred,
            amount_minor,
        });
    }
    Ok(out)
}

pub fn parse_yield_sheet_csv(content: &str) -> Vec<DividendFact> {
    let mut lines = content.lines().filter(|l| !l.trim().is_empty());
    let header = lines.next().unwrap_or("");
    let cols: Vec<String> = header
        .split(',')
        .map(|s| s.trim().to_ascii_lowercase())
        .collect();
    let idx = |name: &str| cols.iter().position(|c| c == name);
    let i_acct = idx("account_name").or_else(|| idx("account"));
    let i_sym = idx("symbol");
    let i_date = idx("txn_date").or_else(|| idx("occurred_on")).or_else(|| idx("date"));
    let i_amt = idx("amount");
    let mut out = Vec::new();
    for line in lines {
        let cells: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        let get = |i: Option<usize>| i.and_then(|p| cells.get(p).copied()).unwrap_or("");
        let Some(amount_minor) = parse_usd_minor(get(i_amt)) else {
            continue;
        };
        let occurred = crate::broker::parse_broker_day(get(i_date)).unwrap_or_default();
        out.push(DividendFact {
            account_name: get(i_acct).to_string(),
            symbol: get(i_sym).to_string(),
            occurred_on: occurred,
            amount_minor,
        });
    }
    out
}

/// Broker vs sheet vs already-posted. Does not post.
pub fn dividend_gap(
    broker: &[DividendFact],
    sheet: &[DividendFact],
    posted: &[DividendFact],
) -> Vec<GapRow> {
    use std::collections::HashSet;
    let sheet_keys: HashSet<_> = sheet.iter().map(DividendFact::key).collect();
    let posted_keys: HashSet<_> = posted.iter().map(DividendFact::key).collect();
    let broker_keys: HashSet<_> = broker.iter().map(DividendFact::key).collect();
    let mut out = Vec::new();
    for fact in broker {
        let k = fact.key();
        let kind = if posted_keys.contains(&k) {
            GapKind::AlreadyPosted
        } else if sheet_keys.contains(&k) {
            GapKind::Match
        } else {
            GapKind::OnlyBroker
        };
        out.push(GapRow {
            fact: fact.clone(),
            kind,
        });
    }
    for fact in sheet {
        if !broker_keys.contains(&fact.key()) {
            out.push(GapRow {
                fact: fact.clone(),
                kind: GapKind::OnlySheet,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_and_only_sheet_and_already_posted() {
        let nvdw = DividendFact {
            account_name: "Income".into(),
            symbol: "NVDW".into(),
            occurred_on: "2026-08-29".into(),
            amount_minor: 7_525,
        };
        let extra = DividendFact {
            account_name: "Income".into(),
            symbol: "HAKY".into(),
            occurred_on: "2026-08-31".into(),
            amount_minor: 2_660,
        };
        let rows = dividend_gap(
            &[nvdw.clone()],
            &[nvdw.clone(), extra.clone()],
            &[nvdw.clone()],
        );
        assert!(rows.iter().any(|r| r.kind == GapKind::AlreadyPosted && r.fact.symbol == "NVDW"));
        assert!(rows.iter().any(|r| r.kind == GapKind::OnlySheet && r.fact.symbol == "HAKY"));
    }
}
