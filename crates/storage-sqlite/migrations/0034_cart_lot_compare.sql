-- Dual-basis sell P/L snapshots and named draft compare.

ALTER TABLE cart_scenario ADD COLUMN name TEXT NOT NULL DEFAULT 'Draft';

ALTER TABLE cart_sell_line ADD COLUMN performance_cost_minor INTEGER;
ALTER TABLE cart_sell_line ADD COLUMN tax_cost_minor INTEGER;
ALTER TABLE cart_sell_line ADD COLUMN performance_gain_minor INTEGER;
ALTER TABLE cart_sell_line ADD COLUMN tax_gain_minor INTEGER;
