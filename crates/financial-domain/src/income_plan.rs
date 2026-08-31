//! Income Plan week grid and Dashboard burndown account identity (domain docs + BR-X).
//! Plan amounts are not inferred. Missing plan stays unknown — never zero.

pub const CONTROL_ACCOUNTS: [&str; 5] = ["Income", "Health", "Roth", "Account 9", "Car"];
pub const BURNDOWN_ACCOUNTS: [&str; 4] = ["Income", "Car", "Health", "Roth"];

/// Map an account name in the data file onto the Income Plan control grid, if it belongs.
pub fn map_control_account(name: &str) -> Option<&'static str> {
    let n = name.trim().to_ascii_lowercase();
    if n == "income" {
        return Some("Income");
    }
    if n.contains("health") || n == "hsa" {
        return Some("Health");
    }
    if n.contains("roth") {
        return Some("Roth");
    }
    if n.contains("account 9") || n == "9" {
        return Some("Account 9");
    }
        if n == "car" || n.starts_with("car ") || n.ends_with(" car") {
        return Some("Car");
    }
    None
}

pub fn is_burndown_account(control: &str) -> bool {
    BURNDOWN_ACCOUNTS.contains(&control)
}

pub fn occurred_in_week(occurred_on: &str, start: &str, end: &str) -> bool {
    let day = if occurred_on.len() >= 10 {
        &occurred_on[..10]
    } else {
        occurred_on
    };
    day >= start && day <= end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_fi_roth_and_excludes_speculation() {
        assert_eq!(map_control_account("FI Roth"), Some("Roth"));
        assert_eq!(map_control_account("For the CAR"), Some("Car"));
        assert_eq!(map_control_account("Speculation"), None);
        assert!(!is_burndown_account("Account 9"));
        assert!(is_burndown_account("Roth"));
    }

    #[test]
    fn week_filter_uses_iso_dates() {
        assert!(occurred_in_week("2026-08-17", "2026-08-15", "2026-08-21"));
        assert!(!occurred_in_week("2026-08-14", "2026-08-15", "2026-08-21"));
    }
}
