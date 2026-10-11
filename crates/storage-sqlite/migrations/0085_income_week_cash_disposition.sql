-- Owner cash disposition for an Income Plan week payer (Deposit received / Skipped).
CREATE TABLE income_week_cash_disposition (
    week_end TEXT NOT NULL,
    security_id TEXT NOT NULL REFERENCES security(security_id),
    disposition TEXT NOT NULL,
    confirmed_at TEXT NOT NULL,
    note TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (week_end, security_id),
    CHECK (disposition IN ('deposit_received', 'skipped'))
);

CREATE INDEX idx_income_week_cash_disposition_security
    ON income_week_cash_disposition (security_id);
