-- Owner exceptions survive series Save and leftover Saturday prune.
ALTER TABLE planned_occurrence ADD COLUMN is_exception INTEGER NOT NULL DEFAULT 0;
