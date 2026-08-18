-- Human classification review. Does not post ledger facts or rewrite MAGI oracles.

CREATE TABLE classification_review (
    review_id TEXT PRIMARY KEY,
    fact_key TEXT NOT NULL UNIQUE,
    classification TEXT NOT NULL,
    status TEXT NOT NULL
);
