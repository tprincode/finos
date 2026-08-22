-- Calculator PlanHistory and position characteristics (schema 14).
-- Plan is owner-controlled per share. Lots never store Plan. Blank ROC stays null.

CREATE TABLE plan_history (
    plan_history_id TEXT PRIMARY KEY,
    security_id TEXT NOT NULL,
    amount_per_share_minor INTEGER NOT NULL,
    amount_scale INTEGER NOT NULL,
    planning_periods_per_year INTEGER NOT NULL,
    effective_from TEXT NOT NULL,
    effective_to TEXT,
    decision_date TEXT NOT NULL,
    decision_reason TEXT NOT NULL DEFAULT ''
);

CREATE UNIQUE INDEX plan_history_open_security
    ON plan_history (security_id) WHERE effective_to IS NULL;

CREATE TABLE position_characteristic (
    security_id TEXT PRIMARY KEY,
    payment_frequency TEXT NOT NULL DEFAULT '',
    risk_tier TEXT NOT NULL DEFAULT '',
    provider TEXT NOT NULL DEFAULT '',
    underlying TEXT NOT NULL DEFAULT '',
    roc_pct_2025_actual_minor INTEGER,
    roc_pct_2026_estimate_minor INTEGER,
    roc_pct_2026_actual_minor INTEGER,
    roc_pct_2024_actual_minor INTEGER,
    roc_scale INTEGER,
    div_type TEXT NOT NULL DEFAULT '',
    needs_roc_research INTEGER NOT NULL DEFAULT 0,
    notes TEXT NOT NULL DEFAULT ''
);
