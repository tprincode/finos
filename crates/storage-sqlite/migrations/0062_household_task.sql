-- Household tasks (MAGI / budget / options / manual). Not collector tickets.

CREATE TABLE task_rule (
    rule_id TEXT PRIMARY KEY,
    code TEXT UNIQUE NOT NULL,
    title TEXT NOT NULL,
    enabled INTEGER NOT NULL,
    cadence TEXT NOT NULL,
    domain TEXT NOT NULL,
    owner_note TEXT NOT NULL DEFAULT ''
);

CREATE TABLE task (
    task_id TEXT PRIMARY KEY,
    rule_id TEXT,
    code TEXT NOT NULL,
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    domain TEXT NOT NULL,
    week_start TEXT NOT NULL,
    due_on TEXT NOT NULL,
    ignore_until TEXT NOT NULL DEFAULT '',
    payload_json TEXT NOT NULL DEFAULT '{}',
    created_on TEXT NOT NULL,
    resolved_on TEXT NOT NULL DEFAULT ''
);

CREATE UNIQUE INDEX task_open_code_week
    ON task (code, week_start)
    WHERE status = 'open';

INSERT INTO task_rule (rule_id, code, title, enabled, cadence, domain, owner_note)
VALUES (
    'rule-magi-cliff-over',
    'magi_cliff_over',
    'Resolve MAGI cliff gap',
    1,
    'weekly',
    'magi',
    ''
);
