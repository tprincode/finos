-- Security master CRF flag. Mirrors SQLite 0012; not a centralization slice.

ALTER TABLE security ADD COLUMN crf INTEGER NOT NULL DEFAULT 0;
