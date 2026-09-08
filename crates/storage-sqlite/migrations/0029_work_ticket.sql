CREATE TABLE work_ticket (
    ticket_id TEXT PRIMARY KEY,
    security_id TEXT NOT NULL,
    symbol TEXT NOT NULL,
    field TEXT NOT NULL,
    code TEXT NOT NULL,
    tool TEXT NOT NULL,
    reason TEXT NOT NULL,
    urls_tried TEXT NOT NULL DEFAULT '[]',
    opened_on TEXT NOT NULL,
    last_seen_on TEXT NOT NULL,
    status TEXT NOT NULL,
    filed_on TEXT NOT NULL DEFAULT '',
    completed_how TEXT NOT NULL DEFAULT '',
    owner_note TEXT NOT NULL DEFAULT '',
    retrieve_run_id TEXT NOT NULL DEFAULT ''
);

CREATE UNIQUE INDEX work_ticket_open_code
    ON work_ticket (security_id, code)
    WHERE status = 'open';
