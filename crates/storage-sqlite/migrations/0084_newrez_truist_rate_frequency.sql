-- Week Ahead interest verify needs frequency + APR. 0074/0083 restored week_ahead
-- but left Newrez/Truist without frequency/apr on fresh migrate, so Loan payments
-- omitted them and could not project interest under the total payment.

UPDATE external_managed_account
SET frequency = COALESCE(NULLIF(frequency, ''), 'monthly'),
    apr_ppm = COALESCE(apr_ppm, 59900),
    pay_process = 'week_ahead'
WHERE account_id = 'a1000001-0000-4000-8000-000000000001'
  AND kind = 'debt';

UPDATE external_managed_account
SET frequency = COALESCE(NULLIF(frequency, ''), 'monthly'),
    apr_ppm = COALESCE(apr_ppm, 39900),
    pay_process = 'week_ahead'
WHERE account_id = 'a1000001-0000-4000-8000-000000000002'
  AND kind = 'debt';
