-- TR-LP-14 style retrieve run ledger. Full candidate payload for admin review.
CREATE TABLE retrieve_run (
    run_id TEXT PRIMARY KEY,
    security_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    requested_at TEXT NOT NULL,
    ok INTEGER NOT NULL,
    code TEXT NOT NULL DEFAULT '',
    message TEXT NOT NULL DEFAULT '',
    attempted INTEGER NOT NULL DEFAULT 0,
    recorded INTEGER NOT NULL DEFAULT 0,
    skipped INTEGER NOT NULL DEFAULT 0,
    unchanged INTEGER NOT NULL DEFAULT 0,
    payload_json TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX retrieve_run_security_requested
    ON retrieve_run (security_id, requested_at DESC);

CREATE INDEX retrieve_run_requested
    ON retrieve_run (requested_at DESC);
