//! Fidelity- and Schwab-shaped CSV capture. Returns staging candidates only (ADR-0010).
//!
//! Keep = cash payments the owner treats as dividends (ETF `DIVIDEND RECEIVED`,
//! money-market dividend/interest). Drop = journals, tax withholding/distributions,
//! purchases, sells, reinvestment, transfers. Observed on Fidelity Accounts_History
//! (2026-08): SPAXX is `DIVIDEND RECEIVED` plus a separate `REINVESTMENT` line.

use crate::require_candidate_amount;
use application_core::contracts::ImportCandidate;
use financial_domain::error::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrokerLayout {
    Fidelity,
    Schwab,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionClass {
    KeepDividend,
    Drop(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DroppedBrokerRow {
    pub action: String,
    pub reason: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerParse {
    pub candidates: Vec<ImportCandidate>,
    pub dropped: Vec<DroppedBrokerRow>,
}

/// Fidelity History nicknames → Finos registered account names (seed Template_Accounts).
pub const DEFAULT_ACCOUNT_ALIASES: &[(&str, &str)] = &[
    ("For the CAR", "Car"),
    ("Health Savings Account", "Health"),
    ("ROTH IRA", "FI Roth"),
];

/// Classify a Fidelity/Schwab Action. Reinvestment is not a cash dividend (lots/DRIP later).
pub fn classify_action(action: &str) -> ActionClass {
    let a = action.trim().to_ascii_lowercase();
    if a.is_empty() {
        return ActionClass::Drop("blank");
    }
    if a.contains("pending") {
        return ActionClass::Drop("pending");
    }
    if a.contains("reinvestment") {
        return ActionClass::Drop("reinvestment");
    }
    if a.contains("journal") {
        return ActionClass::Drop("journal");
    }
    if a.contains("tax w/h") || a.contains("fed tax") || a.contains("state tax") {
        return ActionClass::Drop("tax");
    }
    if a.contains("normal distr") {
        return ActionClass::Drop("tax_disbursement");
    }
    if a.contains("you bought") || a.contains("bought") || a.contains("purchase") {
        return ActionClass::Drop("purchase");
    }
    if a.contains("you sold") || a.starts_with("sold") || a.contains(" sold ") {
        return ActionClass::Drop("sell");
    }
    if a.contains("electronic funds") || a.contains("transfer") {
        return ActionClass::Drop("transfer");
    }
    if a.contains("fee charged") {
        return ActionClass::Drop("fee");
    }
    if a.contains("reverse split") {
        return ActionClass::Drop("corp_action");
    }
    if a.contains("debit card") {
        return ActionClass::Drop("debit_card");
    }
    if a.contains("in lieu of") {
        return ActionClass::Drop("in_lieu");
    }
    if a.contains("return of capital") {
        return ActionClass::KeepDividend;
    }
    if a.contains("dividend") || a.contains("interest") {
        return ActionClass::KeepDividend;
    }
    ActionClass::Drop("other")
}

fn account_nickname(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(idx) = trimmed.rfind('(') {
        let prefix = trimmed[..idx].trim();
        if !prefix.is_empty() && trimmed[idx..].contains(')') {
            return prefix.to_string();
        }
    }
    trimmed.to_string()
}

pub fn resolve_account_name(raw: &str, default_account: Option<&str>) -> String {
    let trimmed = account_nickname(raw);
    if trimmed.is_empty() {
        return default_account.unwrap_or("").to_string();
    }
    if trimmed.eq_ignore_ascii_case("fi roth") {
        return "FI Roth".to_string();
    }
    for (from, to) in DEFAULT_ACCOUNT_ALIASES {
        if trimmed.eq_ignore_ascii_case(from) {
            return (*to).to_string();
        }
    }
    if trimmed.eq_ignore_ascii_case("income") {
        return "Income".to_string();
    }
    if trimmed.eq_ignore_ascii_case("health") {
        return "Health".to_string();
    }
    if trimmed.eq_ignore_ascii_case("car") {
        return "Car".to_string();
    }
    trimmed.to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrfDripClass {
    Drip,
    ZeroDrop,
    CashDividend,
}

/// FI Roth CRF DIVIDEND RECEIVED with amount > 0 is a zero-cost DRIP lot, not cash.
/// $0 CRF rows (ROI sheet artifacts) are dropped.
pub fn crf_drip_class(
    account_name: &str,
    symbol: Option<&str>,
    amount_minor: Option<i64>,
) -> CrfDripClass {
    let Some(sym) = symbol else {
        return CrfDripClass::CashDividend;
    };
    if !sym.eq_ignore_ascii_case("CRF") {
        return CrfDripClass::CashDividend;
    }
    let amount = amount_minor.unwrap_or(0);
    if amount <= 0 {
        return CrfDripClass::ZeroDrop;
    }
    if resolve_account_name(account_name, None).eq_ignore_ascii_case("FI Roth") {
        CrfDripClass::Drip
    } else {
        CrfDripClass::CashDividend
    }
}

pub fn detect_broker(header_line: &str) -> Option<BrokerLayout> {
    let h = strip_bom(header_line).to_ascii_lowercase();
    if h.contains("run date") {
        Some(BrokerLayout::Fidelity)
    } else if h.contains("date")
        && h.contains("account")
        && h.contains("description")
        && h.contains("amount")
    {
        Some(BrokerLayout::Fidelity)
    } else if h.contains("action") && h.contains("date") {
        Some(BrokerLayout::Schwab)
    } else {
        None
    }
}

/// Skip Fidelity title/preamble rows until the real header.
pub fn find_broker_header(content: &str) -> Option<(BrokerLayout, usize)> {
    for (idx, line) in strip_bom(content).lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        if let Some(layout) = detect_broker(line) {
            return Some((layout, idx));
        }
    }
    None
}

/// Parse a broker CSV into import candidates. Does not post ledger facts.
pub fn parse_broker_csv(
    layout: BrokerLayout,
    content: &str,
    default_account: Option<&str>,
) -> Result<Vec<ImportCandidate>, DomainError> {
    Ok(parse_broker_csv_detail(layout, content, default_account)?.candidates)
}

pub fn parse_broker_csv_detail(
    layout: BrokerLayout,
    content: &str,
    default_account: Option<&str>,
) -> Result<BrokerParse, DomainError> {
    let text = strip_bom(content);
    let lines: Vec<&str> = text.lines().collect();
    let header_idx = find_broker_header(text)
        .filter(|(found, _)| *found == layout)
        .map(|(_, idx)| idx)
        .unwrap_or(0);
    if header_idx >= lines.len() {
        return Ok(BrokerParse {
            candidates: Vec::new(),
            dropped: Vec::new(),
        });
    }
    let header = lines[header_idx];
    let cols: Vec<String> = split_csv(header)
        .into_iter()
        .map(|s| s.to_ascii_lowercase())
        .collect();
    let mut out = BrokerParse {
        candidates: Vec::new(),
        dropped: Vec::new(),
    };
    for line in lines.iter().skip(header_idx + 1) {
        if line.trim().is_empty() {
            continue;
        }
        let cells = split_csv(line);
        let action = cell(&cols, &cells, "action")
            .or_else(|| cell(&cols, &cells, "description"))
            .unwrap_or("");
        match classify_action(action) {
            ActionClass::Drop(reason) => {
                out.dropped.push(DroppedBrokerRow {
                    action: action.to_string(),
                    reason,
                });
                continue;
            }
            ActionClass::KeepDividend => {}
        }
        let amount_raw = amount_cell(&cols, &cells).unwrap_or("");
        let amount_minor = parse_usd_minor(amount_raw);
        if amount_minor.is_some() {
            require_candidate_amount(amount_minor, 2)?;
        }
        let account_raw = match layout {
            BrokerLayout::Fidelity => cell(&cols, &cells, "account")
                .filter(|s| !s.is_empty())
                .unwrap_or(""),
            BrokerLayout::Schwab => "",
        };
        let account_name = resolve_account_name(account_raw, default_account);
        let symbol = cell(&cols, &cells, "symbol")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let drip_class = crf_drip_class(&account_name, symbol.as_deref(), amount_minor);
        if drip_class == CrfDripClass::ZeroDrop {
            out.dropped.push(DroppedBrokerRow {
                action: action.to_string(),
                reason: "crf_zero",
            });
            continue;
        }
        let activity_type = if drip_class == CrfDripClass::Drip {
            "drip".to_string()
        } else {
            "dividend".to_string()
        };
        let date_raw = match layout {
            BrokerLayout::Fidelity => date_cell(&cols, &cells, "run date")
                .or_else(|| date_cell(&cols, &cells, "date"))
                .unwrap_or(""),
            BrokerLayout::Schwab => date_cell(&cols, &cells, "date").unwrap_or(""),
        };
        out.candidates.push(ImportCandidate {
            account_name,
            symbol,
            activity_type,
            amount_minor,
            scale: 2,
            occurred_on: parse_broker_day(date_raw).unwrap_or_default(),
        });
    }
    Ok(out)
}

fn strip_bom(s: &str) -> &str {
    s.strip_prefix('\u{feff}').unwrap_or(s)
}

fn cell<'a>(cols: &[String], cells: &'a [String], name: &str) -> Option<&'a str> {
    cols.iter()
        .position(|c| c == name)
        .and_then(|i| cells.get(i))
        .map(|s| s.as_str())
}

fn amount_cell<'a>(cols: &[String], cells: &'a [String]) -> Option<&'a str> {
    cols.iter()
        .position(|c| c == "amount" || c.starts_with("amount ") || c.starts_with("amount("))
        .and_then(|i| cells.get(i))
        .map(|s| s.as_str())
}

