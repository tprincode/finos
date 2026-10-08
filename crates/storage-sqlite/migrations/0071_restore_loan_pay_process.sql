-- 0070 wrongly put myClearbalance, Paytient, and UVA on register.
-- Only Mom shopping (escrow) is Bucket Transaction Managed.
-- Restore prior pay processes from 0056.

UPDATE external_managed_account
SET pay_process = 'element',
    linked_element_id = CASE
        WHEN account_id = 'a1000001-0000-4000-8000-000000000003'
             AND EXISTS (
                 SELECT 1 FROM cash_element
                 WHERE element_id = 'c4793fec-b4a5-47e8-ad6e-f30afef40277'
             )
        THEN 'c4793fec-b4a5-47e8-ad6e-f30afef40277'
        ELSE linked_element_id
    END
WHERE account_id = 'a1000001-0000-4000-8000-000000000003';

UPDATE external_managed_account
SET pay_process = 'element',
    linked_element_id = NULL
WHERE account_id = 'a1000001-0000-4000-8000-000000000005';

-- Paytient had no pay_process before 0070.
UPDATE external_managed_account
SET pay_process = NULL,
    linked_element_id = NULL
WHERE account_id = 'a1000001-0000-4000-8000-000000000004';
