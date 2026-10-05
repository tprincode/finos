-- Statement history from the loan vendor. Amounts are integer cents.
-- Newrez is a 360-payment mortgage. Six regular payments are posted through 2026-09-10.

CREATE TABLE external_loan_vendor (
    account_id TEXT PRIMARY KEY,
    term_payments INTEGER NOT NULL,
    payments_made INTEGER NOT NULL,
    principal_paid_minor INTEGER NOT NULL,
    interest_paid_minor INTEGER NOT NULL,
    escrow_paid_minor INTEGER NOT NULL,
    principal_balance_minor INTEGER NOT NULL,
    escrow_balance_minor INTEGER NOT NULL
);

CREATE TABLE external_loan_vendor_line (
    line_id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    sort_order INTEGER NOT NULL,
    occurred_on TEXT NOT NULL,
    amount_minor INTEGER NOT NULL,
    due_on TEXT NOT NULL,
    description TEXT NOT NULL,
    principal_minor INTEGER NOT NULL,
    interest_minor INTEGER NOT NULL,
    escrow_minor INTEGER NOT NULL,
    late_minor INTEGER NOT NULL,
    principal_balance_minor INTEGER NOT NULL,
    escrow_balance_minor INTEGER NOT NULL
);

INSERT INTO external_loan_vendor (
    account_id, term_payments, payments_made,
    principal_paid_minor, interest_paid_minor, escrow_paid_minor,
    principal_balance_minor, escrow_balance_minor
) VALUES (
    'a1000001-0000-4000-8000-000000000001', 360, 6,
    181786, 896246, 0,
    29818214, 0
);

INSERT INTO external_loan_vendor_line (
    line_id, account_id, sort_order, occurred_on, amount_minor, due_on, description,
    principal_minor, interest_minor, escrow_minor, late_minor,
    principal_balance_minor, escrow_balance_minor
) VALUES
    ('a1000002-0000-4000-8000-000000000001', 'a1000001-0000-4000-8000-000000000001', 1, '2026-09-10', 179672, '2026-09-01', 'Regular Payment', 30676, 148996, 0, 0, 29818214, 0),
    ('a1000002-0000-4000-8000-000000000002', 'a1000001-0000-4000-8000-000000000001', 2, '2026-08-10', 179672, '2026-08-01', 'Regular Payment', 30524, 149148, 0, 0, 29848890, 0),
    ('a1000002-0000-4000-8000-000000000003', 'a1000001-0000-4000-8000-000000000001', 3, '2026-08-05', 0, '2026-08-01', 'Inv Loan Purchase', 0, 6680, 0, 0, 29879414, 0),
    ('a1000002-0000-4000-8000-000000000004', 'a1000001-0000-4000-8000-000000000001', 4, '2026-08-03', 0, '2026-08-01', 'Investor Loan Sale', 0, 6680, 0, 0, 29879414, 0),
    ('a1000002-0000-4000-8000-000000000005', 'a1000001-0000-4000-8000-000000000001', 5, '2026-07-10', 179672, '2026-07-01', 'Regular Payment', 30372, 149300, 0, 0, 29879414, 0),
    ('a1000002-0000-4000-8000-000000000006', 'a1000001-0000-4000-8000-000000000001', 6, '2026-06-10', 179672, '2026-06-01', 'Regular Payment', 30221, 149451, 0, 0, 29909786, 0),
    ('a1000002-0000-4000-8000-000000000007', 'a1000001-0000-4000-8000-000000000001', 7, '2026-05-10', 179672, '2026-05-01', 'Regular Payment', 30071, 149601, 0, 0, 29940007, 0),
    ('a1000002-0000-4000-8000-000000000008', 'a1000001-0000-4000-8000-000000000001', 8, '2026-04-10', 179672, '2026-04-01', 'Regular Payment', 29922, 149750, 0, 0, 29970078, 0);
