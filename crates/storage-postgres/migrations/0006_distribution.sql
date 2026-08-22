-- Distribution characterization. Mirrors SQLite 0013; not a centralization slice.

CREATE TABLE distribution_characterization (
    characterization_id TEXT PRIMARY KEY,
    activity_id TEXT NOT NULL,
    category TEXT NOT NULL,
    amount_minor BIGINT NOT NULL,
    scale INTEGER NOT NULL
);
