-- Migration 006: Projects as first-class entities
-- Decision: D-112 (Project Hierarchy Upgrade)
BEGIN;

CREATE TABLE IF NOT EXISTS projects (
  id           TEXT PRIMARY KEY,
  name         TEXT NOT NULL UNIQUE,
  description  TEXT,
  status       TEXT NOT NULL DEFAULT 'active' 
               CHECK(status IN ('active', 'completed', 'archived')),
  color        TEXT,
  goal_date    TEXT,
  created_at   TEXT NOT NULL,
  updated_at   TEXT NOT NULL,
  is_deleted   INTEGER DEFAULT 0
);

-- Add project_id to tasks and index it
ALTER TABLE tasks ADD COLUMN project_id TEXT REFERENCES projects(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_tasks_project_id ON tasks(project_id);

-- Seeding a default "Inbox" project for newly captured tasks
INSERT OR IGNORE INTO projects (id, name, description, created_at, updated_at)
VALUES ('inbox', 'Inbox', 'Default home for captured tasks.', datetime('now'), datetime('now'));

-- Back-fill existing tasks into the Inbox if they don't have a project
UPDATE tasks SET project_id = 'inbox' WHERE project_id IS NULL;

COMMIT;
