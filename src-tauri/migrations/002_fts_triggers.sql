-- D-100: FTS5 Virtual Table and Sync Triggers
BEGIN;

CREATE VIRTUAL TABLE IF NOT EXISTS journal_fts USING fts5(
    id UNINDEXED,
    content,
    tokenize='unicode61'
);

-- Trigger: Insert into FTS on new entry
CREATE TRIGGER IF NOT EXISTS tr_journal_insert AFTER INSERT ON journal_entries
BEGIN
    INSERT INTO journal_fts (id, content) VALUES (new.id, new.content);
END;

-- Trigger: Update FTS on content change
CREATE TRIGGER IF NOT EXISTS tr_journal_update AFTER UPDATE OF content ON journal_entries
BEGIN
    UPDATE journal_fts SET content = new.content WHERE id = old.id;
END;

-- Trigger: Remove from FTS on soft-delete (D-100)
CREATE TRIGGER IF NOT EXISTS tr_journal_soft_delete AFTER UPDATE OF is_deleted ON journal_entries
WHEN new.is_deleted = 1
BEGIN
    DELETE FROM journal_fts WHERE id = old.id;
END;

-- Trigger: Restore to FTS on un-delete (D-100)
CREATE TRIGGER IF NOT EXISTS tr_journal_restore AFTER UPDATE OF is_deleted ON journal_entries
WHEN new.is_deleted = 0
BEGIN
    INSERT INTO journal_fts (id, content) VALUES (new.id, new.content);
END;

-- Trigger: Permanent delete
CREATE TRIGGER IF NOT EXISTS tr_journal_delete AFTER DELETE ON journal_entries
BEGIN
    DELETE FROM journal_fts WHERE id = old.id;
END;

COMMIT;