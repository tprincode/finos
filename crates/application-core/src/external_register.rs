//! External-account register: date fixes from the tracker sheet, search, and cents.

use serde_json::Value;
use uuid::Uuid;

use crate::contracts::ExternalRegisterLine;
use crate::ports::platform::PlatformError;

/// Spreadsheet rows whose Excel date is outside 2021–2026.
pub fn fixed_occurred_on(source_row: i64) -> Option<&'static str> {
    match source_row {
        23 => Some("2026-09-16"),
        125 => Some("2026-08-14"),
        174 => Some("2026-07-15"),
        340 => Some("2026-04-10"),
        673 => Some("2025-11-15"),
        701 => Some("2025-10-24"),
        800 => Some("2025-09-27"),
        _ => None,
    }
}

pub fn normalize_pay_type(raw: &str) -> String {
    let trimmed = raw.trim();
    match trimmed.to_lowercase().as_str() {
        "" => String::new(),
        "ucard" => "UCARD".into(),
        "cap" => "CAP".into(),
        "checking" | "cheking" => "Checking".into(),
        "sab" | "sable" => "SAB".into(),
        "ppmc" => "PPMC".into(),
        "ppalmc" | "ppal mc" => "PPALMC".into(),
        "ppal" | "ppay" => "PPAL".into(),
        "paypal" => "PAYPAL".into(),
        "pm" | "pmc" => "PM".into(),
        "bjs" | "bj" => "BJS".into(),
        "boa" => "BOA".into(),
        "amz" | "amazon" => "AMZ".into(),
        "hsa" => "HSA".into(),
        "up" => "UP".into(),
        "ebay" => "EBAY".into(),
        "optim" => "OPTIM".into(),
        "check-u" => "Check-U".into(),
        "check-c" => "Check-C".into(),
        _ => trimmed.to_string(),
    }
}

pub fn normalize_category(raw: &str) -> String {
    let trimmed = raw.trim();
    match trimmed.to_lowercase().as_str() {
        "" => String::new(),
        "food" => "Food".into(),
        "cash acct" => "Cash acct".into(),
        "bill acct" => "Bill acct".into(),
        "medical" | "med" => "Medical".into(),
        "pets" => "Pets".into(),
        "mom" => "Mom".into(),
        "home" => "Home".into(),
        "house" => "House".into(),
        "work" => "Work".into(),
        "other" => "Other".into(),
        "rest" => "Rest".into(),
        "tom" => "Tom".into(),
        "gas" => "Gas".into(),
        "auto" => "Auto".into(),
        "barb" => "Barb".into(),
        "hsa" => "HSA".into(),
        "amz credits" => "AMZ Credits".into(),
        "tax refund" => "Tax Refund".into(),
        "taxes" => "Taxes".into(),
        "insurance" => "Insurance".into(),
        "training" => "Training".into(),
        "trading" => "Trading".into(),
        "checking" => "Checking".into(),
        "ira" => "IRA".into(),
        "yikes" => "Yikes".into(),
        "cori" => "Cori".into(),
        _ => trimmed.to_string(),
    }
}

pub fn normalize_vendor(raw: &str) -> String {
    let trimmed = raw.trim();
    let key = trimmed.to_lowercase();
    match key.as_str() {
        "" => return String::new(),
        "amz" | "amazon" | "amx" => return "AMZ".into(),
        "cashback" => return "Cashback".into(),
        "bjs" => return "BJS".into(),
        "walmart" | "waklmart" | "wakmart" => return "Walmart".into(),
        "weg" | "wegmans" | "wegman" => return "Weg".into(),
        "hd" | "homedepot" => return "HD".into(),
        "cvs" => return "CVS".into(),
        "hsa" => return "HSA".into(),
        "usps" | "uspo" => return "USPS".into(),
        "ezpass" | "ez pass" => return "EZ Pass".into(),
        "fl" => return "FL".into(),
        "ff" => return "FF".into(),
        "hf" => return "HF".into(),
        _ => {}
    }
    trimmed
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn amount_to_minor(value: f64) -> i64 {
    (value * 100.0).round() as i64
}

pub fn amount_search_text(minor: i64, scale: u8) -> String {
    let scale = u32::from(scale.max(1));
    let div = 10i64.pow(scale);
    let neg = minor < 0;
    let abs = minor.abs();
    let whole = abs / div;
    let frac = abs % div;
    let fixed = format!(
        "{}{whole}.{frac:0width$}",
        if neg { "-" } else { "" },
        width = scale as usize
    );
    if frac == 0 {
        format!("{fixed} {whole}")
    } else {
        fixed
    }
}

pub fn line_matches(line: &ExternalRegisterLine, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    let amount = amount_search_text(line.amount_minor, line.scale);
    let fields = [
        line.pay_type.as_str(),
        line.occurred_on.as_deref().unwrap_or(""),
        amount.as_str(),
        line.category.as_str(),
        line.vendor.as_str(),
        line.description.as_str(),
        line.true_up_on.as_deref().unwrap_or(""),
    ];
    fields.iter().any(|field| field.to_lowercase().contains(&q))
}

fn blank_to_none(value: Option<String>) -> Option<String> {
    value.and_then(|s| {
        let t = s.trim().to_string();
        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    })
}

pub fn line_from_json(row: &Value) -> Result<ExternalRegisterLine, PlatformError> {
    let line_id = row
        .get("lineId")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| PlatformError::new("missing_line_id", "each register row needs a line id"))?;
    let pay_type = normalize_pay_type(
        row.get("payType")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
    );
    let occurred_on = blank_to_none(
        row.get("occurredOn")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    );
    let amount_minor = row
        .get("amountMinor")
        .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|n| n.round() as i64)))
        .ok_or_else(|| PlatformError::new("missing_amount", "each register row needs an amount"))?;
    let scale = row
        .get("scale")
        .and_then(|v| v.as_u64())
        .map(|n| n as u8)
        .unwrap_or(2);
    Ok(ExternalRegisterLine {
        line_id,
        source_row: line_source_row(row),
        pay_type,
        occurred_on,
        amount_minor,
        scale,
        category: normalize_category(
            row.get("category")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        ),
        vendor: normalize_vendor(
            row.get("vendor")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        ),
        description: row
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string(),
        true_up_on: blank_to_none(
            row.get("trueUpOn")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        ),
        completed: row.get("completed").and_then(|v| v.as_bool()).unwrap_or(false),
        step_transfer: row.get("stepTransfer").and_then(|v| v.as_bool()).unwrap_or(false),
        step_billpay: row.get("stepBillpay").and_then(|v| v.as_bool()).unwrap_or(false),
        step_pay: row.get("stepPay").and_then(|v| v.as_bool()).unwrap_or(false),
    })
}

