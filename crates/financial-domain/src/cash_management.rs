//! Cash Management posting identity. Net is calculated; it is not a third stored amount.

use chrono::Datelike;

use crate::error::DomainError;

/// Household Social Security retirement expected each month (owner lock). Never SSI.
pub const BARBARA_SSA_EXPECTED_MINOR: i64 = 133_100;
pub const TOM_SSA_EXPECTED_MINOR: i64 = 286_500;
pub const SSA_RETIREMENT_LABEL: &str = "Social Security retirement";
pub const SSA_VARIANCE_CODE: &str = "ssa_amount_variance";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SsaPayee {
    Barbara,
    Tom,
}

impl SsaPayee {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Barbara => "barbara",
            Self::Tom => "tom",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Barbara => "Barbara",
            Self::Tom => "Tom",
        }
    }

    pub fn expected_minor(self) -> i64 {
        match self {
            Self::Barbara => BARBARA_SSA_EXPECTED_MINOR,
            Self::Tom => TOM_SSA_EXPECTED_MINOR,
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "barbara" => Some(Self::Barbara),
            "tom" => Some(Self::Tom),
            _ => None,
        }
    }

    pub fn all() -> [Self; 2] {
        [Self::Barbara, Self::Tom]
    }
}

/// Types Cash Management may post.
pub fn is_cash_distribution_type(activity_type: &str) -> bool {
    matches!(
        activity_type,
        "IRA_Distribution" | "Roth_Distribution" | "SSA" | "Withdrawal"
    )
}

pub fn tom_ssa_idempotency_key(year: i32, month: u32) -> String {
    ssa_idempotency_key(SsaPayee::Tom, year, month)
}

pub fn ssa_idempotency_key(payee: SsaPayee, year: i32, month: u32) -> String {
    format!("ssa-{}-{year:04}-{month:02}", payee.as_str())
}

pub fn is_tom_ssa_amount(amount_minor: i64) -> bool {
    amount_minor == TOM_SSA_EXPECTED_MINOR
}

pub fn is_barbara_ssa_amount(amount_minor: i64) -> bool {
    amount_minor == BARBARA_SSA_EXPECTED_MINOR
}

/// Key prefix wins; otherwise exact expected amount. Seed keys like `seed-tom-june-5` classify by amount.
pub fn classify_ssa_row(idempotency_key: &str, amount_minor: i64) -> Option<SsaPayee> {
    let key = idempotency_key.to_ascii_lowercase();
    if key.starts_with("ssa-barbara-") {
        return Some(SsaPayee::Barbara);
    }
    if key.starts_with("ssa-tom-") {
        return Some(SsaPayee::Tom);
    }
    if is_barbara_ssa_amount(amount_minor) {
        return Some(SsaPayee::Barbara);
    }
    if is_tom_ssa_amount(amount_minor) {
        return Some(SsaPayee::Tom);
    }
    None
}

pub fn is_income_account_name(name: &str) -> bool {
    name.trim().eq_ignore_ascii_case("income")
}

pub fn is_ssa_account_name(name: &str) -> bool {
    name.trim().eq_ignore_ascii_case("external")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TomSsaStatus {
    Unconfirmed,
    Confirmed,
    Variance,
}

impl TomSsaStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unconfirmed => "unconfirmed",
            Self::Confirmed => "confirmed",
            Self::Variance => "variance",
        }
    }
}

/// Month status from expected-amount hits plus the optional `ssa-tom-YYYY-MM` confirm row.
/// A miss stays unconfirmed with no posted amount (never $0). Extra expected rows are audit.
pub fn tom_ssa_month_status(
    month_expected_count: usize,
    confirm_amount: Option<i64>,
) -> (TomSsaStatus, bool, Option<i64>) {
    ssa_payee_month_status(SsaPayee::Tom, month_expected_count, confirm_amount)
}

/// Per-payee month status. Extra on this payee means a duplicate row for that person.
pub fn ssa_payee_month_status(
    payee: SsaPayee,
    month_row_count: usize,
    confirm_amount: Option<i64>,
) -> (TomSsaStatus, bool, Option<i64>) {
    let extra_audit = month_row_count >= 2;
    let expected = payee.expected_minor();
    if month_row_count > 0 {
        let posted = confirm_amount
            .or_else(|| (month_row_count > 0).then_some(expected));
        if let Some(amount) = confirm_amount {
            if amount != expected {
                return (TomSsaStatus::Variance, extra_audit, Some(amount));
            }
        }
        return (TomSsaStatus::Confirmed, extra_audit, posted);
    }
    match confirm_amount {
        Some(amount) if amount == expected => (TomSsaStatus::Confirmed, false, Some(amount)),
        Some(amount) => (TomSsaStatus::Variance, false, Some(amount)),
        None => (TomSsaStatus::Unconfirmed, false, None),
    }
}

