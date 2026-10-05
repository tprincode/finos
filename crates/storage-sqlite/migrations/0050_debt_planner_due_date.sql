-- Due date is the scheduled day of the payment.
-- Paytient is paid off and leaves the planner.
-- paid_through stays stored for a later week-ahead checkoff. This migration does not edit it.

ALTER TABLE external_managed_account ADD COLUMN due_on TEXT;

DELETE FROM external_managed_applied
WHERE account_id = 'a1000001-0000-4000-8000-000000000004';

DELETE FROM external_managed_account
WHERE account_id = 'a1000001-0000-4000-8000-000000000004';
