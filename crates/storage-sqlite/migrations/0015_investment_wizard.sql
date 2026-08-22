-- New Investment / Last Price / declaration lookback (schema 15).
-- Price and Plan are not stored on Position or Lot.

CREATE TABLE price_quote (
    price_quote_id TEXT PRIMARY KEY,
    security_id TEXT NOT NULL,
    price_minor INTEGER NOT NULL,
    scale INTEGER NOT NULL,
    currency TEXT NOT NULL DEFAULT 'USD',
    quote_type TEXT NOT NULL DEFAULT 'last',
    as_of_at TEXT NOT NULL,
    retrieved_at TEXT NOT NULL,
    source TEXT NOT NULL,
    validation_status TEXT NOT NULL
);

CREATE INDEX price_quote_security_as_of ON price_quote (security_id, as_of_at DESC);

CREATE TABLE manual_price_override (
    override_id TEXT PRIMARY KEY,
    security_id TEXT NOT NULL,
    price_minor INTEGER NOT NULL,
    scale INTEGER NOT NULL,
    currency TEXT NOT NULL DEFAULT 'USD',
    reason TEXT NOT NULL,
    effective_from TEXT NOT NULL,
    expires_at TEXT,
    status TEXT NOT NULL
);

CREATE TABLE issuer_declaration (
    declaration_id TEXT PRIMARY KEY,
    security_id TEXT NOT NULL,
    amount_per_share_minor INTEGER,
    amount_scale INTEGER NOT NULL,
    payment_period TEXT NOT NULL,
    source TEXT NOT NULL,
    entered_at TEXT NOT NULL,
    superseded_by TEXT
);

CREATE INDEX issuer_declaration_security ON issuer_declaration (security_id, payment_period);

CREATE TABLE retrieval_template (
    security_id TEXT PRIMARY KEY,
    price_source TEXT NOT NULL DEFAULT '',
    source_symbol TEXT NOT NULL DEFAULT '',
    declaration_source TEXT NOT NULL DEFAULT '',
    lookback_count INTEGER NOT NULL DEFAULT 12,
    payment_source TEXT NOT NULL DEFAULT ''
);
