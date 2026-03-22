-- Migration 009: Fix proposed_task_log outcome constraint
-- Decision: D-132 (AI Proposal Status tracking)
BEGIN;

-- SQLite doesn't support ALTER TABLE DROP CONSTRAINT or ALTER COLUMN.
-- We must recreate the table to update the CHECK constraint.

CREATE TABLE proposed_task_log_new (
  id               TEXT PRIMARY KEY,
  conversation_id  TEXT NOT NULL REFERENCES ai_conversations(id),
  source_entry_id  TEXT REFERENCES journal_entries(id),
  outcome          TEXT NOT NULL CHECK(outcome IN ('proposed','accepted','edited','dismissed')),
  proposed_title   TEXT NOT NULL,
  proposed_data    TEXT NOT NULL,
  source_text      TEXT,
  accepted_task_id TEXT REFERENCES tasks(id),
  created_at       TEXT NOT NULL
);

-- Copy existing data, mapping any old data safely if needed
-- Actually we can just copy it directly, but we need to make sure 'proposed' is allowed.
-- Since the old constraint blocked 'proposed', any existing data must be accepted/edited/dismissed.
INSERT INTO proposed_task_log_new (id, conversation_id, source_entry_id, outcome, proposed_title, proposed_data, source_text, accepted_task_id, created_at)
SELECT id, conversation_id, source_entry_id, outcome, proposed_title, proposed_data, source_text, accepted_task_id, created_at
FROM proposed_task_log;

DROP TABLE proposed_task_log;
ALTER TABLE proposed_task_log_new RENAME TO proposed_task_log;

COMMIT;
