-- Position Details owner settings: active flag, weekday patterns, expected tax profile.
-- Patterns and tax profile are not inferred from declarations or ledger cash.

ALTER TABLE position_characteristic ADD COLUMN is_active INTEGER NOT NULL DEFAULT 1;

CREATE TABLE expected_payment_pattern (
    security_id TEXT PRIMARY KEY,
    declaration_weekday TEXT NOT NULL DEFAULT '',
    exdate_weekday TEXT NOT NULL DEFAULT '',
    payday_weekday TEXT NOT NULL DEFAULT ''
);

CREATE TABLE position_tax_profile (
    security_id TEXT PRIMARY KEY,
    expected_handling TEXT NOT NULL DEFAULT ''
);
