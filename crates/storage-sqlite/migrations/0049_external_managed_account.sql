-- Managed external accounts. Charges stay in external_register_line.
-- paid_through is the month the balance is paid up to.
-- A finished charge reduces the balance once. Open charges stay pending.

CREATE TABLE external_managed_account (
    account_id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL,
    charges_interest INTEGER NOT NULL DEFAULT 0,
    starting_minor INTEGER,
    current_minor INTEGER,
    payment_minor INTEGER,
    reduction_minor INTEGER,
    finance_minor INTEGER,
    paid_through TEXT,
    register_key TEXT NOT NULL,
    legacy_vendor TEXT,
    legacy_amount_minor INTEGER,
    sort_order INTEGER NOT NULL
);

CREATE TABLE external_managed_applied (
    account_id TEXT NOT NULL,
    line_id TEXT NOT NULL,
    PRIMARY KEY (account_id, line_id)
);

INSERT INTO external_managed_account (
    account_id, name, kind, charges_interest,
    starting_minor, current_minor, payment_minor, reduction_minor, finance_minor,
    paid_through, register_key, legacy_vendor, legacy_amount_minor, sort_order
) VALUES
    ('a1000001-0000-4000-8000-000000000001', 'Newrez', 'debt', 1, 30000000, 29848900, 179672, 30372, 149300, 'August', 'Newrez', NULL, NULL, 1),
    ('a1000001-0000-4000-8000-000000000002', 'Truist BANK', 'debt', 1, 4201100, 839000, 57499, 54474, 3559, 'August', 'Truist BANK', NULL, NULL, 2),
    ('a1000001-0000-4000-8000-000000000003', 'myClearbalance', 'debt', 0, 733300, 508600, 20300, 20300, NULL, 'September', 'myClearbalance', NULL, NULL, 3),
    ('a1000001-0000-4000-8000-000000000004', 'Paytient', 'debt', 0, NULL, 72820, 19265, 18098, NULL, 'August', 'Paytient', NULL, NULL, 4),
    ('a1000001-0000-4000-8000-000000000005', 'UVA Health 3/22-3/25', 'debt', 0, 354939, 318039, 12300, 12300, NULL, 'September', 'UVA Health 3/22-3/25', NULL, NULL, 5),
    ('a1000001-0000-4000-8000-000000000006', 'Alphaeon Cat', 'debt', 0, 565000, 565000, 23600, 23600, NULL, 'August', 'Alphaeon Cat', 'Alpheon', 23600, 6),
    ('a1000001-0000-4000-8000-000000000007', 'Alphaeon CK', 'debt', 0, 900000, 626400, 25000, 25000, NULL, 'August', 'Alphaeon CK', 'Alpheon', 25000, 7),
    ('a1000001-0000-4000-8000-000000000008', 'Mom shopping', 'credit', 0, NULL, NULL, NULL, NULL, NULL, NULL, 'Mom', NULL, NULL, 8);

-- Completed charges are already in the balances above. The open Paytient charge is not.
INSERT INTO external_managed_applied (account_id, line_id)
SELECT 'a1000001-0000-4000-8000-000000000004', line_id
FROM external_register_line
WHERE completed = 1
  AND lower(trim(vendor)) = 'paytient';

INSERT INTO external_managed_applied (account_id, line_id)
SELECT 'a1000001-0000-4000-8000-000000000006', line_id
FROM external_register_line
WHERE completed = 1
  AND (
      lower(trim(vendor)) = 'alphaeon cat'
      OR lower(trim(category)) = 'alphaeon cat'
      OR (lower(trim(vendor)) IN ('alpheon', 'alphaeon') AND amount_minor = 23600)
  );

INSERT INTO external_managed_applied (account_id, line_id)
SELECT 'a1000001-0000-4000-8000-000000000007', line_id
FROM external_register_line
WHERE completed = 1
  AND (
      lower(trim(vendor)) = 'alphaeon ck'
      OR lower(trim(category)) = 'alphaeon ck'
      OR (lower(trim(vendor)) IN ('alpheon', 'alphaeon') AND amount_minor = 25000)
  );

INSERT INTO external_managed_applied (account_id, line_id)
SELECT 'a1000001-0000-4000-8000-000000000008', line_id
FROM external_register_line
WHERE lower(trim(category)) = 'mom'
  AND (completed = 1 OR step_transfer = 1);
