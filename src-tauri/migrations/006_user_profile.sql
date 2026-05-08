BEGIN;

CREATE TABLE IF NOT EXISTS user_profile (
    id                TEXT PRIMARY KEY,
    fact_key          TEXT UNIQUE NOT NULL,
    content           TEXT NOT NULL,
    category          TEXT NOT NULL CHECK(category IN ('preference', 'habit', 'constraint', 'person', 'goal', 'other')),
    confidence        REAL DEFAULT 1.0,
    source_entry_id   TEXT REFERENCES journal_entries(id) ON DELETE SET NULL,
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_user_profile_category ON user_profile(category);

COMMIT;
