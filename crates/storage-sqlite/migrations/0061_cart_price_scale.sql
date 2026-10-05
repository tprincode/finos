-- Per-share prices at scale 4 (1/10000 USD); proceeds/spend stay USD cents (scale 2).

ALTER TABLE cart_sell_line ADD COLUMN unit_scale INTEGER NOT NULL DEFAULT 2;
ALTER TABLE cart_buy_line ADD COLUMN price_scale INTEGER NOT NULL DEFAULT 2;
