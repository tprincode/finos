-- Truist statement mapped into the same vendor history as Newrez.
-- Interest on the listed payments is daily 3.99% and matches the activity to the penny.
-- Older payments are not reconstructed: a level schedule from the original amount does not tie.

ALTER TABLE external_loan_vendor ADD COLUMN note TEXT;

UPDATE external_managed_account
SET starting_minor = 4201115,
    current_minor = 783470,
    reduction_minor = 54659,
    finance_minor = 2840,
    apr_ppm = 39900,
    frequency = 'monthly',
    due_on = '2026-10-11',
    paid_through = 'September',
    charges_interest = 1
WHERE account_id = 'a1000001-0000-4000-8000-000000000002';

INSERT INTO external_loan_vendor (
    account_id, term_payments, payments_made,
    principal_paid_minor, interest_paid_minor, escrow_paid_minor,
    principal_balance_minor, escrow_balance_minor, note
) VALUES (
    'a1000001-0000-4000-8000-000000000002', 10, 10,
    3417645, 35901, 0,
    783470, 0,
    'Principal paid is the paydown from $42,011.15. Interest is the 10 listed payments. Payoff on 9/28/26 is $7,852.69. Scheduled clear date 11/11/27. Contract maturity 1/11/28.'
);

INSERT INTO external_loan_vendor_line (
    line_id, account_id, sort_order, occurred_on, amount_minor, due_on, description,
    principal_minor, interest_minor, escrow_minor, late_minor,
    principal_balance_minor, escrow_balance_minor
) VALUES
    ('a1000002-0000-4000-8000-000000000011', 'a1000001-0000-4000-8000-000000000002', 1, '2026-09-07', 57499, '2026-09-11', 'Regular Payment', 54659, 2840, 0, 0, 783470, 0),
    ('a1000002-0000-4000-8000-000000000012', 'a1000001-0000-4000-8000-000000000002', 2, '2026-08-07', 57499, '2026-08-11', 'Regular Payment', 54474, 3025, 0, 0, 838129, 0),
    ('a1000002-0000-4000-8000-000000000013', 'a1000001-0000-4000-8000-000000000002', 3, '2026-07-07', 57499, '2026-07-11', 'Regular Payment', 54394, 3105, 0, 0, 892603, 0),
    ('a1000002-0000-4000-8000-000000000014', 'a1000001-0000-4000-8000-000000000002', 4, '2026-06-07', 57499, '2026-06-11', 'Regular Payment', 54106, 3393, 0, 0, 946997, 0),
    ('a1000002-0000-4000-8000-000000000015', 'a1000001-0000-4000-8000-000000000002', 5, '2026-05-07', 57499, '2026-05-11', 'Regular Payment', 54039, 3460, 0, 0, 1001103, 0),
    ('a1000002-0000-4000-8000-000000000016', 'a1000001-0000-4000-8000-000000000002', 6, '2026-04-07', 57499, '2026-04-11', 'Regular Payment', 53741, 3758, 0, 0, 1055142, 0),
    ('a1000002-0000-4000-8000-000000000017', 'a1000001-0000-4000-8000-000000000002', 7, '2026-03-07', 57499, '2026-03-11', 'Regular Payment', 53940, 3559, 0, 0, 1108883, 0),
    ('a1000002-0000-4000-8000-000000000018', 'a1000001-0000-4000-8000-000000000002', 8, '2026-02-07', 57499, '2026-02-11', 'Regular Payment', 53377, 4122, 0, 0, 1162823, 0),
    ('a1000002-0000-4000-8000-000000000019', 'a1000001-0000-4000-8000-000000000002', 9, '2026-01-07', 57499, '2026-01-11', 'Regular Payment', 53198, 4301, 0, 0, 1216200, 0),
    ('a1000002-0000-4000-8000-000000000020', 'a1000001-0000-4000-8000-000000000002', 10, '2025-12-07', 57499, '2025-12-11', 'Regular Payment', 53161, 4338, 0, 0, 1269398, 0);
