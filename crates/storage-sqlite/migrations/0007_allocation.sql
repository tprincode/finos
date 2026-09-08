-- Decision-support allocation targets. Does not post ledger or MAGI facts.
-- target_minor scale 2: 6000 = 60.00 percent.

CREATE TABLE allocation_target (
    target_id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    target_minor INTEGER NOT NULL,
    scale INTEGER NOT NULL DEFAULT 2
);
