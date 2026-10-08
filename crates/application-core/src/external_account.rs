//! Managed external accounts. Balances stay on this list. Charges are read from
//! the external register by category or vendor. Nothing here is a cash element.

use chrono::{Datelike, Duration, NaiveDate};
use serde_json::Value;

use crate::contracts::ExternalManagedAccountSave;
use crate::ports::platform::PlatformError;

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

#[derive(Debug, Clone)]
pub struct RegisterMatch {
    pub category: String,
    pub bucket: String,
    pub loan_name: String,
    pub vendor: String,
    pub amount_minor: i64,
}

#[derive(Debug, Clone)]
pub struct AccountMatch {
    pub kind: String,
    pub register_key: String,
    pub legacy_vendor: Option<String>,
    pub legacy_amount_minor: Option<i64>,
}

pub fn line_settled(
    completed: bool,
    step_transfer: bool,
    step_billpay: bool,
    step_billpay_deposit: bool,
    step_pay: bool,
    step_withdrawal: bool,
) -> bool {
    completed
        || (step_transfer
            && (step_billpay || step_billpay_deposit)
            && step_pay
            && step_withdrawal)
}

/// Mom's credit drops when the transfer step is marked. A debt drops when the
/// charge is finished (transfer plus Bill Pay Deposit, Pay Bill, and Withdrawal).
pub fn line_applied(
    kind: &str,
    completed: bool,
    step_transfer: bool,
    step_billpay: bool,
    step_billpay_deposit: bool,
    step_pay: bool,
    step_withdrawal: bool,
) -> bool {
    if kind == "credit" {
        completed || step_transfer
    } else {
        line_settled(
            completed,
            step_transfer,
            step_billpay,
            step_billpay_deposit,
            step_pay,
            step_withdrawal,
        )
    }
}

fn same_vendor(vendor: &str, legacy: &str) -> bool {
    let vendor = vendor.trim().to_lowercase();
    let legacy = legacy.trim().to_lowercase();
    if vendor == legacy {
        return true;
    }
    matches!(
        (vendor.as_str(), legacy.as_str()),
        ("alpheon", "alphaeon") | ("alphaeon", "alpheon")
    )
}

pub fn line_matches(line: &RegisterMatch, account: &AccountMatch) -> bool {
    let key = account.register_key.trim().to_lowercase();
    if key.is_empty() {
        return false;
    }
    let category = line.category.trim().to_lowercase();
    let bucket = line.bucket.trim().to_lowercase();
    let loan_name = line.loan_name.trim().to_lowercase();
    let vendor = line.vendor.trim().to_lowercase();
    if account.kind == "credit" {
        // Escrow (Mom): funding Bucket or legacy category names the escrow key.
        return bucket == key || category == key;
    }
    // Debts: Loan Name selects the loan. Bucket is funding only.
    if !loan_name.is_empty() && loan_name == key {
        return true;
    }
    // Legacy while Loan Name is still blank.
    if loan_name.is_empty() {
        if category == key || vendor == key {
            return true;
        }
        if let Some(legacy) = account.legacy_vendor.as_deref() {
            if same_vendor(&vendor, legacy) {
                return match account.legacy_amount_minor {
                    Some(amount) => {
                        line.amount_minor == amount || line.amount_minor == -amount
                    }
                    None => true,
                };
            }
        }
    }
    false
}

pub fn next_paid_through(label: &str) -> String {
    let trimmed = label.trim();
    let Some(index) = MONTHS
        .iter()
        .position(|month| month.eq_ignore_ascii_case(trimmed))
    else {
        return trimmed.to_string();
    };
    MONTHS[(index + 1) % MONTHS.len()].to_string()
}

pub fn payments_remaining(
    charges_interest: bool,
    current_minor: Option<i64>,
    payment_minor: Option<i64>,
    apr_ppm: Option<i64>,
    frequency: Option<&str>,
) -> Option<i64> {
    let current = current_minor.filter(|amount| *amount > 0)?;
    let payment = payment_minor.filter(|amount| *amount > 0)?;
    if !charges_interest {
        return Some((current + payment - 1) / payment);
    }
    let apr = apr_ppm.filter(|rate| *rate > 0)?;
    let periods = periods_per_year(frequency?)?;
    let rate = (apr as f64) / 1_000_000.0 / (periods as f64);
    let balance = current as f64;
    let pay = payment as f64;
    if pay <= rate * balance {
        return None;
    }
    let count = (pay / (pay - rate * balance)).ln() / (1.0 + rate).ln();
    if !count.is_finite() || count <= 0.0 {
        return None;
    }
    Some(count.ceil() as i64)
}

