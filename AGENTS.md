# AGENTS.md — HawkwardJournalAI

**AI coding agent memory file. Read this before every session. Do not begin work until you confirm understanding.**

---

## What This Project Does

HawkwardJournalAI is a private, offline-first Windows desktop productivity app built with Tauri v2. It combines a plain-text journal, a full task manager, and a local AI assistant powered by Ollama (llama3.2 default). The AI reads every journal entry automatically on save, extracts actionable tasks, and can manage tasks through natural conversation. All data is stored in a single SQLite file on the user's machine. No internet connection, no accounts, no cloud.

---

## Current Build Status

**Phase:** Phase 2 (Tasks) complete. Moving into Phase 3 (AI Chat & Tool Engine) and Phase 4 (Reports).
**Last built:** Task Management CRUD, First-Class Project Entities, Reactive Event Model (D-96 AppEvent unification), Background Scheduler Suite (reminders & recurrence).
**Open issues:**
- Implement Phase 3 AI tools execution (create_task, search_journal, etc.) with timeout cancellation (D-95).
- Build the 6 Analytical Reports (Phase 4).
- Address D-13 spec violation (Projects recently implemented as first-class entity rather than a text field).
**Spec files:** `AgentDocs/HawkwardJournalAI_MASTER_SPEC_v1.6.md` + `AgentDocs/HawkwardJournalAI_SPEC_ADDENDUM_v1.7.md`
**Locked decisions:** D-01 through D-111 (111 total)

---

## Tech Stack

| Layer              | Technology                                                                                      |
| ------------------ | ----------------------------------------------------------------------------------------------- |
| Desktop shell      | Tauri v2                                                                                        |
| Backend language   | Rust (stable 1.77+, MSVC toolchain on Windows)                                                  |
| Frontend           | Vanilla HTML + CSS + JS — no framework, no bundler                                              |
| Database           | SQLite via `rusqlite` (bundled-full — compiles SQLite from C source)                            |
| AI engine          | Ollama REST API (localhost:11434) — llama3.2 default                                            |
| Async runtime      | Tokio                                                                                           |
| HTTP client        | reqwest (for Ollama API calls)                                                                  |
| Frontend libraries | Chart.js 4.x · Marked.js 9.x · highlight.js 11.x · Flatpickr 4.x (all vendored locally, no CDN) |

---

## File Structure

```
hawkwardjournalai/
├── src-tauri/
│   ├── Cargo.toml                   All dependencies, all pinned
│   ├── tauri.conf.json              Window: 1400×900, minWidth 1200
│   ├── src/
│   │   ├── main.rs                  Tauri entry, plugin registration, window close handler
│   │   ├── lib.rs                   All #[tauri::command] registrations
│   │   ├── error.rs                 AppError enum — every handler returns Result<T, AppError>
│   │   ├── events.rs                AppEvent enum — SINGLE typed event channel "app_event"
│   │   ├── db/                      Layer 3: Data Access
│   │   │   ├── mod.rs               Module declarations (placeholder)
│   │   │   ├── init.rs              7-step startup sequence
│   │   │   ├── paths.rs             resolve_data_dir() — MSI vs portable detection
│   │   │   ├── migrations.rs        Sequential migration runner
│   │   │   ├── journal.rs           journal_* + notebook_* handlers
│   │   │   ├── tasks.rs             task_* + timer_* + attachment_* handlers
│   │   │   ├── ai.rs                conversation_* + message_* + proposed_task_log
│   │   │   ├── settings.rs          setting_get/set/seed
│   │   │   ├── audit.rs             write() + archive_old_entries()
│   │   │   ├── reports.rs           6 report query handlers
│   │   │   └── trash.rs             trash_list + trash_empty
│   │   ├── ai/                      Layer 2: AI Orchestration
│   │   │   ├── mod.rs               Module declarations (placeholder)
│   │   │   ├── client.rs            OllamaClient — single Tokio Mutex
│   │   │   ├── stream.rs            NDJSON parser + emit(AiToken)
│   │   │   ├── tools.rs             6 tool definitions + ToolExecutor + 300s confirm timeout
│   │   │   ├── prompt.rs            PromptComposer — 8 system prompt blocks
│   │   │   ├── analysis.rs          JournalAnalysisPipeline — latest-only dedup
│   │   │   ├── keywords.rs          extract_keywords() stop-word filter
│   │   │   └── fallback.rs          3-pattern regex fallback tool-call parser
│   │   ├── backup/
│   │   │   ├── mod.rs               Module declarations (placeholder)
│   │   │   ├── manual.rs            rusqlite::backup::Backup API (not fs::copy)
│   │   │   ├── auto.rs              tokio interval + backup-on-open
│   │   │   └── export.rs            JSON export + ZIP + import
│   │   ├── scheduler/
│   │   │   ├── mod.rs               Module declarations (placeholder)
│   │   │   ├── reminders.rs         60s poll — local time
│   │   │   ├── recurrence.rs        3600s poll — skip missed occurrences
│   │   │   └── weekly_report.rs     Monday 08:00 OR first Monday launch
│   │   ├── migrations/              ALL wrapped in BEGIN/COMMIT
│   │   │   ├── 001_initial.sql
│   │   │   ├── 002_fts_triggers.sql
│   │   │   ├── 003_analysis_tracking.sql
│   │   │   ├── 004_field_expansion.sql
│   │   │   └── 005_settings_v16.sql
│   │   └── logger.rs                tracing → emit(LogEvent)
├── src/                             Frontend (WebView2)
│   ├── index.html                   Root shell — CSS grid only
│   ├── styles/
│   │   ├── base.css                 CSS variables, reset
│   │   ├── layout.css               3-column grid: 220px | 1fr | 420px
│   │   ├── components.css           Buttons, inputs, badges, chips
│   │   ├── toast.css                Floating toast (position: fixed, bottom-right)
│   │   ├── ai-chat.css              Analysis card, message bubbles
│   │   ├── terminal.css             Terminal bar (optional)
│   │   └── themes/dark.css          light.css
│   ├── js/
│   │   ├── app.js                   Init, tab router, single app_event listener
│   │   ├── ipc.js                   invoke() wrapper — always .catch() → error toast
│   │   ├── terminal.js              Lazy-loaded on toggle
│   │   ├── ai-sidebar.js            Always mounted — analysis card state machine
│   │   ├── notifications.js         Floating toast system
│   │   └── tabs/
│   │       ├── journal.js           VirtualList + cursor pagination
│   │       ├── tasks.js             Calendar/list/kanban + drag-assign
│   │       ├── ai.js                Conversation list + chat window
│   │       ├── reports.js           6 reports + date range picker
│   │       └── settings.js          6 sections + SQL runner
│   ├── views/                       HTML partials (loaded once, show/hide)
│   └── assets/libs/                 Vendored JS (no CDN)
├── data/                            Created at first run
├── backups/
├── attachments/
├── exports/
├── tools/
│   └── rusqlite-check/              Phase 0 compile check crate
├── AGENTS.md                        This file
├── CHANGELOG.md
└── README.md
```

