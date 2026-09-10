//! Cash Management posting identity. Net is calculated; it is not a third stored amount.

use chrono::Datelike;

use crate::error::DomainError;

/// Tom Social Security retirement expected each month (owner lock). Never SSI.
pub const TOM_SSA_EXPECTED_MINOR: i64 = 286_500;
pub const SSA_RETIREMENT_LABEL: &str = "Social Security retirement";
pub const SSA_VARIANCE_CODE: &str = "ssa_amount_variance";

/// Types Cash Management may post.
pub fn is_cash_distribution_type(activity_type: &str) -> bool {
    matches!(
        activity_type,
        "IRA_Distribution" | "Roth_Distribution" | "SSA"
    )
}

pub fn tom_ssa_idempotency_key(year: i32, month: u32) -> String {
    format!("ssa-tom-{year:04}-{month:02}")
}

pub fn is_tom_ssa_amount(amount_minor: i64) -> bool {
    amount_minor == TOM_SSA_EXPECTED_MINOR
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
    let extra_audit = month_expected_count >= 2;
    if month_expected_count > 0 {
        return (
            TomSsaStatus::Confirmed,
            extra_audit,
            Some(TOM_SSA_EXPECTED_MINOR),
        );
    }
    match confirm_amount {
        Some(amount) if is_tom_ssa_amount(amount) => {
            (TomSsaStatus::Confirmed, false, Some(amount))
        }
        Some(amount) => (TomSsaStatus::Variance, false, Some(amount)),
        None => (TomSsaStatus::Unconfirmed, false, None),
    }
}

/// Saturday planned Income IRA draft stays open until one Income IRA posts that week.
pub fn saturday_draft_open(income_ira_posted_this_week: bool) -> bool {
    !income_ira_posted_this_week
}

/// Taxable gross this post would add to household MAGI. None = unknown / not this ticket.
/// Withholding is not an input (G-MAGI-06).
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