fn date_cell<'a>(cols: &[String], cells: &'a [String], name: &str) -> Option<&'a str> {
    cell(cols, cells, name).or_else(|| {
        cols.iter()
            .position(|c| c.starts_with(name))
            .and_then(|i| cells.get(i))
            .map(|s| s.as_str())
    })
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

pub fn parse_usd_minor(raw: &str) -> Option<i64> {
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

pub fn parse_broker_day(raw: &str) -> Option<String> {
    let t = raw.trim();
    if t.len() >= 10 && t.as_bytes().get(4) == Some(&b'-') {
        return Some(t.chars().take(10).collect());
    }
    let sep = if t.contains('/') { '/' } else { '-' };
    let parts: Vec<&str> = t.split(sep).collect();
    if parts.len() != 3 {
        return None;
    }
    let month: u32 = parts[0].parse().ok()?;
    let day: u32 = parts[1].parse().ok()?;
    let year: u32 = parts[2].parse().ok()?;
    if year < 100 {
        return None;
    }
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

    const FIDELITY_HISTORY: &str = "\u{feff}Account History\n\
Downloaded — redacted\n\
\n\
Run Date,Account,Account Number,Action,Symbol,Description,Type,Price ($),Quantity,Commission ($),Fees ($),Accrued Interest ($),Amount ($),Settlement Date\n\
\n\
08-29-2026,Income,XXXXX,DIVIDEND RECEIVED ROUNDHILL ETF TRUST NVDW (NVDW) (Cash),NVDW,ROUNDHILL,Cash,\"\",0,\"\",\"\",\"\",75.25,\"\"\n\
08-31-2026,Income,XXXXX,DIVIDEND RECEIVED FIDELITY GOVERNMENT MONEY MARKET (SPAXX) (Cash),SPAXX,SPAXX,Cash,\"\",0,\"\",\"\",\"\",1.12,\"\"\n\
08-31-2026,For the CAR,XXXXXX,YOU BOUGHT HAKY (Cash),HAKY,AMPLIFY,Cash,12.00,10,\"\",\"\",\"\",-120.00,\"\"\n\
08-30-2026,Income,XXXXX,JOURNALED JNL VS A/C TYPES (Cash),,No Description,Cash,\"\",0,\"\",\"\",\"\",41.64,\"\"\n\
08-30-2026,Income,XXXXX,FED TAX W/H RET (Cash),,No Description,Cash,\"\",0,\"\",\"\",\"\",-12.00,\"\"\n";

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
        assert_eq!(
            detect_broker(FIDELITY.lines().next().unwrap()),
            Some(BrokerLayout::Fidelity)
        );
        assert_eq!(
            detect_broker(SCHWAB.lines().next().unwrap()),
            Some(BrokerLayout::Schwab)
        );
    }

    #[test]
    fn history_preamble_amount_column_hyphen_dates_and_alias() {
        assert_eq!(
            find_broker_header(FIDELITY_HISTORY).map(|(l, _)| l),
            Some(BrokerLayout::Fidelity)
        );
        let parsed =
            parse_broker_csv_detail(BrokerLayout::Fidelity, FIDELITY_HISTORY, None).unwrap();
        assert_eq!(parsed.candidates.len(), 2, "{parsed:?}");
        assert_eq!(parsed.dropped.len(), 3, "{:?}", parsed.dropped);
        let nvdw = parsed
            .candidates
            .iter()
            .find(|c| c.symbol.as_deref() == Some("NVDW"))
            .unwrap();
        assert_eq!(nvdw.amount_minor, Some(7_525));
        assert_eq!(nvdw.occurred_on, "2026-08-29");
        let spaxx = parsed
            .candidates
            .iter()
            .find(|c| c.symbol.as_deref() == Some("SPAXX"))
            .unwrap();
        assert_eq!(spaxx.amount_minor, Some(112));
        assert_eq!(resolve_account_name("For the CAR", None), "Car");
        assert_eq!(
            classify_action("REINVESTMENT FIDELITY GOVERNMENT MONEY MARKET (SPAXX)"),
            ActionClass::Drop("reinvestment")
        );
        let reasons: Vec<_> = parsed.dropped.iter().map(|d| d.reason).collect();
        assert!(reasons.contains(&"purchase"));
        assert!(reasons.contains(&"journal"));
        assert!(reasons.contains(&"tax"));
    }

    const FIDELITY_ACTIVITY: &str = "History: All Accounts\n\
From: 08/01/2026\n\
\n\
\"Date\",\"Account\",\"Symbol\",\"Description\",\"Quantity\",\"Price\",\"Amount\",\"Commission\",\"Fees\",\"Type\"\n\
\"08/20/2026\",\"Income (00000)\",\"YMAX\",\"DIVIDEND RECEIVED\",\"0\",\"$0.00\",\"$5.21\",\"$0.00\",\"$0.00\",\"Cash\"\n\
\"08/31/2026\",\"Income (00000)\",\"SVOL\",\"RETURN OF CAPITAL\",\"0\",\"$0.00\",\"$51.52\",\"$0.00\",\"$0.00\",\"Margin\"\n\
\"08/30/2026\",\"Income (00000)\",,\"JOURNALED JNL VS A/C TYPES\",\"0\",\"$0.00\",\"$41.64\",\"$0.00\",\"$0.00\",\"Cash\"\n\
\"08/31/2026\",\"For the CAR (00000)\",\"HAKY\",\"YOU BOUGHT\",\"10\",\"$12.00\",\"-$120.00\",\"$0.00\",\"$0.00\",\"Cash\"\n";

    #[test]
    fn fidelity_all_accounts_activity_export_keeps_dividend_and_roc() {
        assert_eq!(
            find_broker_header(FIDELITY_ACTIVITY).map(|(l, _)| l),
            Some(BrokerLayout::Fidelity)
        );
        let parsed =
            parse_broker_csv_detail(BrokerLayout::Fidelity, FIDELITY_ACTIVITY, None).unwrap();
        assert_eq!(parsed.candidates.len(), 2, "{parsed:?}");
        let ymax = parsed
            .candidates
            .iter()
            .find(|c| c.symbol.as_deref() == Some("YMAX"))
            .unwrap();
        assert_eq!(ymax.account_name, "Income");
        assert_eq!(ymax.amount_minor, Some(521));
        assert_eq!(ymax.occurred_on, "2026-08-20");
        let svol = parsed
            .candidates
            .iter()
            .find(|c| c.symbol.as_deref() == Some("SVOL"))
            .unwrap();
        assert_eq!(svol.amount_minor, Some(5_152));
        assert_eq!(resolve_account_name("For the CAR (249540701)", None), "Car");
        assert_eq!(
            classify_action("RETURN OF CAPITAL"),
            ActionClass::KeepDividend
        );
    }

    const FIDELITY_CRF: &str = "History: All Accounts\n\
From: 08/01/2026\n\
\n\
\"Date\",\"Account\",\"Symbol\",\"Description\",\"Quantity\",\"Price\",\"Amount\",\"Commission\",\"Fees\",\"Type\"\n\
\"08/06/2026\",\"ROTH IRA (00000)\",\"CRF\",\"DIVIDEND RECEIVED\",\"0\",\"$0.00\",\"$3.18\",\"$0.00\",\"$0.00\",\"Cash\"\n\
\"08/04/2026\",\"ROTH IRA (00000)\",\"CRF\",\"DIVIDEND RECEIVED\",\"0\",\"$0.00\",\"$0.00\",\"$0.00\",\"$0.00\",\"Cash\"\n\
\"08/06/2026\",\"Income (00000)\",\"CRF\",\"DIVIDEND RECEIVED\",\"0\",\"$0.00\",\"$10.00\",\"$0.00\",\"$0.00\",\"Cash\"\n";

    #[test]
    fn fi_roth_crf_dividend_is_drip_not_cash() {
        assert_eq!(
            crf_drip_class("FI Roth", Some("CRF"), Some(318)),
            CrfDripClass::Drip
        );
        assert_eq!(
            crf_drip_class("FI Roth", Some("CRF"), Some(0)),
            CrfDripClass::ZeroDrop
        );
        assert_eq!(
            crf_drip_class("Income", Some("CRF"), Some(1000)),
            CrfDripClass::CashDividend
        );
        let parsed = parse_broker_csv_detail(BrokerLayout::Fidelity, FIDELITY_CRF, None).unwrap();
        let drip = parsed
            .candidates
            .iter()
            .find(|c| c.account_name == "FI Roth" && c.symbol.as_deref() == Some("CRF"))
            .unwrap();
        assert_eq!(drip.activity_type, "drip");
        assert_eq!(drip.amount_minor, Some(318));
        assert_eq!(drip.occurred_on, "2026-08-06");
        let income = parsed
            .candidates
            .iter()
            .find(|c| c.account_name == "Income" && c.symbol.as_deref() == Some("CRF"))
            .unwrap();
        assert_eq!(income.activity_type, "dividend");
        assert!(parsed.dropped.iter().any(|d| d.reason == "crf_zero"));
        assert_eq!(parsed.candidates.len(), 2, "{parsed:?}");
    }
}
