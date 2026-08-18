//! Fixed-scale authoritative money representation (ARCH-07, ADR-0004).

use crate::error::DomainError;

/// Fixed-scale integer money. USD cash uses scale 2 (minor = cents). Quantity scale is separate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Money {
    pub amount_minor: i64,
    pub scale: u8,
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
}
