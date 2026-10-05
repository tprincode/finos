-- Each loan is paid one way.
-- week_ahead: Week Ahead confirm edits interest and drops the balance by principal.
-- register: a Checking and Credit line moves the balance.
-- element: an existing element stays the Week Ahead row. Confirming it drops this balance.
-- A linked element does not also appear as a loan row.

ALTER TABLE external_managed_account ADD COLUMN pay_process TEXT;
ALTER TABLE external_managed_account ADD COLUMN linked_element_id TEXT;

CREATE TABLE external_element_loan_applied (
    account_id TEXT NOT NULL,
    occurrence_id TEXT NOT NULL,
    amount_minor INTEGER NOT NULL,
    PRIMARY KEY (account_id, occurrence_id)
);

UPDATE external_managed_account
SET pay_process = 'week_ahead'
WHERE account_id IN (
    'a1000001-0000-4000-8000-000000000001',
    'a1000001-0000-4000-8000-000000000002'
);

UPDATE external_managed_account
SET pay_process = 'register'
WHERE account_id IN (
    'a1000001-0000-4000-8000-000000000006',
    'a1000001-0000-4000-8000-000000000007',
    'a1000001-0000-4000-8000-000000000008'
);

UPDATE external_managed_account
SET pay_process = 'element',
    linked_element_id = 'c4793fec-b4a5-47e8-ad6e-f30afef40277'
WHERE account_id = 'a1000001-0000-4000-8000-000000000003'
  AND EXISTS (
      SELECT 1 FROM cash_element
      WHERE element_id = 'c4793fec-b4a5-47e8-ad6e-f30afef40277'
  );

UPDATE external_managed_account
SET pay_process = 'element'
WHERE account_id = 'a1000001-0000-4000-8000-000000000003'
  AND pay_process IS NULL;

UPDATE external_managed_account
SET pay_process = 'element'
WHERE account_id = 'a1000001-0000-4000-8000-000000000005';
