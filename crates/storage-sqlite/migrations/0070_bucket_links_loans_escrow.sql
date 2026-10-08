-- Loan/escrow buckets for CCT → Debt planner link. Mom is escrow (not retired).
-- myClearbalance, Paytient, UVA move to register (Bucket Transaction Managed).

INSERT OR IGNORE INTO external_budget_bucket (bucket_id, name, bank, description, budget_category)
VALUES
    ('b1000001-0000-4000-8000-000000000007', 'Mom', '', 'Escrow account Mom shopping', ''),
    ('b1000001-0000-4000-8000-000000000008', 'Alphaeon Cat', '', 'Loan', ''),
    ('b1000001-0000-4000-8000-000000000009', 'Alphaeon CK', '', 'Loan', ''),
    ('b1000001-0000-4000-8000-00000000000a', 'myClearbalance', '', 'Loan', ''),
    ('b1000001-0000-4000-8000-00000000000b', 'Paytient', '', 'Loan', ''),
    ('b1000001-0000-4000-8000-00000000000c', 'UVA Health 3/22-3/25', '', 'Loan', ''),
    ('b1000001-0000-4000-8000-00000000000d', 'Newrez', '', 'Loan', ''),
    ('b1000001-0000-4000-8000-00000000000e', 'Truist BANK', '', 'Loan', '');

UPDATE external_managed_account
SET pay_process = 'register',
    linked_element_id = NULL
WHERE account_id IN (
    'a1000001-0000-4000-8000-000000000003',
    'a1000001-0000-4000-8000-000000000004',
    'a1000001-0000-4000-8000-000000000005'
);