---

## Conventions — Always Follow

**Rust backend:**

- Every `#[tauri::command]` returns `Result<T, AppError>` — no panics, no unwrap in handlers
- Every DB write goes through the Tokio Mutex on the write path (D-33)
- All events emitted via `emit(app, AppEvent::Variant {...})` from `events.rs` — never raw string emit calls
- All file paths stored as relative strings — resolved to absolute at runtime (D-47)
- All dates stored as ISO 8601 UTC — all day calculations use `chrono::Local` (D-101)
- All migration files wrapped in `BEGIN; ... COMMIT;` — never bare DDL statements (D-97)
- Backups always use `rusqlite::backup::Backup` API — never `std::fs::copy` on `.db` files (D-98)

**Frontend:**

- All `invoke()` calls go through `ipc.js` wrapper — always has `.catch()` → `showError()` toast
- **IPC naming rule:** `invoke()` top-level arg keys must be **camelCase** (Tauri v2 requirement). Nested struct fields in JSON payloads follow **snake_case** (Rust serde convention). Single-word params are unaffected.
- All Rust→Frontend events arrive on the single `"app_event"` listener in `app.js`
- Dispatch by `event.payload.type` — never by event name string
- Tab views are loaded once and toggled via CSS `display` — never re-rendered on tab switch
- `marked.min.js` is imported only in `reports.html` — never in journal or other views (D-42)
- Terminal bar is hidden by default — do not show it unless `terminal_visible = true` in settings (D-79)

**AI:**

- AI never writes to the database without user confirmation (D-19)
- All AI confirmation waits have a 300-second timeout that auto-cancels (D-95)
- `ai_chat` returns `Result<String>` — the conversation_id — not `()` (D-105)
- Context injection uses compact one-line format per task (~25 tokens) not full JSON objects (D-94)
- Default `ollama_context_len` is 16384 — not 4096 (D-93)

---

## Immutable Rules — Never Break

1. **No DB write without user confirmation for AI actions** — D-19, D-95. Every AI tool that mutates data waits for explicit YES/NO. 300s timeout auto-cancels.
2. **No `std::fs::copy` for database backups** — use `rusqlite::backup::Backup` API. File copy misses WAL pages. D-98.
3. **No bare DDL in migration files** — every migration is wrapped in `BEGIN/COMMIT`. A crash mid-migration without this produces an unrecoverable state. D-97.
4. **No Markdown in the journal editor** — the journal content textarea is plain text only. `marked.min.js` is loaded only in `reports.html`. D-07, D-42.
5. **No offset-based pagination for `journal_list`** — must use keyset cursor (`created_at < :cursor`). Offset scans the full table. D-104.
6. **No raw string emit() calls** — all events go through the `AppEvent` enum in `events.rs`. D-96.
7. **AI never creates journal entries** — ghostwriter drafts appear in chat only. User copies manually. D-54, D-67.
8. **The right sidebar is always mounted and always visible** — 420px fixed, never hidden. D-23, D-78.
9. **`journal_analysis_queued` event fires immediately** — before the 3-second debounce, not after. D-106.
10. **Subtask depth is two levels maximum** — enforced in the `task_create` handler, not just in UI. D-40.

