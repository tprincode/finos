-- CCT open-charge workflow: Bill Pay, Bill Pay Deposit, Pay Bill, Bill Pay Withdrawal.
ALTER TABLE external_register_line ADD COLUMN step_billpay_deposit INTEGER NOT NULL DEFAULT 0;
ALTER TABLE external_register_line ADD COLUMN step_withdrawal INTEGER NOT NULL DEFAULT 0;
ALTER TABLE external_register_line ADD COLUMN step_billpay_deposit_on TEXT;
ALTER TABLE external_register_line ADD COLUMN step_withdrawal_on TEXT;

UPDATE external_register_line
SET step_billpay_deposit = 1,
    step_withdrawal = 1,
    step_billpay_deposit_on = COALESCE(NULLIF(trim(step_pay_on), ''), step_billpay_on),
    step_withdrawal_on = COALESCE(NULLIF(trim(step_pay_on), ''), step_billpay_on)
WHERE completed = 1
  AND step_transfer = 1
  AND step_billpay = 1
  AND step_pay = 1;
