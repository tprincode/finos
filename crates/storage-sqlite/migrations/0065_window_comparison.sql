-- Price change of the underlying, SPY, and the Nasdaq-100 fund for the same window.
-- Null until that window is saved again.

ALTER TABLE position_backtest_result ADD COLUMN underlying_symbol TEXT NOT NULL DEFAULT '';
ALTER TABLE position_backtest_result ADD COLUMN underlying_return_bps INTEGER;
ALTER TABLE position_backtest_result ADD COLUMN spy_return_bps INTEGER;
ALTER TABLE position_backtest_result ADD COLUMN nasdaq_return_bps INTEGER;
