-- Truist is the car *loan* (Debt planner / Loan element). Due on the 7th (see 0055).
-- Migration 0074 wrongly seeded its Loan Element on day 1.
-- The Car *brokerage* monthly withdrawal is a separate cash_element on account Car — do not touch it.

UPDATE cash_element
SET weekday_or_month_day = '7'
WHERE element_id = 'e1000001-0000-4000-8000-000000000002'
  AND account = 'Loan'
  AND note = 'Truist BANK';

UPDATE external_managed_account
SET due_on = CASE
        WHEN due_on IS NULL OR substr(due_on, 9, 2) != '07'
            THEN '2026-10-07'
        ELSE due_on
    END,
    frequency = COALESCE(NULLIF(frequency, ''), 'monthly')
WHERE account_id = 'a1000001-0000-4000-8000-000000000002'
  AND kind = 'debt';

-- Drop leftover Loan occurrences that were generated while the element said day 1.
DELETE FROM planned_occurrence
WHERE element_id = 'e1000001-0000-4000-8000-000000000002'
  AND account = 'Loan'
  AND note = 'Truist BANK'
  AND confirmed_at IS NULL
  AND substr(occurred_on, 9, 2) != '07';
