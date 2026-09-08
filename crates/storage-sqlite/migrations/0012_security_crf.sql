-- Security master CRF flag (zero-cost DRIP is security + origin, not account-kind).

ALTER TABLE security ADD COLUMN crf INTEGER NOT NULL DEFAULT 0;
