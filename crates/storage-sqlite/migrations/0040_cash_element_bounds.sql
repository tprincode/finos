-- Element series window. Empty start/stop means unbounded.
ALTER TABLE cash_element ADD COLUMN start_on TEXT NOT NULL DEFAULT '';
ALTER TABLE cash_element ADD COLUMN stop_on TEXT NOT NULL DEFAULT '';
