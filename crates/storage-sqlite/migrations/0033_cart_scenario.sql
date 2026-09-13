-- Decision-support swap cart. Drafts do not post lots or MAGI.

CREATE TABLE cart_scenario (
    scenario_id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    account_name TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT 'Swap',
    status TEXT NOT NULL DEFAULT 'draft',
    as_of TEXT NOT NULL,
    cash_yield_bps INTEGER NOT NULL DEFAULT 0,
    override_reason TEXT,
    created_at TEXT NOT NULL,
    agreed_at TEXT
);

CREATE TABLE cart_sell_line (
    line_id TEXT PRIMARY KEY,
    scenario_id TEXT NOT NULL,
    lot_id TEXT NOT NULL,
    security_id TEXT,
    symbol TEXT NOT NULL,
    qty_minor INTEGER NOT NULL,
    qty_scale INTEGER NOT NULL DEFAULT 2,
    unit_minor INTEGER NOT NULL,
    proceeds_minor INTEGER NOT NULL,
    is_cash INTEGER NOT NULL DEFAULT 0,
    original_cost_minor INTEGER,
    FOREIGN KEY (scenario_id) REFERENCES cart_scenario(scenario_id)
);

CREATE TABLE cart_buy_line (
    line_id TEXT PRIMARY KEY,
    scenario_id TEXT NOT NULL,
    security_id TEXT NOT NULL,
    symbol TEXT NOT NULL,
    qty_whole INTEGER NOT NULL,
    last_minor INTEGER NOT NULL,
    spend_minor INTEGER NOT NULL,
    plan_annual_minor INTEGER,
    FOREIGN KEY (scenario_id) REFERENCES cart_scenario(scenario_id)
);

CREATE TABLE cart_eval_snapshot (
    scenario_id TEXT PRIMARY KEY,
    remaining_minor INTEGER NOT NULL,
    spend_minor INTEGER NOT NULL,
    leftover_minor INTEGER NOT NULL,
    buy_annual_minor INTEGER,
    surrendered_annual_minor INTEGER,
    leftover_annual_minor INTEGER,
    net_annual_minor INTEGER,
    net_monthly_minor INTEGER,
    net_weekly_minor INTEGER,
    insufficient_lot_qty INTEGER NOT NULL DEFAULT 0,
    cash_floor_warn INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (scenario_id) REFERENCES cart_scenario(scenario_id)
);

CREATE TABLE cart_execute_step (
    step_id TEXT PRIMARY KEY,
    scenario_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    activity_id TEXT,
    assignment_id TEXT,
    lot_id TEXT,
    FOREIGN KEY (scenario_id) REFERENCES cart_scenario(scenario_id)
);
