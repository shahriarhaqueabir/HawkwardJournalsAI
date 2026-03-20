# Changelog

## 2026-03-19

### Task Management & AI Integration

- **Task CRUD Implementation**: Completed backend handlers and frontend UI for task creation, listing, updating, and searching (`src-tauri/src/db/tasks.rs`, `src-tauri/src/lib.rs`, `src/js/tabs/tasks.js`).
- **Task Detail Panel**: Implemented a slide-in detail panel for editing task properties including priority, status, due date, and energy levels (`src/index.html`, `src/styles/tasks.css`, `src/js/tabs/tasks.js`).
- **AI-to-Task Quick Add**: Wired up "Add" buttons in the AI Analysis card to instantly create tasks from AI suggestions (`src/js/ai-sidebar.js`).
- **Event-Driven UI**: Standardized "app_event" listener in `ai-sidebar.js` to handle all background analysis states (D-96 compliance).
- **Bug Fixes**:
  - Fixed "Ollama status" display in sidebar footer.
  - Corrected task list styling and priority-based color coding.

### Architectural Unification & Project Hierarchy

- **Project Entities**: Promoted Projects to first-class entities (`Project > Task > Subtask`) with dedicated table and CRUD commands (`src-tauri/src/db/projects.rs`).
- **Project Filtering**: Implemented frontend project filters and project-aware task creation (`src/js/tabs/tasks.js`, `src/index.html`).
- **Reactive Event Model**: refactored system to be 100% event-driven. All major state changes (Save, Complete, Delete) emit typed events on a unified `app_event` channel (D-96).
- **AI Sidebar Overhaul**: Redesigned AI thinking partner with premium aesthetics (`src/styles/ai-chat.css`) and structured action extraction.
- **Intelligence Trigger**: Decoupled AI analysis from UI commands; the pipeline now triggers reactively on the `JournalSaved` event.
- **AI Learning Loop**: Implemented the foundation for reinforcement learning by logging all user interactions (accept/reject) with AI suggestions to `proposed_task_log` (`src-tauri/src/db/ai.rs`).
- **Recurrence Idempotency**: Added completion guards to prevent duplicate task generation on status retries (`src-tauri/src/lib.rs`).

### Project Alignment & Recovery Loop (Phase 2)
- **Backend API Standardization**: 
  - `task_create` now correctly returns the complete Task object instead of ID string (Spec §8A).
  - Enforced 2-level maximum subtask depth constraint natively in the Rust command handler (D-40).
  - Shifted `task_list` filtering to granular `exclude_statuses: Vec<String>` from coarse `include_completed: bool`.
- **UI Architecture Constraints**:
  - Restructured Task Detail panel layout to use position absolute, ensuring the right AI Sidebar is strictly never obscured (D-23, D-78).
  - Modernized JS handlers to accept updated API object outputs.
- **Background Scheduler Suite**:
  - Refactored recurrence to be an **Interval-Based Rolling Chain**, detaching generation from the UX "done" event (D-120). Checks automatically in background.
  - Formally implemented `reminders.rs` worker that pushes system notifications, natively omitting un-fireable `idea`, `done`, and `cancelled` status states (D-113).
  - Added primary 60s asynchronous runtime loop to `main` context.
  - Restored 5-second delayed boot sequence to capture missed Weekly Reviews on Monday application launches (D-109).

## 2026-03-18

### AI Analysis Pipeline Hardening
- **Event-Queue-Worker Architecture**: Implemented background analysis worker to decouple UI from AI latency (`src-tauri/src/lib.rs`).
- **Memory Safety**:
  - Fixed "Double Move" AppHandle errors by using `handle.clone()` for separate async scopes.
  - Standardized `AnalysisState` to use owned `String` types instead of potentially invalid `str` slices.
  - Implemented `MAX_QUEUE` (100) limit to prevent memory bloat during high-frequency typing.
- **Robustness & Error Handling**:
  - Implemented 2-attempt retry logic for Ollama cold-starts/transient failures (`src-tauri/src/ai/client.rs`).
  - Added "Ghost Entry" race condition check: worker verifies entry exists before emitting results.
  - Added "Model Missing" detection: emits `ai_model_missing` event if `llama3.2` isn't pulled.
  - Implemented schema-safe JSON parsing with fallback to prevent crashes from malformed AI output.
- **Deduplication**: Added content hashing to avoid re-analyzing entries that haven't changed.
- **Frontend Sync**: Updated `ai-sidebar.js` and `journal.js` to handle standardized object-based payloads and show processing states.

### GitHub & Security Preparation
- **Secure .gitignore**: Blocked all user data (`/data`, `/backups`, `/attachments`, `/exports`) and SQLite temporary files while allowing critical `Cargo.lock` files.
- **Scaffolding**: Added `.gitkeep` to all local data directories to maintain structure in the repo.
- **Documentation**: Overhauled `README.md` with professional project overviews, setup guides, and privacy highlights.
- **Prompt Engineering**: Refined `src-tauri/src/ai/prompt.rs` to include the `mood` field and restrict output to JSON only.

