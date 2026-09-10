//! Home account-value series: holdings market value and custodian rollups.
//! Market value is open quantity × last price. Missing price stays unknown — never $0.

pub const FIDELITY_TOTAL_ID: &str = "fidelity-total";
pub const FIDELITY_TOTAL_NAME: &str = "Fidelity Total";
pub const SCHWAB_TOTAL_ID: &str = "schwab-total";
pub const SCHWAB_TOTAL_NAME: &str = "Schwab Total";

pub const RISK_FOUNDATION: &str = "Foundation";
pub const RISK_CORE: &str = "Core";
pub const RISK_ON: &str = "Risk On";
pub const RISK_UNDECIDED: &str = "Undecided";
pub const RISK_FOUNDATION_ID: &str = "risk-Foundation";
pub const RISK_CORE_ID: &str = "risk-Core";
pub const RISK_ON_ID: &str = "risk-Risk On";
pub const RISK_UNDECIDED_ID: &str = "risk-Undecided";
pub const RISK_TIERS: [&str; 4] = [RISK_FOUNDATION, RISK_CORE, RISK_ON, RISK_UNDECIDED];

/// Owner risk buckets for Home. Blank / unknown is Undecided — never assumed Risk On.
pub fn risk_bucket(raw: &str) -> &'static str {
    match crate::plan_review::normalize_risk_tier(raw).as_str() {
        RISK_FOUNDATION => RISK_FOUNDATION,
        RISK_CORE => RISK_CORE,
        RISK_ON => RISK_ON,
        _ => RISK_UNDECIDED,
    }
}

pub fn risk_series_id(tier: &str) -> &'static str {
    match tier {
        RISK_FOUNDATION => RISK_FOUNDATION_ID,
        RISK_CORE => RISK_CORE_ID,
        RISK_ON => RISK_ON_ID,
        _ => RISK_UNDECIDED_ID,
    }
}

pub fn risk_series_name(tier: &str) -> String {
    format!("Risk {tier}")
}

pub fn parse_risk_series_id(account_id: &str) -> Option<&'static str> {
    match account_id {
        RISK_FOUNDATION_ID => Some(RISK_FOUNDATION),
        RISK_CORE_ID => Some(RISK_CORE),
        RISK_ON_ID => Some(RISK_ON),
        RISK_UNDECIDED_ID => Some(RISK_UNDECIDED),
        _ => None,
    }
}

/// Sum one risk bucket. Missing last price stays unknown — never $0.
pub fn risk_bucket_total(values: &[Option<i64>]) -> (Option<i64>, bool) {
    let mut sum = 0i64;
    let mut known = 0u32;
    let mut missing = 0u32;
    for mv in values {
        match mv {
            Some(v) => {
                sum += *v;
                known += 1;
            }
            None => missing += 1,
        }
    }
    if known == 0 && missing == 0 {
        return (Some(0), true);
    }
    if known == 0 {
        return (None, false);
    }
    (Some(sum), missing == 0)
}

/// Broker/custodian from the stored Accounts table (`Template_Accounts.xlsx` brokerage).
/// ENERGYX is Direct. Unknown names are Other — not Fidelity.
pub fn account_custodian(name: &str) -> &'static str {
    let n = name.trim().to_ascii_lowercase();
    if n.is_empty() || n == "external" {
        return "Other";
    }
    if n.contains("robinhood") {
        return "Robinhood";
    }
    if n.contains("schwab") || crate::income_plan::map_control_account(name) == Some("Account 9") {
        return "Schwab";
    }
    if n == "energyx" || n == "energy" || n.starts_with("energyx") {
        return "Direct";
    }
    if matches!(
        n.as_str(),
        "income" | "car" | "health" | "fi roth" | "roth" | "speculation"
    ) || crate::income_plan::map_control_account(name)
        .is_some_and(|c| matches!(c, "Income" | "Car" | "Health" | "Roth"))
    {
        return "Fidelity";
    }
    "Other"
}

pub fn is_fidelity_holdings_account(name: &str) -> bool {
    account_custodian(name) == "Fidelity"
}

pub fn is_schwab_holdings_account(name: &str) -> bool {
    account_custodian(name) == "Schwab"
}

/// Sum holdings MV for one custodian. Unknown stays unknown.
pub fn custodian_total_from_accounts(
    rows: &[(String, Option<i64>)],
    custodian: &str,
) -> (Option<i64>, bool) {
    let mut sum = 0i64;
    let mut known = 0u32;
    let mut missing = 0u32;
    for (name, mv) in rows {
        if account_custodian(name) != custodian {
            continue;
        }
        match mv {
            Some(v) => {
                sum += *v;
                known += 1;
            }
            None => missing += 1,
        }
    }
    if known == 0 && missing == 0 {
        return (Some(0), true);
    }
    if known == 0 {
        return (None, false);
    }
    (Some(sum), missing == 0)
}

