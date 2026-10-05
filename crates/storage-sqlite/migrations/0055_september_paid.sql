-- September is paid on Newrez and Truist. The next payment is in October.
-- Truist is due on the 7th of each month. Newrez stays on the 10th.

UPDATE external_managed_account
SET paid_through = 'September',
    due_on = '2026-10-10',
    current_minor = 29818214,
    reduction_minor = 30676,
    finance_minor = 148996
WHERE account_id = 'a1000001-0000-4000-8000-000000000001';

UPDATE external_managed_account
SET paid_through = 'September',
    due_on = '2026-10-07'
WHERE account_id = 'a1000001-0000-4000-8000-000000000002';

UPDATE external_loan_vendor_line
SET due_on = occurred_on
WHERE account_id = 'a1000001-0000-4000-8000-000000000002';

UPDATE external_loan_vendor
SET note = 'Original|$42,011.15
Paid to date|$34,176.45
Payoff on 9/28/26|$7,852.69
Scheduled clear|11/7/27
Contract maturity|1/11/28'
WHERE account_id = 'a1000001-0000-4000-8000-000000000002';
