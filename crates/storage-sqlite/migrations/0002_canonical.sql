CREATE TABLE account (
    account_id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL
);

CREATE TABLE security (
    security_id TEXT PRIMARY KEY,
    symbol TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL
);

CREATE TABLE symbol_alias (
    alias_id TEXT PRIMARY KEY,
    security_id TEXT NOT NULL,
    alias TEXT NOT NULL UNIQUE
);

CREATE TABLE evidence (
    evidence_id TEXT PRIMARY KEY,
    content_hash TEXT NOT NULL UNIQUE,
    filename TEXT NOT NULL
);

CREATE TABLE import_batch (
    batch_id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    evidence_id TEXT NOT NULL,
    status TEXT NOT NULL,
    UNIQUE(source_id, content_hash)
);

CREATE TABLE import_candidate (
    candidate_id TEXT PRIMARY KEY,
    batch_id TEXT NOT NULL,
    account_name TEXT NOT NULL,
    symbol TEXT,
    activity_type TEXT NOT NULL,
    amount_minor INTEGER,
    scale INTEGER NOT NULL,
    occurred_on TEXT NOT NULL,
    posted_activity_id TEXT
);

CREATE TABLE activity_event (
    activity_id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    security_id TEXT,
    activity_type TEXT NOT NULL,
    amount_minor INTEGER NOT NULL,
    scale INTEGER NOT NULL,
    occurred_on TEXT NOT NULL,
    corrects_activity_id TEXT,
    import_batch_id TEXT,
    idempotency_key TEXT NOT NULL UNIQUE
);

CREATE TABLE audit_record (
    audit_id TEXT PRIMARY KEY,
    action TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    recorded_at TEXT NOT NULL
);

CREATE TABLE app_exception (
    exception_id TEXT PRIMARY KEY,
    code TEXT NOT NULL,
    message TEXT NOT NULL,
    acknowledged INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);
