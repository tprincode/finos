//! Position details are a reporting rollup over open lots. Not a ledger post (ADR-0008).

use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LotPositionInput {
    pub account_id: Uuid,
    pub security_id: Uuid,
    pub remaining_quantity_minor: i64,
    pub remaining_performance_minor: i64,
    pub remaining_tax_minor: i64,
    pub quantity_scale: u8,
    pub scale: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositionRollup {
    pub account_id: Uuid,
    pub security_id: Uuid,
    pub remaining_quantity_minor: i64,
    pub remaining_performance_minor: i64,
    pub remaining_tax_minor: i64,
    pub quantity_scale: u8,
    pub scale: u8,
    pub lot_count: u64,
}

/// Group remaining lots by account and security. Dual basis stays separate.
/// Fully consumed lots (remaining quantity 0) are omitted. This is not cash.
pub fn rollup_open_positions(lots: &[LotPositionInput]) -> Vec<PositionRollup> {
    let mut out: Vec<PositionRollup> = Vec::new();
    for lot in lots {
        if lot.remaining_quantity_minor == 0 {
            continue;
        }
        if let Some(existing) = out.iter_mut().find(|p| {
            p.account_id == lot.account_id && p.security_id == lot.security_id
        }) {
            existing.remaining_quantity_minor += lot.remaining_quantity_minor;
            existing.remaining_performance_minor += lot.remaining_performance_minor;
            existing.remaining_tax_minor += lot.remaining_tax_minor;
            existing.lot_count += 1;
        } else {
            out.push(PositionRollup {
                account_id: lot.account_id,
                security_id: lot.security_id,
                remaining_quantity_minor: lot.remaining_quantity_minor,
                remaining_performance_minor: lot.remaining_performance_minor,
                remaining_tax_minor: lot.remaining_tax_minor,
                quantity_scale: lot.quantity_scale,
                scale: lot.scale,
                lot_count: 1,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn position_rollup_is_not_cash() {
        let cash_before = 50_000i64;
        let account = Uuid::from_u128(1);
        let security = Uuid::from_u128(2);
        let rolled = rollup_open_positions(&[
            LotPositionInput {
                account_id: account,
                security_id: security,
                remaining_quantity_minor: 10,
                remaining_performance_minor: 100_000,
                remaining_tax_minor: 80_000,
                quantity_scale: 0,
                scale: 2,
            },
            LotPositionInput {
                account_id: account,
                security_id: security,
                remaining_quantity_minor: 10,
                remaining_performance_minor: 40_000,
                remaining_tax_minor: 40_000,
                quantity_scale: 0,
                scale: 2,
            },
        ]);
        assert_eq!(rolled.len(), 1);
        assert_eq!(rolled[0].remaining_quantity_minor, 20);
        assert_eq!(rolled[0].remaining_performance_minor, 140_000);
        assert_eq!(rolled[0].remaining_tax_minor, 120_000);
        assert_eq!(rolled[0].lot_count, 2);
        assert_eq!(cash_before, 50_000);
        assert_ne!(rolled[0].remaining_performance_minor, cash_before);
    }
}
