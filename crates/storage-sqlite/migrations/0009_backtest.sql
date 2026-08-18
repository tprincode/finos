-- Decision-support backtest runs. Does not post ledger, lots, or MAGI facts.
-- hypothetical_pnl_minor scale 2: 12500 = $125.00 hypothetical P&L.

CREATE TABLE backtest_run (
    run_id TEXT PRIMARY KEY,
    scenario TEXT NOT NULL,
    hypothetical_pnl_minor INTEGER NOT NULL,
    scale INTEGER NOT NULL DEFAULT 2,
    completed_at TEXT NOT NULL
);
