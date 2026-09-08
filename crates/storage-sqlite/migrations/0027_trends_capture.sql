-- Trends week close + Sat period_start for capture workspace (TR-AC-01 / T6).
ALTER TABLE trends_week_source ADD COLUMN period_start TEXT NOT NULL DEFAULT '';
ALTER TABLE trends_week_source ADD COLUMN closed INTEGER NOT NULL DEFAULT 0;

-- Effective-dated ACA poverty threshold config (T5); no hardcoded 84600 in UI.
CREATE TABLE IF NOT EXISTS aca_threshold_rule (
    rule_id TEXT PRIMARY KEY,
    coverage_year INTEGER NOT NULL,
    household_size INTEGER NOT NULL,
    location_code TEXT NOT NULL,
    threshold_minor INTEGER NOT NULL,
    scale INTEGER NOT NULL,
    UNIQUE(coverage_year, household_size, location_code)
);

INSERT OR IGNORE INTO aca_threshold_rule (rule_id, coverage_year, household_size, location_code, threshold_minor, scale)
VALUES ('aca-2026-2-contiguous', 2026, 2, 'US-contiguous', 8460000, 2);
