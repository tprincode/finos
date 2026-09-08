//! Explicit lots, dual basis, CRF DRIP policy. Never FIFO (V1.1 §6.1, §12 M4).

use uuid::Uuid;

use crate::error::DomainError;
use crate::money::{rescale, Money};

/// Fractional share scale for estimated CRF DRIP lots (cash ÷ close).
pub const DRIP_QUANTITY_SCALE: u8 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LotOrigin {
    Purchase,
    Drip,
    Option,
    Transfer,
}

impl LotOrigin {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Purchase => "purchase",
            Self::Drip => "drip",
            Self::Option => "option",
            Self::Transfer => "transfer",
        }
    }

    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.to_ascii_lowercase().as_str() {
            "purchase" | "buy" => Ok(Self::Purchase),
            "drip" => Ok(Self::Drip),
            "option" => Ok(Self::Option),
            "transfer" => Ok(Self::Transfer),
            _ => Err(DomainError::InvalidLotOrigin),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DualBasis {
    pub performance: Money,
    pub tax: Money,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LotOpenSpec {
    pub origin: LotOrigin,
    pub quantity_minor: i64,
    pub quantity_scale: u8,
    pub basis: DualBasis,
    pub crf_zero_cost: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LotConsumption {
    pub quantity_minor: i64,
    pub performance_minor: i64,
    pub tax_minor: i64,
    pub remaining_quantity_minor: i64,
    pub remaining_performance_minor: i64,
    pub remaining_tax_minor: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LotCostView {
    pub lot_id: Uuid,
    pub remaining_quantity_minor: i64,
    pub remaining_performance_minor: i64,
}

/// Sale/option close must name a lot. Missing id is FIFO and is refused.
pub fn require_explicit_lot(lot_id: Option<Uuid>) -> Result<Uuid, DomainError> {
    lot_id.ok_or(DomainError::FifoNotAssumed)
}

/// Zero-cost DRIP lots are legitimate only when the **security** is CRF.
/// Account kind is not an alias for this policy (FI Roth is auto-capture only).
pub fn zero_cost_drip_allowed(security_is_crf: bool) -> bool {
    security_is_crf
}

/// Automatic CRF DRIP capture is enabled only for FI Roth.
pub fn automatic_drip_capture_allowed(account_kind: &str) -> bool {
    account_kind.eq_ignore_ascii_case("fi_roth")
}

pub fn is_zero_cost(basis: DualBasis) -> bool {
    basis.performance.amount_minor == 0 && basis.tax.amount_minor == 0
}

/// Share quantity for a cash-amount CRF DRIP: cash ÷ close at [`DRIP_QUANTITY_SCALE`].
/// Unknown or non-positive price stays unknown (never a silent zero qty).
pub fn drip_quantity_from_cash(
    cash_minor: i64,
    cash_scale: u8,
    price_minor: Option<i64>,
    price_scale: u8,
) -> Result<(i64, u8), DomainError> {
    if cash_minor <= 0 {
        return Err(DomainError::InsufficientLotQuantity);
    }
    let Some(price_minor) = price_minor else {
        return Err(DomainError::UnknownAmount);
    };
    if price_minor <= 0 {
        return Err(DomainError::NonpositivePrice);
    }
    let common = cash_scale.max(price_scale);
    let cash = i128::from(rescale(cash_minor, cash_scale, common));
    let price = i128::from(rescale(price_minor, price_scale, common));
    if price <= 0 {
        return Err(DomainError::NonpositivePrice);
    }
    let factor = 10i128.pow(u32::from(DRIP_QUANTITY_SCALE));
    let num = cash.saturating_mul(factor);
    let q = num / price;
    let r = (num % price).abs();
    let qty = if r.saturating_mul(2) >= price.abs() {
        if num >= 0 {
            q.saturating_add(1)
        } else {
            q.saturating_sub(1)
        }
    } else {
        q
    };
    if qty <= 0 || qty > i128::from(i64::MAX) {
        return Err(DomainError::InsufficientLotQuantity);
    }
    Ok((qty as i64, DRIP_QUANTITY_SCALE))
}

pub fn prepare_lot_open(
    security_is_crf: bool,
    origin: LotOrigin,
    quantity_minor: i64,
    quantity_scale: u8,
    performance: Money,
    tax: Money,
) -> Result<LotOpenSpec, DomainError> {
    if quantity_minor <= 0 {
        return Err(DomainError::InsufficientLotQuantity);
    }
    if performance.scale != tax.scale {
        return Err(DomainError::ScaleMismatch);
    }
    let basis = DualBasis { performance, tax };
    let crf_zero_cost = origin == LotOrigin::Drip && is_zero_cost(basis);
    if crf_zero_cost && !zero_cost_drip_allowed(security_is_crf) {
        return Err(DomainError::ZeroCostDripNotCrf);
    }
    Ok(LotOpenSpec {
        origin,
        quantity_minor,
        quantity_scale,
        basis,
        crf_zero_cost,
    })
}

pub fn consume_lot(
    remaining_quantity_minor: i64,
    remaining_performance_minor: i64,
    remaining_tax_minor: i64,
    take_quantity_minor: i64,
) -> Result<LotConsumption, DomainError> {
    if take_quantity_minor <= 0 || take_quantity_minor > remaining_quantity_minor {
        return Err(DomainError::InsufficientLotQuantity);
    }
    if take_quantity_minor == remaining_quantity_minor {
        return Ok(LotConsumption {
            quantity_minor: take_quantity_minor,
            performance_minor: remaining_performance_minor,
            tax_minor: remaining_tax_minor,
            remaining_quantity_minor: 0,
            remaining_performance_minor: 0,
            remaining_tax_minor: 0,
        });
    }
    let performance_minor =
        remaining_performance_minor * take_quantity_minor / remaining_quantity_minor;
    let tax_minor = remaining_tax_minor * take_quantity_minor / remaining_quantity_minor;
    Ok(LotConsumption {
        quantity_minor: take_quantity_minor,
        performance_minor,
        tax_minor,
        remaining_quantity_minor: remaining_quantity_minor - take_quantity_minor,
        remaining_performance_minor: remaining_performance_minor - performance_minor,
        remaining_tax_minor: remaining_tax_minor - tax_minor,
    })
}

/// Lowest-cost-first ranking only. Does not assign a lot.
pub fn recommend_lowest_cost_first(lots: &[LotCostView]) -> Vec<Uuid> {
    let mut ranked: Vec<LotCostView> = lots
        .iter()
        .copied()
        .filter(|l| l.remaining_quantity_minor > 0)
        .collect();
    ranked.sort_by(|a, b| {
        let a_key = (a.remaining_performance_minor as i128)
            * (b.remaining_quantity_minor as i128);
        let b_key = (b.remaining_performance_minor as i128)
            * (a.remaining_quantity_minor as i128);
        a_key.cmp(&b_key).then(a.lot_id.cmp(&b.lot_id))
    });
    ranked.into_iter().map(|l| l.lot_id).collect()
}

pub fn lifetime_gains(proceeds_minor: i64, performance_cost_minor: i64, tax_cost_minor: i64) -> (i64, i64) {
    (
        proceeds_minor - performance_cost_minor,
        proceeds_minor - tax_cost_minor,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::Money;

    fn usd(minor: i64) -> Money {
        Money {
            amount_minor: minor,
            scale: 2,
        }
    }

    #[test]
    fn sale_without_lot_id_is_not_fifo() {
        let err = require_explicit_lot(None).unwrap_err();
        assert_eq!(err, DomainError::FifoNotAssumed);
        assert!(require_explicit_lot(Some(Uuid::nil())).is_ok());
    }

    #[test]
    fn taxable_zero_cost_drip_is_refused() {
        let err = prepare_lot_open(
            false,
            LotOrigin::Drip,
            10,
            0,
            usd(0),
            usd(0),
        )
        .unwrap_err();
        assert_eq!(err, DomainError::ZeroCostDripNotCrf);
    }

    #[test]
    fn crf_zero_cost_drip_is_allowed() {
        let spec = prepare_lot_open(true, LotOrigin::Drip, 10, 0, usd(0), usd(0)).unwrap();
        assert!(spec.crf_zero_cost);
        let purchase = prepare_lot_open(true, LotOrigin::Purchase, 4, 0, usd(0), usd(0)).unwrap();
        assert!(!purchase.crf_zero_cost);
        assert!(automatic_drip_capture_allowed("fi_roth"));
        assert!(!automatic_drip_capture_allowed("crf"));
        assert!(!automatic_drip_capture_allowed("taxable"));
        assert!(!zero_cost_drip_allowed(false));
        assert!(zero_cost_drip_allowed(true));
    }

    #[test]
    fn dual_basis_stays_separate_through_partial_sale() {
        let used = consume_lot(10, 100_000, 80_000, 5).unwrap();
        assert_eq!(used.performance_minor, 50_000);
        assert_eq!(used.tax_minor, 40_000);
        assert_ne!(used.performance_minor, used.tax_minor);
        let (perf_gain, tax_gain) = lifetime_gains(60_000, used.performance_minor, used.tax_minor);
        assert_eq!(perf_gain, 10_000);
        assert_eq!(tax_gain, 20_000);
    }

    #[test]
    fn drip_quantity_from_cash_uses_close_and_fractional_scale() {
        let (qty, scale) = drip_quantity_from_cash(318, 2, Some(636), 2).unwrap();
        assert_eq!(scale, DRIP_QUANTITY_SCALE);
        assert_eq!(qty, 5_000);
        let (rounded, _) = drip_quantity_from_cash(318, 2, Some(700), 2).unwrap();
        assert_eq!(rounded, 4_543);
        assert_eq!(
            drip_quantity_from_cash(318, 2, None, 2).unwrap_err(),
            DomainError::UnknownAmount
        );
        assert_eq!(
            drip_quantity_from_cash(318, 2, Some(0), 2).unwrap_err(),
            DomainError::NonpositivePrice
        );
        assert_eq!(
            drip_quantity_from_cash(0, 2, Some(636), 2).unwrap_err(),
            DomainError::InsufficientLotQuantity
        );
    }

    #[test]
    fn lowest_cost_first_is_recommendation_only() {
        let cheap = Uuid::from_u128(1);
        let expensive = Uuid::from_u128(2);
        let order = recommend_lowest_cost_first(&[
            LotCostView {
                lot_id: expensive,
                remaining_quantity_minor: 10,
                remaining_performance_minor: 100_000,
            },
            LotCostView {
                lot_id: cheap,
                remaining_quantity_minor: 10,
                remaining_performance_minor: 40_000,
            },
        ]);
        assert_eq!(order[0], cheap);
        assert_eq!(require_explicit_lot(Some(expensive)).unwrap(), expensive);
    }
}
