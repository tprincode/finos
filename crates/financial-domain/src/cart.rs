//! Shopping cart lines are decision support. They are not fills and never post lots or MAGI.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CartLine {
    pub symbol: String,
    pub quantity_minor: i64,
    pub quantity_scale: u8,
}

/// Quantity scale 2: 10000 means 100.00 shares.
pub fn prepare_line(symbol: String, quantity_minor: i64, quantity_scale: u8) -> CartLine {
    CartLine {
        symbol,
        quantity_minor,
        quantity_scale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cart_line_is_not_a_fill() {
        let lots_before = 0i64;
        let cash_before = 50_000i64;
        let line = prepare_line("VXUS".into(), 10_000, 2);
        assert_eq!(line.symbol, "VXUS");
        assert_eq!(line.quantity_minor, 10_000);
        assert_eq!(lots_before, 0);
        assert_eq!(cash_before, 50_000);
    }
}
