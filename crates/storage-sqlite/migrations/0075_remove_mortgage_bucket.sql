-- Mortgage is not a funding bucket for now. Newrez loan stays on the Debt planner.
-- Clear CCT lines that used Mortgage as a funding store.

UPDATE external_register_line
SET bucket = ''
WHERE lower(trim(bucket)) = 'mortgage';

DELETE FROM external_budget_bucket
WHERE lower(trim(name)) = 'mortgage';
