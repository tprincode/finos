//! MLP common-unit declarations from SEC 8-K only. Future pays are derived.
//! Vendor / IR calendars are off. Tests use synthetic MLP1 — not a household ticker.

use chrono::{Datelike, NaiveDate};

pub const ADAPTER_KIND: &str = "mlp_sec_8k";
pub const LEGACY_SOURCE: &str = "energytransfer";
pub const SOURCE_SEC_8K: &str = "sec_8k";
pub const SOURCE_DERIVED_TEMPLATE: &str = "derived_template";
pub const MLP_SEC_CIK: &str = "0001276187";
pub const CODE_SEC_403: &str = "sec_403";
pub const CODE_PAYABLE_DATE_MOVED: &str = "payable_date_moved";
pub const CODE_OWNER_AMOUNT: &str = "mlp_sec_owner_amount";
/// First owner prompt for the next unpaid quarter. Then every three months (7th).
pub const OWNER_AMOUNT_ASK_START: &str = "2026-11-07";

pub fn is_adapter_kind(source: &str) -> bool {
    let s = source.trim().to_ascii_lowercase();
    s == ADAPTER_KIND || s == LEGACY_SOURCE
}

/// Route by kind, IR host, or this CIK. Not by household ticker.
pub fn routes_fetch(source: &str, source_url: Option<&str>) -> bool {
    is_adapter_kind(source)
        || source_url.is_some_and(|u| is_ir_url(u) || is_sec_history_url(u))
}

pub fn last_run_stamp(outcome: &str) -> String {
    format!("mlp_sec_8k:{outcome}")
}

pub fn is_ir_url(url: &str) -> bool {
    let u = url.trim().to_ascii_lowercase();
    if u.is_empty() || u.contains("sec.gov") {
        return false;
    }
    u.contains("ir.energytransfer.com")
        || u.contains("energytransferpartners.gcs-web.com")
        || u.contains("energytransfer.com")
}

pub fn is_sec_history_url(url: &str) -> bool {
    let u = url.trim().to_ascii_lowercase();
    if !u.contains("sec.gov") {
        return false;
    }
    u.contains("cik=0001276187")
        || u.contains("cik=1276187")
        || u.contains("/edgar/data/1276187/")
        || u.contains("/edgar/data/0001276187/")
}

pub fn atom_url() -> String {
    format!(
        "https://www.sec.gov/cgi-bin/browse-edgar?action=getcompany&CIK={MLP_SEC_CIK}&type=8-K&owner=exclude&count=20&output=atom"
    )
}

pub fn sec_403_reason() -> String {
    format!("blocked: sec_403 {}", atom_url())
}

pub fn no_new_8k_miss(pay_on: &str) -> String {
    if pay_on.trim().is_empty() {
        "No common-unit 8-K in this board window.".into()
    } else {
        format!("No common-unit 8-K for {pay_on} this board window.")
    }
}

/// 7th of the payable month, not before 2026-11-07.
pub fn owner_ask_on_for_pay(pay_on: &str) -> Option<String> {
    let d = NaiveDate::parse_from_str(pay_on.trim(), "%Y-%m-%d").ok()?;
    let ask = NaiveDate::from_ymd_opt(d.year(), d.month(), 7)?;
    let start = NaiveDate::parse_from_str(OWNER_AMOUNT_ASK_START, "%Y-%m-%d").ok()?;
    if ask < start {
        None
    } else {
        Some(ask.to_string())
    }
}

/// Prompt only in the ask window, and only if that quarter has no declared $.
pub fn owner_amount_ask_due(as_of: &str, next_pay_on: &str, has_declared_amount: bool) -> bool {
    if has_declared_amount || next_pay_on.trim().is_empty() {
        return false;
    }
    let Some(ask) = owner_ask_on_for_pay(next_pay_on) else {
        return false;
    };
    as_of.trim() >= ask.as_str()
}