pub fn fidelity_total_from_accounts(rows: &[(String, Option<i64>)]) -> (Option<i64>, bool) {
    custodian_total_from_accounts(rows, "Fidelity")
}

pub fn schwab_total_from_accounts(rows: &[(String, Option<i64>)]) -> (Option<i64>, bool) {
    custodian_total_from_accounts(rows, "Schwab")
}

/// Stored Trends week $ for a Home chart. Missing account/week stays unknown.
pub fn trends_balance_for_account(
    name: &str,
    fidelity_total_minor: i64,
    schwab_total_minor: i64,
    income_balance_minor: Option<i64>,
    car_balance_minor: Option<i64>,
    health_balance_minor: Option<i64>,
    roth_balance_minor: Option<i64>,
    speculation_balance_minor: Option<i64>,
) -> Option<i64> {
    let n = name.trim().to_ascii_lowercase();
    if n.contains("fidelity") {
        return Some(fidelity_total_minor);
    }
    if n.contains("schwab") || crate::income_plan::map_control_account(name) == Some("Account 9") {
        return Some(schwab_total_minor);
    }
    match crate::income_plan::map_income_plan_account(name).unwrap_or("") {
        "Income" => income_balance_minor,
        "Car" => car_balance_minor,
        "Health" => health_balance_minor,
        "Roth" => roth_balance_minor,
        "Speculation" => speculation_balance_minor,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn household_names_follow_stored_accounts_brokerage() {
        assert_eq!(account_custodian("Income"), "Fidelity");
        assert_eq!(account_custodian("Car"), "Fidelity");
        assert_eq!(account_custodian("FI Roth"), "Fidelity");
        assert_eq!(account_custodian("9"), "Schwab");
        assert_eq!(account_custodian("Account 9"), "Schwab");
        assert_eq!(account_custodian("Speculation"), "Fidelity");
        assert_eq!(account_custodian("ENERGYX"), "Direct");
        assert_eq!(account_custodian("EnergyX"), "Direct");
        assert_eq!(account_custodian("Energy"), "Direct");
        assert_eq!(account_custodian("Robinhood"), "Robinhood");
        assert_eq!(account_custodian("Schwab Brokerage"), "Schwab");
        assert_eq!(account_custodian("External"), "Other");
        assert_eq!(account_custodian("Mystery"), "Other");
    }

    #[test]
    fn fidelity_total_skips_robinhood_energyx_and_keeps_unknown() {
        let rows = vec![
            ("Income".into(), Some(1000)),
            ("Car".into(), Some(400)),
            ("9".into(), Some(2500)),
            ("Robinhood".into(), Some(9999)),
            ("ENERGYX".into(), Some(8000)),
        ];
        assert_eq!(fidelity_total_from_accounts(&rows), (Some(1400), true));
        assert_eq!(schwab_total_from_accounts(&rows), (Some(2500), true));

        let missing = vec![("Income".into(), None), ("Car".into(), Some(400))];
        assert_eq!(fidelity_total_from_accounts(&missing), (Some(400), false));

        let none = vec![("Income".into(), None)];
        assert_eq!(fidelity_total_from_accounts(&none), (None, false));
    }

    #[test]
    fn trends_balance_maps_stored_week_not_energyx() {
        assert_eq!(
            trends_balance_for_account(
                "Fidelity Total",
                3100,
                320,
                Some(2100),
                None,
                None,
                None,
                None,
            ),
            Some(3100)
        );
        assert_eq!(
            trends_balance_for_account("Income", 3100, 320, Some(2100), None, None, None, None),
            Some(2100)
        );
        assert_eq!(
            trends_balance_for_account("ENERGYX", 3100, 320, Some(2100), None, None, None, None),
            None
        );
    }

    #[test]
    fn risk_bucket_maps_owner_tiers_and_keeps_blank_undecided() {
        assert_eq!(risk_bucket("Foundation"), RISK_FOUNDATION);
        assert_eq!(risk_bucket("core"), RISK_CORE);
        assert_eq!(risk_bucket("HighRisk"), RISK_ON);
        assert_eq!(risk_bucket(""), RISK_UNDECIDED);
        assert_eq!(risk_bucket("maybe later"), RISK_UNDECIDED);
        assert_eq!(risk_series_id(RISK_ON), RISK_ON_ID);
        assert_eq!(parse_risk_series_id(RISK_CORE_ID), Some(RISK_CORE));
        assert_eq!(risk_bucket_total(&[Some(100), Some(50)]), (Some(150), true));
        assert_eq!(risk_bucket_total(&[Some(100), None]), (Some(100), false));
        assert_eq!(risk_bucket_total(&[None]), (None, false));
        assert_eq!(risk_bucket_total(&[]), (Some(0), true));
    }
}
