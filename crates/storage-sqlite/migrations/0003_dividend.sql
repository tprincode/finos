CREATE TABLE dividend_declaration (
    declaration_id TEXT PRIMARY KEY,
    security_symbol TEXT NOT NULL,
    declared_on TEXT NOT NULL,
    amount_minor INTEGER NOT NULL,
    scale INTEGER NOT NULL
);

CREATE TABLE dividend_actual (
    actual_id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    security_id TEXT,
    occurred_on TEXT NOT NULL,
    amount_minor INTEGER NOT NULL,
    scale INTEGER NOT NULL,
    activity_id TEXT,
    idempotency_key TEXT NOT NULL UNIQUE
);

CREATE TABLE income_plan (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    planned_amount_minor INTEGER NOT NULL DEFAULT 0,
    scale INTEGER NOT NULL DEFAULT 2
);
