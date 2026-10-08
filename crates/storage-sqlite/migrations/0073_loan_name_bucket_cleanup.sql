-- Buckets are funding stores only. Loan identity moves to loan_name / account_name.
-- Remove loan names that were incorrectly seeded as buckets.

ALTER TABLE external_register_line ADD COLUMN loan_name TEXT NOT NULL DEFAULT '';

ALTER TABLE external_managed_account ADD COLUMN account_name TEXT NOT NULL DEFAULT '';

-- Creditor / family on the statement (Account Name). Loan Name stays name + register_key.
UPDATE external_managed_account SET account_name = 'Alphaeon'
WHERE account_id IN (
    'a1000001-0000-4000-8000-000000000006',
    'a1000001-0000-4000-8000-000000000007'
);
UPDATE external_managed_account SET account_name = name
WHERE IFNULL(account_name, '') = '';

-- Move misused loan buckets on CCT into loan_name; restore funding bucket.
UPDATE external_register_line
SET loan_name = trim(bucket),
    bucket = CASE lower(trim(bucket))
        WHEN 'alphaeon cat' THEN 'Medical'
        WHEN 'alphaeon ck' THEN 'Medical'
        WHEN 'paytient' THEN 'Medical'
        WHEN 'uva health 3/22-3/25' THEN 'Medical'
        WHEN 'myclearbalance' THEN 'Cash'
        WHEN 'truist bank' THEN 'Cash'
        WHEN 'newrez' THEN 'Mortgage'
        ELSE ''
    END
WHERE lower(trim(bucket)) IN (
    'alphaeon cat',
    'alphaeon ck',
    'paytient',
    'uva health 3/22-3/25',
    'myclearbalance',
    'truist bank',
    'newrez'
);

DELETE FROM external_budget_bucket
WHERE lower(trim(name)) IN (
    'alphaeon cat',
    'alphaeon ck',
    'paytient',
    'uva health 3/22-3/25',
    'myclearbalance',
    'truist bank',
    'newrez'
);

-- Keep Mom + Mortgage as funding stores if missing.
INSERT OR IGNORE INTO external_budget_bucket (bucket_id, name, bank, description, budget_category)
VALUES
    ('b1000001-0000-4000-8000-000000000007', 'Mom', '', 'Escrow funding store', 'Mom'),
    ('b1000001-0000-4000-8000-00000000000f', 'Mortgage', '', 'Housing envelope', 'Mortgage');
