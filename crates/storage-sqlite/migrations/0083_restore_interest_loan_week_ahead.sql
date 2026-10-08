-- Interest-bearing debts use Week Ahead Loan payments (projected interest + verify,
-- principal-only paydown). Migration 0074 forced every debt onto Scheduled element → CCT,
-- which skipped interest verification and could reduce Current by the full payment.
-- Zero-interest medical loans stay on element → CCT. Car brokerage withdrawal is untouched.

-- Undo Truist element confirm that drafted CCT without interest split.
DELETE FROM external_register_line
WHERE line_id IN (
    SELECT line_id FROM external_element_cct_draft
    WHERE account_id = 'a1000001-0000-4000-8000-000000000002'
);
DELETE FROM external_element_cct_draft
WHERE account_id = 'a1000001-0000-4000-8000-000000000002';

UPDATE planned_occurrence
SET confirmed_at = NULL
WHERE element_id = 'e1000001-0000-4000-8000-000000000002'
  AND account = 'Loan'
  AND note = 'Truist BANK'
  AND occurred_on = '2026-10-07';

-- Newrez + Truist (and any other interest debt) back on Week Ahead interest path.
UPDATE external_managed_account
SET pay_process = 'week_ahead'
WHERE kind = 'debt'
  AND inactive = 0
  AND charges_interest = 1;
