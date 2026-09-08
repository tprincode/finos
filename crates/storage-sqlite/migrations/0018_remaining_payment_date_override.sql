-- Owner overrides of remaining-year payment dates (schema 18).
-- System-proposed dates stay derived from declaration cadence; only owner edits are stored.

CREATE TABLE remaining_payment_date_override (
    override_id TEXT PRIMARY KEY,
    security_id TEXT NOT NULL,
    original_pay_on TEXT NOT NULL DEFAULT '',
    pay_on TEXT NOT NULL,
    recorded_at TEXT NOT NULL,
    superseded_by TEXT
);

CREATE INDEX remaining_payment_date_override_security
    ON remaining_payment_date_override (security_id, recorded_at, override_id);
