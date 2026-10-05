-- SPAXX baseline at first confirm-sells; cash alignment uses actual sell/buy dollars.

ALTER TABLE cart_scenario ADD COLUMN execute_cash_baseline_minor INTEGER;
