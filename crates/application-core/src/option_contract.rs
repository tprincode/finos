//! Equity option contracts (OCC). Money is integer cents (scale 2).

use crate::contracts::{OptionContractListBody, OptionContractRecord};
use crate::ports::canonical::Canonical;
use crate::ports::platform::PlatformError;
use chrono::Local;
use serde_json::Value;
use uuid::Uuid;

pub const ERR_NOT_OCC: &str = "Not an OCC symbol.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedOcc {
    pub underlying: String,
    pub expiry_on: String,
    pub put_call: String,
    pub strike_minor: i64,
    pub raw: String,
}

fn expiry_from_yymmdd(yymmdd: &str) -> Option<String> {
    if yymmdd.len() != 6 || !yymmdd.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let yy: i32 = yymmdd[0..2].parse().ok()?;
    let mm: u32 = yymmdd[2..4].parse().ok()?;
    let dd: u32 = yymmdd[4..6].parse().ok()?;
    if !(1..=12).contains(&mm) || !(1..=31).contains(&dd) {
        return None;
    }
    let year = 2000 + yy;
    let date = chrono::NaiveDate::from_ymd_opt(year, mm, dd)?;
    Some(date.format("%Y-%m-%d").to_string())
}

fn dollars_to_strike_minor(raw: &str) -> Option<i64> {
    let n: f64 = raw.parse().ok()?;
    if !n.is_finite() || n < 0.0 {
        return None;
    }
    Some((n * 100.0).round() as i64)
}

/// OSI 21-char and compact broker paste. Strike is cents (scale 2).
pub fn parse_occ(symbol: &str) -> Option<ParsedOcc> {
    let upper = symbol.trim().to_uppercase();
    if upper.is_empty() {
        return None;
    }

    if upper.len() == 21 {
        let root = upper[0..6].trim();
        let yymmdd = &upper[6..12];
        let cp = &upper[12..13];
        let strike_digits = &upper[13..21];
        if root
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
            && yymmdd.chars().all(|c| c.is_ascii_digit())
            && (cp == "C" || cp == "P")
            && strike_digits.chars().all(|c| c.is_ascii_digit())
        {
            let expiry_on = expiry_from_yymmdd(yymmdd)?;
            let strike_raw: i64 = strike_digits.parse().ok()?;
            let strike_minor = strike_raw / 10;
            let raw: String = upper.chars().filter(|c| !c.is_whitespace()).collect();
            return Some(ParsedOcc {
                underlying: root.to_string(),
                expiry_on,
                put_call: cp.to_string(),
                strike_minor,
                raw,
            });
        }
    }

    let compact: String = upper
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .trim_start_matches('.')
        .to_string();
    // ROOT + (YYMMDD | 1YYMMDD) + C|P + decimal strike
    let re = regex_lite_parse(&compact)?;
    Some(re)
}

fn regex_lite_parse(compact: &str) -> Option<ParsedOcc> {
    let bytes = compact.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_uppercase() {
        i += 1;
    }
    if i == 0 || i > 6 {
        return None;
    }
    let underlying = &compact[0..i];
    let rest = &compact[i..];
    let mut j = 0;
    let rb = rest.as_bytes();
    while j < rb.len() && rb[j].is_ascii_digit() {
        j += 1;
    }
    if j != 6 && j != 7 {
        return None;
    }
    let mut yymmdd = rest[0..j].to_string();
    if yymmdd.len() == 7 {
        if !yymmdd.starts_with('1') {
            return None;
        }
        yymmdd = yymmdd[1..].to_string();
    }
    if j >= rb.len() {
        return None;
    }
    let cp = rest.as_bytes()[j] as char;
    if cp != 'C' && cp != 'P' {
        return None;
    }
    let strike_raw = &rest[j + 1..];
    if strike_raw.is_empty() {
        return None;
    }
    let expiry_on = expiry_from_yymmdd(&yymmdd)?;
    let strike_minor = dollars_to_strike_minor(strike_raw)?;
    Some(ParsedOcc {
        underlying: underlying.to_string(),
        expiry_on,
        put_call: cp.to_string(),
        strike_minor,
        raw: compact.to_string(),
    })
}

