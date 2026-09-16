-- Week cash recon: optional per-account cash_symbol; activity note for Cash_Adjust reason.
ALTER TABLE account ADD COLUMN cash_symbol TEXT;
ALTER TABLE activity_event ADD COLUMN note TEXT NOT NULL DEFAULT '';
