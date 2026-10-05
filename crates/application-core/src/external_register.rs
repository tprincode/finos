//! External-account register: date fixes from the tracker sheet, search, and cents.

use rust_xlsxwriter::Workbook;
use serde_json::Value;
use uuid::Uuid;

use crate::contracts::{
    ExternalRegisterExportGetBody, ExternalRegisterExportLine, ExternalRegisterLine,
};
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
        "medical-mom" | "medical mom" | "medicalmom" => "Medical-mom".into(),
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
        step_billpay_deposit: row
            .get("stepBillpayDeposit")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        step_pay: row.get("stepPay").and_then(|v| v.as_bool()).unwrap_or(false),
        step_withdrawal: row.get("stepWithdrawal").and_then(|v| v.as_bool()).unwrap_or(false),
        step_transfer_on: None,
        step_billpay_on: None,
        step_billpay_deposit_on: None,
        step_pay_on: None,
        step_withdrawal_on: None,
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

pub fn mark_step_from_json(
    json: &Value,
) -> Result<(Vec<Uuid>, String, Option<String>), PlatformError> {
    let (line_ids, _) = true_up_from_json(json)?;
    let step = json
        .get("step")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if !matches!(
        step.as_str(),
        "transfer" | "billpay" | "billpay_deposit" | "pay" | "withdrawal"
    ) {
        return Err(PlatformError::new(
            "bad_step",
            "step must be transfer, billpay, billpay_deposit, pay, or withdrawal",
        ));
    }
    let ticked_on = blank_to_none(
        json.get("tickedOn")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    );
    Ok((line_ids, step, ticked_on))
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
            step_billpay_deposit: true,
            step_pay: true,
            step_withdrawal: true,
            step_transfer_on: None,
            step_billpay_on: None,
            step_billpay_deposit_on: None,
            step_pay_on: None,
            step_withdrawal_on: None,
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
        assert_eq!(normalize_category("medical-mom"), "Medical-mom");
        assert_eq!(normalize_category("Medical mom"), "Medical-mom");
        assert_eq!(normalize_vendor("amz"), "AMZ");
        assert_eq!(normalize_vendor("Amazon"), "AMZ");
        assert_eq!(normalize_pay_type("ucard"), "UCARD");
        assert_eq!(normalize_pay_type("Ucard"), "UCARD");
        assert_eq!(normalize_pay_type("Cheking"), "Checking");
        assert_eq!(normalize_pay_type("cap"), "CAP");
        assert_eq!(normalize_pay_type("Cap"), "CAP");
    }
}

fn export_lines_from_json(json: &Value) -> Result<Vec<ExternalRegisterExportLine>, PlatformError> {
    let Some(arr) = json.get("lines").and_then(|v| v.as_array()) else {
        return Err(PlatformError::new(
            "missing_lines",
            "ExternalRegisterExportGet needs lines",
        ));
    };
    let mut out = Vec::with_capacity(arr.len());
    for row in arr {
        out.push(ExternalRegisterExportLine {
            pay_type: row
                .get("payType")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            occurred_on: row
                .get("occurredOn")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .filter(|s| !s.trim().is_empty()),
            amount_minor: row
                .get("amountMinor")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            scale: row
                .get("scale")
                .and_then(|v| v.as_u64())
                .unwrap_or(2)
                .min(8) as u8,
            category: row
                .get("category")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            vendor: row
                .get("vendor")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            description: row
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            true_up_on: row
                .get("trueUpOn")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .filter(|s| !s.trim().is_empty()),
        });
    }
    Ok(out)
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn fmt_money(minor: i64, scale: u8) -> String {
    let div = 10_i64.pow(u32::from(scale));
    let sign = if minor < 0 { "-" } else { "" };
    let abs = minor.abs();
    let whole = abs / div;
    let frac = abs % div;
    format!("{sign}{whole}.{:0width$}", frac, width = scale as usize)
}

fn completed_print_html(lines: &[ExternalRegisterExportLine], printed_at: &str) -> String {
    let mut rows = String::new();
    for line in lines {
        rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td class=\"numeric\">{}</td><td>{}</td><td>{}</td><td>{}</td><td class=\"completed\">{}</td></tr>",
            escape_html(&line.pay_type),
            escape_html(line.occurred_on.as_deref().unwrap_or("")),
            escape_html(&fmt_money(line.amount_minor, line.scale)),
            escape_html(&line.category),
            escape_html(&line.vendor),
            escape_html(&line.description),
            escape_html(line.true_up_on.as_deref().unwrap_or("")),
        ));
    }
    format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><title>CCT Completed</title>
