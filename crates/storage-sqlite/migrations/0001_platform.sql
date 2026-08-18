CREATE TABLE device (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    device_id TEXT NOT NULL,
    device_name TEXT NOT NULL,
    database_id TEXT NOT NULL
);

CREATE TABLE snapshot_head (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    snapshot_id TEXT NOT NULL,
    parent_snapshot_id TEXT,
    change_sequence INTEGER NOT NULL,
    last_event_at TEXT NOT NULL,
    schema_version TEXT NOT NULL,
    calculation_version TEXT NOT NULL,
    app_version TEXT NOT NULL,
    database_hash TEXT NOT NULL,
    evidence_manifest_hash TEXT NOT NULL,
    validation_status TEXT NOT NULL,
    restore_test_status TEXT NOT NULL
);

CREATE TABLE handoff_review (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    acknowledged INTEGER NOT NULL DEFAULT 0,
    published_snapshot_id TEXT
);
