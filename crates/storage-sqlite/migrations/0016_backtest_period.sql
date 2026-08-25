-- Owner-dated regime windows and calculated results (schema 16).
-- Periods are retained; results are not stored on Position or Lot.

CREATE TABLE backtest_period (
    period_id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    name TEXT NOT NULL DEFAULT '',
    start_on TEXT NOT NULL,
    end_on TEXT NOT NULL,
    benchmark_symbol TEXT NOT NULL DEFAULT '',
    selection_reason TEXT NOT NULL,
    method TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'approved',
    recorded_at TEXT NOT NULL
);

CREATE TABLE position_backtest_result (
    result_id TEXT PRIMARY KEY,
    security_id TEXT NOT NULL,
    period_id TEXT NOT NULL,
    price_return_bps INTEGER,
    total_return_bps INTEGER,
    cushion_bps INTEGER,
    max_drawdown_bps INTEGER,
    recovery_ratio_bps INTEGER,
    recovery_days INTEGER,
    income_reliability_bps INTEGER,
    bear_relative_bps INTEGER,
    downside_capture_bps INTEGER,
    upside_capture_bps INTEGER,
    completeness TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT '',
    calculated_at TEXT NOT NULL
);

CREATE INDEX position_backtest_result_security
    ON position_backtest_result (security_id, period_id, calculated_at);
