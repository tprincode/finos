//! Fixed-scale authoritative money representation (ARCH-07, ADR-0004).

use crate::error::DomainError;

/// USD cash / reporting totals use scale 2 (minor = cents).
pub const USD_CENTS_SCALE: u8 = 2;

/// Fixed-scale integer money. USD cash uses scale 2 (minor = cents). Quantity scale is separate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Money {
    pub amount_minor: i64,
    pub scale: u8,
}

/// Convert a fixed-scale integer to another scale. Half-up away from zero.
pub fn rescale(amount_minor: i64, from_scale: u8, to_scale: u8) -> i64 {
    if from_scale == to_scale {
        return amount_minor;
    }
    if from_scale > to_scale {
        let div = 10i64.pow((from_scale - to_scale) as u32);
        if div == 0 {
            return amount_minor;
        }
        let q = amount_minor / div;
        let r = (amount_minor % div).abs();
        if r * 2 >= div {
            q + if amount_minor >= 0 { 1 } else { -1 }
        } else {
            q
        }
    } else {
        amount_minor.saturating_mul(10i64.pow((to_scale - from_scale) as u32))
    }
}

/// Reporting money: lot/unit-cost scales 3–6 become USD cents.
pub fn to_usd_cents(amount_minor: i64, scale: u8) -> i64 {
    rescale(amount_minor, scale, USD_CENTS_SCALE)
}

/// Unknown amounts stay unknown. They are never coerced to zero.
pub fn require_known_amount(amount_minor: Option<i64>, scale: u8) -> Result<Money, DomainError> {
    match amount_minor {
        Some(amount_minor) => Ok(Money {
            amount_minor,
            scale,
        }),
        None => Err(DomainError::UnknownAmount),
    }
}

#[cfg(test)]
mod tests {
    use super::{require_known_amount, Money};

    #[test]
    fn constructs_fixed_scale_money_without_infrastructure() {
        let m = Money {
            amount_minor: 12_345,
            scale: 2,
        };
        assert_eq!(m.amount_minor, 12_345);
        assert_eq!(m.scale, 2);
    }

    #[test]
    fn unknown_amount_is_not_zero() {
        let err = require_known_amount(None, 2).unwrap_err();
        assert!(matches!(err, crate::error::DomainError::UnknownAmount));
        let known = require_known_amount(Some(0), 2).unwrap();
        assert_eq!(known.amount_minor, 0);
    }

    #[test]
    fn rescale_lot_unit_cost_scale_to_cents() {
        assert_eq!(super::to_usd_cents(66_309_999, 6), 6_631);
        assert_eq!(super::to_usd_cents(207_000, 2), 207_000);
        assert_eq!(super::rescale(10, 0, 3), 10_000);
        assert_eq!(super::rescale(1_923, 3, 3), 1_923);
    }
}
