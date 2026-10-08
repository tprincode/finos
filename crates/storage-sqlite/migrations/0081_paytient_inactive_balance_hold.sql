-- Inactive loans must not keep shifting Current from CCT settle.
-- Re-zero Paytient after reconcile applied completed lines post-restore.

UPDATE external_managed_account
SET inactive = 1,
    current_minor = 0
WHERE account_id = 'a1000001-0000-4000-8000-000000000004';

-- Drop applied markers so a future reactivation can re-settle cleanly if needed.
DELETE FROM external_managed_applied
WHERE account_id = 'a1000001-0000-4000-8000-000000000004';
