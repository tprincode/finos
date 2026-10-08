-- Undo 0068: Mom shopping credit must match category/bucket Mom, not household Medical.
-- Clear applied rows and reset current to starting so reconcile rebuilds Mom-only history.
UPDATE external_managed_account
SET register_key = 'Mom',
    current_minor = starting_minor
WHERE account_id = 'a1000001-0000-4000-8000-000000000008';

DELETE FROM external_managed_applied
WHERE account_id = 'a1000001-0000-4000-8000-000000000008';
