-- Decision-support shopping cart. Does not post ledger, lots, or MAGI facts.
-- quantity_minor scale 2: 10000 = 100.00 shares.

CREATE TABLE cart_item (
    item_id TEXT PRIMARY KEY,
    symbol TEXT NOT NULL,
    quantity_minor INTEGER NOT NULL,
    quantity_scale INTEGER NOT NULL DEFAULT 2
);
