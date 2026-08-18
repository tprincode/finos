//! Versioned Calculator Plan and cash burndown (V1.1 §6.1). Plan remaining is not actual cash.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Burndown {
    pub cash_minor: i64,
    pub obligation_minor: i64,
    pub surplus_minor: i64,
    pub sufficient: bool,
}

/// Deposit and dividend increase cash. Withdraw decreases it. Other types are not cash here.
pub fn cash_effect(activity_type: &str, amount_minor: i64) -> i64 {
    match activity_type.to_ascii_lowercase().as_str() {
        "deposit" | "dividend" => amount_minor,
        "withdraw" | "withdrawal" => -amount_minor,
        _ => 0,
    }
}

pub fn sum_cash<'a, I>(items: I) -> i64
where
    I: IntoIterator<Item = (&'a str, i64)>,
{
    items
        .into_iter()
        .map(|(activity_type, amount)| cash_effect(activity_type, amount))
        .sum()
}

pub fn project_burndown(cash_minor: i64, obligation_minor: i64) -> Burndown {
    let surplus_minor = cash_minor - obligation_minor;
    Burndown {
        cash_minor,
        obligation_minor,
        surplus_minor,
        sufficient: surplus_minor >= 0,
    }
}

pub fn next_plan_version(current: u32) -> u32 {
    current.saturating_add(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_remaining_is_not_actual_cash() {
        let posted_actual = 50_000;
        let plan_remaining = 3_000_000;
        let cash = sum_cash([("dividend", posted_actual)]);
        assert_eq!(cash, posted_actual);
        assert_ne!(cash, cash + plan_remaining);
        let burn = project_burndown(cash, plan_remaining);
        assert_eq!(burn.obligation_minor, plan_remaining);
        assert_eq!(burn.surplus_minor, posted_actual - plan_remaining);
        assert!(!burn.sufficient);
    }

    #[test]
    fn burndown_surplus_is_exact_cents() {
        let burn = project_burndown(4_000_000, 3_000_000);
        assert_eq!(burn.surplus_minor, 1_000_000);
        assert!(burn.sufficient);
        let short = project_burndown(100, 101);
        assert_eq!(short.surplus_minor, -1);
        assert!(!short.sufficient);
    }

    #[test]
    fn withdraw_reduces_cash_fee_does_not() {
        let cash = sum_cash([
            ("deposit", 10_000),
            ("withdraw", 1_000),
            ("fee", 500),
            ("sell", 2_000),
        ]);
        assert_eq!(cash, 9_000);
    }
}
