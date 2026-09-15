-- Per-account cash on the weekly snapshot. Historical rows stay NULL.
ALTER TABLE account_balance_snapshot ADD COLUMN cash_minor INTEGER;