pub fn periods_per_year(frequency: &str) -> Option<i64> {
    match frequency {
        "weekly" => Some(52),
        "monthly" => Some(12),
        "quarterly" => Some(4),
        "annual" => Some(1),
        _ => None,
    }
}

pub fn period_interest(balance_minor: i64, apr_ppm: i64, periods: i64) -> i64 {
    if balance_minor <= 0 || apr_ppm <= 0 || periods <= 0 {
        return 0;
    }
    let numerator = balance_minor.saturating_mul(apr_ppm);
    let denominator = 1_000_000 * periods;
    (numerator + denominator / 2) / denominator
}

pub fn projected_split(
    balance_minor: i64,
    payment_minor: i64,
    charges_interest: bool,
    apr_ppm: Option<i64>,
    frequency: Option<&str>,
) -> Option<(i64, i64)> {
    if balance_minor <= 0 || payment_minor <= 0 {
        return None;
    }
    if !charges_interest {
        return Some((payment_minor.min(balance_minor), 0));
    }
    let periods = periods_per_year(frequency?)?;
    let interest = period_interest(balance_minor, apr_ppm?, periods);
    // Interest is a slice of the contractual payment — it can never exceed it.
    if interest >= payment_minor {
        return None;
    }
    let principal = payment_minor - interest;
    if principal <= 0 {
        return None;
    }
    Some((principal, interest))
}

pub fn paid_through_month(iso: &str) -> Option<&'static str> {
    let day = iso.get(..10).unwrap_or(iso);
    let date = NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()?;
    MONTHS.get((date.month() as usize).checked_sub(1)?).copied()
}

pub fn normalize_pay_process(raw: &str) -> Option<String> {
    match raw.trim().to_lowercase().as_str() {
        "week_ahead" | "week ahead" => Some("week_ahead".into()),
        "register" | "checking" | "checking and credit" => Some("register".into()),
        "element" | "linked element" | "linked_element" => Some("element".into()),
        _ => None,
    }
}

pub fn next_due(due_on: &str, frequency: &str) -> Option<String> {
    let date = NaiveDate::parse_from_str(due_on, "%Y-%m-%d").ok()?;
    let next = match frequency {
        "weekly" => date.checked_add_signed(Duration::days(7))?,
        "monthly" => add_months(date, 1)?,
        "quarterly" => add_months(date, 3)?,
        "annual" => add_months(date, 12)?,
        _ => return None,
    };
    Some(next.format("%Y-%m-%d").to_string())
}

fn add_months(date: NaiveDate, months: i32) -> Option<NaiveDate> {
    let month0 = date.month() as i32 - 1 + months;
    let year = date.year() + month0.div_euclid(12);
    let month = (month0.rem_euclid(12) + 1) as u32;
    let day = date.day();
    if let Some(exact) = NaiveDate::from_ymd_opt(year, month, day) {
        return Some(exact);
    }
    let first_next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)?
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)?
    };
    first_next.checked_sub_signed(Duration::days(1))
}

pub fn normalize_frequency(raw: &str) -> Option<String> {
    match raw.trim().to_lowercase().as_str() {
        "" => None,
        "weekly" | "week" => Some("weekly".into()),
        "monthly" | "month" => Some("monthly".into()),
        "quarterly" | "quarter" => Some("quarterly".into()),
        "annual" | "yearly" | "year" => Some("annual".into()),
        _ => None,
    }
}

