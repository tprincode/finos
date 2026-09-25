-- Slice 2 Week Ahead: register series + unconfirmed occurrences.
-- No dividend days. Confirm posts through existing cash writers.
CREATE TABLE cash_element (
    element_id TEXT PRIMARY KEY,
    account TEXT NOT NULL,
    kind TEXT NOT NULL,
    cadence TEXT NOT NULL,
    amount_minor INTEGER NOT NULL,
    note TEXT NOT NULL DEFAULT '',
    weekday_or_month_day TEXT NOT NULL DEFAULT ''
);

CREATE TABLE planned_occurrence (
    occurrence_id TEXT PRIMARY KEY,
    element_id TEXT NOT NULL,
    account TEXT NOT NULL,
    kind TEXT NOT NULL,
    occurred_on TEXT NOT NULL,
    amount_minor INTEGER NOT NULL,
    confirmed_at TEXT,
    note TEXT NOT NULL DEFAULT '',
    FOREIGN KEY (element_id) REFERENCES cash_element(element_id)
);
