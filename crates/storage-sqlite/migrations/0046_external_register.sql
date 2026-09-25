-- External (non-brokerage) spending register. Not cash elements and not cash lots.

CREATE TABLE external_register_line (
    line_id TEXT PRIMARY KEY,
    source_row INTEGER,
    pay_type TEXT NOT NULL DEFAULT '',
    occurred_on TEXT,
    amount_minor INTEGER NOT NULL,
    scale INTEGER NOT NULL DEFAULT 2,
    category TEXT NOT NULL DEFAULT '',
    vendor TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    true_up_on TEXT
);

CREATE INDEX external_register_source_row ON external_register_line (source_row);
