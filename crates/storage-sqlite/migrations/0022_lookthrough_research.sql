-- Owner-confirmed look-through research for risk profiling (not PositionExposure).
-- Empty JSON is unknown. Holdings and sector weights stay unknown until an issuer top table is parsed.
-- Risk tier suggestion in this blob never auto-applies classification (TR-PD-25).

ALTER TABLE position_characteristic ADD COLUMN lookthrough_json TEXT NOT NULL DEFAULT '{}';
