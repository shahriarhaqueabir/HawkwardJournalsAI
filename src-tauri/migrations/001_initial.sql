-- D-97: wrap all DDL in an explicit transaction.
BEGIN;

CREATE TABLE IF NOT EXISTS schema_migrations (
  version     INTEGER PRIMARY KEY,
  name        TEXT NOT NULL,
  applied_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS notebooks (
  id          TEXT PRIMARY KEY,
  name        TEXT NOT NULL UNIQUE,
  description TEXT,
  color       TEXT,
  sort_order  INTEGER DEFAULT 0,
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL,
  is_deleted  INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS journal_entries (
  id                    TEXT PRIMARY KEY,
  notebook_id           TEXT REFERENCES notebooks(id) ON DELETE SET NULL,
  title                 TEXT,
  content               TEXT NOT NULL DEFAULT '',
  emotions              TEXT NOT NULL DEFAULT '[]',
  tags                  TEXT NOT NULL DEFAULT '[]',
  word_count            INTEGER DEFAULT 0,
  last_analysis_conv_id TEXT,
  last_analysed_at      TEXT,
  created_at            TEXT NOT NULL,
  updated_at            TEXT NOT NULL,
  is_deleted            INTEGER DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_journal_notebook ON journal_entries(notebook_id);
CREATE INDEX IF NOT EXISTS idx_journal_created  ON journal_entries(created_at);
CREATE INDEX IF NOT EXISTS idx_journal_deleted  ON journal_entries(is_deleted);

CREATE TABLE IF NOT EXISTS tasks (
  id                 TEXT PRIMARY KEY,
  parent_task_id     TEXT REFERENCES tasks(id) ON DELETE CASCADE,
  title              TEXT NOT NULL,
  description        TEXT,
  status             TEXT NOT NULL DEFAULT 'todo'
                     CHECK(status IN ('todo','in_progress','done','cancelled')),
  priority           TEXT NOT NULL DEFAULT 'medium'
                     CHECK(priority IN ('low','medium','high','urgent')),
  due_date           TEXT,
  due_time           TEXT,
  reminder_at        TEXT,
  reminder_fired     INTEGER DEFAULT 0,
  time_estimate      INTEGER,
  time_logged        INTEGER DEFAULT 0,
  actual_start_date  TEXT,
  tags               TEXT NOT NULL DEFAULT '[]',
  labels             TEXT NOT NULL DEFAULT '[]',
  category           TEXT,
  project            TEXT,
  notes              TEXT,
  recurrence         TEXT,
  next_occurrence    TEXT,
  energy_level       TEXT CHECK(energy_level IN ('deep_focus','light','admin','errand') OR energy_level IS NULL),
  context_tag        TEXT CHECK(context_tag IN ('computer','phone','errands','home','anywhere') OR context_tag IS NULL),
  linked_url         TEXT,
  sort_order         INTEGER DEFAULT 0,
  ai_created         INTEGER DEFAULT 0,
  ai_conversation_id TEXT,
  created_at         TEXT NOT NULL,
  updated_at         TEXT NOT NULL,
  completed_at       TEXT,
  is_deleted         INTEGER DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_tasks_status     ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_due_date   ON tasks(due_date);
CREATE INDEX IF NOT EXISTS idx_tasks_priority   ON tasks(priority);
CREATE INDEX IF NOT EXISTS idx_tasks_project    ON tasks(project);
CREATE INDEX IF NOT EXISTS idx_tasks_parent     ON tasks(parent_task_id);
CREATE INDEX IF NOT EXISTS idx_tasks_deleted    ON tasks(is_deleted);

CREATE TABLE IF NOT EXISTS task_dependencies (
  id               TEXT PRIMARY KEY,
  blocked_task_id  TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  blocking_task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  created_at       TEXT NOT NULL,
  UNIQUE(blocked_task_id, blocking_task_id),
  CHECK(blocked_task_id != blocking_task_id)
);

CREATE TABLE IF NOT EXISTS task_attachments (
  id           TEXT PRIMARY KEY,
  task_id      TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  file_name    TEXT NOT NULL,
  file_path    TEXT NOT NULL,
  mime_type    TEXT,
  size_bytes   INTEGER,
  file_missing INTEGER DEFAULT 0,
  created_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS time_logs (
  id          TEXT PRIMARY KEY,
  task_id     TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  started_at  TEXT NOT NULL,
  ended_at    TEXT,
  duration    INTEGER,
  note        TEXT,
  created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS ai_conversations (
  id              TEXT PRIMARY KEY,
  title           TEXT,
  model           TEXT NOT NULL,
  source          TEXT NOT NULL DEFAULT 'sidebar'
                  CHECK(source IN ('sidebar','ai_tab','analysis','weekly_plan')),
  source_entry_id TEXT REFERENCES journal_entries(id) ON DELETE SET NULL,
  created_at      TEXT NOT NULL,
  updated_at      TEXT NOT NULL,
  is_deleted      INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS ai_messages (
  id              TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL REFERENCES ai_conversations(id) ON DELETE CASCADE,
  role            TEXT NOT NULL CHECK(role IN ('user','assistant','tool','system')),
  content         TEXT NOT NULL,
  tool_name       TEXT,
  tool_args       TEXT,
  tool_result     TEXT,
  confirmed       INTEGER,
  model           TEXT,
  tokens_in       INTEGER,
  tokens_out      INTEGER,
  created_at      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS app_settings (
  key        TEXT PRIMARY KEY,
  value      TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS audit_log (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  action      TEXT NOT NULL CHECK(action IN ('create','update','delete','restore')),
  entity      TEXT NOT NULL,
  entity_id   TEXT NOT NULL,
  actor       TEXT NOT NULL CHECK(actor IN ('user','ai')),
  changes     TEXT,
  ai_conv_id  TEXT,
  created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS proposed_task_log (
  id               TEXT PRIMARY KEY,
  conversation_id  TEXT NOT NULL REFERENCES ai_conversations(id),
  source_entry_id  TEXT REFERENCES journal_entries(id),
  proposed_title   TEXT NOT NULL,
  proposed_data    TEXT NOT NULL,
  source_text      TEXT,
  outcome          TEXT NOT NULL CHECK(outcome IN ('accepted','edited','dismissed')),
  accepted_task_id TEXT REFERENCES tasks(id),
  created_at       TEXT NOT NULL
);

COMMIT;
