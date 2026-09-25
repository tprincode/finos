-- One named plan owns the sell lines. Slot A and slot B buy scenarios share them.

CREATE TABLE cart_plan (
    plan_id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    account_name TEXT NOT NULL,
    name TEXT NOT NULL,
    as_of TEXT NOT NULL,
    cash_yield_bps INTEGER NOT NULL DEFAULT 0,
    deposit_minor INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

ALTER TABLE cart_scenario ADD COLUMN plan_id TEXT;
ALTER TABLE cart_scenario ADD COLUMN slot TEXT NOT NULL DEFAULT 'A';

INSERT INTO cart_plan (
    plan_id, account_id, account_name, name, as_of, cash_yield_bps, deposit_minor, created_at)
SELECT scenario_id, account_id, account_name, name, as_of, cash_yield_bps, 0, created_at
FROM cart_scenario;

UPDATE cart_scenario SET plan_id = scenario_id WHERE plan_id IS NULL;

CREATE TABLE cart_sell_line_v2 (
    line_id TEXT PRIMARY KEY,
    plan_id TEXT NOT NULL,
    lot_id TEXT NOT NULL,
    security_id TEXT,
    symbol TEXT NOT NULL,
    qty_minor INTEGER NOT NULL,
    qty_scale INTEGER NOT NULL DEFAULT 2,
    unit_minor INTEGER NOT NULL,
    proceeds_minor INTEGER NOT NULL,
    is_cash INTEGER NOT NULL DEFAULT 0,
    original_cost_minor INTEGER,
    performance_cost_minor INTEGER,
    tax_cost_minor INTEGER,
    performance_gain_minor INTEGER,
    tax_gain_minor INTEGER,
    FOREIGN KEY (plan_id) REFERENCES cart_plan(plan_id)
);

INSERT INTO cart_sell_line_v2 (
    line_id, plan_id, lot_id, security_id, symbol, qty_minor, qty_scale, unit_minor,
    proceeds_minor, is_cash, original_cost_minor, performance_cost_minor, tax_cost_minor,
    performance_gain_minor, tax_gain_minor)
SELECT
    line_id, scenario_id, lot_id, security_id, symbol, qty_minor, qty_scale, unit_minor,
    proceeds_minor, is_cash, original_cost_minor, performance_cost_minor, tax_cost_minor,
    performance_gain_minor, tax_gain_minor
FROM cart_sell_line;

DROP TABLE cart_sell_line;
ALTER TABLE cart_sell_line_v2 RENAME TO cart_sell_line;
