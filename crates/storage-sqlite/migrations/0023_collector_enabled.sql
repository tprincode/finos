-- Standing-order enable flag. Assigned source + enabled=1 enters DeclarationRefresh.
ALTER TABLE retrieval_template ADD COLUMN collector_enabled INTEGER NOT NULL DEFAULT 0;
