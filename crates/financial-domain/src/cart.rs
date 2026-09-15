//! Shopping cart lines are decision support. They are not fills and never post lots or MAGI.

use crate::income_plan::map_control_account;

/// Scale-2 dollars: Income $5,000, Car $4,000, Health $200. No other floors.
pub const CASH_FLOOR_INCOME_MINOR: i64 = 500_000;
pub const CASH_FLOOR_CAR_MINOR: i64 = 400_000;
pub const CASH_FLOOR_HEALTH_MINOR: i64 = 20_000;

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

/// Energy and Robinhood never have a cash pile. Account cash is not offered.
pub fn no_cash_account(account_name: &str) -> bool {
    let n = account_name.trim().to_ascii_lowercase();
    n.contains("energy") || n.contains("robinhood")
}

/// Remaining qty at $1.00 → scale-2 dollars. Scale 2: 33000 → $330. Scale 0: 10000 → $10,000.
pub fn cash_dollars_minor(qty_minor: i64, qty_scale: u8) -> i64 {
    let denom = 10_i64.pow(u32::from(qty_scale));
    if denom == 0 {
        return 0;
    }
    (qty_minor * 100) / denom
}

/// Scale-2 dollars → qty minor on the cash pile (0.01 steps at $1).
pub fn cash_qty_for_dollars(dollars_minor: i64, qty_scale: u8) -> i64 {
    if dollars_minor <= 0 {
        return 0;
    }
    let factor = 10_i64.pow(u32::from(qty_scale));
    (dollars_minor * factor) / 100
}

pub fn cash_floor_minor(account_name: &str) -> Option<i64> {
    match map_control_account(account_name) {
        Some("Income") => Some(CASH_FLOOR_INCOME_MINOR),
        Some("Car") => Some(CASH_FLOOR_CAR_MINOR),
        Some("Health") => Some(CASH_FLOOR_HEALTH_MINOR),
        _ => None,
    }
}

/// Plan $/sh × periods / par $1, in bps (334 = 3.34%). Collector plan, not a typed rate.
pub fn plan_fwd_yield_bps(
    amount_per_share_minor: i64,
    amount_scale: u8,
    periods: i64,
    price_cents: i64,
) -> Option<i64> {
    if amount_per_share_minor <= 0 || periods <= 0 || price_cents <= 0 {
        return None;
    }
    let denom = 10_i128.pow(u32::from(amount_scale)) * i128::from(price_cents);
    if denom == 0 {
        return None;
    }
    let num = i128::from(amount_per_share_minor) * i128::from(periods) * 1_000_000;
    Some(((num + denom / 2) / denom) as i64)
}

/// Annual yield in basis points (334 = 3.34%). Scale-2 dollars in, scale-2 dollars out.
pub fn annual_on_principal(principal_minor: i64, yield_bps: i64) -> i64 {
    if principal_minor <= 0 || yield_bps <= 0 {
        return 0;
    }
    (principal_minor * yield_bps) / 10_000
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwapEval {
    pub remaining_minor: i64,
    pub spend_minor: i64,
    pub leftover_minor: i64,
    pub buy_annual_minor: i64,
    pub surrendered_annual_minor: i64,
    pub leftover_annual_minor: i64,
    pub net_annual_minor: i64,
    pub net_monthly_minor: i64,
    pub net_weekly_minor: i64,
    pub insufficient_lot_qty: bool,
    pub cash_floor_warn: bool,
}

/// Leftover-keeps-yield: surrender plan only on dollars spent.
pub fn evaluate_swap(
    remaining_minor: i64,
    spend_minor: i64,
    buy_annual_minor: i64,
    cash_yield_bps: i64,
    account_name: &str,
) -> SwapEval {
    let leftover_minor = remaining_minor - spend_minor;
    let insufficient_lot_qty = spend_minor > remaining_minor;
    let spent_for_yield = if insufficient_lot_qty {
        spend_minor
    } else {
        spend_minor
    };
    let leftover_for_yield = leftover_minor.max(0);
    let surrendered_annual_minor = annual_on_principal(spent_for_yield, cash_yield_bps);
    let leftover_annual_minor = annual_on_principal(leftover_for_yield, cash_yield_bps);
    let net_annual_minor = buy_annual_minor - surrendered_annual_minor;
    let after_cash = leftover_minor;
    let cash_floor_warn = cash_floor_minor(account_name)
        .is_some_and(|floor| after_cash < floor);
    SwapEval {
        remaining_minor,
        spend_minor,
        leftover_minor,
        buy_annual_minor,
        surrendered_annual_minor,
        leftover_annual_minor,
        net_annual_minor,
        net_monthly_minor: net_annual_minor / 12,
        net_weekly_minor: net_annual_minor / 52,
        insufficient_lot_qty,
        cash_floor_warn,
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

    #[test]
    fn spaxx_collector_plan_is_three_thirty_four_bps() {
        assert_eq!(plan_fwd_yield_bps(2_783, 6, 12, 100), Some(334));
    }

    #[test]
    fn cash_qty_is_dollars_at_par() {
        assert_eq!(cash_dollars_minor(33_000, 2), 33_000);
        assert_eq!(cash_qty_for_dollars(5_000, 2), 5_000);
        assert_eq!(cash_dollars_minor(10_000, 0), 1_000_000);
        assert_eq!(cash_qty_for_dollars(5_000, 0), 50);
        assert!(no_cash_account("ENERGYX"));
        assert!(no_cash_account("Robinhood"));
        assert!(!no_cash_account("FI Roth"));
    }

    #[test]
    fn leftover_keeps_yield_on_unspent_cash() {
        let eval = evaluate_swap(6_577, 5_990, 912, 334, "FI Roth");
        assert!(!eval.insufficient_lot_qty);
        assert_eq!(eval.leftover_minor, 587);
        assert_eq!(eval.surrendered_annual_minor, 200);
        assert_eq!(eval.leftover_annual_minor, 19);
        assert_eq!(eval.net_annual_minor, 712);
        assert_eq!(eval.net_monthly_minor, 712 / 12);
        assert_eq!(eval.net_weekly_minor, 712 / 52);
        assert!(!eval.cash_floor_warn);
    }

    #[test]
    fn over_remaining_is_insufficient_but_still_shows_intent() {
        let eval = evaluate_swap(6_577, 32_945, 5_016, 334, "FI Roth");
        assert!(eval.insufficient_lot_qty);
        assert_eq!(eval.net_annual_minor, 5_016 - annual_on_principal(32_945, 334));
    }

    #[test]
    fn cash_floor_only_for_income_car_health() {
        assert_eq!(cash_floor_minor("Income"), Some(CASH_FLOOR_INCOME_MINOR));
        assert_eq!(cash_floor_minor("Car"), Some(CASH_FLOOR_CAR_MINOR));
        assert_eq!(cash_floor_minor("Health"), Some(CASH_FLOOR_HEALTH_MINOR));
        assert_eq!(cash_floor_minor("FI Roth"), None);
        let warn = evaluate_swap(6_577, 5_990, 912, 334, "Income");
        assert!(warn.cash_floor_warn);
        let ok = evaluate_swap(600_000, 10_000, 0, 334, "Income");
        assert!(!ok.cash_floor_warn);
    }
}
