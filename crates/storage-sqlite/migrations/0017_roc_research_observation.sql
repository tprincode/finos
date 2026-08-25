-- Current-year 19a-1 / owner-override ROC research provenance (schema 17).
-- System rows stay when the owner overrides the working percent.

CREATE TABLE roc_research_observation (
    observation_id TEXT PRIMARY KEY,
    security_id TEXT NOT NULL,
    roc_pct_minor INTEGER,
    scale INTEGER NOT NULL DEFAULT 2,
    tax_year TEXT NOT NULL,
    source TEXT NOT NULL,
    source_url TEXT NOT NULL DEFAULT '',
    method TEXT NOT NULL DEFAULT '',
    as_of TEXT NOT NULL DEFAULT '',
    kind TEXT NOT NULL DEFAULT 'estimate',
    established_how TEXT NOT NULL DEFAULT '',
    owner_override INTEGER NOT NULL DEFAULT 0,
    recorded_at TEXT NOT NULL
);

CREATE INDEX roc_research_observation_security
    ON roc_research_observation (security_id, recorded_at, observation_id);