/// Household extra-audit: a third unexpected SSA row, a duplicate payee, or an unclassified row.
pub fn ssa_household_extra_audit(month_ssa_count: usize, duplicate_payee: bool, unclassified: bool) -> bool {
    month_ssa_count > 2 || duplicate_payee || unclassified
}

/// Saturday planned Income IRA draft stays open until one Income IRA posts that week.
pub fn saturday_draft_open(income_ira_posted_this_week: bool) -> bool {
    !income_ira_posted_this_week
}

/// Taxable gross this post would add to household MAGI. None = unknown / not this ticket.
/// Withholding is not an input (G-MAGI-06).
/// How a posted cash line is treated for tax. Amounts stay unknown until the owner fact is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CashTaxSection {
    IraOrdinary,
    Roth,
    TaxableBrokerage,
    Ssa,
}

impl CashTaxSection {
    pub fn id(self) -> &'static str {
        match self {
            Self::IraOrdinary => "ira",
            Self::Roth => "roth",
            Self::TaxableBrokerage => "taxable",
            Self::Ssa => "ssa",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::IraOrdinary => "IRA ordinary",
            Self::Roth => "Roth",
            Self::TaxableBrokerage => "Taxable brokerage",
            Self::Ssa => "Social Security retirement",
        }
    }

    pub fn tax_note(self) -> &'static str {
        match self {
            Self::IraOrdinary => {
                "Income and Speculation IRA withdrawals are one ordinary income type."
            }
            Self::Roth => "Roth is not Marketplace MAGI.",
            Self::TaxableBrokerage => {
                "Tax unknown until the 1099 is in or ROC versus ordinary is known."
            }
            Self::Ssa => "Social Security retirement. MAGI treatment stays unknown.",
        }
    }
}

pub fn cash_tax_section(activity_type: &str, account_kind: &str) -> CashTaxSection {
    let kind = account_kind.trim().to_ascii_lowercase();
    if activity_type == "SSA" || kind == "external" {
        return CashTaxSection::Ssa;
    }
    if activity_type == "Roth_Distribution" || kind == "roth" || kind == "fi_roth" {
        return CashTaxSection::Roth;
    }
    if activity_type == "IRA_Distribution" || kind == "ira" {
        return CashTaxSection::IraOrdinary;
    }
    CashTaxSection::TaxableBrokerage
}

pub fn magi_add_minor(activity_type: &str, gross_minor: Option<i64>) -> Option<i64> {
    match activity_type {
        "IRA_Distribution" => gross_minor,
        "Roth_Distribution" => Some(0),
        "SSA" => None,
        _ => None,
    }
}

pub fn tax_payment_credit_minor(federal_minor: i64, state_minor: i64) -> i64 {
    federal_minor + state_minor
}

/// Calendar month of an event date (TR-AC-06). Not a sum of Sat–Fri week cells.
pub fn occurred_in_calendar_month(occurred_on: &str, year: i32, month: u32) -> bool {
    let Some(date) = crate::trends::parse_iso_date(occurred_on) else {
        return false;
    };
    date.year() == year && date.month() == month
}

pub fn net_minor(gross_minor: i64, federal_minor: i64, state_minor: i64) -> i64 {
    gross_minor - federal_minor - state_minor
}

/// Live seed kinds (13 Sep 2026): Income/Speculation/9 = ira; FI Roth = fi_roth;
/// Car/Robinhood/ENERGYX = taxable; Health = hsa; External = taxable (SSA by name).
pub fn cash_activity_allowed_for_account(
    activity_type: &str,
    account_name: &str,
    account_kind: &str,
) -> Result<(), DomainError> {
    if !is_cash_distribution_type(activity_type) {
        return Err(DomainError::CashDistributionType);
    }
    let kind = account_kind.trim().to_ascii_lowercase();
    let allowed = match activity_type {
        "SSA" => is_ssa_account_name(account_name),
        "IRA_Distribution" => kind == "ira",
        "Roth_Distribution" => kind == "roth" || kind == "fi_roth",
        "Withdrawal" => kind == "taxable" && !is_ssa_account_name(account_name),
        _ => false,
    };
    if allowed {
        Ok(())
    } else {
        Err(DomainError::CashAccountKind)
    }
}