fn today_iso() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

fn now_stamp() -> String {
    Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

pub async fn contract_cover_preview(
    canonical: &dyn Canonical,
    json: &Value,
) -> Result<crate::contracts::OptionCoverPreviewBody, PlatformError> {
    let account_raw = json.get("accountId").and_then(|v| v.as_str()).unwrap_or("");
    let account_id = Uuid::parse_str(account_raw.trim())
        .map_err(|_| PlatformError::new("account_required", "accountId is required"))?;
    let symbol = json
        .get("symbol")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let quantity = json.get("quantity").and_then(|v| v.as_i64()).unwrap_or(1);
    let securities = canonical.security_list().await?;
    let Some(security) = securities
        .into_iter()
        .find(|row| row.symbol.eq_ignore_ascii_case(&symbol))
    else {
        return Ok(crate::contracts::OptionCoverPreviewBody {
            pieces: Vec::new(),
            average_minor: None,
            scale: 2,
            refused: "uncovered_refused".into(),
        });
    };
    let candidates = canonical
        .option_cover_candidates(account_id, security.security_id)
        .await?;
    let pieces: Vec<_> = candidates
        .into_iter()
        .map(|row| crate::option_cover::SharePiece {
            lot_id: row.lot_id.to_string(),
            opened_on: row.opened_on,
            shares: row.shares,
            cost_minor: row.cost_minor,
        })
        .collect();
    match crate::option_cover::take_cover(pieces, 100 * quantity.max(1)) {
        Ok((taken, _)) => {
            let shares: i64 = taken.iter().map(|row| row.shares).sum();
            let cost: i64 = taken.iter().map(|row| row.cost_minor).sum();
            let average = if shares > 0 { Some(cost / shares) } else { None };
            Ok(crate::contracts::OptionCoverPreviewBody {
                pieces: taken
                    .into_iter()
                    .map(|row| crate::contracts::OptionCoverPreviewPiece {
                        lot_id: row.lot_id,
                        shares: row.shares,
                        cost_minor: row.cost_minor,
                    })
                    .collect(),
                average_minor: average,
                scale: 2,
                refused: String::new(),
            })
        }
        Err(code) => Ok(crate::contracts::OptionCoverPreviewBody {
            pieces: Vec::new(),
            average_minor: None,
            scale: 2,
            refused: code.into(),
        }),
    }
}

pub async fn contract_list(
    canonical: &dyn Canonical,
    status: Option<String>,
) -> Result<OptionContractListBody, PlatformError> {
    Ok(OptionContractListBody {
        items: canonical.option_contract_list(status).await?,
    })
}

pub async fn contract_create(
    canonical: &dyn Canonical,
    json: &Value,
) -> Result<OptionContractRecord, PlatformError> {
    let occ_symbol = json
        .get("occSymbol")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let parsed = parse_occ(&occ_symbol).ok_or_else(|| {
        PlatformError::new(ERR_NOT_OCC, ERR_NOT_OCC)
    })?;
    let side = json
        .get("side")
        .and_then(|v| v.as_str())
        .unwrap_or("short")
        .to_ascii_lowercase();
    if side != "short" && side != "long" {
        return Err(PlatformError::new("bad_side", "side must be short or long"));
    }
    let quantity = json
        .get("quantity")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    if quantity <= 0 {
        return Err(PlatformError::new("bad_qty", "quantity must be positive"));
    }
    let open_premium_minor = json.get("openPremiumMinor").and_then(|v| v.as_i64());
    let open_on = json
        .get("openOn")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if open_on.is_empty() {
        return Err(PlatformError::new("missing_open_on", "openOn is required"));
    }
    let account_raw = json
        .get("accountId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let account_id = Uuid::parse_str(&account_raw)
        .map_err(|_| PlatformError::new("account_required", "accountId is required"))?;
    let account = canonical.account_get(account_id).await?;
    if !crate::option_cover::account_allowed(&account.name) {
        return Err(PlatformError::new(
            "account_refused",
            "contracts are limited to Income, Speculation, and Account 9",
        ));
    }
    let cover = json
        .get("cover")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if let Some(code) = crate::option_cover::refuse_contract(&side, &parsed.put_call, cover) {
        return Err(PlatformError::new(code, code));
    }
    let prior_balance_minor = json
        .get("priorBalanceMinor")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let underlying_last_minor = json.get("underlyingLastMinor").and_then(|v| v.as_i64());
    let mut takes = Vec::new();
    if side == "short" {
        let securities = canonical.security_list().await?;
        let security = securities
            .into_iter()
            .find(|row| row.symbol.eq_ignore_ascii_case(&parsed.underlying));
        let Some(security) = security else {
            return Err(PlatformError::new(
                "uncovered_refused",
                "uncovered_refused",
            ));
        };
        let candidates = canonical
            .option_cover_candidates(account_id, security.security_id)
            .await?;
        let pieces = candidates
            .into_iter()
            .map(|row| crate::option_cover::SharePiece {
                lot_id: row.lot_id.to_string(),
                opened_on: row.opened_on,
                shares: row.shares,
                cost_minor: row.cost_minor,
            })
            .collect();
        let (taken, _) = crate::option_cover::take_cover(pieces, 100 * quantity)
            .map_err(|code| PlatformError::new(code, code))?;
        takes = taken
            .into_iter()
            .map(|take| {
                Uuid::parse_str(&take.lot_id).map(|lot_id| crate::contracts::OptionCoverTake {
                    lot_id,
                    shares: take.shares,
                    cost_minor: take.cost_minor,
                })
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| PlatformError::new("parse_error", e.to_string()))?;
    }
    let posts = crate::option_cover::posts_for_open(
        &side,
        &parsed.put_call,
        &open_on,
        open_premium_minor,
        prior_balance_minor,
    )
    .map_err(|code| PlatformError::new(code, code))?;
    let stamp = now_stamp();
    let contract_id = Uuid::new_v4();
    let record = OptionContractRecord {
        contract_id,
        occ_symbol: parsed.raw.clone(),
        underlying: parsed.underlying,
        expiry_on: parsed.expiry_on,
        put_call: parsed.put_call.clone(),
        strike_minor: parsed.strike_minor,
        scale: 2,
        account_id: Some(account_id.to_string()),
        side: side.clone(),
        quantity,
        open_premium_minor: open_premium_minor.unwrap_or(0),
        open_premium_blank: open_premium_minor.is_none(),
        open_on: open_on.clone(),
        prior_balance_minor,
        underlying_last_minor,
        live_underlying_minor: None,
        option_mid_minor: None,
        quote_as_of: String::new(),
        status: "open".into(),
        roll_to_contract_id: String::new(),
        close_premium_minor: None,
        closed_on: String::new(),
        payload_json: if open_premium_minor.is_none() {
            "{\"openPremiumBlank\":true}".into()
        } else {
            "{}".into()
        },
        created_on: stamp.clone(),
        updated_on: stamp,
    };
    let saved = canonical.option_contract_insert(record).await?;
    if !takes.is_empty() {
        canonical.option_cover_save(contract_id, &takes).await?;
    }
    for post in posts {
        canonical
            .option_premium_insert(&crate::contracts::OptionPremiumPostRecord {
                post_id: Uuid::new_v4(),
                contract_id,
                week_start: post.week_start,
                category: post.category,
                amount_minor: post.amount_minor,
                scale: 2,
                reason: post.reason,
            })
            .await?;
    }
    Ok(saved)
}

pub async fn contract_quote_set(
    canonical: &dyn Canonical,
    json: &Value,
) -> Result<OptionContractRecord, PlatformError> {
    let id = json
        .get("contractId")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| PlatformError::new("missing_contract_id", "contractId is required"))?;
    let mut row = canonical.option_contract_get(id).await?;
    row.underlying_last_minor = json.get("underlyingLastMinor").and_then(|v| v.as_i64());
    row.option_mid_minor = json.get("optionMidMinor").and_then(|v| v.as_i64());
    row.quote_as_of = json
        .get("quoteAsOf")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(today_iso);
    row.updated_on = now_stamp();
    canonical.option_contract_update(row).await
}

pub async fn contract_roll(
    canonical: &dyn Canonical,
    json: &Value,
) -> Result<OptionContractListBody, PlatformError> {
    let id = json
        .get("contractId")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| PlatformError::new("missing_contract_id", "contractId is required"))?;
    let new_occ = json
        .get("newOccSymbol")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let parsed = parse_occ(&new_occ).ok_or_else(|| {
        PlatformError::new(ERR_NOT_OCC, ERR_NOT_OCC)
    })?;
    let close_premium_minor = json
        .get("closePremiumMinor")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| PlatformError::new("missing_close_premium", "closePremiumMinor is required"))?;
    let new_open_premium_minor = json
        .get("newOpenPremiumMinor")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| {
            PlatformError::new("missing_new_premium", "newOpenPremiumMinor is required")
        })?;
    let new_open_on = json
        .get("newOpenOn")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if new_open_on.is_empty() {
        return Err(PlatformError::new("missing_open_on", "newOpenOn is required"));
    }
    let mut old = canonical.option_contract_get(id).await?;
    if let Some(code) = crate::option_cover::refuse_contract(&old.side, &parsed.put_call, "") {
        return Err(PlatformError::new(code, code));
    }
    if old.side == "short" && !old.underlying.eq_ignore_ascii_case(&parsed.underlying) {
        return Err(PlatformError::new(
            "uncovered_refused",
            "a roll onto a different underlying must reserve a new cover",
        ));
    }
    if old.status != "open" {
        return Err(PlatformError::new("not_open", "only open contracts can roll"));
    }
    let quantity = json
        .get("quantity")
        .and_then(|v| v.as_i64())
        .unwrap_or(old.quantity);
    if quantity <= 0 {
        return Err(PlatformError::new("bad_qty", "quantity must be positive"));
    }
    let stamp = now_stamp();
    let new_id = Uuid::new_v4();
    let new_row = OptionContractRecord {
        contract_id: new_id,
        occ_symbol: parsed.raw.clone(),
        underlying: parsed.underlying,
        expiry_on: parsed.expiry_on,
        put_call: parsed.put_call.clone(),
        strike_minor: parsed.strike_minor,
        scale: 2,
        account_id: old.account_id.clone(),
        side: old.side.clone(),
        quantity,
        open_premium_minor: new_open_premium_minor,
        open_premium_blank: false,
        open_on: new_open_on.clone(),
        prior_balance_minor: old.prior_balance_minor,
        underlying_last_minor: old.underlying_last_minor,
        live_underlying_minor: None,
        option_mid_minor: None,
        quote_as_of: String::new(),
        status: "open".into(),
        roll_to_contract_id: String::new(),
        close_premium_minor: None,
        closed_on: String::new(),
        payload_json: "{}".into(),
        created_on: stamp.clone(),
        updated_on: stamp.clone(),
    };
    old.status = "rolled".into();
    old.close_premium_minor = Some(close_premium_minor);
    old.closed_on = new_open_on.clone();
    old.roll_to_contract_id = new_id.to_string();
    old.updated_on = stamp;
    let side = old.side.clone();
    let old_right = old.put_call.clone();
    let old_open = old.open_premium_minor;
    canonical.option_contract_update(old).await?;
    canonical.option_contract_insert(new_row).await?;
    if side == "short" {
        canonical.option_cover_move(id, new_id).await?;
    }
    let close_post = crate::option_cover::post_for_close(
        &side,
        &old_right,
        &new_open_on,
        old_open,
        close_premium_minor,
    )
    .map_err(|code| PlatformError::new(code, code))?;
    canonical
        .option_premium_insert(&crate::contracts::OptionPremiumPostRecord {
            post_id: Uuid::new_v4(),
            contract_id: id,
            week_start: close_post.week_start,
            category: close_post.category,
            amount_minor: close_post.amount_minor,
            scale: 2,
            reason: close_post.reason,
        })
        .await?;
    for post in crate::option_cover::posts_for_open(
        &side,
        &parsed.put_call,
        &new_open_on,
        Some(new_open_premium_minor),
        0,
    )
    .map_err(|code| PlatformError::new(code, code))?
    {
        canonical
            .option_premium_insert(&crate::contracts::OptionPremiumPostRecord {
                post_id: Uuid::new_v4(),
                contract_id: new_id,
                week_start: post.week_start,
                category: post.category,
                amount_minor: post.amount_minor,
                scale: 2,
                reason: post.reason,
            })
            .await?;
    }
    contract_list(canonical, None).await
}

