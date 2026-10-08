-- Phase 3–4: Loan Elements draft CCT on Week Ahead confirm; debts settle on CCT.
-- Explicit association_kind separates loan / distribution / withdrawal.

ALTER TABLE cash_element ADD COLUMN association_kind TEXT NOT NULL DEFAULT '';

UPDATE cash_element
SET association_kind = 'distribution'
WHERE lower(trim(account)) IN ('income', 'ssa_2026', 'fi roth', 'roth');

UPDATE cash_element
SET association_kind = 'withdrawal'
WHERE lower(trim(account)) IN ('car', 'health', 'account 9', '9');

CREATE TABLE IF NOT EXISTS external_element_cct_draft (
    occurrence_id TEXT PRIMARY KEY NOT NULL,
    line_id TEXT NOT NULL,
    account_id TEXT NOT NULL
);

-- Dedicated Loan-book elements (stable ids). Cadence monthly; day from planner due when known.
INSERT OR IGNORE INTO cash_element (
    element_id, account, kind, cadence, amount_minor, note, weekday_or_month_day,
    start_on, stop_on, association_kind
) VALUES
    ('e1000001-0000-4000-8000-000000000001', 'Loan', 'Withdrawal', 'monthly', 179672, 'Newrez', '1', '', '', 'loan'),
    ('e1000001-0000-4000-8000-000000000002', 'Loan', 'Withdrawal', 'monthly', 57499, 'Truist BANK', '1', '', '', 'loan'),
    ('e1000001-0000-4000-8000-000000000003', 'Loan', 'Withdrawal', 'monthly', 20300, 'myClearbalance', '1', '', '', 'loan'),
    ('e1000001-0000-4000-8000-000000000004', 'Loan', 'Withdrawal', 'monthly', 19265, 'Paytient', '1', '', '', 'loan'),
    ('e1000001-0000-4000-8000-000000000005', 'Loan', 'Withdrawal', 'monthly', 12300, 'UVA Health 3/22-3/25', '1', '', '', 'loan'),
    ('e1000001-0000-4000-8000-000000000006', 'Loan', 'Withdrawal', 'monthly', 23600, 'Alphaeon Cat', '1', '', '', 'loan'),
    ('e1000001-0000-4000-8000-000000000007', 'Loan', 'Withdrawal', 'monthly', 25000, 'Alphaeon CK', '1', '', '', 'loan');

-- Sync amounts from planner when present.
UPDATE cash_element
SET amount_minor = (
    SELECT payment_minor FROM external_managed_account
    WHERE account_id = 'a1000001-0000-4000-8000-000000000001' AND payment_minor IS NOT NULL
)
WHERE element_id = 'e1000001-0000-4000-8000-000000000001'
  AND EXISTS (
      SELECT 1 FROM external_managed_account
      WHERE account_id = 'a1000001-0000-4000-8000-000000000001' AND payment_minor IS NOT NULL
  );

UPDATE cash_element
SET amount_minor = (
    SELECT payment_minor FROM external_managed_account
    WHERE account_id = 'a1000001-0000-4000-8000-000000000002' AND payment_minor IS NOT NULL
)
WHERE element_id = 'e1000001-0000-4000-8000-000000000002'
  AND EXISTS (
      SELECT 1 FROM external_managed_account
      WHERE account_id = 'a1000001-0000-4000-8000-000000000002' AND payment_minor IS NOT NULL
  );

UPDATE cash_element
SET amount_minor = (
    SELECT payment_minor FROM external_managed_account
    WHERE account_id = 'a1000001-0000-4000-8000-000000000003' AND payment_minor IS NOT NULL
)
WHERE element_id = 'e1000001-0000-4000-8000-000000000003'
  AND EXISTS (
      SELECT 1 FROM external_managed_account
      WHERE account_id = 'a1000001-0000-4000-8000-000000000003' AND payment_minor IS NOT NULL
  );

