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

/// One Plan $/share window. `effective_to` empty means still current.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanAmountWindow<'a> {
    pub effective_from: &'a str,
    pub effective_to: &'a str,
    pub amount_per_share_minor: i64,
    pub amount_scale: u8,
}

/// Plan of record for a pay date: current Plan while the date is unoccurred;
/// the window that covered that day after it passes.
pub fn plan_amount_as_of(windows: &[PlanAmountWindow<'_>], pay_on: &str) -> Option<(i64, u8)> {
    let pay = if pay_on.len() >= 10 {
        &pay_on[..10]
    } else {
        pay_on.trim()
    };
    if pay.is_empty() {
        return None;
    }
    windows
        .iter()
        .filter(|w| {
            let from = if w.effective_from.len() >= 10 {
                &w.effective_from[..10]
            } else {
                w.effective_from
            };
            if from > pay {
                return false;
            }
            let to = w.effective_to.trim();
            if to.is_empty() {
                return true;
            }
            let to = if to.len() >= 10 { &to[..10] } else { to };
            pay < to
        })
        .max_by_key(|w| w.effective_from)
        .map(|w| (w.amount_per_share_minor, w.amount_scale))
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
    fn plan_amount_as_of_freezes_after_pay_date() {
        let v1 = PlanAmountWindow {
            effective_from: "2026-01-01",
            effective_to: "2026-10-04",
            amount_per_share_minor: 50,
            amount_scale: 2,
        };
        let v2 = PlanAmountWindow {
            effective_from: "2026-10-04",
            effective_to: "",
            amount_per_share_minor: 60,
            amount_scale: 2,
        };
        let windows = [v1, v2];
        assert_eq!(plan_amount_as_of(&windows, "2026-10-03"), Some((50, 2)));
        assert_eq!(plan_amount_as_of(&windows, "2026-10-04"), Some((60, 2)));
        assert_eq!(plan_amount_as_of(&windows, "2026-11-03"), Some((60, 2)));
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
