BEGIN;

-- AI Pinned Memory (Long-term profile facts)
CREATE TABLE ai_pinned_memory (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    importance INTEGER NOT NULL DEFAULT 1, -- 1-5
    metadata TEXT, -- JSON blob for category/context
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Search index for conversations (if not already there)
-- Actually, we have ai_messages. Let's make it searchable via FTS if we want deep search.
-- For now, literal search is fine for private local use.

COMMIT;
