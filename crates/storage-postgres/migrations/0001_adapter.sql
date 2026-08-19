-- PostgreSQL types for the same isolated tables as storage-sqlite (ADR-0005).
-- Money and quantities are BIGINT minor units. Identifiers stay TEXT UUIDs.

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

CREATE TABLE activity_event (
    activity_id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    security_id TEXT,
    activity_type TEXT NOT NULL,
    amount_minor BIGINT NOT NULL,
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

CREATE TABLE dividend_declaration (
    declaration_id TEXT PRIMARY KEY,
    security_symbol TEXT NOT NULL,
    declared_on TEXT NOT NULL,
    amount_minor BIGINT NOT NULL,
    scale INTEGER NOT NULL
);

CREATE TABLE dividend_actual (
    actual_id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    security_id TEXT,
    occurred_on TEXT NOT NULL,
    amount_minor BIGINT NOT NULL,
    scale INTEGER NOT NULL,
    activity_id TEXT,
    idempotency_key TEXT NOT NULL UNIQUE
);

CREATE TABLE lot (
    lot_id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    security_id TEXT NOT NULL,
    opened_on TEXT NOT NULL,
    origin TEXT NOT NULL,
    quantity_minor BIGINT NOT NULL,
    remaining_quantity_minor BIGINT NOT NULL,
    quantity_scale INTEGER NOT NULL,
    performance_basis_minor BIGINT NOT NULL,
    tax_basis_minor BIGINT NOT NULL,
    remaining_performance_minor BIGINT NOT NULL,
    remaining_tax_minor BIGINT NOT NULL,
    scale INTEGER NOT NULL,
    crf_zero_cost INTEGER NOT NULL DEFAULT 0,
    opening_activity_id TEXT
);
