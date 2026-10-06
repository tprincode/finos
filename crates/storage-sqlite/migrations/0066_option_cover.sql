-- Share cover for a short call, and the week a premium posts.
-- Prior balance uses an empty week_start. Cash covers are not stored.

ALTER TABLE option_contract ADD COLUMN prior_balance_minor INTEGER NOT NULL DEFAULT 0;

CREATE TABLE option_cover_lot (
    cover_id TEXT PRIMARY KEY,
    contract_id TEXT NOT NULL,
    lot_id TEXT NOT NULL,
    quantity_minor INTEGER NOT NULL,
    cost_minor INTEGER NOT NULL,
    scale INTEGER NOT NULL DEFAULT 2
);

CREATE TABLE option_premium_post (
    post_id TEXT PRIMARY KEY,
    contract_id TEXT NOT NULL,
    week_start TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL,
    amount_minor INTEGER NOT NULL,
    scale INTEGER NOT NULL DEFAULT 2,
    reason TEXT NOT NULL
);

CREATE INDEX option_cover_lot_contract ON option_cover_lot (contract_id);
CREATE INDEX option_premium_post_contract ON option_premium_post (contract_id);
