BEGIN;

ALTER TABLE journal_entries ADD COLUMN analysis_summary TEXT;
ALTER TABLE journal_entries ADD COLUMN analysis_mood TEXT;
ALTER TABLE journal_entries ADD COLUMN analysis_insights TEXT NOT NULL DEFAULT '[]';

COMMIT;
