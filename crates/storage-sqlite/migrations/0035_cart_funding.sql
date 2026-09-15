-- Opening wizard: how the swap is funded. kind stays Swap.

ALTER TABLE cart_scenario ADD COLUMN funding_source TEXT NOT NULL DEFAULT 'sellLots';
