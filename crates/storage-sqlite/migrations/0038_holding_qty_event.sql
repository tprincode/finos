-- Dated remaining qty for chart reconstruction (sales / reverse splits).
-- Does not change lot remaining. Days before the first event stay 0.
CREATE TABLE holding_qty_event (
    event_id TEXT PRIMARY KEY,
    security_id TEXT NOT NULL,
    occurred_on TEXT NOT NULL,
    remaining_quantity_minor INTEGER NOT NULL,
    quantity_scale INTEGER NOT NULL,
    kind TEXT NOT NULL,
    UNIQUE(security_id, occurred_on, kind)
);
