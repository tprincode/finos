-- Optimistic concurrency on account (M8 slice 4). SQLite ignores expectedVersion.

ALTER TABLE account ADD COLUMN row_version INTEGER NOT NULL DEFAULT 1;