pub fn owner_amount_reason(pay_on: &str) -> String {
    format!(
        "Enter the declared quarterly amount for {pay_on}. Future Plan $ is unchanged until you enter it."
    )
}

/// last_run_ok: SEC URL, remaining 2026 date in place, not overdue for an owner amount.
/// A missing 8-K before the Nov 7 window is waiting, not a miss.
pub fn last_run_ok(
    stored_url: &str,
    remaining_derived_ok: bool,
    owner_amount_overdue: bool,
) -> bool {
    is_sec_history_url(stored_url) && remaining_derived_ok && !owner_amount_overdue
}

pub fn business_days_between(from: NaiveDate, to: NaiveDate) -> i64 {
    let (start, end, sign) = if to >= from {
        (from, to, 1i64)
    } else {
        (to, from, -1i64)
    };
    let mut d = start;
    let mut days = 0i64;
    while d < end {
        d = match d.succ_opt() {
            Some(n) => n,
            None => break,
        };
        if d.weekday().number_from_monday() <= 5 {
            days += 1;
        }
    }
    days * sign
}

pub fn within_three_business_days(a: &str, b: &str) -> bool {
    let Ok(da) = NaiveDate::parse_from_str(a.trim(), "%Y-%m-%d") else {
        return false;
    };
    let Ok(db) = NaiveDate::parse_from_str(b.trim(), "%Y-%m-%d") else {
        return false;
    };
    business_days_between(da, db).abs() <= 3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ir_hosts_are_banned_sec_is_not() {
        assert!(is_ir_url("https://ir.energytransfer.com/distribution-history-et"));
        assert!(is_ir_url("https://www.energytransfer.com/investor-relations"));
        assert!(is_ir_url(
            "https://energytransferpartners.gcs-web.com/distribution-history"
        ));
        assert!(!is_ir_url(&atom_url()));
        assert!(is_sec_history_url(&atom_url()));
        assert!(!is_sec_history_url(
            "https://ir.energytransfer.com/distribution-history-et"
        ));
        assert!(is_adapter_kind("mlp_sec_8k"));
        assert!(is_adapter_kind("energytransfer"));
        assert!(!is_adapter_kind("amplify"));
        assert!(routes_fetch(
            "energytransfer",
            Some("https://ir.energytransfer.com/distribution-history-et")
        ));
        assert!(routes_fetch("", Some(&atom_url())));
        assert!(!routes_fetch("amplify", Some("https://amplifyetfs.com/HAKY")));
        assert_eq!(last_run_stamp("page"), "mlp_sec_8k:page");
        assert_eq!(last_run_stamp("sec_403"), "mlp_sec_8k:sec_403");
        assert_eq!(last_run_stamp("empty"), "mlp_sec_8k:empty");
    }

    #[test]
    fn last_run_rejects_ir_and_overdue_owner_amount() {
        let sec = atom_url();
        assert!(last_run_ok(&sec, true, false));
        assert!(!last_run_ok(
            "https://ir.energytransfer.com/distribution-history-et",
            true,
            false
        ));
        assert!(!last_run_ok(&sec, false, false));
        assert!(!last_run_ok(&sec, true, true));
        assert!(!owner_amount_ask_due("2026-09-08", "2026-11-19", false));
        assert!(owner_amount_ask_due("2026-11-07", "2026-11-19", false));
        assert!(!owner_amount_ask_due("2026-11-07", "2026-11-19", true));
        assert_eq!(
            owner_ask_on_for_pay("2026-11-19").as_deref(),
            Some("2026-11-07")
        );
        assert_eq!(
            owner_ask_on_for_pay("2027-02-19").as_deref(),
            Some("2027-02-07")
        );
    }

    #[test]
    fn nov_21_is_two_business_days_from_nov_19_2026() {
        assert!(within_three_business_days("2026-11-19", "2026-11-21"));
        assert!(!within_three_business_days("2026-11-19", "2026-11-27"));
    }
}