pub async fn contract_close(
    canonical: &dyn Canonical,
    json: &Value,
) -> Result<OptionContractRecord, PlatformError> {
    let id = json
        .get("contractId")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| PlatformError::new("missing_contract_id", "contractId is required"))?;
    let how = json
        .get("how")
        .and_then(|v| v.as_str())
        .unwrap_or("closed")
        .to_ascii_lowercase();
    let status = if how == "assigned" {
        "assigned"
    } else {
        "closed"
    };
    let close_premium_minor = json.get("closePremiumMinor").and_then(|v| v.as_i64());
    let closed_on = json
        .get("closedOn")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if closed_on.is_empty() {
        return Err(PlatformError::new("missing_closed_on", "closedOn is required"));
    }
    let mut row = canonical.option_contract_get(id).await?;
    if row.status != "open" {
        return Err(PlatformError::new("not_open", "only open contracts can close"));
    }
    let side = row.side.clone();
    let right = row.put_call.clone();
    let open_premium = row.open_premium_minor;
    row.status = status.into();
    row.close_premium_minor = close_premium_minor;
    row.closed_on = closed_on.clone();
    row.updated_on = now_stamp();
    let saved = canonical.option_contract_update(row).await?;
    if how == "assigned" {
        canonical.option_cover_assign(id).await?;
    } else {
        canonical.option_cover_release(id).await?;
    }
    if let Some(close_premium) = close_premium_minor {
        let post = crate::option_cover::post_for_close(
            &side,
            &right,
            &closed_on,
            open_premium,
            close_premium,
        )
        .map_err(|code| PlatformError::new(code, code))?;
        canonical
            .option_premium_insert(&crate::contracts::OptionPremiumPostRecord {
                post_id: Uuid::new_v4(),
                contract_id: id,
                week_start: post.week_start,
                category: post.category,
                amount_minor: post.amount_minor,
                scale: 2,
                reason: post.reason,
            })
            .await?;
    }
    Ok(saved)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_tsll_fixtures() {
        let a = parse_occ(".TSLL1270115C20.7").expect("dot form");
        assert_eq!(a.underlying, "TSLL");
        assert_eq!(a.expiry_on, "2027-01-15");
        assert_eq!(a.put_call, "C");
        assert_eq!(a.strike_minor, 2070);

        let b = parse_occ("TSLL270115C20.7").expect("short form");
        assert_eq!(b.underlying, "TSLL");
        assert_eq!(b.expiry_on, "2027-01-15");
        assert_eq!(b.strike_minor, 2070);

        let c = parse_occ(".AAPL251219P250").expect("aapl put");
        assert_eq!(c.underlying, "AAPL");
        assert_eq!(c.expiry_on, "2025-12-19");
        assert_eq!(c.put_call, "P");
        assert_eq!(c.strike_minor, 25000);
    }

    #[test]
    fn osi_aapl_call() {
        let p = parse_occ("AAPL  250117C00150000").expect("osi");
        assert_eq!(p.underlying, "AAPL");
        assert_eq!(p.expiry_on, "2025-01-17");
        assert_eq!(p.put_call, "C");
        assert_eq!(p.strike_minor, 15000);
    }

    #[test]
    fn bad_symbol_is_none() {
        assert!(parse_occ("TSLL").is_none());
        assert!(parse_occ("").is_none());
    }
}
