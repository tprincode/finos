-- Truist facts sit in a two-column list beside the ring. The prose note is retired.

UPDATE external_loan_vendor
SET note = 'Original|$42,011.15
Paid to date|$34,176.45
Payoff on 9/28/26|$7,852.69
Scheduled clear|11/11/27
Contract maturity|1/11/28'
WHERE account_id = 'a1000001-0000-4000-8000-000000000002';
