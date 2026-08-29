-- Distribution inception (ISO date). Empty means full 12-declaration lookback is required.
ALTER TABLE retrieval_template ADD COLUMN inception_on TEXT NOT NULL DEFAULT '';
