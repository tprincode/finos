-- Advisory analysis runs (ADR-0012). Never stores API keys. Does not post ledger facts.

CREATE TABLE analysis_run (
    run_id TEXT PRIMARY KEY,
    prompt TEXT NOT NULL,
    recommendation TEXT NOT NULL,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    status TEXT NOT NULL
);
