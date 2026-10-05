-- A cart cannot be open and closed at the same time.
--
-- Executing one slot of a plan never closed the other slot, so the comparison copy stayed a
-- draft for ever. The owner saw "Income - 9/26/26" in Open carts and in Executed carts at the
-- same time, and opening either showed the identical screen: cart_sell_line is keyed by
-- plan_id, so the leftover draft reported the sale that had already executed.
--
-- application-core now supersedes sibling slots at execution time. This heals the rows that
-- were already written. Superseded rather than deleted: an executed plan's other slot is
-- history the owner may want to read, and discard would drop its buy lines with it.

UPDATE cart_scenario
SET status = 'superseded'
WHERE status NOT IN ('complete', 'superseded')
  AND plan_id IN (
      SELECT plan_id FROM cart_scenario WHERE status = 'complete'
  );
