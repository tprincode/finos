-- Remove duplicate active issuer declarations (same security + pay period).
-- Keep the newest row per period; enforce one active row going forward.

DELETE FROM issuer_declaration
WHERE superseded_by IS NULL
  AND declaration_id NOT IN (
    SELECT declaration_id
    FROM (
      SELECT declaration_id,
             ROW_NUMBER() OVER (
               PARTITION BY security_id, payment_period
               ORDER BY entered_at DESC, declaration_id DESC
             ) AS rn
      FROM issuer_declaration
      WHERE superseded_by IS NULL
    )
    WHERE rn = 1
  );

CREATE UNIQUE INDEX IF NOT EXISTS issuer_declaration_active_period
  ON issuer_declaration (security_id, payment_period)
  WHERE superseded_by IS NULL;
