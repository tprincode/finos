-- Daily holdings market-value series for Home account charts (qty × last price).
-- account_id is the registered account UUID, or 'fidelity-total' for the Fidelity rollup.

CREATE TABLE account_market_value_daily (
    snapshot_id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    account_name TEXT NOT NULL,
    as_of TEXT NOT NULL,
    market_value_minor INTEGER,
    market_value_complete INTEGER NOT NULL,
    scale INTEGER NOT NULL,
    captured_at TEXT NOT NULL,
    UNIQUE(account_id, as_of)
);

CREATE INDEX idx_account_market_value_daily_as_of
    ON account_market_value_daily(as_of);