-- Restore Paytient row deleted from live DB. Alphaeon Cat/CK stay on register for CCT
-- matching (Loan type label shows Register; only Mom credit is Bucket Transaction Managed).

INSERT OR IGNORE INTO external_managed_account (
    account_id, name, kind, charges_interest,
    starting_minor, current_minor, payment_minor, reduction_minor, finance_minor,
    paid_through, register_key, legacy_vendor, legacy_amount_minor, sort_order,
    pay_process, linked_element_id
) VALUES (
    'a1000001-0000-4000-8000-000000000004',
    'Paytient',
    'debt',
    0,
    NULL,
    72820,
    19265,
    18098,
    NULL,
    'August',
    'Paytient',
    NULL,
    NULL,
    4,
    'register',
    NULL
);

UPDATE external_managed_account
SET pay_process = 'register',
    linked_element_id = NULL
WHERE account_id IN (
    'a1000001-0000-4000-8000-000000000006',
    'a1000001-0000-4000-8000-000000000007'
)
  AND IFNULL(pay_process, '') != 'register';
