-- M4 lots / dual basis / assignments.
-- Money scale: 2 (USD cents). Quantity scale: documented per lot (tests use 0 = whole shares).
-- Performance (price-paid) and tax-adjusted basis are separate columns; never mixed.

CREATE TABLE lot (
    lot_id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    security_id TEXT NOT NULL,
    opened_on TEXT NOT NULL,
    origin TEXT NOT NULL,
    quantity_minor INTEGER NOT NULL,
    remaining_quantity_minor INTEGER NOT NULL,
    quantity_scale INTEGER NOT NULL,
    performance_basis_minor INTEGER NOT NULL,
    tax_basis_minor INTEGER NOT NULL,
    remaining_performance_minor INTEGER NOT NULL,
    remaining_tax_minor INTEGER NOT NULL,
    scale INTEGER NOT NULL,
    crf_zero_cost INTEGER NOT NULL DEFAULT 0,
    opening_activity_id TEXT
);

CREATE TABLE lot_assignment (
    assignment_id TEXT PRIMARY KEY,
    lot_id TEXT NOT NULL,
    activity_id TEXT NOT NULL,
    quantity_minor INTEGER NOT NULL,
    quantity_scale INTEGER NOT NULL,
    proceeds_minor INTEGER NOT NULL,
    performance_cost_minor INTEGER NOT NULL,
    tax_cost_minor INTEGER NOT NULL,
    scale INTEGER NOT NULL,
    UNIQUE(lot_id, activity_id)
);