<style>
body {{ font-family: Segoe UI, Arial, sans-serif; font-size: 11px; color: #111; }}
h1 {{ font-size: 16px; margin: 0 0 0.4rem; }}
p {{ margin: 0 0 0.6rem; }}
table {{ border-collapse: collapse; width: 100%; }}
th, td {{ border: 1px solid #bbb; padding: 0.2rem 0.35rem; text-align: left; }}
th {{ background: #f0f0f0; }}
td.numeric {{ text-align: right; }}
td.completed {{ background: #7dcea0; font-weight: 700; }}
</style></head><body>
<h1>Checking and Credit Transactions — Completed</h1>
<p>Printed {printed} · {count} rows</p>
<table>
<thead><tr><th>Pay type</th><th>Date</th><th>Total spent</th><th>Category</th><th>Vendor</th><th>Description</th><th>Completed</th></tr></thead>
<tbody>{rows}</tbody>
</table>
</body></html>"#,
        printed = escape_html(printed_at),
        count = lines.len(),
        rows = rows
    )
}

fn completed_text(lines: &[ExternalRegisterExportLine], printed_at: &str) -> String {
    let mut text = format!(
        "CCT Completed\nPrinted {printed_at}\nRows {}\nPay type | Date | Total spent | Category | Vendor | Description | Completed\n",
        lines.len()
    );
    for line in lines {
        text.push_str(&format!(
            "{} | {} | {} | {} | {} | {} | {}\n",
            line.pay_type,
            line.occurred_on.as_deref().unwrap_or(""),
            fmt_money(line.amount_minor, line.scale),
            line.category,
            line.vendor,
            line.description,
            line.true_up_on.as_deref().unwrap_or(""),
        ));
    }
    text
}

fn simple_pdf(text: &str) -> Vec<u8> {
    let (w, h) = (792, 612);
    let escaped = text
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)");
    let mut tj = String::from("BT /F1 8 Tf 24  ");
    tj.push_str(&(h - 28).to_string());
    tj.push_str(" Td 10 TL ");
    for line in escaped.lines().take(55) {
        tj.push('(');
        tj.push_str(line);
        tj.push_str(")' T* ");
    }
    tj.push_str("ET");
    let stream = tj.into_bytes();
    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objects.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    objects.push(
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w} {h}] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>"
        )
        .into_bytes(),
    );
    let mut contents = format!("<< /Length {} >>\nstream\n", stream.len()).into_bytes();
    contents.extend_from_slice(&stream);
    contents.extend_from_slice(b"\nendstream");
    objects.push(contents);
    objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec());
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0u32];
    for (i, obj) in objects.iter().enumerate() {
        offsets.push(out.len() as u32);
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(obj);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in offsets.iter().skip(1) {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

fn completed_xlsx(lines: &[ExternalRegisterExportLine]) -> Result<Vec<u8>, String> {
    let mut wb = Workbook::new();
    let sheet = wb.add_worksheet();
    sheet.set_name("Completed").map_err(|e| e.to_string())?;
    let headers = [
        "Pay type",
        "Date",
        "Total spent",
        "Category",
        "Vendor",
        "Description",
        "Completed",
    ];
    for (c, name) in headers.iter().enumerate() {
        sheet
            .write_string(0, c as u16, *name)
            .map_err(|e| e.to_string())?;
    }
    for (r, line) in lines.iter().enumerate() {
        let row = (r + 1) as u32;
        sheet
            .write_string(row, 0, &line.pay_type)
            .map_err(|e| e.to_string())?;
        sheet
            .write_string(row, 1, line.occurred_on.as_deref().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        sheet
            .write_string(row, 2, &fmt_money(line.amount_minor, line.scale))
            .map_err(|e| e.to_string())?;
        sheet
            .write_string(row, 3, &line.category)
            .map_err(|e| e.to_string())?;
        sheet
            .write_string(row, 4, &line.vendor)
            .map_err(|e| e.to_string())?;
        sheet
            .write_string(row, 5, &line.description)
            .map_err(|e| e.to_string())?;
        sheet
            .write_string(row, 6, line.true_up_on.as_deref().unwrap_or(""))
            .map_err(|e| e.to_string())?;
    }
    wb.save_to_buffer().map_err(|e| e.to_string())
}

fn b64(bytes: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i];
        let b1 = if i + 1 < bytes.len() { bytes[i + 1] } else { 0 };
        let b2 = if i + 2 < bytes.len() { bytes[i + 2] } else { 0 };
        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        if i + 1 < bytes.len() {
            out.push(T[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if i + 2 < bytes.len() {
            out.push(T[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
        i += 3;
    }
    out
}

/// Format the Completed CCT rows the UI already filtered. No SQLite round-trip.
pub fn export_completed_from_json(json: &Value) -> Result<ExternalRegisterExportGetBody, PlatformError> {
    let lines = export_lines_from_json(json)?;
    let format = json
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("html")
        .to_ascii_lowercase();
    let printed_at = json
        .get("printedAt")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let stamp = if printed_at.is_empty() {
        chrono::Local::now().format("%Y-%m-%d").to_string()
    } else {
        printed_at.chars().take(10).collect::<String>()
    };
    let print_html = completed_print_html(&lines, &printed_at);
    let (bytes, default_file_name, format_out) = match format.as_str() {
        "xlsx" | "excel" => (
            completed_xlsx(&lines).map_err(|e| PlatformError::new("xlsx", e))?,
            format!("cct-completed-{stamp}.xlsx"),
            "xlsx",
        ),
        "html" | "printdocument" => (
            print_html.as_bytes().to_vec(),
            format!("cct-completed-{stamp}.html"),
            "printHtml",
        ),
        _ => (
            simple_pdf(&completed_text(&lines, &printed_at)),
            format!("cct-completed-{stamp}.pdf"),
            "pdf",
        ),
    };
    Ok(ExternalRegisterExportGetBody {
        format: format_out.into(),
        default_file_name,
        bytes_base64: b64(&bytes),
        print_html,
    })
}
