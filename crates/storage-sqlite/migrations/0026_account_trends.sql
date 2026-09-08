-- Weekly Trends facts: point-in-time account balances + entered week metrics.
-- Calculated fields (FID+SCH, Wk to Wk, DIV Delta, Total Cash, Acct9 70% proxy) are derived in queries.

CREATE TABLE account_balance_snapshot (
    snapshot_id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    period_end TEXT NOT NULL,
    balance_minor INTEGER NOT NULL,
    scale INTEGER NOT NULL,
    captured_at TEXT NOT NULL,
    UNIQUE(account_id, period_end)
);

CREATE INDEX idx_account_balance_snapshot_period
    ON account_balance_snapshot(period_end);

CREATE TABLE trends_week_source (
    period_end TEXT PRIMARY KEY,
    profit_minor INTEGER NOT NULL,
    monthly_divs_minor INTEGER NOT NULL,
    fidelity_total_minor INTEGER NOT NULL,
    schwab_total_minor INTEGER NOT NULL,
    income_cash_minor INTEGER NOT NULL,
    acct9_cash_minor INTEGER NOT NULL,
    acct9_etf_value_minor INTEGER NOT NULL,
    scale INTEGER NOT NULL,
    captured_at TEXT NOT NULL
);
