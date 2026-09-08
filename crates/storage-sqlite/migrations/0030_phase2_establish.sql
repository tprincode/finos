ALTER TABLE retrieval_template ADD COLUMN roc_source_url TEXT NOT NULL DEFAULT '';
ALTER TABLE retrieval_template ADD COLUMN history_url_attempts INTEGER NOT NULL DEFAULT 0;

CREATE TABLE collector_field_decision (
    security_id TEXT NOT NULL,
    field TEXT NOT NULL,
    decision TEXT NOT NULL,
    noted_on TEXT NOT NULL,
    PRIMARY KEY (security_id, field)
);