---

## Known Decisions

| Decision | What it is                                              | Why                                                                                             |
| -------- | ------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| D-50     | `rusqlite bundled-full` — not `tauri-plugin-sql`        | Full connection control, FTS5 guaranteed, no platform variance                                  |
| D-93     | Default context 16384, not 4096                         | 4096 is too small for the guaranteed minimum task tiers at real-world usage                     |
| D-94     | Compact task injection format (~25 tok/task)            | Full JSON objects overflow the context budget                                                   |
| D-96     | Single `AppEvent` enum, single `"app_event"` channel    | Eliminates string-literal event name mismatches across 12+ events                               |
| D-97     | `BEGIN/COMMIT` in every migration file                  | Without it, a crash mid-migration leaves the DB in an unrecoverable state                       |
| D-98     | `rusqlite::backup::Backup` for all backups              | `fs::copy` misses uncommitted WAL pages silently                                                |
| D-99     | Backup fires on open + on close, not just on schedule   | Scheduled backup never fires if app runs in Exit mode (default)                                 |
| D-100    | FTS5 soft-delete + restore triggers                     | Without them, deleted entries surface in AI pre-seed context results                            |
| D-104    | Keyset pagination for `journal_list`                    | Offset scans full table — O(n) per page. Keyset is O(log n) regardless of depth                 |
| D-105    | `ai_chat` returns `conversation_id`                     | Frontend must know the conversation ID before first token arrives                               |
| D-107    | `ProposedTaskInput` fully defined struct                | Frontend and Rust independently building this struct will produce different shapes              |
| D-108    | Ollama models dir: auto-detect, show in Settings        | Detection order: `OLLAMA_MODELS` env → `app_settings` override → `%USERPROFILE%\.ollama\models` |
| D-109    | Weekly review: Monday 08:00 OR first Monday launch      | Old behaviour missed the review if app wasn't running at 08:00                                  |
| D-110    | Web search deferred to backlog (B-03)                   | No acceptable provider chosen yet                                                               |
| D-111    | 7 tools only — no file system, shell, or code execution | Not requested, not in scope                                                                     |

---

## Database Tables (13 total)

`schema_migrations` · `notebooks` · `journal_entries` · `journal_fts` (virtual) · `journal_emotions_flat` (view) · `tasks` · `task_dependencies` · `task_attachments` · `time_logs` · `ai_conversations` · `ai_messages` · `app_settings` · `audit_log` · `proposed_task_log`

---

## AI Tools (7 total)

| Tool             | Confirmation required? | Notes                       |
| ---------------- | ---------------------- | --------------------------- |
| `create_task`    | ✅ Yes                 | Writes to DB                |
| `update_task`    | ✅ Yes                 | Writes to DB                |
| `complete_task`  | ✅ Yes                 | Writes to DB                |
| `list_tasks`     | No                     | Read only                   |
| `search_journal` | No                     | Read only                   |
| `fetch_url`      | No                     | Network fetch, no DB write  |
| `web_search`     | —                      | Backlog B-03, not built yet |

---

## Feature Backlog (do not build these)

- **B-01** — AI Training Zone (structured Q&A to build user_profile)
- **B-02** — AI Insights Tab (synthesised pattern observations)
- **B-03** — Web Search (configurable search endpoint, user specifies provider)

---

## Session Start Checklist

Before writing any code, confirm:

- [ ] Read this file
- [ ] Read `CHANGELOG.md` (last 3 entries)
- [ ] Tell back: current build status + phase + what was last built + open issues
- [ ] Confirm which phase you are working in
- [ ] Confirm the specific task before starting

**Do not begin work until the human confirms your understanding is correct.**

---

## Session End Checklist

Before closing the session:

- [ ] Update `CHANGELOG.md` with every file created or modified
- [ ] Update this file: file structure, conventions, known decisions if anything changed
- [ ] State current project status in one sentence
- [ ] State the logical next step

Prompts for specific use cases:
e:\Abir\LocalCodeRepo\HawkwardJournalAI\AgentDocs\P-01_research_review_response_process.md
e:\Abir\LocalCodeRepo\HawkwardJournalAI\AgentDocs\P-02_getting_unstuck_prompts.md
e:\Abir\LocalCodeRepo\HawkwardJournalAI\AgentDocs\P-03_self_evaluation_prompts.md
e:\Abir\LocalCodeRepo\HawkwardJournalAI\AgentDocs\P-04_debugging_prompts.md
e:\Abir\LocalCodeRepo\HawkwardJournalAI\AgentDocs\P-05_accuracy_checking_prompts.md

project plan:
e:\Abir\LocalCodeRepo\HawkwardJournalAI\AgentDocs\HawkwardJournalAI_MASTER_SPEC_v1.6.md
e:\Abir\LocalCodeRepo\HawkwardJournalAI\AgentDocs\HawkwardJournalAI_SPEC_ADDENDUM_v1.7.md