UPDATE cash_element
SET amount_minor = (
    SELECT payment_minor FROM external_managed_account
    WHERE account_id = 'a1000001-0000-4000-8000-000000000004' AND payment_minor IS NOT NULL
)
WHERE element_id = 'e1000001-0000-4000-8000-000000000004'
  AND EXISTS (
      SELECT 1 FROM external_managed_account
      WHERE account_id = 'a1000001-0000-4000-8000-000000000004' AND payment_minor IS NOT NULL
  );

UPDATE cash_element
SET amount_minor = (
    SELECT payment_minor FROM external_managed_account
    WHERE account_id = 'a1000001-0000-4000-8000-000000000005' AND payment_minor IS NOT NULL
)
WHERE element_id = 'e1000001-0000-4000-8000-000000000005'
  AND EXISTS (
      SELECT 1 FROM external_managed_account
      WHERE account_id = 'a1000001-0000-4000-8000-000000000005' AND payment_minor IS NOT NULL
  );

UPDATE cash_element
SET amount_minor = (
    SELECT payment_minor FROM external_managed_account
    WHERE account_id = 'a1000001-0000-4000-8000-000000000006' AND payment_minor IS NOT NULL
)
WHERE element_id = 'e1000001-0000-4000-8000-000000000006'
  AND EXISTS (
      SELECT 1 FROM external_managed_account
      WHERE account_id = 'a1000001-0000-4000-8000-000000000006' AND payment_minor IS NOT NULL
  );

UPDATE cash_element
SET amount_minor = (
    SELECT payment_minor FROM external_managed_account
    WHERE account_id = 'a1000001-0000-4000-8000-000000000007' AND payment_minor IS NOT NULL
)
WHERE element_id = 'e1000001-0000-4000-8000-000000000007'
  AND EXISTS (
      SELECT 1 FROM external_managed_account
      WHERE account_id = 'a1000001-0000-4000-8000-000000000007' AND payment_minor IS NOT NULL
  );

-- All debts use Scheduled element. Mom escrow stays register.
UPDATE external_managed_account
SET pay_process = 'element',
    linked_element_id = 'e1000001-0000-4000-8000-000000000001',
    account_name = CASE WHEN IFNULL(account_name, '') = '' THEN 'Newrez' ELSE account_name END
WHERE account_id = 'a1000001-0000-4000-8000-000000000001';

UPDATE external_managed_account
SET pay_process = 'element',
    linked_element_id = 'e1000001-0000-4000-8000-000000000002',
    account_name = CASE WHEN IFNULL(account_name, '') = '' THEN 'Truist' ELSE account_name END
WHERE account_id = 'a1000001-0000-4000-8000-000000000002';

UPDATE external_managed_account
SET pay_process = 'element',
    linked_element_id = 'e1000001-0000-4000-8000-000000000003',
    account_name = CASE WHEN IFNULL(account_name, '') = '' THEN 'myClearbalance' ELSE account_name END
WHERE account_id = 'a1000001-0000-4000-8000-000000000003';

UPDATE external_managed_account
SET pay_process = 'element',
    linked_element_id = 'e1000001-0000-4000-8000-000000000004',
    account_name = CASE WHEN IFNULL(account_name, '') = '' THEN 'Paytient' ELSE account_name END
WHERE account_id = 'a1000001-0000-4000-8000-000000000004';

UPDATE external_managed_account
SET pay_process = 'element',
    linked_element_id = 'e1000001-0000-4000-8000-000000000005',
    account_name = CASE WHEN IFNULL(account_name, '') = '' THEN 'UVA Health' ELSE account_name END
WHERE account_id = 'a1000001-0000-4000-8000-000000000005';

UPDATE external_managed_account
SET pay_process = 'element',
    linked_element_id = 'e1000001-0000-4000-8000-000000000006',
    account_name = CASE WHEN IFNULL(account_name, '') = '' THEN 'Alphaeon' ELSE account_name END
WHERE account_id = 'a1000001-0000-4000-8000-000000000006';

UPDATE external_managed_account
SET pay_process = 'element',
    linked_element_id = 'e1000001-0000-4000-8000-000000000007',
    account_name = CASE WHEN IFNULL(account_name, '') = '' THEN 'Alphaeon' ELSE account_name END
WHERE account_id = 'a1000001-0000-4000-8000-000000000007';
