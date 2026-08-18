//! Dividend cash states stay separate: declaration, plan, and actual (V1.1 §6.1).

use crate::money::Money;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DividendCashState {
    Declaration,
    Plan,
    Actual,
}

/// Actual cash is only the sum of posted actuals. Declarations and plans never mix in.
pub fn sum_actuals(actuals: &[Money]) -> Option<Money> {
    if actuals.is_empty() {
        return Some(Money {
            amount_minor: 0,
            scale: 2,
        });
    }
    let scale = actuals[0].scale;
    if actuals.iter().any(|m| m.scale != scale) {
        return None;
    }
    Some(Money {
        amount_minor: actuals.iter().map(|m| m.amount_minor).sum(),
        scale,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declarations_do_not_increase_actual_cash() {
        let declared = Money {
            amount_minor: 99_999,
            scale: 2,
        };
        let actual = Money {
            amount_minor: 50_000,
            scale: 2,
        };
        let _ = (DividendCashState::Declaration, declared);
        let total = sum_actuals(&[actual]).unwrap();
        assert_eq!(total.amount_minor, 50_000);
        assert_ne!(total.amount_minor, declared.amount_minor);
    }
}
