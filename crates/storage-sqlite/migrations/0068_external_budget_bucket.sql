-- Budget bucket catalog (Debt planner). Line.bucket values must match a name here or blank.
CREATE TABLE IF NOT EXISTS external_budget_bucket (
    bucket_id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    bank TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    budget_category TEXT NOT NULL DEFAULT ''
);

INSERT OR IGNORE INTO external_budget_bucket (bucket_id, name, bank, description, budget_category)
VALUES
    ('b1000001-0000-4000-8000-000000000001', 'Food', '', '', 'Food'),
    ('b1000001-0000-4000-8000-000000000002', 'Cash', '', '', 'Cash'),
    ('b1000001-0000-4000-8000-000000000003', 'Bills', '', '', 'Bills'),
    ('b1000001-0000-4000-8000-000000000004', 'Pets', '', '', 'Pets'),
    ('b1000001-0000-4000-8000-000000000005', 'Medical', '', '', 'Medical'),
    ('b1000001-0000-4000-8000-000000000006', 'HSA', '', '', 'HSA');

-- Credit key follows the Medical catalog bucket (retired Mom bucket name).
UPDATE external_managed_account
SET register_key = 'Medical'
WHERE account_id = 'a1000001-0000-4000-8000-000000000008'
  AND register_key = 'Mom';
