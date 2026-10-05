-- Equity option contracts (OCC). Money is integer cents (scale 2). No floats.

CREATE TABLE option_contract (
    contract_id TEXT PRIMARY KEY,
    occ_symbol TEXT NOT NULL,
    underlying TEXT NOT NULL,
    expiry_on TEXT NOT NULL,
    put_call TEXT NOT NULL,
    strike_minor INTEGER NOT NULL,
    scale INTEGER NOT NULL DEFAULT 2,
    account_id TEXT,
    side TEXT NOT NULL,
    quantity INTEGER NOT NULL,
    open_premium_minor INTEGER NOT NULL,
    open_on TEXT NOT NULL,
    underlying_last_minor INTEGER,
    option_mid_minor INTEGER,
    quote_as_of TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL,
    roll_to_contract_id TEXT NOT NULL DEFAULT '',
    close_premium_minor INTEGER,
    closed_on TEXT NOT NULL DEFAULT '',
    payload_json TEXT NOT NULL DEFAULT '{}',
    created_on TEXT NOT NULL,
    updated_on TEXT NOT NULL
);

CREATE INDEX option_contract_open_expiry
    ON option_contract (expiry_on)
    WHERE status = 'open';