pub fn accounts_from_json(json: &Value) -> Result<Vec<ExternalManagedAccountSave>, PlatformError> {
    let rows = json
        .get("accounts")
        .and_then(|value| value.as_array())
        .ok_or_else(|| PlatformError::new("missing_accounts", "account manager save needs accounts"))?;
    let mut accounts = Vec::new();
    for row in rows {
        let mut account: ExternalManagedAccountSave = serde_json::from_value(row.clone())
            .map_err(|_| PlatformError::new("bad_account", "account row could not be read"))?;
        account.name = account.name.trim().to_string();
        if account.name.is_empty() {
            return Err(PlatformError::new("missing_name", "each account needs a name"));
        }
        account.register_key = account.register_key.trim().to_string();
        if let Some(label) = account.paid_through.as_mut() {
            *label = label.trim().to_string();
            if label.is_empty() {
                account.paid_through = None;
            }
        }
        if let Some(due) = account.due_on.as_mut() {
            *due = due.trim().to_string();
            if due.is_empty() {
                account.due_on = None;
            }
        }
        if let Some(frequency) = account.frequency.as_mut() {
            match normalize_frequency(frequency) {
                Some(next) => *frequency = next,
                None => account.frequency = None,
            }
        }
        match normalize_pay_process(&account.pay_process) {
            Some(process) => account.pay_process = process,
            None => {
                return Err(PlatformError::new(
                    "missing_process",
                    "choose how this loan is paid",
                ))
            }
        }
        if account.pay_process != "element" {
            account.linked_element_id = None;
        }
        accounts.push(account);
    }
    if accounts.is_empty() {
        return Err(PlatformError::new(
            "missing_accounts",
            "account manager save needs at least one account",
        ));
    }
    Ok(accounts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn debt(key: &str, legacy: Option<&str>, amount: Option<i64>) -> AccountMatch {
        AccountMatch {
            kind: "debt".to_string(),
            register_key: key.to_string(),
            legacy_vendor: legacy.map(str::to_string),
            legacy_amount_minor: amount,
        }
    }

    #[test]
    fn paytient_open_charge_matches_and_is_not_applied() {
        let line = RegisterMatch {
            category: "Medical".to_string(),
            bucket: String::new(),
            loan_name: String::new(),
            vendor: "Paytient".to_string(),
            amount_minor: 18098,
        };
        assert!(line_matches(&line, &debt("Paytient", None, None)));
        assert!(!line_applied("debt", false, false, false, false, false, false));
    }

    #[test]
    fn alphaeon_legacy_rows_split_by_payment_amount() {
        let cat = debt("Alphaeon Cat", Some("Alpheon"), Some(23600));
        let ck = debt("Alphaeon CK", Some("Alpheon"), Some(25000));
        let two_thirty_six = RegisterMatch {
            category: "Medical".to_string(),
            bucket: String::new(),
            loan_name: String::new(),
            vendor: "Alpheon".to_string(),
            amount_minor: 23600,
        };
        let two_fifty = RegisterMatch {
            category: "Medical".to_string(),
            bucket: String::new(),
            loan_name: String::new(),
            vendor: "Alpheon".to_string(),
            amount_minor: 25000,
        };
        let other = RegisterMatch {
            category: "Medical".to_string(),
            bucket: String::new(),
            loan_name: String::new(),
            vendor: "Alpheon".to_string(),
            amount_minor: 10000,
        };
        assert!(line_matches(&two_thirty_six, &cat));
        assert!(!line_matches(&two_thirty_six, &ck));
        assert!(line_matches(&two_fifty, &ck));
        assert!(!line_matches(&two_fifty, &cat));
        assert!(!line_matches(&other, &cat));
        assert!(!line_matches(&other, &ck));
        let named = RegisterMatch {
            category: "Medical".to_string(),
            bucket: String::new(),
            loan_name: String::new(),
            vendor: "Alphaeon Cat".to_string(),
            amount_minor: 23600,
        };
        assert!(line_matches(&named, &cat));
        assert!(!line_matches(&named, &ck));
    }

    #[test]
    fn loan_name_links_debt_bucket_funds_escrow() {
        let cat = debt("Alphaeon Cat", Some("Alpheon"), Some(23600));
        let ck = debt("Alphaeon CK", Some("Alpheon"), Some(25000));
        let mom = AccountMatch {
            kind: "credit".to_string(),
            register_key: "Mom".to_string(),
            legacy_vendor: None,
            legacy_amount_minor: None,
        };
        assert!(line_matches(
            &RegisterMatch {
                category: "Medical".to_string(),
                bucket: "Medical".to_string(),
                loan_name: "Alphaeon Cat".to_string(),
                vendor: "Alpheon".to_string(),
                amount_minor: 23600,
            },
            &cat,
        ));
        assert!(!line_matches(
            &RegisterMatch {
                category: "Medical".to_string(),
                bucket: "Medical".to_string(),
                loan_name: "Alphaeon Cat".to_string(),
                vendor: "Alpheon".to_string(),
                amount_minor: 23600,
            },
            &ck,
        ));
        // Bucket alone must not select a loan.
        assert!(!line_matches(
            &RegisterMatch {
                category: "Medical".to_string(),
                bucket: "Alphaeon Cat".to_string(),
                loan_name: String::new(),
                vendor: "Walmart".to_string(),
                amount_minor: 23600,
            },
            &cat,
        ));
        assert!(line_matches(
            &RegisterMatch {
                category: "Medical".to_string(),
                bucket: "Mom".to_string(),
                loan_name: String::new(),
                vendor: "Walmart".to_string(),
                amount_minor: 1000,
            },
            &mom,
        ));
    }

    #[test]
    fn medical_credit_matches_category_or_bucket() {
        let medical = AccountMatch {
            kind: "credit".to_string(),
            register_key: "Medical".to_string(),
            legacy_vendor: None,
            legacy_amount_minor: None,
        };
        assert!(line_matches(
            &RegisterMatch {
                category: "Medical".to_string(),
                bucket: String::new(),
                loan_name: String::new(),
                vendor: "Paytient".to_string(),
                amount_minor: 100,
            },
            &medical,
        ));
        assert!(line_matches(
            &RegisterMatch {
                category: "Food".to_string(),
                bucket: "Medical".to_string(),
                loan_name: String::new(),
                vendor: "CVS".to_string(),
                amount_minor: 2500,
            },
            &medical,
        ));
        assert!(!line_matches(
            &RegisterMatch {
                category: "Food".to_string(),
                bucket: String::new(),
                loan_name: String::new(),
                vendor: "Walmart".to_string(),
                amount_minor: 100,
            },
            &medical,
        ));
        assert!(!line_matches(
            &RegisterMatch {
                category: "Medical-mom".to_string(),
                bucket: String::new(),
                loan_name: String::new(),
                vendor: "CVS".to_string(),
                amount_minor: 2500,
            },
            &medical,
        ));
        assert!(line_applied("credit", false, true, false, false, false, false));
        assert!(!line_applied("credit", false, false, false, false, false, false));
    }

    #[test]
    fn paid_through_advances_one_month() {
        assert_eq!(next_paid_through("August"), "September");
        assert_eq!(next_paid_through("December"), "January");
    }

    #[test]
    fn interest_remaining_uses_the_rate_and_zero_percent_counts_the_last_partial() {
        assert_eq!(
            payments_remaining(true, Some(29848900), Some(179672), None, Some("monthly")),
            None
        );
        assert_eq!(
            payments_remaining(false, Some(508600), Some(20300), None, None),
            Some(26)
        );
        assert_eq!(
            payments_remaining(true, Some(1_000_000), Some(50_000), Some(60_000), Some("monthly")),
            Some(22)
        );
        assert_eq!(period_interest(1_000_000, 60_000, 12), 5_000);
        assert_eq!(
            projected_split(1_000_000, 50_000, true, Some(60_000), Some("monthly")),
            Some((45_000, 5_000))
        );
        // Newrez-scale: $298,182.14 @ 5.99% monthly vs $1,796.72 payment.
        let newrez = projected_split(29_818_214, 179_672, true, Some(59_900), Some("monthly"))
            .expect("Newrez payment covers interest");
        assert!(
            newrez.1 < 179_672,
            "interest {} must stay under payment 179672",
            newrez.1
        );
        assert_eq!(newrez.0 + newrez.1, 179_672);
        assert!(
            projected_split(29_818_214, 100_000, true, Some(59_900), Some("monthly")).is_none(),
            "when interest would exceed payment, split is refused"
        );
        assert_eq!(next_due("2026-01-31", "monthly").as_deref(), Some("2026-02-28"));
        assert_eq!(next_due("2026-05-15", "monthly").as_deref(), Some("2026-06-15"));
        assert_eq!(paid_through_month("2026-10-07"), Some("October"));
    }

    #[test]
    fn pay_process_keeps_week_ahead_off_the_element() {
        let base = serde_json::json!({
            "accounts": [{
                "accountId": "a1000001-0000-4000-8000-000000000003",
                "name": "myClearbalance",
                "payProcess": "element"
            }]
        });
        let open = accounts_from_json(&base).expect("element without a link yet");
        assert_eq!(open[0].pay_process, "element");
        assert!(open[0].linked_element_id.is_none());
        let linked = serde_json::json!({
            "accounts": [{
                "accountId": "a1000001-0000-4000-8000-000000000001",
                "name": "Newrez",
                "payProcess": "Week Ahead",
                "linkedElementId": "c4793fec-b4a5-47e8-ad6e-f30afef40277"
            }]
        });
        let accounts = accounts_from_json(&linked).expect("week ahead");
        assert_eq!(accounts[0].pay_process, "week_ahead");
        assert!(accounts[0].linked_element_id.is_none());
        assert_eq!(
            accounts_from_json(&serde_json::json!({
                "accounts": [{
                    "accountId": "a1000001-0000-4000-8000-000000000008",
                    "name": "Mom shopping"
                }]
            }))
            .unwrap_err()
            .code,
            "missing_process"
        );
    }
}
