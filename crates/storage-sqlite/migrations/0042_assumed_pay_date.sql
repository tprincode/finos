-- Assumed next-year pay dates (holes only). Vendor issuer_pay_date prunes the same slot.
CREATE TABLE assumed_pay_date (
    assumed_pay_date_id TEXT PRIMARY KEY NOT NULL,
    security_id TEXT NOT NULL REFERENCES security(security_id),
    pay_on TEXT NOT NULL,
    cadence TEXT NOT NULL,
    provenance TEXT NOT NULL DEFAULT 'assumed_next_year',
    assumed_on TEXT NOT NULL,
    UNIQUE (security_id, pay_on)
);

CREATE INDEX idx_assumed_pay_date_security ON assumed_pay_date (security_id);
