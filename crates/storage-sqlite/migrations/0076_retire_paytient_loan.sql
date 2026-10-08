-- Paytient is paid off and is no longer a loan. Remove planner row, Loan element,
-- and related Week Ahead / CCT draft links. Historical CCT vendor lines stay.

DELETE FROM external_element_cct_draft
WHERE account_id = 'a1000001-0000-4000-8000-000000000004';

DELETE FROM planned_occurrence
WHERE element_id = 'e1000001-0000-4000-8000-000000000004';

DELETE FROM cash_element
WHERE element_id = 'e1000001-0000-4000-8000-000000000004';

DELETE FROM external_managed_applied
WHERE account_id = 'a1000001-0000-4000-8000-000000000004';

DELETE FROM external_managed_account
WHERE account_id = 'a1000001-0000-4000-8000-000000000004';

DELETE FROM external_budget_bucket
WHERE lower(trim(name)) = 'paytient';
