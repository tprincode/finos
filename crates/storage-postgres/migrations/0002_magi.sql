-- MAGI facts, rule, coverage, adjustments. Isolated from ledger (ADR-0005).
-- Money scale 2 (USD cents) stored as BIGINT minor units.

CREATE TABLE magi_rule (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    threshold_minor BIGINT NOT NULL,
    safety_reserve_minor BIGINT NOT NULL,
    scale INTEGER NOT NULL DEFAULT 2
);

CREATE TABLE magi_fact (
    fact_id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL UNIQUE,
    treatment TEXT NOT NULL,
    amount_minor BIGINT NOT NULL,
    scale INTEGER NOT NULL,
    category TEXT NOT NULL DEFAULT ''
);

CREATE TABLE magi_coverage (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    completeness TEXT NOT NULL DEFAULT 'complete',
    remaining_minor BIGINT NOT NULL DEFAULT 0,
    withholding_minor BIGINT NOT NULL DEFAULT 0,
    form_total_minor BIGINT NOT NULL DEFAULT 0,
    warnings_json TEXT NOT NULL DEFAULT '[]'
);

CREATE TABLE magi_adjustment (
    adjustment_id TEXT PRIMARY KEY,
    amount_minor BIGINT NOT NULL,
    scale INTEGER NOT NULL DEFAULT 2,
    status TEXT NOT NULL,
    reason TEXT NOT NULL DEFAULT ''
);
