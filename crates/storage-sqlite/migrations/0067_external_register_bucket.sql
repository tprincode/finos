-- Bucket is whose budget funded the charge (Mom, Cash acct, Bill acct).
-- Blank stays blank — never copy category into bucket.
ALTER TABLE external_register_line ADD COLUMN bucket TEXT NOT NULL DEFAULT '';