/// Gross must be known and positive. Withholding cannot exceed gross.
/// Roth refuses any withholding. Net = gross − fed − state.
pub fn validate_cash_distribution(
    activity_type: &str,
    gross_minor: Option<i64>,
    federal_minor: i64,
    state_minor: i64,
) -> Result<i64, DomainError> {
    if !is_cash_distribution_type(activity_type) {
        return Err(DomainError::CashDistributionType);
    }
    let Some(gross) = gross_minor else {
        return Err(DomainError::UnknownAmount);
    };
    if gross <= 0 {
        return Err(DomainError::CashDistributionIdentity);
    }
    if federal_minor < 0 || state_minor < 0 {
        return Err(DomainError::CashDistributionIdentity);
    }
    if activity_type == "Roth_Distribution" && (federal_minor != 0 || state_minor != 0) {
        return Err(DomainError::RothWithholdingNotAllowed);
    }
    let net = net_minor(gross, federal_minor, state_minor);
    if net < 0 {
        return Err(DomainError::CashDistributionIdentity);
    }
    Ok(net)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prove_subtotal_identity() {
        assert_eq!(validate_cash_distribution("IRA_Distribution", Some(100_000), 18_000, 4_500).unwrap(), 77_500);
        assert_eq!(net_minor(100_000, 18_000, 4_500), 77_500);
    }

    #[test]
    fn unknown_gross_is_not_zero() {
        assert_eq!(
            validate_cash_distribution("IRA_Distribution", None, 0, 0).unwrap_err(),
            DomainError::UnknownAmount
        );
    }

    #[test]
    fn withholding_cannot_exceed_gross() {
        assert_eq!(
            validate_cash_distribution("IRA_Distribution", Some(100), 80, 30).unwrap_err(),
            DomainError::CashDistributionIdentity
        );
    }

    #[test]
    fn roth_refuses_withholding() {
        assert_eq!(
            validate_cash_distribution("Roth_Distribution", Some(10_000), 1, 0).unwrap_err(),
            DomainError::RothWithholdingNotAllowed
        );
        assert_eq!(
            validate_cash_distribution("Roth_Distribution", Some(10_000), 0, 0).unwrap(),
            10_000
        );
    }

    #[test]
    fn withdrawal_posts_like_ira() {
        assert_eq!(
            validate_cash_distribution("Withdrawal", Some(10_000), 1_000, 500).unwrap(),
            8_500
        );
    }

    #[test]
    fn ssa_posts_as_social_security_retirement() {
        assert_eq!(
            validate_cash_distribution("SSA", Some(TOM_SSA_EXPECTED_MINOR), 0, 0).unwrap(),
            TOM_SSA_EXPECTED_MINOR
        );
        assert_eq!(SSA_RETIREMENT_LABEL, "Social Security retirement");
        assert!(!SSA_RETIREMENT_LABEL.contains("SSI"));
    }

    #[test]
    fn tom_ssa_miss_is_unconfirmed_not_zero() {
        let (status, extra, posted) = tom_ssa_month_status(0, None);
        assert_eq!(status, TomSsaStatus::Unconfirmed);
        assert!(!extra);
        assert_eq!(posted, None);
    }

    #[test]
    fn tom_ssa_expected_confirms_and_extra_is_audit() {
        let (status, extra, posted) = tom_ssa_month_status(2, None);
        assert_eq!(status, TomSsaStatus::Confirmed);
        assert!(extra);
        assert_eq!(posted, Some(TOM_SSA_EXPECTED_MINOR));
    }

    #[test]
    fn barbara_and_tom_are_separate_expected_amounts() {
        assert_eq!(SsaPayee::Barbara.expected_minor(), 133_100);
        assert_eq!(SsaPayee::Tom.expected_minor(), 286_500);
        assert_eq!(
            ssa_idempotency_key(SsaPayee::Barbara, 2026, 7),
            "ssa-barbara-2026-07"
        );
        assert_eq!(classify_ssa_row("seed-tom-june-5", 286_500), Some(SsaPayee::Tom));
        assert_eq!(
            classify_ssa_row("barb-jul-3", 133_100),
            Some(SsaPayee::Barbara)
        );
        assert!(ssa_household_extra_audit(3, false, false));
        assert!(!ssa_household_extra_audit(2, false, false));
        let (status, extra, posted) = ssa_payee_month_status(SsaPayee::Barbara, 1, Some(133_100));
        assert_eq!(status, TomSsaStatus::Confirmed);
        assert!(!extra);
        assert_eq!(posted, Some(133_100));
    }

    #[test]
    fn tom_ssa_variance_keeps_received_amount() {
        let (status, extra, posted) = tom_ssa_month_status(0, Some(280_000));
        assert_eq!(status, TomSsaStatus::Variance);
        assert!(!extra);
        assert_eq!(posted, Some(280_000));
    }

    #[test]
    fn saturday_draft_closes_after_income_ira() {
        assert!(saturday_draft_open(false));
        assert!(!saturday_draft_open(true));
    }

    #[test]
    fn cash_tax_sections_follow_owner_account_types() {
        assert_eq!(
            cash_tax_section("IRA_Distribution", "ira"),
            CashTaxSection::IraOrdinary
        );
        assert_eq!(
            cash_tax_section("IRA_Distribution", "ira"),
            cash_tax_section("IRA_Distribution", "IRA")
        );
        assert_eq!(
            cash_tax_section("Roth_Distribution", "roth"),
            CashTaxSection::Roth
        );
        assert_eq!(
            cash_tax_section("Roth_Distribution", "fi_roth"),
            CashTaxSection::Roth
        );
        assert_eq!(
            cash_tax_section("Withdrawal", "taxable"),
            CashTaxSection::TaxableBrokerage
        );
        assert_eq!(cash_tax_section("SSA", "external"), CashTaxSection::Ssa);
        assert!(CashTaxSection::TaxableBrokerage.tax_note().contains("1099"));
        assert!(CashTaxSection::IraOrdinary.tax_note().contains("Speculation"));
    }

    #[test]
    fn cash_type_must_match_the_account_the_owner_sees() {
        assert!(cash_activity_allowed_for_account("IRA_Distribution", "Income", "ira").is_ok());
        assert!(cash_activity_allowed_for_account("IRA_Distribution", "Speculation", "ira").is_ok());
        assert!(cash_activity_allowed_for_account("Roth_Distribution", "FI Roth", "fi_roth").is_ok());
        assert!(cash_activity_allowed_for_account("Roth_Distribution", "Roth", "roth").is_ok());
        assert!(cash_activity_allowed_for_account("Withdrawal", "Car", "taxable").is_ok());
        assert!(cash_activity_allowed_for_account("Withdrawal", "Robinhood", "taxable").is_ok());
        assert!(cash_activity_allowed_for_account("SSA", "External", "taxable").is_ok());
        assert!(cash_activity_allowed_for_account("SSA", "External", "external").is_ok());
        assert_eq!(
            cash_activity_allowed_for_account("Withdrawal", "Income", "ira").unwrap_err(),
            DomainError::CashAccountKind
        );
        assert_eq!(
            cash_activity_allowed_for_account("IRA_Distribution", "Car", "taxable").unwrap_err(),
            DomainError::CashAccountKind
        );
        assert_eq!(
            cash_activity_allowed_for_account("Roth_Distribution", "Income", "ira").unwrap_err(),
            DomainError::CashAccountKind
        );
        assert_eq!(
            cash_activity_allowed_for_account("SSA", "Car", "taxable").unwrap_err(),
            DomainError::CashAccountKind
        );
        assert_eq!(
            cash_activity_allowed_for_account("Withdrawal", "External", "taxable").unwrap_err(),
            DomainError::CashAccountKind
        );
        assert_eq!(
            cash_activity_allowed_for_account("Withdrawal", "Health", "hsa").unwrap_err(),
            DomainError::CashAccountKind
        );
    }

    #[test]
    fn withholding_does_not_change_magi_add() {
        let add = magi_add_minor("IRA_Distribution", Some(100_000));
        assert_eq!(add, Some(100_000));
        assert_eq!(tax_payment_credit_minor(18_000, 4_500), 22_500);
        assert_eq!(magi_add_minor("IRA_Distribution", Some(100_000)), add);
        assert_eq!(magi_add_minor("Roth_Distribution", Some(10_000)), Some(0));
        assert_eq!(magi_add_minor("SSA", Some(TOM_SSA_EXPECTED_MINOR)), None);
    }

    #[test]
    fn july_third_is_july_not_june_week_sum() {
        assert!(occurred_in_calendar_month("2026-07-03", 2026, 7));
        assert!(!occurred_in_calendar_month("2026-07-03", 2026, 6));
    }
}
