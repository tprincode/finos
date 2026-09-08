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
/// Never panics on extreme scale deltas: scales down to 0, scales up toward i64 bounds.
pub fn rescale(amount_minor: i64, from_scale: u8, to_scale: u8) -> i64 {
    if from_scale == to_scale {
        return amount_minor;
    }
    if from_scale > to_scale {
        let delta = u32::from(from_scale - to_scale);
        let Some(div) = 10i64.checked_pow(delta) else {
            return 0;
        };
        if div == 0 {
            return amount_minor;
        }
        let q = amount_minor / div;
        let r = (amount_minor % div).abs();
        if r.saturating_mul(2) >= div {
            if amount_minor >= 0 {
                q.saturating_add(1)
            } else {
                q.saturating_add(-1)
            }
        } else {
            q
        }
    } else {
        let delta = u32::from(to_scale - from_scale);
        let Some(factor) = 10i64.checked_pow(delta) else {
            return if amount_minor == 0 {
                0
            } else if amount_minor > 0 {
                i64::MAX
            } else {
                i64::MIN
            };
        };
        amount_minor.saturating_mul(factor)
    }
}

/// Same dollars at different scales (C5). 27069@5 == 270690@6.
pub fn amounts_equal(a: i64, a_scale: u8, b: i64, b_scale: u8) -> bool {
    if a == b && a_scale == b_scale {
        return true;
    }
    let to = a_scale.max(b_scale);
    rescale(a, a_scale, to) == rescale(b, b_scale, to)
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

    #[test]
    fn rescale_does_not_panic_on_extreme_scale() {
        assert_eq!(super::rescale(1, 0, 40), i64::MAX);
        assert_eq!(super::rescale(-1, 0, 40), i64::MIN);
        assert_eq!(super::rescale(1000, 40, 2), 0);
    }

    #[test]
    fn amounts_equal_across_scale() {
        assert!(super::amounts_equal(27_069, 5, 270_690, 6));
        assert!(super::amounts_equal(27_124, 5, 271_240, 6));
        assert!(!super::amounts_equal(100, 2, 999, 2));
    }
}
