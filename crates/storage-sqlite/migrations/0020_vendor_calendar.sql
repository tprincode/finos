-- Vendor standing order: researched URL, calendar policy, last-run, issuer remaining-year dates.

ALTER TABLE retrieval_template ADD COLUMN source_url TEXT NOT NULL DEFAULT '';
ALTER TABLE retrieval_template ADD COLUMN calendar_policy TEXT NOT NULL DEFAULT '';
ALTER TABLE retrieval_template ADD COLUMN last_run_at TEXT NOT NULL DEFAULT '';
ALTER TABLE retrieval_template ADD COLUMN last_run_ok INTEGER;
ALTER TABLE retrieval_template ADD COLUMN last_run_message TEXT NOT NULL DEFAULT '';

CREATE TABLE issuer_pay_date (
    pay_date_id TEXT PRIMARY KEY,
    security_id TEXT NOT NULL,
    pay_on TEXT NOT NULL,
    source TEXT NOT NULL,
    recorded_at TEXT NOT NULL,
    superseded_by TEXT
);

CREATE INDEX issuer_pay_date_security
    ON issuer_pay_date (security_id, pay_on, recorded_at);