## 2026-03-16
- Initialized project scaffold per spec.
- Added directories: src-tauri, src-tauri/src, src-tauri/src/db, src-tauri/src/ai, src-tauri/src/backup, src-tauri/src/scheduler, src-tauri/migrations, src, src/styles, src/styles/themes, src/js, src/js/tabs, src/views, src/assets, src/assets/libs, data, backups, attachments, exports.
- Added files: AGENTS.md, README.md, src-tauri/Cargo.toml, src-tauri/tauri.conf.json, src-tauri/src/main.rs, src-tauri/src/lib.rs, src-tauri/src/error.rs, src-tauri/src/events.rs, src-tauri/src/logger.rs.
- Added files: src-tauri/src/db/init.rs, src-tauri/src/db/paths.rs, src-tauri/src/db/migrations.rs, src-tauri/src/db/journal.rs, src-tauri/src/db/tasks.rs, src-tauri/src/db/ai.rs, src-tauri/src/db/settings.rs, src-tauri/src/db/audit.rs, src-tauri/src/db/reports.rs, src-tauri/src/db/trash.rs.
- Added files: src-tauri/src/ai/client.rs, src-tauri/src/ai/stream.rs, src-tauri/src/ai/tools.rs, src-tauri/src/ai/prompt.rs, src-tauri/src/ai/analysis.rs, src-tauri/src/ai/keywords.rs, src-tauri/src/ai/fallback.rs.
- Added files: src-tauri/src/backup/manual.rs, src-tauri/src/backup/auto.rs, src-tauri/src/backup/export.rs, src-tauri/src/scheduler/reminders.rs, src-tauri/src/scheduler/recurrence.rs, src-tauri/src/scheduler/weekly_report.rs.
- Added files: src-tauri/migrations/001_initial.sql, src-tauri/migrations/002_fts_triggers.sql, src-tauri/migrations/003_analysis_tracking.sql, src-tauri/migrations/004_field_expansion.sql, src-tauri/migrations/005_settings_v16.sql.
- Added files: src/index.html, src/styles/base.css, src/styles/layout.css, src/styles/components.css, src/styles/toast.css, src/styles/ai-chat.css, src/styles/terminal.css, src/styles/themes/dark.css, src/styles/themes/light.css, src/js/app.js, src/js/ipc.js, src/js/terminal.js, src/js/ai-sidebar.js, src/js/notifications.js, src/js/tabs/journal.js, src/js/tabs/tasks.js, src/js/tabs/ai.js, src/js/tabs/reports.js, src/js/tabs/settings.js.
- Updated files: CHANGELOG.md.
- Added Phase 0 compile-check crate: tools/rusqlite-check (Cargo.toml, Cargo.lock, src/main.rs).
- Updated files: AGENTS.md.
- Attempted `rusqlite` bundled-full build; failed due to Cargo 1.80.0 not supporting `edition2024` required by `time` v0.3.47.
- Added placeholder module files: src-tauri/src/db/mod.rs, src-tauri/src/ai/mod.rs, src-tauri/src/backup/mod.rs, src-tauri/src/scheduler/mod.rs.
- Filled empty Rust files with `// TODO: implement` placeholders, including src-tauri/Build.rs and all empty files in db/, ai/, backup/, scheduler/.
- Updated files: AGENTS.md, CHANGELOG.md.
- Fixed startup panic in DB init by setting `journal_mode` via `pragma_update_and_check` (avoids rusqlite "Execute returned results" error).
- D-97: Wrapped `src-tauri/migrations/001_initial.sql` in `BEGIN; ... COMMIT;`.
- D-97: Updated `src-tauri/src/db/migrations.rs` so each migration runs inside `conn.transaction()`, while stripping the outer `BEGIN/COMMIT` from each SQL file to avoid nested-transaction errors.
- Fixed startup panic: actually strip outer `BEGIN/COMMIT` before `tx.execute_batch(...)` and added migration runner tests (`src-tauri/src/db/migrations.rs`).
- Journal persistence hardening: added IPC wrapper + toast notifications (`src/js/ipc.js`, `src/js/notifications.js`, `src/styles/toast.css`, `src/index.html`).
- Journal autosave: prevent duplicate-entry races with in-flight save guard + no-op when unchanged (`src/js/tabs/journal.js`).
- App shell: route all invokes through wrapper + prevent tab-switch crash, added missing tab placeholders (`src/js/app.js`, `src/index.html`).
- DB journal writes: persist `word_count` and normalize empty titles (`src-tauri/src/db/journal.rs`, `src-tauri/src/lib.rs`).
- App name is official updated to HawkwardJournal. DB name is hawkward.db
