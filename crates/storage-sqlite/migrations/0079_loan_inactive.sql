-- Inactive loans stay in the DB (history intact) but leave the Debt planner list.
-- Restore Paytient as inactive with zero current balance after mistaken deletes.

ALTER TABLE external_managed_account ADD COLUMN inactive INTEGER NOT NULL DEFAULT 0;

INSERT OR IGNORE INTO external_managed_account (
    account_id, name, kind, charges_interest,
    starting_minor, current_minor, payment_minor, reduction_minor, finance_minor,
    paid_through, register_key, legacy_vendor, legacy_amount_minor, sort_order,
    pay_process, linked_element_id, account_name, inactive
) VALUES (
    'a1000001-0000-4000-8000-000000000004',
    'Paytient',
    'debt',
    0,
    NULL,
    0,
    19265,
    NULL,
    NULL,
    'August',
    'Paytient',
    NULL,
    NULL,
    4,
    'element',
    NULL,
    'Paytient',
    1
);

UPDATE external_managed_account
SET inactive = 1,
    current_minor = 0,
    account_name = CASE WHEN IFNULL(account_name, '') = '' THEN 'Paytient' ELSE account_name END,
    register_key = CASE WHEN IFNULL(register_key, '') = '' THEN 'Paytient' ELSE register_key END
WHERE account_id = 'a1000001-0000-4000-8000-000000000004';
