-- Loan setup stores the annual rate and how often the payment is due.
-- apr_ppm is parts per million: 6.125% is 61250. frequency is weekly, monthly, quarterly, or annual.
-- A confirmed week-ahead payment is one row. The balance drop is principal only.

ALTER TABLE external_managed_account ADD COLUMN apr_ppm INTEGER;
ALTER TABLE external_managed_account ADD COLUMN frequency TEXT;

CREATE TABLE external_loan_payment (
    payment_id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    due_on TEXT NOT NULL,
    principal_minor INTEGER NOT NULL,
    interest_minor INTEGER NOT NULL,
    UNIQUE (account_id, due_on)
);
