-- The day a Checking and Credit step was ticked. Empty on rows ticked before this column existed.

ALTER TABLE external_register_line ADD COLUMN step_transfer_on TEXT;
ALTER TABLE external_register_line ADD COLUMN step_billpay_on TEXT;
ALTER TABLE external_register_line ADD COLUMN step_pay_on TEXT;
