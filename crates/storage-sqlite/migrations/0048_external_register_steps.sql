-- A charge stays open until transfer, bill pay entry, and pay bill are all marked.
ALTER TABLE external_register_line ADD COLUMN step_transfer INTEGER NOT NULL DEFAULT 0;
ALTER TABLE external_register_line ADD COLUMN step_billpay INTEGER NOT NULL DEFAULT 0;
ALTER TABLE external_register_line ADD COLUMN step_pay INTEGER NOT NULL DEFAULT 0;

UPDATE external_register_line
SET step_transfer = 1, step_billpay = 1, step_pay = 1
WHERE completed = 1;

-- The three rows closed by the transfer button stay open, with transfer already marked.
UPDATE external_register_line
SET completed = 0,
    true_up_on = NULL,
    step_transfer = 1,
    step_billpay = 0,
    step_pay = 0
WHERE source_row IN (6, 7, 8);
