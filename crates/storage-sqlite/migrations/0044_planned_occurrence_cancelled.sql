-- Cancelled planned hits stay on the cadence date so horizon does not refill them.
ALTER TABLE planned_occurrence ADD COLUMN is_cancelled INTEGER NOT NULL DEFAULT 0;