fn line_source_row(row: &Value) -> Option<i64> {
    row.get("sourceRow").and_then(|v| {
        if v.is_null() {
            None
        } else {
            v.as_i64()
        }
    })
}

pub fn lines_from_json(json: &Value) -> Result<Vec<ExternalRegisterLine>, PlatformError> {
    let rows = json
        .get("lines")
        .and_then(|v| v.as_array())
        .ok_or_else(|| PlatformError::new("missing_lines", "register save needs lines"))?;
    rows.iter().map(line_from_json).collect()
}

pub fn true_up_from_json(json: &Value) -> Result<(Vec<Uuid>, Option<String>), PlatformError> {
    let ids = json
        .get("lineIds")
        .and_then(|v| v.as_array())
        .ok_or_else(|| PlatformError::new("missing_line_ids", "true up needs line ids"))?;
    let mut line_ids = Vec::new();
    for id in ids {
        let text = id.as_str().unwrap_or("");
        let parsed = Uuid::parse_str(text).map_err(|_| {
            PlatformError::new("bad_line_id", "true up line id is not a uuid")
        })?;
        line_ids.push(parsed);
    }
    if line_ids.is_empty() {
        return Err(PlatformError::new(
            "missing_line_ids",
            "true up needs at least one row",
        ));
    }
    let true_up_on = blank_to_none(
        json.get("trueUpOn")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    );
    Ok((line_ids, true_up_on))
}

pub fn mark_step_from_json(json: &Value) -> Result<(Vec<Uuid>, String), PlatformError> {
    let (line_ids, _) = true_up_from_json(json)?;
    let step = json
        .get("step")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if !matches!(step.as_str(), "transfer" | "billpay" | "pay") {
        return Err(PlatformError::new(
            "bad_step",
            "step must be transfer, billpay, or pay",
        ));
    }
    Ok((line_ids, step))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seven_bad_dates_match_the_plan() {
        assert_eq!(fixed_occurred_on(23), Some("2026-09-16"));
        assert_eq!(fixed_occurred_on(125), Some("2026-08-14"));
        assert_eq!(fixed_occurred_on(174), Some("2026-07-15"));
        assert_eq!(fixed_occurred_on(340), Some("2026-04-10"));
        assert_eq!(fixed_occurred_on(673), Some("2025-11-15"));
        assert_eq!(fixed_occurred_on(701), Some("2025-10-24"));
        assert_eq!(fixed_occurred_on(800), Some("2025-09-27"));
        assert_eq!(fixed_occurred_on(24), None);
    }

    #[test]
    fn cents_round_sheet_amounts() {
        assert_eq!(amount_to_minor(9.47), 947);
        assert_eq!(amount_to_minor(89.0), 8900);
        assert_eq!(amount_to_minor(-3.4), -340);
        assert_eq!(amount_to_minor(-355.803558), -35580);
    }

    #[test]
    fn search_hits_description_and_pay_type_ignoring_case() {
        let line = ExternalRegisterLine {
            line_id: Uuid::nil(),
            source_row: Some(1),
            pay_type: "Ucard".into(),
            occurred_on: Some("2026-06-22".into()),
            amount_minor: 1578,
            scale: 2,
            category: "Cash acct".into(),
            vendor: "amz".into(),
            description: "amz grow bug lights".into(),
            true_up_on: Some("2026-06-27".into()),
            completed: true,
            step_transfer: true,
            step_billpay: true,
            step_pay: true,
        };
        assert!(line_matches(&line, "grow"));
        assert!(line_matches(&line, "UCARD"));
        assert!(line_matches(&line, "15.78"));
        assert!(line_matches(&line, "2026-06-27"));
        assert!(!line_matches(&line, "zzz"));
        assert!(line_matches(&line, "  "));
    }

    #[test]
    fn names_collapse_to_one_spelling() {
        assert_eq!(normalize_category("food"), "Food");
        assert_eq!(normalize_category("Food"), "Food");
        assert_eq!(normalize_vendor("amz"), "AMZ");
        assert_eq!(normalize_vendor("Amazon"), "AMZ");
        assert_eq!(normalize_pay_type("ucard"), "UCARD");
        assert_eq!(normalize_pay_type("Ucard"), "UCARD");
        assert_eq!(normalize_pay_type("Cheking"), "Checking");
        assert_eq!(normalize_pay_type("cap"), "CAP");
        assert_eq!(normalize_pay_type("Cap"), "CAP");
    }
}
