-- MAGI adjustments plus versioned Calculator Plan (schema 6).
-- Money scale 2 (USD cents). Plan remaining never writes actual cash.

CREATE TABLE magi_adjustment (
    adjustment_id TEXT PRIMARY KEY,
    amount_minor INTEGER NOT NULL,
    scale INTEGER NOT NULL DEFAULT 2,
    status TEXT NOT NULL,
    reason TEXT NOT NULL DEFAULT ''
);

CREATE TABLE calculator_plan (
    plan_id TEXT PRIMARY KEY,
    version INTEGER NOT NULL UNIQUE,
    remaining_minor INTEGER NOT NULL,
    scale INTEGER NOT NULL DEFAULT 2,
    approved_on TEXT NOT NULL,
    supersedes_plan_id TEXT
);
