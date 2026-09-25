-- A green transaction date is complete. The true-up date stays as stored.
ALTER TABLE external_register_line ADD COLUMN completed INTEGER NOT NULL DEFAULT 0;

UPDATE external_register_line
SET completed = 1
WHERE true_up_on IS NOT NULL AND trim(true_up_on) <> '';
