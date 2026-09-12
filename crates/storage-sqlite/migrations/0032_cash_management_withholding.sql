ALTER TABLE activity_event ADD COLUMN federal_withholding_minor INTEGER NOT NULL DEFAULT 0;
ALTER TABLE activity_event ADD COLUMN state_withholding_minor INTEGER NOT NULL DEFAULT 0;
