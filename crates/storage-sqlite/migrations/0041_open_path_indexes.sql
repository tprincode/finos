-- Open-path indexes and unique keys. Dedupe before unique so household DBs migrate.
-- Ledger FKs stay application-enforced (seed rollup rows are not always account UUIDs).

DELETE FROM planned_occurrence
WHERE occurrence_id NOT IN (
    SELECT occurrence_id FROM (
        SELECT occurrence_id,
               ROW_NUMBER() OVER (
                   PARTITION BY element_id, occurred_on
                   ORDER BY CASE
                       WHEN confirmed_at IS NOT NULL AND confirmed_at != '' THEN 0
                       ELSE 1
                   END,
                   occurrence_id
               ) AS rn
        FROM planned_occurrence
    ) ranked
    WHERE rn = 1
);

DELETE FROM price_quote
WHERE price_quote_id NOT IN (
    SELECT price_quote_id FROM (
        SELECT price_quote_id,
               ROW_NUMBER() OVER (
                   PARTITION BY security_id, as_of_at, source
                   ORDER BY retrieved_at DESC, price_quote_id
               ) AS rn
        FROM price_quote
    ) ranked
    WHERE rn = 1
);

CREATE INDEX IF NOT EXISTS idx_activity_event_account_on
    ON activity_event (account_id, occurred_on);
CREATE INDEX IF NOT EXISTS idx_activity_event_security_on
    ON activity_event (security_id, occurred_on);
CREATE INDEX IF NOT EXISTS idx_activity_event_corrects
    ON activity_event (corrects_activity_id);

CREATE INDEX IF NOT EXISTS idx_dividend_actual_account_on
    ON dividend_actual (account_id, occurred_on);
CREATE INDEX IF NOT EXISTS idx_dividend_actual_security_on
    ON dividend_actual (security_id, occurred_on);

CREATE INDEX IF NOT EXISTS idx_lot_account_security
    ON lot (account_id, security_id);
CREATE INDEX IF NOT EXISTS idx_lot_opened_on
    ON lot (opened_on);

CREATE INDEX IF NOT EXISTS idx_planned_occurrence_account_on
    ON planned_occurrence (account, occurred_on);
CREATE UNIQUE INDEX IF NOT EXISTS planned_occurrence_element_on
    ON planned_occurrence (element_id, occurred_on);

CREATE INDEX IF NOT EXISTS idx_plan_history_security_from
    ON plan_history (security_id, effective_from);

CREATE INDEX IF NOT EXISTS idx_work_ticket_security_status_seen
    ON work_ticket (security_id, status, last_seen_on);

CREATE UNIQUE INDEX IF NOT EXISTS price_quote_security_as_of_source
    ON price_quote (security_id, as_of_at, source);

-- CHECKs via triggers: SQLite cannot ADD CHECK to existing tables.
CREATE TRIGGER IF NOT EXISTS trg_activity_event_scale_type_ins
BEFORE INSERT ON activity_event
BEGIN
    SELECT RAISE(ABORT, 'activity_event scale/type')
    WHERE NEW.scale < 0 OR NEW.scale > 8 OR TRIM(NEW.activity_type) = '';
END;
CREATE TRIGGER IF NOT EXISTS trg_activity_event_scale_type_upd
BEFORE UPDATE ON activity_event
BEGIN
    SELECT RAISE(ABORT, 'activity_event scale/type')
    WHERE NEW.scale < 0 OR NEW.scale > 8 OR TRIM(NEW.activity_type) = '';
END;
