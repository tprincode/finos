-- Last successful issuer page body hash. Unchanged GET skips re-parse and extra exceptions.

ALTER TABLE retrieval_template ADD COLUMN last_content_hash TEXT NOT NULL DEFAULT '';
