//! Backtest runs are hypothetical. They never post ledger, lots, or MAGI facts.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BacktestRun {
    pub scenario: String,
    pub hypothetical_pnl_minor: i64,
    pub scale: u8,
}

/// Scale 2: 12500 means $125.00 hypothetical P&L. Not a posted activity.
pub fn prepare_run(scenario: String, hypothetical_pnl_minor: i64, scale: u8) -> BacktestRun {
    BacktestRun {
        scenario,
        hypothetical_pnl_minor,
        scale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backtest_run_is_not_a_posted_activity() {
        let posted_before = 0i64;
        let cash_before = 50_000i64;
        let run = prepare_run("vxus-rebalance".into(), 12_500, 2);
        assert_eq!(run.hypothetical_pnl_minor, 12_500);
        assert_eq!(posted_before, 0);
        assert_eq!(cash_before, 50_000);
        assert_ne!(run.hypothetical_pnl_minor, cash_before);
    }
}
