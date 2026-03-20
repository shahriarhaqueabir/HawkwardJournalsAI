-- D-97: wrap all DDL in an explicit transaction.
BEGIN;

-- 1. Create Projects Table (D-13 upgrade to Entity)
CREATE TABLE IF NOT EXISTS projects (
  id          TEXT PRIMARY KEY,
  name        TEXT NOT NULL UNIQUE,
  description TEXT,
  status      TEXT NOT NULL DEFAULT 'active'
              CHECK(status IN ('active', 'on_hold', 'completed', 'cancelled', 'idea')),
  color       TEXT,
  goal_date   TEXT,
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL,
  is_deleted  INTEGER DEFAULT 0
);

-- 2. Add Project ID to Tasks (FK)
-- The initial schema (001) already has a 'project' text field; we'll add the FK column.
ALTER TABLE tasks ADD COLUMN project_id TEXT REFERENCES projects(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_tasks_project_id ON tasks(project_id);

-- 3. Seed Default Inbox Project
INSERT OR IGNORE INTO projects (id, name, description, status, color, created_at, updated_at)
VALUES ('inbox', 'Inbox', 'Default capture bucket', 'active', '#6c8ef7', STRFTIME('%Y-%m-%dT%H:%M:%SZ', 'now'), STRFTIME('%Y-%m-%dT%H:%M:%SZ', 'now'));

COMMIT;
