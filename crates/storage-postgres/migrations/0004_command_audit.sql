-- Central command identity audit (AC-ARCH-10). Postgres-only; SQLite/desktop unchanged.

CREATE TABLE command_audit (
    audit_id TEXT PRIMARY KEY,
    user_sub TEXT NOT NULL,
    device_id TEXT NOT NULL,
    correlation_id TEXT NOT NULL,
    command_name TEXT NOT NULL,
    ok INTEGER NOT NULL,
    error_code TEXT,
    recorded_at TEXT NOT NULL
);
