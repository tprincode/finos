-- Distribution characterization (ROC / 1099 category). Does not post ledger cash.

CREATE TABLE distribution_characterization (
    characterization_id TEXT PRIMARY KEY,
    activity_id TEXT NOT NULL,
    category TEXT NOT NULL,
    amount_minor INTEGER NOT NULL,
    scale INTEGER NOT NULL
);
