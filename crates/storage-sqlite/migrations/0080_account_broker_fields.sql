-- Locked brokerage account fields that never landed on `account`
-- (table was still only account_id, name, kind).

ALTER TABLE account ADD COLUMN cash_symbol TEXT NOT NULL DEFAULT '';
ALTER TABLE account ADD COLUMN broker_account_number TEXT NOT NULL DEFAULT '';
ALTER TABLE account ADD COLUMN min_balance_target_minor INTEGER;
