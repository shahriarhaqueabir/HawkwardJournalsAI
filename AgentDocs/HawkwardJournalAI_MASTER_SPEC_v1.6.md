# PersonalLifeOS — Master Specification v1.6
## Single Authoritative Reference Document
**Date:** 2026-03-15 | **Status:** All Decisions Locked · Ready for Phase 0
**Total locked decisions:** D-01 through D-107

---

## WHAT CHANGED IN v1.6

All engineering review fixes applied. Four new sections added.

| Change | Source | Impact |
|--------|--------|--------|
| D-93 | Default context raised to 16384 | Settings seed, hardware warning, prompt composer |
| D-94 | Compact task injection format (20–30 tok/task) | `prompt.rs` format function |
| D-95 | AI confirm timeout: 300s auto-cancel | `tools.rs` oneshot channel with timeout |
| D-96 | Single typed event system (`AppEvent` enum) | All `emit()` calls, all JS listeners |
| D-97 | Migration files wrapped in `BEGIN/COMMIT` | All 4 migration files |
| D-98 | Backup uses SQLite Online Backup API | `backup/manual.rs`, `backup/auto.rs` |
| D-99 | Backup fires on open (if stale) and on close | `db/init.rs`, `main.rs` window event |
| D-100 | FTS5 soft-delete + restore triggers (2 new) | Migration 005 |
| D-101 | Timezone policy: local wall-clock for day calc | `recurrence.rs`, `reminders.rs` |
| D-102 | `task_restore` restores subtasks | `db/tasks.rs` |
| D-103 | `conversation_switch_model` accepts `Option<String>` | `db/ai.rs` |
| D-104 | `journal_list` uses keyset (cursor) pagination | `db/journal.rs`, `VirtualList` |
| D-105 | `ai_chat` returns `conversation_id` | `db/ai.rs` |
| D-106 | `journal_analysis_queued` event fires immediately | `ai/analysis.rs` |
| D-107 | `ProposedTaskInput` struct fully defined | `ai/analysis.rs` |
| — | Energy/context filters changed to multi-select | `db/tasks.rs` |
| — | Emotion filter moved into SQL (not post-filter) | `db/journal.rs` |
| — | `backup_manual` accepts path from frontend dialog | `backup/manual.rs` |
| — | `trash_empty` requires confirmation + file-exists guard | `db/trash.rs` |
| — | `timer_start` guards against completed/cancelled tasks | `db/tasks.rs` |
| — | Source label map defined: UI ↔ DB values | `ai.js` |

---

## TABLE OF CONTENTS

1. [Project Identity](#1-project-identity)
2. [Complete Decision Register — D-01 through D-107](#2-complete-decision-register)
3. [Tech Stack — Complete Reference](#3-tech-stack)
4. [Architecture Layers](#4-architecture-layers)
5. [Application Architecture — Process & IPC](#5-application-architecture)
6. [UI Layout Specification](#6-ui-layout-specification)
7. [UI Features & Functionalities Breakdown](#7-ui-features--functionalities-breakdown)
8. [Unified IPC Contract — Front-to-Back Data Calls](#8-unified-ipc-contract)
9. [Typed Event System](#9-typed-event-system)
10. [Database — Initialisation & Pragmas](#10-database-initialisation--pragmas)
11. [Database — Schema (All Tables)](#11-database-schema)
12. [Database — Cross-Table Queries](#12-cross-table-queries)
13. [AI Integration Architecture](#13-ai-integration-architecture)
14. [Journal Auto-Analysis Pipeline](#14-journal-auto-analysis-pipeline)
15. [Thinking Partner Persona](#15-thinking-partner-persona)
16. [Data Management & Portability](#16-data-management--portability)
17. [System Tray & Auto-Start](#17-system-tray--auto-start)
18. [Build Phases](#18-build-phases)
19. [File & Directory Scaffolding](#19-file--directory-scaffolding)
20. [Risk Register](#20-risk-register)
21. [Feature Backlog](#21-feature-backlog)
22. [Verify Before Use](#22-verify-before-use)

---

## 1. PROJECT IDENTITY

| Field | Value |
|-------|-------|
| **Name** | PersonalLifeOS |
| **Type** | Desktop window app — personal productivity |
| **Purpose** | Private offline journaling + full daily life task management with embedded local AI |
| **Platform** | Windows (primary). Tauri cross-compiles to macOS/Linux at zero extra cost |
| **Runtime** | Tauri v2 (Rust backend + WebView2 frontend) |
| **AI Engine** | Local Ollama — llama3.2 default, all locally installed models available |
| **Database** | SQLite — single portable `.db` file, WAL mode, auto-detects MSI vs portable |
| **Internet** | Zero runtime internet dependency |
| **Auth** | None — single-user personal machine |
| **Default context** | 16384 tokens (D-93) |
| **Min RAM** | 8GB supported (with AI context degradation warning); 16GB recommended |
| **Theme** | Dark (always). Light available in Settings |
| **Density** | Comfortable — balanced spacing |
| **Startup Tab** | Journal (always, every launch) |

### Design Mandate
> Simple, smart, and fast. Every feature must earn its place.
> Use well-tested, existing libraries. No experimental dependencies. No cloud lock-in.
> The app must work forever on a disconnected Windows machine.

---

## 2. COMPLETE DECISION REGISTER

### Core Architecture (D-01 – D-28) — v1.1

| # | Topic | Decision |
|---|-------|----------|
| D-01 | Desktop Framework | **Tauri v2** — lean binary, native WebView2 |
| D-02 | Frontend | **Vanilla HTML/CSS/JS** — no framework overhead |
| D-03 | Database Driver | superseded by D-50 |
| D-04 | AI Runtime | **Ollama REST API** `localhost:11434` |
| D-05 | AI Tool-Calling | **Ollama native** + JSON fallback (D-38) |
| D-06 | Web Research | **Ollama tool-calling only** — fully offline |
| D-07 | Journal Format | **Plain text** |
| D-08 | Journal Timestamps | **Auto-stamped at creation** — never overridable |
| D-09 | Journal Organisation | **Notebooks + Tags** |
| D-10 | Journal Streak | **Reports only** |
| D-11 | Task Fields | **All four field groups** |
| D-12 | Task Default View | **Calendar / Agenda** |
| D-13 | Task Project Field | **First-class entity (table)** — includes ID, name, colour, goal date |
| D-14 | Task Recurrence | **Fixed schedule** (D-39) |
| D-15 | Reports Output | **On-screen + PDF + Markdown** |
| D-16 | Weekly Review | **Auto Monday 08:00** |
| D-17 | AI Suggestions | **Tasks + journal combined** |
| D-18 | Reminders | **Floating toast bottom-right** (D-81); OS toast in tray (D-73) |
| D-19 | AI Confirmation | **User must confirm ALL AI actions** before DB write. 300s timeout (D-95) |
| D-20 | AI Context Injection | **Smart inject** extended by D-56 minimums |
| D-21 | AI Conversation History | **Persist to DB** |
| D-22 | Global Search | **AI handles via tools** |
| D-23 | Right Sidebar | **Always visible, 420px fixed** |
| D-24 | UI Theme | **Dark always** — Light in Settings |
| D-25 | UI Density | **Comfortable** |
| D-26 | DB Portability | File + backup + export + import |
| D-27 | Deleted Items | **Trash bin** — manual permanent delete |
| D-28 | Security | **None** — BitLocker advisory (D-48) |

### Data Integrity (D-29 – D-50) — v1.2

| # | Decision |
|---|----------|
| D-29 | Two-mode path: MSI → `%APPDATA%`; Portable → `./data/` |
| D-30 | `schema_migrations` table + sequential runner |
| D-31 | `PRAGMA integrity_check` + schema version check + safety copy before restore |
| D-32 | Auto-archive audit_log at 100k rows or 365 days |
| D-33 | Tokio Mutex on write path + `PRAGMA busy_timeout=5000` |
| D-34 | Fixed truncation order: system+tools (never) → overdue → today → upcoming → entry → journal → completed (first) |
| D-35 | Fresh `chrono::Local::now()` on every AI request |
| D-36 | Return structured error to model for invalid dates |
| D-37 | Model-missing: 404 → refresh list → block chat → prompt reselection |
| D-38 | Fallback parser: `<tool_call>` tags → fence → raw JSON |
| D-39 | Recurrence: skip missed, advance to next future date |
| D-40 | Subtask depth: two levels max, enforced in handler |
| D-41 | FTS5: explicit `unicode61 remove_diacritics 0` + 4 sync triggers (D-100 adds 2 more) |
| D-42 | Marked.js only in `reports.html` |
| D-43 | Detect `NULL ended_at` timers on startup, recovery toast |
| D-44 | Single Tokio Mutex on `OllamaClient` |
| D-45 | VirtualList + keyset pagination from Phase 1 Day 1 |
| D-46 | `fetch_url` returns raw HTML, AI detects walls |
| D-47 | Attachment paths: relative, resolved at runtime |
| D-48 | BitLocker advisory in README + Settings + export dialogs |
| D-49 | ISO 8601 only in tool args. System prompt injects `{TOMORROW}`, `{NEXT_MONDAY}` |
| D-50 | `rusqlite` (bundled-full) — tauri-plugin-sql removed |

### AI Persona (D-51 – D-60) — v1.3

| # | Decision |
|---|----------|
| D-51 | Brain Dump = journal entry. Not a separate mode. |
| D-52 | Analysis fires on explicit save only (not debounce) |
| D-53 | Task extraction: edit + selectively confirm |
| D-54 | Ghostwriter: draft in AI chat, user copies manually |
| D-55 | Thinking Partner always active across all surfaces |
| D-56 | Smart inject: overdue + today + upcoming always injected |
| D-57 | Rust FTS5 pre-seeds context before AI sees the entry |
| D-58 | Socratic probing: no question limit |
| D-59 | Analysis: 3s debounce + `IN_FLIGHT` deduplication (latest only) |
| D-60 | `last_analysis_conv_id` + `last_analysed_at` on journal entries |

### Field Expansion (D-61 – D-77) — v1.4

| # | Decision |
|---|----------|
| D-61 | Upcoming tasks: next 14 days, Tier 3 guaranteed minimum |
| D-62 | Upcoming: lowest truncation priority |
| D-63 | `list_upcoming()`: `due_date > today AND due_date <= today+14` |
| D-64 | New task fields: energy_level, context_tag, linked_url, dependencies |
| D-65 | Fixed schema — no custom user fields |
| D-66 | Emotions: 10-value named multi-select, replaces 1–5 mood |
| D-67 | AI never creates journal entries directly |
| D-68 | No Daily Briefing panel |
| D-69 | Weekly Planning: auto Monday 08:00 + manual trigger |
| D-70 | JSON import only: Replace and Merge modes |
| D-71 | Auto-start: opt-in, off by default |
| D-72 | Window close: Exit (default) or Minimise to tray |
| D-73 | Tray: OS toast when tray active |
| D-74 | Reports: quick-select presets + custom date range |
| D-75 | Report 6: Energy & Focus |
| D-76 | Fixed keyboard shortcuts |
| D-77 | Upcoming window: 14 days, fixed |

### UI Clarifications (D-78 – D-92) — v1.5

| # | Decision |
|---|----------|
| D-78 | Right sidebar: 420px fixed |
| D-79 | Terminal bar: optional, hidden by default |
| D-80 | Nav rail bottom: Ollama status + DB path + app version |
| D-81 | Reminders: floating toast bottom-right |
| D-82 | Trash tasks: parent only + subtask count note |
| D-83 | Import merge: live version wins |
| D-84 | Model switch: warning dialog → new conversation |
| D-85 | Calendar drag: assigns due date immediately |
| D-86 | Ollama offline: app opens normally, AI shows offline |
| D-87 | Attachments: no size limit |
| D-88 | New journal entry: auto-saves current silently |
| D-89 | Analysis card: fixed above chat input |
| D-90 | SQL runner: user can toggle read-only off |
| D-91 | Insights tab: backlog |
| D-92 | AI Training Zone: backlog |

### Engineering Review Fixes (D-93 – D-107) — v1.6

| # | Topic | Decision |
|---|-------|----------|
| D-93 | Default context length | **16384 tokens**. Options: 4096/8192/16384/32768. RAM warning shown if system < 16GB. |
| D-94 | Compact injection format | One-line-per-task format in `PromptComposer`: `[id6] title \| PRIORITY \| date \| energy \| project` — ~25 tokens/task |
| D-95 | AI confirm timeout | **300 seconds**. Auto-cancel with toast: "AI action timed out — not applied." Releases OllamaClient Mutex. |
| D-96 | Event system | **Single typed `AppEvent` enum** in `events.rs`. Single `"app_event"` channel. JS dispatches by `payload.type`. |
| D-97 | Migration atomicity | Every migration file wrapped in `BEGIN; ... COMMIT;` transaction |
| D-98 | Backup implementation | **`rusqlite::backup::Backup` API** — replaces `std::fs::copy`. Verifies output with `PRAGMA integrity_check`. |
| D-99 | Backup triggers | Three triggers: scheduled interval + **on open if stale** + **on close** |
| D-100 | FTS5 soft-delete | Two new triggers: `journal_fts_soft_delete` (removes from index on `is_deleted=1`) and `journal_fts_restore` |
| D-101 | Timezone policy | **Local wall-clock** for all day-based calculations. UTC for all storage. |
| D-102 | `task_restore` | Restores parent AND all subtasks: `WHERE id = ? OR parent_task_id = ?` |
| D-103 | `conversation_switch_model` | Accepts `Option<String>` for `current_conversation_id`. None = update setting only. |
| D-104 | `journal_list` pagination | **Keyset cursor** (`cursor: Option<String>`) — not offset. VirtualList passes `created_at` of last item. |
| D-105 | `ai_chat` return type | Returns `Result<String>` — the active `conversation_id`. Frontend receives before first token. |
| D-106 | Analysis queued event | `AppEvent::JournalAnalysisQueued` fires **immediately** on `trigger_journal_analysis` call, before 3s debounce. |
| D-107 | `ProposedTaskInput` | Fully defined struct: `accepted`, `original_title`, `original_data`, `source_text`, `edited_*` fields. |

---

## 3. TECH STACK — COMPLETE REFERENCE

### 3A. Runtime

| Component | Technology | Version | Notes |
|-----------|-----------|---------|-------|
| Desktop shell | **Tauri** | 2.x | Rust backend, WebView2 renderer |
| Backend language | **Rust** | stable 1.77+ | Requires MSVC toolchain on Windows |
| Frontend | **Vanilla HTML + CSS + JS** | — | No build step, no bundler |
| WebView renderer | **Microsoft WebView2** | Bundled | Windows only; auto-installed if missing |
| IPC bridge | Tauri `invoke()` + `emit()` | Built-in | Typed via `AppEvent` enum (D-96) |

### 3B. Rust Crates (Cargo.toml — Final, All Pinned)

```toml
[dependencies]
tauri                     = { version = "2",    features = ["tray-icon"] }
tauri-plugin-fs           = "2"
tauri-plugin-dialog       = "2"
tauri-plugin-shell        = "2"
tauri-plugin-notification = "2"
tauri-plugin-autostart    = "2"
rusqlite                  = { version = "0.31", features = ["bundled-full"] }
# bundled-full: compiles SQLite from C source; FTS5, JSON1, RTREE guaranteed
# Requires MSVC C compiler on Windows — verify in Phase 0 before any app code
tokio                     = { version = "1",    features = ["full"] }
reqwest                   = { version = "0.12", features = ["json", "stream"] }
serde                     = { version = "1",    features = ["derive"] }
serde_json                = "1"
chrono                    = { version = "0.4",  features = ["serde"] }
uuid                      = { version = "1",    features = ["v4"] }
tracing                   = "0.1"
tracing-subscriber        = { version = "0.3",  features = ["env-filter"] }
regex                     = "1"
zip                       = "2"
dirs                      = "5"
sysinfo                   = "0.30"   # RAM detection for hardware warning (D-93)
```

### 3C. Frontend Libraries (Vendored — `/src/assets/libs/`)

| Library | Purpose | Version | Loaded in |
|---------|---------|---------|-----------|
| **Chart.js** | Reports: bar, line, doughnut, stacked area | 4.x | `reports.html` only |
| **Marked.js** | Markdown render for AI narratives | 9.x | `reports.html` only (D-42) |
| **highlight.js** | Terminal bar syntax colouring | 11.x | `index.html` (lazy) |
| **Flatpickr** | Date/time pickers for tasks and reminders | 4.x | `tasks.html` |

No CDN calls. All libraries copied to `/src/assets/libs/` before Phase 0.

### 3D. Ollama API Contract

| Operation | Method + Path | Notes |
|-----------|-------------|-------|
| Health check | `GET /` | 200 = running |
| List models | `GET /api/tags` | Used to populate model selector |
| Chat + tools | `POST /api/chat` | `stream: true` → NDJSON chunks |
| Tool-calling | In `/api/chat` payload | Supported: llama3.2, llama3.1, mistral-nemo |
| Model info | `GET /api/ps` | Returns active model size for RAM warning |
| Version | `GET /api/version` | Shown in Settings > About |

### 3E. Hardware Requirements

| RAM | Status | Behaviour |
|-----|--------|-----------|
| 16GB+ | **Recommended** | Full context (16384 tokens), all features |
| 8–15GB | Supported | Context reduced to 8192; warning shown in nav rail |
| < 8GB | Not recommended | Context reduced to 4096; strong warning on startup |

RAM is detected via `sysinfo` crate at startup. Warning is a permanent amber dot on the Ollama status row in the nav rail with a tooltip explaining the limitation.

---

## 4. ARCHITECTURE LAYERS

```
╔══════════════════════════════════════════════════════════════════╗
║  LAYER 0 — PRESENTATION                                         ║
║  Vanilla HTML + CSS + JS running in WebView2                    ║
║  ┌────────────┐ ┌──────────────────┐ ┌──────────────────────┐  ║
║  │ Left Nav   │ │  Main Workspace  │ │   AI Sidebar (420px) │  ║
║  │ Rail 220px │ │  Tab Views       │ │   Always Mounted     │  ║
║  └────────────┘ └──────────────────┘ └──────────────────────┘  ║
║  ┌────────────────────────────────────────────────────────────┐  ║
║  │ Terminal Bar (optional, hidden by default)                  │  ║
║  └────────────────────────────────────────────────────────────┘  ║
║  ┌──────────────────────────────────┐                           ║
║  │ Floating Toast System            │  (position: fixed)        ║
║  └──────────────────────────────────┘                           ║
╠══════════════════════════════════════════════════════════════════╣
║  LAYER 1 — IPC BOUNDARY (Tauri invoke / emit)                   ║
║                                                                  ║
║  Frontend → Backend:  invoke("command_name", payload)            ║
║  Backend → Frontend:  emit("app_event", AppEvent variant)        ║
║                                                                  ║
║  Contract: every invoke returns Result<T, AppError>              ║
║  Contract: every event is a variant of the AppEvent enum        ║
╠══════════════════════════════════════════════════════════════════╣
║  LAYER 2 — APPLICATION LOGIC (Rust)                              ║
║                                                                  ║
║  ┌──────────────┐  ┌───────────────┐  ┌─────────────────────┐  ║
║  │ Command      │  │ AI Orchestr.  │  │ Scheduler Suite     │  ║
║  │ Handlers     │  │ OllamaClient  │  │ Reminders           │  ║
║  │ (Tauri cmds) │  │ ToolExecutor  │  │ Recurrence          │  ║
║  │              │  │ PromptComposer│  │ AutoBackup          │  ║
║  │              │  │ AnalysisPipe  │  │ WeeklyReport        │  ║
║  └──────┬───────┘  └──────┬────────┘  └──────────┬──────────┘  ║
║         │                 │                        │             ║
║         └─────────────────┴────────────────────────┘            ║
║                           │                                      ║
╠═══════════════════════════╪══════════════════════════════════════╣
║  LAYER 3 — DATA ACCESS    │                                      ║
║                           ▼                                      ║
║  ┌────────────────────────────────────────────────────────────┐  ║
║  │ DB Module (rusqlite)                                        │  ║
║  │ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐   │  ║
║  │ │ journal  │ │ tasks    │ │ ai       │ │ settings     │   │  ║
║  │ │ .rs      │ │ .rs      │ │ .rs      │ │ audit        │   │  ║
║  │ └──────────┘ └──────────┘ └──────────┘ └──────────────┘   │  ║
║  │ Single write-path Tokio Mutex (D-33)                        │  ║
║  └────────────────────────────────────────────────────────────┘  ║
╠══════════════════════════════════════════════════════════════════╣
║  LAYER 4 — STORAGE                                               ║
║                                                                  ║
║  ┌──────────────────────┐  ┌──────────────────┐                 ║
║  │ SQLite (bundled)     │  │ File System       │                 ║
║  │ personallifeos.db    │  │ ./attachments/    │                 ║
║  │ WAL mode             │  │ ./backups/        │                 ║
║  │ FTS5 virtual table   │  │ ./exports/        │                 ║
║  └──────────────────────┘  └──────────────────┘                 ║
╠══════════════════════════════════════════════════════════════════╣
║  LAYER 5 — EXTERNAL SERVICES (Local Only)                        ║
║                                                                  ║
║  ┌──────────────────────────────────────────────────────────┐   ║
║  │ Ollama (localhost:11434)                                   │   ║
║  │ llama3.2 default — any installed model selectable          │   ║
║  │ REST API — NDJSON streaming                                │   ║
║  └──────────────────────────────────────────────────────────┘   ║
╚══════════════════════════════════════════════════════════════════╝
```

### Layer Interaction Rules

- Layer 0 (Presentation) **never** accesses Layer 3 (Data Access) directly. All data flows through Layer 1 (IPC).
- Layer 2 (Application Logic) **owns** all business rules. No business logic in Layer 0.
- Layer 3 (Data Access) **never** calls Layer 2. Data flows up, not down.
- Layer 5 (Ollama) is accessed **only** from Layer 2 via `OllamaClient`. Never from Layer 0.
- The Tokio Mutex in Layer 3 **serialises all writes**. Reads are concurrent (WAL mode).

---

## 5. APPLICATION ARCHITECTURE — PROCESS & IPC

### 5A. Process Tree

```
Windows Process: personallifeos.exe
│
├── Rust Main Thread
│   ├── Tauri app init + plugin registration
│   ├── Window event handler (close → backup + tray)
│   └── Startup sequence (§10)
│
├── Tokio Runtime (async workers)
│   ├── OllamaClient — single Mutex, HTTP streaming
│   ├── JournalAnalysisPipeline — tokio::spawn per entry
│   ├── ReminderScheduler — 60s interval poll
│   ├── RecurrenceScheduler — 3600s interval check
│   ├── AutoBackupScheduler — configurable interval
│   └── WeeklyReportScheduler — Monday 08:00 local
│
├── Tauri Command Thread Pool
│   └── Handles all invoke() calls from WebView
│
└── WebView2 Process (separate OS process, sandboxed)
    └── Renders HTML/CSS/JS frontend
```

### 5B. Window Layout

```
┌─────────────────────────────────────────────────────────────────────┐
│  PersonalLifeOS                                            [─][□][×] │
├──────────┬──────────────────────────────────────┬───────────────────┤
│  LEFT    │        MAIN CONTENT AREA             │   RIGHT SIDEBAR   │
│  220px   │        flex-grow                     │   420px fixed     │
│  fixed   │                                      │   always visible  │
│          │  ┌──────────────────────────────┐   │                   │
│ ● Journal│  │                              │   │ [analysis card]   │
│   Tasks  │  │  Active tab view             │   │ ─────────────── │
│   AI     │  │                              │   │ [chat history]    │
│   Reports│  │                              │   │                   │
│   Settings│ └──────────────────────────────┘   │ [input + send ↵]  │
│          │                                      │                   │
│ ──────── │                                      │                   │
│ 🟢 model │                                      │                   │
│ ./data/  │                                      │                   │
│ v1.0.0   │                                      │                   │
├──────────┴──────────────────────────────────────┴───────────────────┤
│  TERMINAL BAR  [hidden by default — Ctrl+` to toggle]              │
└─────────────────────────────────────────────────────────────────────┘
                                        ┌───────────────────────────┐
                                        │  ⏰ Floating Toast         │
                                        │  bottom-right, fixed      │
                                        └───────────────────────────┘
```

### 5C. CSS Grid

```css
:root {
  --sidebar-left-w:  220px;
  --sidebar-right-w: 420px;
  --terminal-h:      180px;
  --color-bg:        #0f1117;
  --color-surface:   #1a1d27;
  --color-border:    #2a2d3e;
  --color-accent:    #6c8ef7;
  --color-text:      #e2e4ef;
  --color-muted:     #7a7f9a;
  --color-success:   #4caf82;
  --color-warn:      #f0b429;
  --color-error:     #e05252;
  --color-ai:        #29d9c2;
  --font-ui:         'Segoe UI', system-ui, sans-serif;
  --font-mono:       'Cascadia Code', 'Consolas', monospace;
  --radius:          6px;
  --space:           12px;
}

body {
  display: grid;
  grid-template-columns: var(--sidebar-left-w) 1fr var(--sidebar-right-w);
  grid-template-rows: 1fr;
  height: 100vh;
  overflow: hidden;
}
body.terminal-visible {
  grid-template-rows: 1fr var(--terminal-h);
}
```

---

## 6. UI LAYOUT SPECIFICATION

### 6A. Left Sidebar (220px fixed)

```
┌─────────────────────┐
│  🧠 PersonalLifeOS  │
├─────────────────────┤
│  📓 Journal         │  ← 44px height, active = 3px accent border + tint
│  ✅ Tasks           │
│  🤖 AI              │
│  📊 Reports         │
│  ⚙️  Settings        │
├─────────────────────┤
│  🟢 llama3.2        │  ← click = mini popover (model, version, retry)
│  📁 ./data/plo.db   │  ← truncated, tooltip = full path
│  v1.0.0             │
└─────────────────────┘
```

Ollama dot colours: 🟢 connected · 🟡 loading/retry · 🔴 unreachable
When RAM < 16GB: 🟡 amber dot with tooltip "AI context limited — see Settings > AI"

### 6B. Right Sidebar (420px fixed, always visible — D-23, D-78)

```
┌────────────────────────────────────────┐
│ [llama3.2           ▼]         [🗑️]   │  ← model selector + clear chat
├────────────────────────────────────────┤
│ 📓 Analysing: "Thursday reflections"  │  ← context indicator (smart inject)
├────────────────────────────────────────┤
│  ┌──────────────────────────────────┐  │
│  │  ANALYSIS CARD (when active)     │  │  ← fixed, above chat, D-89
│  │  Narrative + proposed tasks      │  │
│  │  [Save selected (2)] [Dismiss]   │  │
│  └──────────────────────────────────┘  │
│                                        │
│  ╭──────────────────────────────────╮  │
│  │ User message              →      │  │
│  ╰──────────────────────────────────╯  │
│  ╭──────────────────────────────────╮  │
│  │ ← Assistant response             │  │
│  ╰──────────────────────────────────╯  │
│  ┌──────────────────────────────────┐  │
│  │ 🔧 create_task — "Call dentist"  │  │  ← tool card (collapsible)
│  │ ✅ CONFIRM   ✗ CANCEL            │  │  ← confirmation card (D-19, D-95)
│  └──────────────────────────────────┘  │
│                                        │
├────────────────────────────────────────┤
│ [                              ] ↵     │  ← Enter=send, Shift+Enter=newline
└────────────────────────────────────────┘
```

### 6C. Terminal Bar (optional, hidden by default — D-79)

```
[ ALL ] [ AI ] [ DB ] [ ERRORS ]          [⎘ copy] [clear] [▲ collapse]
────────────────────────────────────────────────────────────────────────
10:42:01 [INFO][APP ] Application started — portable mode
10:42:01 [INFO][DB  ] 4 migrations applied
10:42:02 [INFO][AI  ] Ollama reachable — llama3.2 / context: 16384
10:42:15 [INFO][AI  ] Prompt: 1,240 tokens (context: 16384, budget: 8192)
10:42:17 [INFO][TOOL] create_task → "Call dentist" | HIGH | 2026-03-20
10:42:17 [INFO][DB  ] INSERT tasks id=abc-123
10:42:18 [WARN][AI  ] Context truncated: upcoming tasks reduced to 8
```

Log colours: INFO=var(--color-text) · WARN=var(--color-warn) · ERROR=var(--color-error) · AI=var(--color-ai) · DB=var(--color-success) · TOOL=magenta
Max 2000 DOM lines. Logs accumulate in memory regardless of visibility.
Ctrl+\` toggles. Settings > Appearance > "Show terminal bar" toggle.

### 6D. Floating Toast (bottom-right, fixed — D-81)

```css
.toast-container {
  position: fixed;
  bottom: 24px;
  right: 24px;
  z-index: 9999;
  display: flex;
  flex-direction: column-reverse;
  gap: 8px;
}
```

Max 3 toasts stacked. Auto-dismiss 8s. Action button extends timer.

### 6E. Window Config

```json
{ "title": "PersonalLifeOS", "width": 1400, "height": 900,
  "minWidth": 1200, "minHeight": 700, "resizable": true,
  "decorations": true, "center": true }
```

### 6F. Keyboard Shortcuts (D-76 — fixed, no customisation)

| Shortcut | Action |
|----------|--------|
| `Ctrl+S` | Save journal entry (triggers analysis) |
| `Ctrl+N` | New journal entry / New task |
| `Ctrl+Enter` | Send AI message |
| `Ctrl+\`` | Toggle terminal bar |
| `Ctrl+M` | Focus main content area |

---

## 7. UI FEATURES & FUNCTIONALITIES BREAKDOWN

### 7A. Journal Tab

**Entry List Panel (left ~340px within main)**

| Feature | Interaction | Behaviour |
|---------|------------|----------|
| Notebook pill tabs | Click | Filters list to that notebook. "All" shows all. |
| "+ New Notebook" pill | Click | Opens inline name + colour picker |
| Notebook rename | Double-click pill or ⋮ menu | Inline edit |
| Notebook delete | ⋮ menu → Delete | Moves entries to "No Notebook", soft-deletes notebook |
| Search bar | Type (300ms debounce) | FTS5 MATCH query. Highlights matches in results. |
| Filter: emotions | Multi-select dropdown | Filters via SQL JOIN on `journal_emotions_flat` |
| Filter: tags | Multi-select dropdown | Filters via `json_each(tags)` |
| Filter: date range | Date range picker | `created_at BETWEEN` |
| Sort | Dropdown: newest/oldest/word count | Changes ORDER BY |
| Entry list row | Click | Loads entry into editor |
| Entry list row | Hover | Shows full title tooltip if truncated |
| Virtual scroll | Auto | VirtualList: 72px rows, max 30 DOM nodes + spacers (D-45) |
| Drag to reorder | Not supported | Sort is always by the sort field |

**Entry Editor Panel (right)**

| Feature | Interaction | Behaviour |
|---------|------------|----------|
| Title field | Type | Optional. Large single line. |
| Emotion picker | Click chips | Multi-select, 0–N emotions. 10 values. |
| Notebook dropdown | Select | Assigns entry to notebook |
| Tag chip input | Type comma/Enter | Autocomplete from existing tags. Remove with ×. |
| Word count | Auto | Live count, updates every keystroke |
| Content textarea | Type | Plain text only. Monospace font. No Markdown. |
| Auto-save | 2s idle debounce | Calls `journal_update`. Does NOT trigger analysis. |
| Explicit save | Ctrl+S | Calls `journal_update` then `trigger_journal_analysis`. |
| "+ New Entry" button | Click | Auto-saves current entry silently, opens blank (D-88) |
| "Analyse with AI" button | Click | Injects current entry into AI sidebar with analysis prompt |
| Move to Trash | Button + confirm dialog | Sets `is_deleted = 1` |
| Footer: last analysed | Display | "Last analysed: 2 hours ago" + link to conversation |
| Footer: timestamps | Display | "Created Mon 15 Mar" · "Edited 10 minutes ago" |

**Analysis Card (in AI Sidebar — fires on explicit save)**

| Feature | Interaction | Behaviour |
|---------|------------|----------|
| IDLE state | — | Card not visible |
| QUEUED state | Fires immediately on Ctrl+S | Card shows "Analysis scheduled…" (D-106) |
| PENDING state | Fires after 3s debounce | Spinner: "Analysing your entry…" |
| STREAMING state | Tokens arrive | Narrative text renders live |
| REVIEW state | Analysis complete | Shows narrative + proposed tasks |
| Task checkbox | Click | Toggles inclusion in save |
| Task inline edit [✏] | Click | Expands: title, priority, due date, tags editable |
| "Save selected (N)" | Click | Calls `confirm_proposed_tasks`, creates tasks, card resolves |
| "Dismiss all" | Click | Calls `dismiss_proposed_tasks`, card resolves, no tasks created |
| Queue | Auto | If card is in REVIEW and user saves another entry, new analysis queues |

---

### 7B. Tasks Tab

**Calendar/Agenda View (default — D-12)**

| Feature | Interaction | Behaviour |
|---------|------------|----------|
| Week navigation | < Prev / Today / Next > | Shifts week strip |
| Day column | View | Shows tasks for that day, sorted priority DESC + due_time ASC |
| Overdue section | Auto | Pinned above calendar in red when overdue tasks exist |
| No-date section | Auto | Pinned below calendar |
| Task count badge | Display | Shows count per day in week strip |
| Quick-add | Click day header, type, Enter | Creates task with that due date |
| Drag no-date task | Drag to day column | Calls `task_assign_date`, assigns immediately (D-85) |
| Task card click | Click | Opens Task Detail Panel |
| Inline status | Click badge | Cycles status: todo → in_progress → done |
| View toggle | Button group | Switch to List or Kanban |

**List View**

| Feature | Interaction | Behaviour |
|---------|------------|----------|
| Columns | Display | Title, status, priority, due date, project, energy, context, tags |
| Column sort | Click header | Sorts by that column, toggles ASC/DESC |
| Filter panel | Dropdown controls | Status, priority, project, category, energy (multi), context (multi), tags (multi), date range |
| Multi-select | Checkbox column | Select multiple tasks |
| Bulk status | Select + dropdown | Changes status on all selected tasks |
| Quick-add bar | Type + Enter | Creates task (no due date) |
| Task row click | Click | Opens Task Detail Panel |

**Kanban View**

| Feature | Interaction | Behaviour |
|---------|------------|----------|
| 4 columns | Display | Todo · In Progress · Done · Cancelled |
| Card | Display | Title, priority dot, due date, energy badge, context badge, tag chips, ⏱ time, 🔴 blocker indicator |
| Drag between columns | Drag | Updates task `status` immediately |
| Card click | Click | Opens Task Detail Panel |

**Task Detail Panel (slide-in from right, overlays right sidebar temporarily)**

| Section | Fields | Features |
|---------|--------|---------|
| Core | Title (required), description, status, priority | Inline edit all fields |
| Time | Due date, due time, reminder, estimate (min), logged (min) | Flatpickr date/time pickers |
| Timer | Start/Stop button, session list | `timer_start`, `timer_stop`. Guards: not on done/cancelled tasks (D-107 fix) |
| Manual time log | Duration + note + date | `time_log_manual` |
| Organisation | Project (autocomplete), category (autocomplete), tags, labels | Chip inputs |
| Energy & Context | Dropdown (4 energy values), dropdown (5 context values) | |
| Reference | URL field + 🔗 Open button | Opens in default browser via `tauri-plugin-shell` |
| Recurrence | Type dropdown + config | daily/weekly/monthly/custom. Shows next occurrence. |
| Notes | Textarea | Additional plain text |
| Subtasks | List + "+ Add subtask" | Max depth 2 (parent → subtask). Subtask has same status/priority fields. |
| Dependencies | "Blocked by" list + "Blocks" list | "+ Add blocker" search. Max 10 per task. No circular deps. |
| Attachments | File list + "+ Add file" | `attachment_add` opens dialog. Relative paths (D-47). No size limit (D-87). ⚠️ if `file_missing`. |
| History | Audit trail | Every create/update/delete with actor (user/AI) and field diff |
| Soft delete | "Move to Trash" + confirm | Sets `is_deleted = 1` on parent + all subtasks |

---

### 7C. AI Tab

**Conversation List (left 260px panel)**

| Feature | Interaction | Behaviour |
|---------|------------|----------|
| Conversation rows | Click | Loads into chat window |
| Source filter tabs | Click | All · Sidebar · AI Tab · Analysis · Weekly Plan (maps to DB values — §8C) |
| Search bar | Type | `conversation_search` across message content |
| "+ New" button | Click | Opens blank chat in AI tab |
| Title | Click to rename | Inline edit. Auto-generated from first message. |
| 🗑 delete | Click | Soft-deletes conversation |
| "View source entry" | Link (analysis only) | Switches to Journal tab, loads that entry |

**Chat Window**

| Feature | Interaction | Behaviour |
|---------|------------|----------|
| Message bubbles | Display | User=right, assistant=left, tool=card block |
| Streaming response | Auto | Live token append + blinking cursor |
| "Stop generating" | Button (during stream) | Cancels current Ollama request (releases Mutex) |
| Tool call card | Collapsible | Shows tool name, args, result, confirmed/cancelled status |
| Confirmation card | YES/NO buttons | D-19 confirmation flow. 300s timeout auto-cancels (D-95). |
| Code blocks | Auto | highlight.js syntax colouring |
| Context panel | Collapsible at top | Shows what smart inject included: "📓 3 entries injected · ✅ 12 overdue tasks" |
| Conversation search | Search bar | Filters by message content |

**Model Selector (in AI sidebar header)**

| Feature | Behaviour |
|---------|----------|
| Dropdown | Populated from `ollama_list_models`. Shows all locally installed models. |
| Change model (no active conversation) | Updates `app_settings.ollama_model`. No dialog. |
| Change model (mid-conversation) | Warning dialog: "Switching starts a new conversation. Continue?" (D-84) |
| Model missing | Error card: "Model X not found. Select a replacement." + refreshed dropdown (D-37) |

---

### 7D. Reports Tab

**Shared Controls (all 6 reports)**

| Feature | Interaction | Behaviour |
|---------|------------|----------|
| Date range presets | Click: 7d / 30d / 90d | Runs report for that period |
| Custom range | Two Flatpickr inputs + Go | Runs report for custom period |
| Range persists | Auto | Each report's range saved to `app_settings` |
| Export PDF | Button | `window.print()` with `@media print` CSS |
| Export Markdown | Button | Save dialog → `.md` file |
| Regenerate | Button | Re-runs AI for narrative/suggestions/insights |

**Report 1 — Task Summary**

| Widget | Data source |
|--------|------------|
| Status doughnut chart | `tasks.status` counts |
| Priority bar chart | `tasks.priority` counts |
| Project bar chart (top 10) | Grouped by `tasks.project` |
| Stats grid | Total · Done this week · Overdue · Due today · Avg completion days |
| Overdue table | Top 10 overdue: title, due date, priority, days overdue |
| AI Extraction widget | From `proposed_task_log`: proposed / accepted / dismissed / acceptance rate |

**Report 2 — Weekly Review (auto Monday 08:00 + manual)**

| Widget | Data source |
|--------|------------|
| Stats row | Tasks completed · created · overdue · journal entries · words written |
| AI Narrative | Ollama-generated 2–3 paragraph summary (Marked.js rendered) |
| Weekly Plan sub-section | Streamed from `generate_weekly_plan` |
| Upcoming week table | Tasks due in next 7 days, grouped by day |
| "Regenerate" button | Re-runs AI narrative |
| Last generated timestamp | Shows when auto-generated |

**Report 3 — Time Tracking**

| Widget | Data source |
|--------|------------|
| Daily hours line chart | `time_logs.started_at` + `duration` grouped by day |
| Project bar chart (top 10) | `time_logs` JOINed to `tasks.project` |
| Sessions table | task title · estimated · logged · delta (over/under) |
| Total this period | Sum of `time_logs.duration` |

**Report 4 — Journal Stats**

| Widget | Data source |
|--------|------------|
| Entry frequency bar chart | Count per day |
| Writing streak | Consecutive days with ≥1 entry — current + longest |
| Emotion stacked area chart | `journal_emotions_flat` view |
| Emotion frequency table | emotion · count · % · trend (↑↓→) |
| Word count trend area chart | `word_count` per entry over time |
| Notebooks breakdown pie | Entries per notebook |
| Top tags bar chart (15) | `json_each(tags)` counts |

**Report 5 — AI Suggestions**

| Widget | Behaviour |
|--------|----------|
| 5–8 suggestion cards | Each: title + 2–3 sentence explanation |
| "Create Task" button per card | Sends to AI sidebar confirmation flow |
| "Regenerate" button | Calls `get_ai_suggestions_report` with current date range |

**Report 6 — Energy & Focus**

| Widget | Data source |
|--------|------------|
| Completion by energy level doughnut | `tasks.energy_level` WHERE `status='done'` |
| Time-of-day completion heatmap (bar) | Hour from `time_logs.started_at` |
| Energy by day-of-week grouped bar | `strftime('%w', started_at)` + `energy_level` |
| Context tag breakdown | `tasks.context_tag` counts |
| AI insight paragraph | Ollama-generated summary at bottom |
| Graceful empty state | "Add energy levels to tasks to see patterns here" |

---

### 7E. Settings Tab

**Section 1 — AI Configuration**

| Control | Type | Behaviour |
|---------|------|----------|
| Ollama base URL | Text input | Default: `http://localhost:11434` |
| Test connection | Button | Pings Ollama, shows result in terminal |
| Active model | Dropdown | Populated from `ollama_list_models` |
| Temperature | Slider 0.0–2.0 | Step 0.1, live value display |
| Top-P | Slider 0.0–1.0 | Step 0.05 |
| Top-K | Number input | Integer 1–200 |
| Context length | Select | 4096 / 8192 / 16384 / 32768. RAM warning shown. |
| System prompt override | Textarea | Empty = use built-in; filled = replaces built-in |
| Reset to defaults | Button | Restores all AI settings to seed values |

**Section 2 — Database**

| Control | Behaviour |
|---------|----------|
| DB path display | Shows resolved path + "Open folder" button |
| DB size | Live display |
| Manual backup | Frontend opens save dialog → passes path to `backup_manual(path)` (D-98, D-99) |
| Restore from backup | File open dialog → `PRAGMA integrity_check` → safety copy → overwrite (D-31, D-98) |
| Auto-backup triggers | Three toggles: "On schedule" · "On open if stale" · "On close" (D-99) |
| Auto-backup interval | Select: hourly / 6h / daily / weekly |
| Backup directory | Path input + Browse |
| Keep last N | Number input (default 10) |
| Export JSON | Save dialog → `export_json` |
| Export ZIP | Save dialog → `export_zip` (JSON + attachments) |
| Import JSON | File open → mode select (Replace/Merge) → `import_json` → summary toast |
| Backup list | Table: filename · date · size · [Restore] [Delete] |
| Audit log row count | Display + "Archive now" button |

**Section 3 — SQL Runner**

| Feature | Behaviour |
|---------|----------|
| Query textarea | Multi-line SQL input |
| Read-only toggle | ON by default. OFF = full INSERT/UPDATE/DELETE/DROP access. (D-90) |
| Execute button | Runs query, shows results |
| Results table | Scrollable, max 500 rows |
| Query history | Last 20, selectable from dropdown |
| All queries logged | To terminal (when visible) + log viewer |

**Section 4 — Log Viewer**

| Feature | Behaviour |
|---------|----------|
| Level filter | ALL / INFO / WARN / ERROR / AI / DB / TOOL |
| Text search | Filters log lines |
| Export | Saves filtered view to `.log` file |
| Clear | Clears in-memory log (not DB audit_log) |
| Available | Regardless of terminal bar visibility (D-79) |

**Section 5 — Trash**

| Feature | Behaviour |
|---------|----------|
| Sub-tabs | Journal Entries · Tasks · Notebooks |
| Task rows | Parent task name + "Includes N subtasks" note (D-82) |
| Restore | Calls `task_restore` → restores parent + all subtasks (D-102) |
| Delete Permanently | Calls `task_purge` → hard DELETE + attachment file removal (file-exists checked) |
| "Empty Trash" | Confirmation dialog (required) → `trash_empty` → all soft-deleted + attachment files removed |

**Section 6 — Appearance & System**

| Control | Behaviour |
|---------|----------|
| Theme toggle | Dark / Light — applies `data-theme` on `body` |
| Font size | Small / Medium / Large — applies `data-size` on `body` |
| Show terminal bar | Toggle (default OFF) — D-79 |
| Terminal height | Slider 120–400px (shown when terminal enabled) |
| Always on top | Toggle |
| Weekly report time | Time input (day fixed Monday) |
| On window close | Dropdown: Exit / Minimise to tray |
| Launch at Windows startup | Toggle (default OFF) |
| About section | App version · Tauri version · WebView2 version · Ollama version · Security advisory |
| Keyboard shortcuts | Reference table |

---

### 7F. Floating Toast System

| Toast type | Trigger | Content | Action |
|-----------|---------|---------|--------|
| Task reminder | `ReminderScheduler` fires | Task title + due time | "View Task" → Tasks tab |
| Orphaned timer | Startup detection | Task title + started time | "Resolve" → timer dialog |
| Backup saved | Any backup completes | File path + size | — |
| Import complete | `import_json` finishes | Summary counts (imported/skipped) | — |
| Analysis failed | Pipeline error | Error detail | "Retry" → re-triggers |
| Weekly report ready | Monday scheduler | Week date range | "View Report" → Reports tab |
| AI confirm timeout | 300s elapsed | "Action timed out — not applied" | — |
| Model missing | Ollama 404 | Model name | "Settings" → model selector |

---

## 8. UNIFIED IPC CONTRACT

**The complete mapping between every frontend `invoke()` call and its Rust handler.**
Every handler returns `Result<T, AppError>`. Frontend always handles both Ok and Err branches.

### 8A. Frontend → Backend (invoke calls)

#### Notebooks

| Frontend call | Rust handler | Parameters | Returns |
|--------------|-------------|-----------|---------|
| `invoke('notebook_create', p)` | `notebook_create` | `name, description?, color?` | `Notebook` |
| `invoke('notebook_list')` | `notebook_list` | — | `Vec<Notebook>` |
| `invoke('notebook_update', p)` | `notebook_update` | `id, name?, description?, color?, sort_order?` | `Notebook` |
| `invoke('notebook_delete', p)` | `notebook_delete` | `id` | `()` |
| `invoke('notebook_restore', p)` | `notebook_restore` | `id` | `Notebook` |
| `invoke('notebook_purge', p)` | `notebook_purge` | `id` | `()` |

#### Journal

| Frontend call | Rust handler | Parameters | Returns |
|--------------|-------------|-----------|---------|
| `invoke('journal_create', p)` | `journal_create` | `title?, content, notebook_id?, emotions: Vec<String>, tags: Vec<String>` | `JournalEntry` |
| `invoke('journal_update', p)` | `journal_update` | `id, title?, content?, notebook_id?, emotions?, tags?` | `JournalEntry` |
| `invoke('journal_get', p)` | `journal_get` | `id` | `JournalEntry` |
| `invoke('journal_list', p)` | `journal_list` | `notebook_id?, sort, limit, cursor?: String` | `Vec<JournalSummary>` |
| `invoke('journal_search', p)` | `journal_search` | `query, emotions?: Vec<String>, date_from?, date_to?, limit` | `Vec<JournalSearchResult>` |
| `invoke('journal_soft_delete', p)` | `journal_soft_delete` | `id` | `()` |
| `invoke('journal_restore', p)` | `journal_restore` | `id` | `JournalEntry` |
| `invoke('journal_purge', p)` | `journal_purge` | `id` | `()` |
| `invoke('journal_get_all_tags')` | `journal_get_all_tags` | — | `Vec<String>` |
| `invoke('trigger_journal_analysis', p)` | `trigger_journal_analysis` | `entry_id` | `()` (async fire-and-forget) |
| `invoke('confirm_proposed_tasks', p)` | `confirm_proposed_tasks` | `proposed: Vec<ProposedTaskInput>, conversation_id, source_entry_id` | `Vec<Task>` |
| `invoke('dismiss_proposed_tasks', p)` | `dismiss_proposed_tasks` | `proposed: Vec<ProposedTaskInput>, conversation_id, source_entry_id` | `()` |

#### Tasks

| Frontend call | Rust handler | Parameters | Returns |
|--------------|-------------|-----------|---------|
| `invoke('task_create', p)` | `task_create` | `title, description?, status?, priority?, due_date?, due_time?, reminder_at?, time_estimate?, tags?, labels?, category?, project?, notes?, recurrence?, energy_level?, context_tag?, linked_url?, parent_task_id?, ai_created: bool, ai_conversation_id?` | `Task` |
| `invoke('task_update', p)` | `task_update` | `id, fields: TaskUpdatePayload` | `Task` |
| `invoke('task_get', p)` | `task_get` | `id` | `TaskDetail` |
| `invoke('task_list', p)` | `task_list` | `status?, priority?, project?, category?, energy_levels?: Vec<String>, context_tags?: Vec<String>, tags?, due_before?, due_after?, include_no_date, parent_only, sort, limit, cursor?` | `Vec<TaskSummary>` |
| `invoke('task_list_for_date', p)` | `task_list_for_date` | `date: String (ISO 8601)` | `Vec<TaskSummary>` |
| `invoke('task_list_overdue')` | `task_list_overdue` | — | `Vec<TaskSummary>` |
| `invoke('task_list_upcoming')` | `task_list_upcoming` | — | `Vec<TaskSummary>` |
| `invoke('task_assign_date', p)` | `task_assign_date` | `id, due_date` | `Task` |
| `invoke('task_bulk_update', p)` | `task_bulk_update` | `ids: Vec<String>, status` | `Vec<Task>` |
| `invoke('task_soft_delete', p)` | `task_soft_delete` | `id` | `()` |
| `invoke('task_restore', p)` | `task_restore` | `id` | `Task` |
| `invoke('task_purge', p)` | `task_purge` | `id` | `()` |
| `invoke('task_get_all_tags')` | `task_get_all_tags` | — | `Vec<String>` |
| `invoke('task_get_all_projects')` | `task_get_all_projects` | — | `Vec<String>` |
| `invoke('task_get_all_categories')` | `task_get_all_categories` | — | `Vec<String>` |
| `invoke('task_add_dependency', p)` | `task_add_dependency` | `blocked_task_id, blocking_task_id` | `()` |
| `invoke('task_remove_dependency', p)` | `task_remove_dependency` | `blocked_task_id, blocking_task_id` | `()` |
| `invoke('timer_start', p)` | `timer_start` | `task_id` | `TimeLog` |
| `invoke('timer_stop', p)` | `timer_stop` | `time_log_id, note?` | `TimeLog` |
| `invoke('timer_resolve_orphan', p)` | `timer_resolve_orphan` | `time_log_id, ended_at, note?` | `TimeLog` |
| `invoke('timer_discard_orphan', p)` | `timer_discard_orphan` | `time_log_id` | `()` |
| `invoke('time_log_manual', p)` | `time_log_manual` | `task_id, duration, note?, date?` | `TimeLog` |
| `invoke('attachment_add', p)` | `attachment_add` | `task_id` (dialog opened in JS first, path passed) | `TaskAttachment` |
| `invoke('attachment_remove', p)` | `attachment_remove` | `id` | `()` |
| `invoke('attachment_open', p)` | `attachment_open` | `id` | `()` |

#### AI

| Frontend call | Rust handler | Parameters | Returns |
|--------------|-------------|-----------|---------|
| `invoke('ai_chat', p)` | `ai_chat` | `conversation_id?: String, message, model, tab_context` | `String` (conversation_id — D-105) |
| `invoke('ai_confirm', p)` | `ai_confirm` | `call_id: String, confirmed: bool` | `()` |
| `invoke('conversation_list', p)` | `conversation_list` | `source?: String, limit, offset` | `Vec<ConversationSummary>` |
| `invoke('conversation_get', p)` | `conversation_get` | `id` | `AiConversationDetail` |
| `invoke('conversation_rename', p)` | `conversation_rename` | `id, title` | `()` |
| `invoke('conversation_delete', p)` | `conversation_delete` | `id` | `()` |
| `invoke('conversation_search', p)` | `conversation_search` | `query` | `Vec<ConversationSearchResult>` |
| `invoke('conversation_switch_model', p)` | `conversation_switch_model` | `current_conversation_id?: String, new_model` | `Option<AiConversation>` (D-103) |
| `invoke('continue_analysis_conversation', p)` | `continue_analysis_conversation` | `conversation_id, message` | `()` (streams) |

#### Reports

| Frontend call | Rust handler | Parameters | Returns |
|--------------|-------------|-----------|---------|
| `invoke('get_task_summary_report', p)` | `get_task_summary_report` | `date_from, date_to` | `TaskSummaryReport` |
| `invoke('get_weekly_review_report', p)` | `get_weekly_review_report` | `week_start` | `WeeklyReviewReport` |
| `invoke('regenerate_weekly_review', p)` | `regenerate_weekly_review` | `week_start` | `()` (streams) |
| `invoke('generate_weekly_plan', p)` | `generate_weekly_plan` | `week_start` | `()` (streams) |
| `invoke('get_time_tracking_report', p)` | `get_time_tracking_report` | `date_from, date_to` | `TimeTrackingReport` |
| `invoke('get_journal_stats_report', p)` | `get_journal_stats_report` | `date_from, date_to` | `JournalStatsReport` |
| `invoke('get_ai_suggestions_report', p)` | `get_ai_suggestions_report` | `date_from, date_to` | `()` (streams) |
| `invoke('get_energy_focus_report', p)` | `get_energy_focus_report` | `date_from, date_to` | `EnergyFocusReport` |
| `invoke('proposed_task_stats', p)` | `proposed_task_stats` | `date_from, date_to` | `ProposedTaskStats` |
| `invoke('get_emotion_frequencies', p)` | `get_emotion_frequencies` | `date_from, date_to` | `Vec<EmotionFrequency>` |
| `invoke('get_emotion_trend', p)` | `get_emotion_trend` | `date_from, date_to, bucket` | `Vec<EmotionTrendPoint>` |

#### Settings & System

| Frontend call | Rust handler | Parameters | Returns |
|--------------|-------------|-----------|---------|
| `invoke('setting_get', p)` | `setting_get` | `key` | `Option<JsonValue>` |
| `invoke('setting_set', p)` | `setting_set` | `key, value: JsonValue` | `()` |
| `invoke('settings_get_all')` | `settings_get_all` | — | `HashMap<String, JsonValue>` |
| `invoke('settings_reset_ai')` | `settings_reset_ai` | — | `()` |
| `invoke('get_schema_version')` | `get_schema_version` | — | `SchemaVersion` |
| `invoke('backup_manual', p)` | `backup_manual` | `path: String` (from JS dialog) | `BackupResult` |
| `invoke('backup_restore', p)` | `backup_restore` | `path: String, mode: String` | `()` |
| `invoke('export_json')` | `export_json` | — (JS opens dialog first) | `()` |
| `invoke('export_zip')` | `export_zip` | — | `()` |
| `invoke('import_json', p)` | `import_json` | `path: String, mode: 'replace'|'merge'` | `ImportReport` |
| `invoke('audit_stats')` | `audit_stats` | — | `AuditStats` |
| `invoke('audit_manual_archive')` | `audit_manual_archive` | — | `ArchiveResult` |
| `invoke('trash_list')` | `trash_list` | — | `Vec<TrashItem>` |
| `invoke('trash_empty')` | `trash_empty` | — (JS confirms first) | `TrashEmptyResult` |
| `invoke('set_autostart', p)` | `set_autostart` | `enabled: bool` | `()` |
| `invoke('ollama_health_check')` | `ollama_health_check` | — | `OllamaHealth` |
| `invoke('ollama_list_models')` | `ollama_list_models` | — | `Vec<OllamaModel>` |
| `invoke('sql_execute', p)` | `sql_execute` | `query: String, readonly: bool` | `SqlResult` |
| `invoke('toggle_terminal', p)` | `toggle_terminal` | `visible: bool` | `()` |

### 8B. `ProposedTaskInput` — Fully Defined (D-107)

```rust
pub struct ProposedTaskInput {
    pub accepted:         bool,
    pub original_title:   String,
    pub original_data:    String,         // full JSON of AI proposal (for log)
    pub source_text:      Option<String>, // excerpt from journal entry
    pub edited_title:     Option<String>,
    pub edited_priority:  Option<String>, // 'low'|'medium'|'high'|'urgent'
    pub edited_due_date:  Option<String>, // ISO 8601 YYYY-MM-DD
    pub edited_tags:      Option<Vec<String>>,
    pub edited_notes:     Option<String>,
}
// outcome: accepted (no edits), edited (any edit_* field set), dismissed (!accepted)
```

### 8C. Conversation Source Label Map (Mismatch-05 fix)

```javascript
// ai.js — canonical mapping between UI labels and DB values
const CONVERSATION_SOURCE_MAP = {
    'All':          null,           // no filter
    'Sidebar':      'sidebar',
    'AI Tab':       'ai_tab',       // UI "AI Tab" ≠ DB 'ai_tab' — always use this map
    'Analysis':     'analysis',
    'Weekly Plan':  'weekly_plan',
};
```

---

## 9. TYPED EVENT SYSTEM

### 9A. `AppEvent` Enum (D-96)

Single Rust enum. Single emit channel `"app_event"`. JS dispatches by `payload.type`.

```rust
// src-tauri/src/events.rs
#[derive(Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AppEvent {

    // ── AI STREAMING ──────────────────────────────────────────────────
    AiToken {
        token:  String,
        done:   bool,
        source: AiTokenSource,    // distinguishes chat vs analysis vs report
    },
    AiToolPending {
        call_id:     String,
        name:        String,
        args:        serde_json::Value,
        description: String,
    },
    AiToolResult {
        call_id:   String,
        name:      String,
        result:    serde_json::Value,
        confirmed: bool,
    },

    // ── JOURNAL ANALYSIS ──────────────────────────────────────────────
    JournalAnalysisQueued {             // fires immediately on trigger call (D-106)
        entry_id: String,
    },
    JournalAnalysisStarted {            // fires after 3s debounce
        entry_id:    String,
        entry_title: Option<String>,
    },
    JournalAnalysisComplete {
        entry_id:            String,
        conversation_id:     String,
        proposed_tasks:      Vec<ProposedTask>,
        narrative:           String,
        follow_up_questions: Vec<String>,
    },
    JournalAnalysisFailed {
        entry_id: String,
        reason:   String,
    },

    // ── TASKS ─────────────────────────────────────────────────────────
    ReminderFired {
        task_id:    String,
        task_title: String,
        due_date:   String,
    },
    OrphanedTimers {
        timers: Vec<OrphanedTimer>,
    },

    // ── SYSTEM ────────────────────────────────────────────────────────
    ModelMissing {
        missing:   String,
        available: Vec<String>,
    },
    BackupCompleted {
        path:       String,
        size_bytes: u64,
    },
    WeeklyReportReady {
        week_start: String,
    },
    AiConfirmTimeout {
        call_id:   String,
        tool_name: String,
    },
    LogEvent {
        timestamp: String,
        level:     LogLevel,
        source:    LogSource,
        message:   String,
    },
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum AiTokenSource {
    Chat,
    JournalAnalysis,
    WeeklyReport,
    AiSuggestions,
}

// Single emit helper — replaces all scattered emit() calls
pub fn emit(app: &AppHandle, event: AppEvent) {
    app.emit("app_event", event).ok();
}
```

### 9B. Frontend Event Dispatcher

```javascript
// app.js — single listener, dispatches by type
import { listen } from '@tauri-apps/api/event';

const handlers = {
    ai_token:                     handleAiToken,
    ai_tool_pending:              handleToolPending,
    ai_tool_result:               handleToolResult,
    journal_analysis_queued:      handleAnalysisQueued,
    journal_analysis_started:     handleAnalysisStarted,
    journal_analysis_complete:    handleAnalysisComplete,
    journal_analysis_failed:      handleAnalysisFailed,
    reminder_fired:               handleReminder,
    orphaned_timers:              handleOrphanedTimers,
    model_missing:                handleModelMissing,
    backup_completed:             handleBackupCompleted,
    weekly_report_ready:          handleWeeklyReportReady,
    ai_confirm_timeout:           handleConfirmTimeout,
    log_event:                    handleLogEvent,
};

listen('app_event', (event) => {
    const handler = handlers[event.payload.type];
    if (handler) {
        handler(event.payload);
    } else {
        console.warn('[app_event] Unhandled type:', event.payload.type);
    }
});
```

---

## 10. DATABASE INITIALISATION & PRAGMAS

**File:** `src-tauri/src/db/init.rs`

```sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = NORMAL;
PRAGMA busy_timeout = 5000;
PRAGMA temp_store = MEMORY;
```

**Startup sequence (every launch):**
1. `resolve_data_dir()` — MSI vs portable (D-29)
2. `apply_pragmas(conn)` — WAL + foreign keys + busy timeout + log `PRAGMA compile_options`
3. `run_migrations(conn)` — all files wrapped in `BEGIN/COMMIT` (D-97)
4. `seed_settings(conn)` — INSERT OR IGNORE defaults (context default = 16384, D-93)
5. `recover_orphaned_timers(conn)` — detect NULL ended_at, emit `OrphanedTimers` (D-43)
6. `backup_if_stale(conn, data_dir)` — async backup if last backup > interval (D-99)
7. `ollama_health_check()` — async, non-blocking (D-86); RAM check via `sysinfo` (D-93)
8. `emit(app, AppEvent::StartupComplete)` — frontend opens Journal tab

**Backup on close (D-99):**
```rust
WindowEvent::CloseRequested { api, .. } => {
    if get_setting_sync("auto_backup_enabled").unwrap_or(true) {
        let _ = backup_to_timestamped_file(&data_dir);  // SQLite Backup API, synchronous
    }
    let behaviour = get_setting_sync("close_behaviour").unwrap_or("exit".into());
    if behaviour == "tray" { api.prevent_close(); window.hide().ok(); }
}
```

**Migration files (all transaction-wrapped — D-97):**
```
src-tauri/src/migrations/
  001_initial.sql             BEGIN; ... COMMIT;
  002_fts_triggers.sql        BEGIN; ... COMMIT;  (includes 4 triggers: insert, update, delete + soft_delete + restore from D-100)
  003_analysis_tracking.sql   BEGIN; ... COMMIT;
  004_field_expansion.sql     BEGIN; ... COMMIT;
  005_fts_softdelete.sql      BEGIN; ... COMMIT;  (soft-delete + restore triggers if not in 002)
```

---

## 11. DATABASE SCHEMA

### 11A. `schema_migrations`
```sql
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY, name TEXT NOT NULL, applied_at TEXT NOT NULL
);
```

### 11B. `notebooks`
```sql
CREATE TABLE notebooks (
  id TEXT PRIMARY KEY, name TEXT NOT NULL UNIQUE, description TEXT,
  color TEXT, sort_order INTEGER DEFAULT 0,
  created_at TEXT NOT NULL, updated_at TEXT NOT NULL, is_deleted INTEGER DEFAULT 0
);
```

### 11C. `journal_entries`
```sql
CREATE TABLE journal_entries (
  id TEXT PRIMARY KEY,
  notebook_id TEXT REFERENCES notebooks(id) ON DELETE SET NULL,
  title TEXT, content TEXT NOT NULL,
  emotions TEXT DEFAULT '[]',         -- JSON array, 10 named values
  tags TEXT DEFAULT '[]',
  word_count INTEGER DEFAULT 0,
  last_analysis_conv_id TEXT,
  last_analysed_at TEXT,
  created_at TEXT NOT NULL,           -- immutable after INSERT (D-08)
  updated_at TEXT NOT NULL,
  is_deleted INTEGER DEFAULT 0
);
CREATE INDEX idx_journal_notebook ON journal_entries(notebook_id);
CREATE INDEX idx_journal_created  ON journal_entries(created_at);
CREATE INDEX idx_journal_deleted  ON journal_entries(is_deleted);
```

**Emotion values:** `anxious` `frustrated` `sad` `overwhelmed` `neutral` `calm` `energised` `grateful` `happy` `focused`

### 11D. `journal_fts` (Virtual Table)
```sql
CREATE VIRTUAL TABLE journal_fts USING fts5(
  id UNINDEXED, title, content,
  content='journal_entries', content_rowid='rowid',
  tokenize='unicode61 remove_diacritics 0'
);
-- 5 triggers: insert, update, delete, soft_delete (is_deleted→1), restore (is_deleted→0)
```

### 11E. `journal_emotions_flat` (View)
```sql
CREATE VIEW journal_emotions_flat AS
SELECT je.id AS entry_id, je.created_at AS entry_date,
  je.notebook_id, je.word_count, json_each.value AS emotion
FROM journal_entries je, json_each(je.emotions)
WHERE je.is_deleted = 0;
```

### 11F. `tasks`
```sql
CREATE TABLE tasks (
  id TEXT PRIMARY KEY,
  parent_task_id TEXT REFERENCES tasks(id) ON DELETE CASCADE,
  title TEXT NOT NULL,
  description TEXT,
  status TEXT NOT NULL DEFAULT 'todo'
    CHECK(status IN ('todo','in_progress','done','cancelled')),
  priority TEXT NOT NULL DEFAULT 'medium'
    CHECK(priority IN ('low','medium','high','urgent')),
  due_date TEXT, due_time TEXT,
  reminder_at TEXT, reminder_fired INTEGER DEFAULT 0,
  time_estimate INTEGER, time_logged INTEGER DEFAULT 0,
  actual_start_date TEXT,
  tags TEXT DEFAULT '[]', labels TEXT DEFAULT '[]',
  category TEXT, project TEXT, notes TEXT,
  recurrence TEXT, next_occurrence TEXT,
  energy_level TEXT CHECK(energy_level IN ('deep_focus','light','admin','errand') OR energy_level IS NULL),
  context_tag TEXT CHECK(context_tag IN ('computer','phone','errands','home','anywhere') OR context_tag IS NULL),
  linked_url TEXT,
  sort_order INTEGER DEFAULT 0,
  ai_created INTEGER DEFAULT 0, ai_conversation_id TEXT,
  created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
  completed_at TEXT, is_deleted INTEGER DEFAULT 0
);
-- 11 indexes including partial indexes for energy, context, upcoming
```

### 11G. `task_dependencies`
```sql
CREATE TABLE task_dependencies (
  id TEXT PRIMARY KEY,
  blocked_task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  blocking_task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  created_at TEXT NOT NULL,
  UNIQUE(blocked_task_id, blocking_task_id),
  CHECK(blocked_task_id != blocking_task_id)
);
```
Rules: no self-reference, no circular deps, max 10 per task, user-managed only (not AI tools).

### 11H. `task_attachments`
```sql
CREATE TABLE task_attachments (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  file_name TEXT NOT NULL,
  file_path TEXT NOT NULL,    -- RELATIVE: "attachments/<task_id>/<filename>"
  mime_type TEXT, size_bytes INTEGER,
  file_missing INTEGER DEFAULT 0,  -- set 1 if file not found at path
  created_at TEXT NOT NULL
);
```
No size limit (D-87). Paths are relative (D-47). Resolved to absolute at runtime.

### 11I. `time_logs`
```sql
CREATE TABLE time_logs (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  started_at TEXT NOT NULL, ended_at TEXT,   -- NULL = running or orphaned
  duration INTEGER,                           -- minutes, NULL if open
  note TEXT, created_at TEXT NOT NULL
);
```

### 11J. `ai_conversations`
```sql
CREATE TABLE ai_conversations (
  id TEXT PRIMARY KEY, title TEXT,
  model TEXT NOT NULL,
  source TEXT NOT NULL DEFAULT 'sidebar'
    CHECK(source IN ('sidebar','ai_tab','analysis','weekly_plan')),
  source_entry_id TEXT REFERENCES journal_entries(id) ON DELETE SET NULL,
  created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
  is_deleted INTEGER DEFAULT 0
);
```

### 11K. `ai_messages`
```sql
CREATE TABLE ai_messages (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL REFERENCES ai_conversations(id) ON DELETE CASCADE,
  role TEXT NOT NULL CHECK(role IN ('user','assistant','tool','system')),
  content TEXT NOT NULL,
  tool_name TEXT, tool_args TEXT, tool_result TEXT,
  confirmed INTEGER,        -- NULL=N/A, 1=confirmed, 0=cancelled
  model TEXT, tokens_in INTEGER, tokens_out INTEGER,
  created_at TEXT NOT NULL
);
```

### 11L. `app_settings`
```sql
CREATE TABLE app_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT NOT NULL);
```

**Seed defaults (D-93 updates context to 16384):**

| Key | Default | Change in v1.6 |
|-----|---------|---------------|
| `ollama_context_len` | `16384` | **Updated from 4096** |
| `auto_backup_on_open` | `true` | **New** |
| `auto_backup_on_close` | `true` | **New** |
| `timezone_policy` | `"local"` | **New** |
| All others | unchanged | — |

### 11M. `audit_log`
```sql
CREATE TABLE audit_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  action TEXT NOT NULL CHECK(action IN ('create','update','delete','restore')),
  entity TEXT NOT NULL, entity_id TEXT NOT NULL,
  actor TEXT NOT NULL CHECK(actor IN ('user','ai')),
  changes TEXT, ai_conv_id TEXT, created_at TEXT NOT NULL
);
```
Auto-archive: 100k rows or 365 days (D-32).

### 11N. `proposed_task_log`
```sql
CREATE TABLE proposed_task_log (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL REFERENCES ai_conversations(id),
  source_entry_id TEXT REFERENCES journal_entries(id),
  proposed_title TEXT NOT NULL, proposed_data TEXT NOT NULL,
  source_text TEXT,
  outcome TEXT NOT NULL CHECK(outcome IN ('accepted','edited','dismissed')),
  accepted_task_id TEXT REFERENCES tasks(id),
  created_at TEXT NOT NULL
);
```

---

## 12. CROSS-TABLE QUERIES

### Full Task List with Dependency Status
```sql
SELECT t.id, t.title, t.status, t.priority, t.due_date, t.due_time,
  t.project, t.energy_level, t.context_tag, t.tags,
  COUNT(DISTINCT s.id) AS subtask_count,
  COUNT(DISTINCT CASE WHEN bt.status NOT IN ('done','cancelled') THEN d.id END) AS blocker_count
FROM tasks t
LEFT JOIN tasks s ON s.parent_task_id = t.id AND s.is_deleted = 0
LEFT JOIN task_dependencies d ON d.blocked_task_id = t.id
LEFT JOIN tasks bt ON bt.id = d.blocking_task_id AND bt.is_deleted = 0
WHERE t.is_deleted = 0 AND t.parent_task_id IS NULL
[dynamic energy_levels IN (...), context_tags IN (...), other filters]
GROUP BY t.id ORDER BY [sort] LIMIT ? OFFSET ?
```

### Journal Search with Emotion Filter (SQL, not post-filter — Mismatch-04 fix)
```sql
SELECT je.id, je.title, snippet(journal_fts, 2, '<mark>', '</mark>', '...', 20) AS excerpt,
  je.emotions, je.tags, je.created_at
FROM journal_fts
JOIN journal_entries je ON je.id = journal_fts.id
[JOIN journal_emotions_flat jef ON jef.entry_id = je.id -- only when emotions filter provided]
WHERE journal_fts MATCH ?
  AND je.is_deleted = 0
  [AND jef.emotion IN (?, ?, ?)]    -- parameterised, inside query not post-filter
  [AND je.created_at BETWEEN ? AND ?]
ORDER BY rank LIMIT ?
```

### Trash Query (D-82)
```sql
SELECT 'task' AS type, t.id, t.title AS name, t.created_at,
  (SELECT COUNT(*) FROM tasks s WHERE s.parent_task_id = t.id AND s.is_deleted = 1) AS subtask_count
FROM tasks t WHERE t.is_deleted = 1 AND t.parent_task_id IS NULL
UNION ALL
SELECT 'journal_entry', id, COALESCE(title, substr(content,1,60)), created_at, 0
FROM journal_entries WHERE is_deleted = 1
UNION ALL
SELECT 'notebook', id, name, created_at, 0
FROM notebooks WHERE is_deleted = 1
ORDER BY created_at DESC
```

### Report 6 — Energy by Hour
```sql
SELECT CAST(strftime('%H', tl.started_at) AS INTEGER) AS hour,
  t.energy_level, COUNT(*) AS count
FROM time_logs tl JOIN tasks t ON t.id = tl.task_id
WHERE tl.started_at BETWEEN ? AND ? AND tl.duration IS NOT NULL
  AND t.energy_level IS NOT NULL AND t.is_deleted = 0
GROUP BY hour, t.energy_level ORDER BY hour ASC
```

---

## 13. AI INTEGRATION ARCHITECTURE

### 13A. Smart Inject — Context Budget

At default 16384 context, budget = 8192 tokens (50%).

**Compact injection format (D-94):**
```
⚠️ [abc123] Call dentist | HIGH | 2026-03-01 (12d overdue) | 🧠 | Work
```
~25 tokens per task. 55 tasks (15+20+20) = ~1375 tokens for all three tiers.

**Tiers:**
```
TIER 1 — OVERDUE     (priority 1) — 15 tasks max — never truncated first
TIER 2 — TODAY       (priority 2) — 20 tasks max
TIER 3 — UPCOMING    (priority 3) — 20 tasks max, 14 days ahead (D-77) — truncated first
SMART INJECT         (conditional, D-20) — journal entries, open tasks, current entry
```

Token budget enforcement: if injected context exceeds 8192 tokens, truncate in order: completed tasks → oldest journal → tasks without due date → upcoming → today → overdue. System prompt + tools never cut.

### 13B. System Prompt Blocks

```
[BLOCK 1 — THINKING PARTNER PERSONA]      always
[BLOCK 2 — CORE RULES]                    always
[BLOCK 3 — DATE/TIME/TAB]                 fresh chrono::Local::now() every request
[BLOCK 4 — GUARANTEED MINIMUMS]           overdue + today + upcoming (compact format)
[BLOCK 5 — SMART INJECT]                  conditional
[BLOCK 6 — FTS5 PRE-SEED]                analysis mode only
[BLOCK 7 — MODE INSTRUCTIONS]             chat | analysis | ghostwriter | weekly_plan
[BLOCK 8 — ISO 8601 DATE RULE]            {DATE} {TOMORROW} {NEXT_MONDAY}
```

### 13C. AI Confirm Timeout (D-95)

```rust
// tools.rs — 300s timeout on every tool confirmation wait
match timeout(Duration::from_secs(300), rx).await {
    Ok(Ok(confirmed)) => Ok(confirmed),
    Err(_) => {
        // Auto-cancel, release Mutex
        emit(app, AppEvent::AiConfirmTimeout { call_id, tool_name });
        Ok(false)  // treated as cancel
    }
}
```

### 13D. Six Tools (sent with every request)

`create_task` · `update_task` · `complete_task` · `list_tasks` · `search_journal` · `fetch_url`

All include: `energy_level`, `context_tag`, `linked_url` on task tools.
Dependencies: NOT exposed as AI tools (user-managed only).

### 13E. Fallback Parser (D-38)

Three-pattern regex in priority order:
1. `<tool_call>{...}</tool_call>` tags
2. ` ```json { "name": "..." } ``` ` fence blocks
3. First complete JSON object with `"name"` key

System prompt hardened: "When calling a tool, output ONLY the tool_call block. No preamble."

---

## 14. JOURNAL AUTO-ANALYSIS PIPELINE

### Trigger Chain (D-52, D-59, D-106)

```
User presses Ctrl+S
    ↓
journal_save() — synchronous, returns immediately to UI
    ↓
trigger_journal_analysis(entry_id) — returns ()
    │
    └─ emit(JournalAnalysisQueued { entry_id })  ← IMMEDIATE, no wait (D-106)
       Card shows "Analysis scheduled…"
    ↓ (tokio::spawn)
[latest-only deduplication: if a newer trigger exists for this entry_id, abort]
    ↓ (3s debounce)
emit(JournalAnalysisStarted { entry_id })
Card transitions to PENDING spinner
    ↓
Step 1: FTS5 Pre-Seed (D-57)
  keyword extraction → related journal entries + related open tasks → RelatedContext
    ↓
Step 2: Build prompt (persona + minimums + FTS5 context + analysis instructions + entry)
    ↓
Step 3: OllamaClient::stream_chat() — acquires Mutex
  emit(AiToken { source: JournalAnalysis }) per token
    ↓
Step 4: Parse response
  JSON block → ProposedTask[], ConversationHook[], follow_up_questions[]
  Narrative → plain prose
    ↓
Step 5: Persist
  INSERT ai_conversation (source='analysis', source_entry_id=entry_id)
  INSERT ai_messages
  UPDATE journal_entries SET last_analysis_conv_id, last_analysed_at
    ↓
Step 6: emit(JournalAnalysisComplete { ... })
Card transitions to REVIEW state
```

### Analysis Card State Machine

```
IDLE ──[Ctrl+S]──► QUEUED ──[3s debounce]──► PENDING ──[stream]──► STREAMING ──[parse done]──► REVIEW
                                                                                                    │
                                                                              [Save selected / Dismiss]
                                                                                                    ↓
                                                                                               RESOLVED
```

---

## 15. THINKING PARTNER PERSONA (D-55)

Active on all surfaces at all times. Core behaviours:

1. **Contextual synthesis** — checks FTS5 pre-seed before asking "which project?"
2. **Socratic probing** — no question limit (D-58), targeted, most important first
3. **Ghostwriting** — plain text draft in chat, user copies manually (D-54)
4. **Task extraction** — err toward more proposals, user decides
5. **Honesty** — never invents context matches

Weekly Planning output: day-by-day plan using energy levels, context tags, due dates, dependencies. Auto Monday 08:00 + manual trigger (D-69).

---

## 16. DATA MANAGEMENT & PORTABILITY

### Directory Structure
```
<project_dir>/
├── data/personallifeos.db          WAL mode, bundled SQLite
├── backups/                        plo_YYYY-MM-DD_HHMM.db (max N, default 10)
├── attachments/<task_id>/          no size limit (D-87), relative paths (D-47)
└── exports/                        JSON and ZIP exports
```

### Backup Strategy (D-98, D-99)
All backups use `rusqlite::backup::Backup` API (not `std::fs::copy`).
Three triggers: scheduled interval · on open if stale · on close.
Pre-restore: `PRAGMA integrity_check` on backup → schema version check → safety copy → replace.

### Import (D-70, D-83)
Replace: wipe → auto safety backup → import all.
Merge: same `id` = live wins, skip imported. Post-import toast shows counts.
Order: notebooks → journal_entries → tasks → attachments → time_logs → ai_conversations → ai_messages.

### Timezone Policy (D-101)
All day calculations use `chrono::Local`. All storage uses ISO 8601 UTC.

---

## 17. SYSTEM TRAY & AUTO-START

Tray active when `close_behaviour = "tray"`. Tray menu: Open · Tasks due today (N) · Overdue (N, if > 0) · Settings · Quit.

Reminder delivery:

| App state | Delivery |
|-----------|---------|
| Window open | Floating toast (D-81) |
| Window open + tray mode | Floating toast + OS toast |
| Hidden in tray | OS toast only |
| Not running | None |

Auto-start: `tauri-plugin-autostart`. Opt-in, off by default (D-71).

---

## 18. BUILD PHASES

### Phase 0: Project Bootstrap
- [ ] **`cargo add rusqlite --features bundled-full` — verify compiles on Windows MSVC first**
- [ ] `cargo tauri init personallifeos`
- [ ] Configure `tauri.conf.json` (1400×900, minWidth 1200)
- [ ] All Cargo dependencies added + pinned
- [ ] `db/paths.rs` — `resolve_data_dir()` MSI vs portable (D-29)
- [ ] All migration files created with `BEGIN/COMMIT` wrappers (D-97)
- [ ] `db/migrations.rs` — sequential runner (D-30)
- [ ] Log `PRAGMA compile_options` → confirm FTS5 at startup
- [ ] `db/init.rs` — full startup sequence (7 steps)
- [ ] Seed defaults: `ollama_context_len = 16384` (D-93)
- [ ] CSS grid: 220px | 1fr | 420px, terminal as optional row (D-78, D-79)
- [ ] Left sidebar with bottom section (D-80)
- [ ] `events.rs` — `AppEvent` enum + `emit()` helper (D-96)
- [ ] `notifications.js` — floating toast system (D-81)
- [ ] Tab load-once/show-hide pattern
- [ ] `LogEmitter` wired to terminal (optional) and log viewer (always)
- [ ] `toggle_terminal` handler + Ctrl+\` shortcut
- [ ] `recover_orphaned_timers()` (D-43)
- [ ] Ollama health check async + RAM check via sysinfo (D-86, D-93)
- [ ] Right sidebar shell (420px, model selector, input area)
- [ ] Single `"app_event"` listener in `app.js` with dispatch table (D-96)
- **✅ Deliverable:** App opens regardless of Ollama. Layout correct. Events typed. Toast system works.

### Phase 1: Journal Module
- [ ] All journal + notebook Rust handlers
- [ ] `journal_list` with keyset cursor pagination (D-104)
- [ ] Migration 002: FTS5 + 5 triggers (insert, update, delete, soft_delete, restore — D-100)
- [ ] Migration 003: analysis tracking columns
- [ ] Migration 004: emotions column + data migration (D-66)
- [ ] `VirtualList` with cursor pagination (D-45, D-104)
- [ ] Notebook pill tabs, emotion picker, tag chips, editor
- [ ] Auto-save (2s) does not trigger analysis
- [ ] Explicit save → `JournalAnalysisQueued` immediate + 3s debounce → pipeline (D-52, D-106)
- [ ] Analysis pipeline: dedup (latest-only), keyword extract, FTS5 pre-seed (D-57, D-59)
- [ ] Analysis card state machine in `ai-sidebar.js` (D-89)
- [ ] `ProposedTaskInput` struct + `confirm_proposed_tasks` (D-107)
- [ ] `dismiss_proposed_tasks` + `proposed_task_log` writes
- [ ] New entry flow: auto-save current silently (D-88)
- **✅ Deliverable:** Full journal CRUD. Analysis fires immediately on save. Card above chat. Keyset pagination works.

### Phase 2: Tasks Module
- [ ] Migration 004: energy_level, context_tag, linked_url, task_dependencies, file_missing
- [ ] All task Rust handlers
- [ ] `task_restore` restores parent + subtasks (D-102)
- [ ] `timer_start` guards: not done/cancelled, no duplicate open timer
- [ ] `task_assign_date` handler (D-85)
- [ ] `task_list` with `energy_levels: Vec<String>`, `context_tags: Vec<String>` (multi-select fix)
- [ ] `task_list_upcoming(today, 14, 20)` (D-63)
- [ ] `task_add_dependency` — circular check + max 10 guard
- [ ] Calendar/agenda view + drag-assign (D-85)
- [ ] List view (multi-select filters), Kanban (4 columns, drag)
- [ ] Task detail panel — all sections
- [ ] `RecurrenceScheduler` — local time, skip missed (D-39, D-101)
- [ ] `ReminderScheduler` — local time comparison (D-101) → float toast or OS toast (D-81, D-73)
- [ ] Attachment dialog owned by JS, path passed to `attachment_add` (Mismatch-07 fix)
- **✅ Deliverable:** Full task management. Drag assigns dates. Multi-select filters work. Reminders fire.

### Phase 3: AI Chat & Tool Engine
- [ ] `OllamaClient::stream_chat()` with single Mutex (D-44)
- [ ] `PromptComposer` — compact format (D-94), all 8 blocks, guaranteed minimums (D-56, D-61)
- [ ] Token budget enforcement with truncation order (D-34)
- [ ] All 6 tool handlers
- [ ] Confirmation flow with 300s timeout + auto-cancel toast (D-95)
- [ ] `ai_chat` returns `conversation_id` (D-105)
- [ ] `conversation_switch_model` with `Option<String>` + warning dialog (D-84, D-103)
- [ ] JSON fallback parser (D-38)
- [ ] Typed `AppEvent` wired to all AI emit points (D-96)
- [ ] AI tab: conversation list + chat window + context panel + source label map (§8C)
- [ ] Conversation persistence + auto-title
- [ ] Model missing detection + recovery (D-37)
- [ ] Thinking Partner persona (D-55) + all mode blocks
- [ ] Weekly Planning handler (D-69)
- **✅ Deliverable:** AI fully functional. Typed events. Model switch safe. Timeout auto-cancels.

### Phase 4: Reports Module
- [ ] All 6 report Rust handlers with `date_from` + `date_to`
- [ ] `get_journal_stats_report` uses `journal_emotions_flat` view
- [ ] `ReportDateRange` component — presets + custom (D-74)
- [ ] Report 1–6 built and correct
- [ ] Weekly review + plan render sequence defined (Mismatch-06 fix)
- [ ] `WeeklyReportScheduler` → review + plan (D-69)
- [ ] PDF `@media print` + Markdown export
- [ ] Streak calculation
- [ ] `weekly_report_ready` toast (D-81)
- **✅ Deliverable:** All 6 reports correct. Weekly auto-generation confirmed. Toasts fire.

### Phase 5: Settings, System, Backup
- [ ] Settings 6-section nav
- [ ] `backup_manual(path)` — JS dialog → path passed to handler (Mismatch-07)
- [ ] Backup uses `rusqlite::backup::Backup` API (D-98)
- [ ] Three backup triggers: scheduled + on-open + on-close (D-99)
- [ ] `trash_empty` — JS confirmation dialog required + file-exists guard (Mismatch-08)
- [ ] `trash_list` query with subtask count (D-82)
- [ ] `task_restore` confirmed: restores subtasks (D-102)
- [ ] SQL runner (D-90 — user toggles read-only)
- [ ] Terminal toggle Settings control (D-79)
- [ ] Tray + OS toast (D-72, D-73)
- [ ] Auto-start (D-71)
- [ ] `import_json` Replace/Merge with live-wins merge conflict (D-83)
- [ ] Settings > AI context length with RAM warning (D-93)
- **✅ Deliverable:** All settings working. Backup correct. Tray works. Import verified.

### Phase 6: Polish & Hardening
- [ ] All keyboard shortcuts (D-76) including Ctrl+\`
- [ ] All `invoke()` calls have `.catch()` → error toast
- [ ] First-run: create dirs, run migrations, seed settings, RAM check, open Journal
- [ ] Window state persistence
- [ ] `PRAGMA ANALYZE` on startup
- [ ] README with prerequisites, setup, security advisory (D-48)
- [ ] AGENTS.md + CHANGELOG.md
- [ ] `cargo tauri build` → `.msi` + portable `.exe`
- [ ] Smoke test: all analysis paths, model switch, confirm timeout, tray, backup/restore, import
- **✅ Deliverable:** Production build. Fully functional on clean Windows + Ollama machine.

---

## 19. FILE & DIRECTORY SCAFFOLDING

```
personallifeos/
│
├── src-tauri/                              Rust backend (Tauri)
│   ├── Cargo.toml                          All dependencies — all pinned
│   ├── Cargo.lock
│   ├── tauri.conf.json                     Window config: 1400×900, minWidth 1200
│   ├── build.rs
│   ├── icons/                              App icons (ICO, PNG at multiple sizes)
│   │   ├── icon.ico
│   │   ├── 32x32.png
│   │   ├── 128x128.png
│   │   └── 128x128@2x.png
│   │
│   ├── src/
│   │   ├── main.rs                         Tauri entry: plugin registration, window events
│   │   │                                   On CloseRequested: backup-on-close (D-99)
│   │   ├── lib.rs                          All #[tauri::command] registrations
│   │   ├── error.rs                        AppError enum — serialisable, all variants
│   │   ├── events.rs                       AppEvent enum + emit() helper (D-96)
│   │   │
│   │   ├── db/                             Layer 3 — Data Access
│   │   │   ├── mod.rs                      DbState type, connection pool
│   │   │   ├── init.rs                     Startup sequence (7 steps), backup-on-open (D-99)
│   │   │   ├── paths.rs                    resolve_data_dir() — MSI vs portable (D-29)
│   │   │   ├── migrations.rs               Sequential runner — INSERT OR IGNORE version record
│   │   │   ├── journal.rs                  journal_* + notebook_* handlers
│   │   │   │                               journal_list uses keyset cursor (D-104)
│   │   │   ├── tasks.rs                    task_* + timer_* + attachment_* handlers
│   │   │   │                               task_restore restores subtasks (D-102)
│   │   │   │                               timer_start guards completed/cancelled (fix)
│   │   │   │                               task_assign_date for calendar drag (D-85)
│   │   │   │                               task_list accepts Vec<String> for energy/context (fix)
│   │   │   ├── ai.rs                       conversation_* + message_* + proposed_task_log
│   │   │   │                               ai_chat returns conversation_id (D-105)
│   │   │   │                               conversation_switch_model accepts Option<String> (D-103)
│   │   │   ├── settings.rs                 setting_get/set/seed — context default 16384 (D-93)
│   │   │   ├── audit.rs                    write() + archive_old_entries() (D-32)
│   │   │   ├── reports.rs                  All 6 report query handlers
│   │   │   │                               emotion filter in SQL (D-104 emotion fix)
│   │   │   └── trash.rs                    trash_list (subtask count — D-82) + trash_empty
│   │   │                                   trash_empty: file-exists guard before deletion (fix)
│   │   │
│   │   ├── ai/                             Layer 2 — AI Orchestration
│   │   │   ├── mod.rs
│   │   │   ├── client.rs                   OllamaClient — single Tokio Mutex (D-44)
│   │   │   │                               stream_chat, fetch_models, health_check
│   │   │   ├── stream.rs                   NDJSON stream parser + emit(AiToken)
│   │   │   ├── tools.rs                    6 tool definitions + ToolExecutor
│   │   │   │                               wait_for_confirmation with 300s timeout (D-95)
│   │   │   │                               PENDING map: HashMap<call_id, oneshot::Sender>
│   │   │   ├── prompt.rs                   PromptComposer::build() — 8 blocks
│   │   │   │                               format_task_for_injection() compact format (D-94)
│   │   │   │                               guaranteed minimums: overdue+today+upcoming (D-56,D-61)
│   │   │   │                               token budget enforcement + truncation order (D-34)
│   │   │   ├── analysis.rs                 JournalAnalysisPipeline
│   │   │   │                               LATEST_PENDING: dedup map (latest-only, D-59)
│   │   │   │                               emit(JournalAnalysisQueued) immediately (D-106)
│   │   │   │                               ProposedTaskInput struct (D-107)
│   │   │   ├── keywords.rs                 extract_keywords() — stop-word filter, ≥4 chars (D-57)
│   │   │   └── fallback.rs                 Three-pattern regex fallback parser (D-38)
│   │   │
│   │   ├── backup/                         Backup + Export + Import
│   │   │   ├── mod.rs
│   │   │   ├── manual.rs                   backup_to_path() — rusqlite::backup::Backup (D-98)
│   │   │   │                               backup_manual(path) — path from JS dialog (fix)
│   │   │   │                               backup_restore() — integrity_check gate (D-31)
│   │   │   ├── auto.rs                     backup_if_stale() on-open (D-99)
│   │   │   │                               tokio interval scheduler
│   │   │   │                               rotate_backups() — keep last N
│   │   │   └── export.rs                   export_json() + export_zip() + import_json()
│   │   │                                   merge conflict: live wins (D-83)
│   │   │
│   │   ├── scheduler/                      Layer 2 — Scheduler Suite
│   │   │   ├── mod.rs
│   │   │   ├── reminders.rs                60s poll — local time comparison (D-101)
│   │   │   │                               emit(ReminderFired) → toast (D-81) or OS toast (D-73)
│   │   │   ├── recurrence.rs               3600s poll — local date, skip missed (D-39, D-101)
│   │   │   │                               LATEST_PENDING dedup for same logic
│   │   │   └── weekly_report.rs            Monday 08:00 local — review + weekly plan (D-69)
│   │   │                                   emit(WeeklyReportReady)
│   │   │
│   │   ├── migrations/                     SQL migration files — ALL BEGIN/COMMIT wrapped (D-97)
│   │   │   ├── 001_initial.sql             Full schema: all 13 tables + indexes
│   │   │   ├── 002_fts_triggers.sql        FTS5 virtual table + 5 triggers (D-41, D-100)
│   │   │   ├── 003_analysis_tracking.sql   last_analysis_conv_id, last_analysed_at, proposed_task_log
│   │   │   ├── 004_field_expansion.sql     energy_level, context_tag, linked_url, task_dependencies,
│   │   │   │                               file_missing, emotions column + data migration
│   │   │   └── 005_settings_v16.sql        context default 16384, backup trigger settings (D-93, D-99)
│   │   │
│   │   └── logger.rs                       tracing subscriber → emit(LogEvent) (D-96)
│   │                                       LogLevel + LogSource enums matching AppEvent
│   │
│   └── tests/                              Rust integration tests
│       ├── db_migrations_test.rs           Verify all migrations apply cleanly + idempotent
│       ├── task_restore_test.rs            Verify subtasks restored (D-102)
│       └── backup_api_test.rs              Verify rusqlite::backup produces valid DB (D-98)
│
├── src/                                    WebView2 Frontend
│   │
│   ├── index.html                          Root shell — CSS grid layout only
│   │                                       Loads: base.css, layout.css, app.js, ai-sidebar.js,
│   │                                       notifications.js. Does NOT load terminal.js (loaded on toggle).
│   │
│   ├── styles/
│   │   ├── base.css                        CSS variables (all --color-*, --font-*, --radius, --space)
│   │   │                                   CSS reset, body defaults
│   │   ├── layout.css                      3-column grid: 220px | 1fr | 420px
│   │   │                                   body.terminal-visible: adds terminal row
│   │   │                                   #left-sidebar, #main-content, #right-sidebar, #terminal
│   │   ├── components.css                  Buttons, inputs, badges, chips, modals, dropdowns
│   │   │                                   .badge-status-* .badge-priority-* .badge-energy-*
│   │   │                                   VirtualList container styles
│   │   ├── toast.css                       .toast-container (fixed, bottom-right)
│   │   │                                   .toast (individual toast card)
│   │   │                                   .toast-stack animation
│   │   ├── ai-chat.css                     Right sidebar layout
│   │   │                                   .analysis-card (fixed above chat)
│   │   │                                   .message-bubble (user/assistant variants)
│   │   │                                   .tool-card, .confirm-card
│   │   ├── terminal.css                    Terminal bar: filter tabs, log lines, colour coding
│   │   └── themes/
│   │       ├── dark.css                    Dark theme — overrides CSS variables
│   │       └── light.css                   Light theme — overrides CSS variables
│   │
│   ├── js/
│   │   ├── app.js                          App init, tab router, single app_event listener (D-96)
│   │   │                                   Registers all AppEvent handlers
│   │   │                                   terminal toggle (body.terminal-visible class)
│   │   │                                   orphaned timer toast on startup
│   │   │
│   │   ├── ipc.js                          invoke() wrapper with typed error handling
│   │   │                                   All invoke calls go through here
│   │   │                                   .catch() always calls showError() toast
│   │   │
│   │   ├── terminal.js                     Log event renderer — loaded lazily on toggle
│   │   │                                   VirtualList for terminal (max 2000 lines)
│   │   │                                   Filter by level/source
│   │   │
│   │   ├── ai-sidebar.js                   Always mounted (D-23)
│   │   │                                   Model selector + switch warning (D-84)
│   │   │                                   Analysis card state machine: IDLE→QUEUED→PENDING→STREAMING→REVIEW→RESOLVED
│   │   │                                   ProposedTaskInput builder for confirm/dismiss
│   │   │                                   Tool confirmation cards with 300s timeout indicator
│   │   │                                   Context indicator (smart inject summary)
│   │   │
│   │   ├── notifications.js                Floating toast system (D-81)
│   │   │                                   Toast queue, max 3 visible, 8s auto-dismiss
│   │   │                                   Toast types: reminder, orphan, backup, import, failed, report
│   │   │
│   │   └── tabs/
│   │       ├── journal.js                  VirtualList with cursor (D-45, D-104)
│   │       │                               Notebook pill tabs + CRUD
│   │       │                               Emotion picker component (10 values, multi-select)
│   │       │                               Auto-save debounce (2s, no analysis)
│   │       │                               Explicit save → trigger_journal_analysis
│   │       │                               New entry: auto-save current (D-88)
│   │       │                               Dirty-field map for batched task_update calls
│   │       │
│   │       ├── tasks.js                    Calendar/agenda view with week navigation
│   │       │                               Drag no-date task to day column (D-85)
│   │       │                               List view with multi-select energy/context filter
│   │       │                               Kanban view with drag between columns
│   │       │                               Task detail slide-in panel — all sections
│   │       │                               Timer start/stop, manual time log
│   │       │                               Dirty-field map → batched task_update
│   │       │                               Attachment: JS opens dialog, passes path (Mismatch-07)
│   │       │
│   │       ├── ai.js                       Conversation list with source filter tabs
│   │       │                               CONVERSATION_SOURCE_MAP (§8C, Mismatch-05)
│   │       │                               Full chat window: streaming, tool cards, context panel
│   │       │                               Model switch warning dialog (D-84)
│   │       │
│   │       ├── reports.js                  ReportDateRange component — presets + custom (D-74)
│   │       │                               Chart.js integration for all 6 reports
│   │       │                               Weekly review + plan render sequence (Mismatch-06)
│   │       │                               PDF print trigger + Markdown save dialog
│   │       │
│   │       └── settings.js                 6-section sub-nav
│   │                                       Backup: JS opens save dialog → passes path (Mismatch-07)
│   │                                       Trash: confirmation before trash_empty (Mismatch-08)
│   │                                       Context length selector with RAM warning (D-93)
│   │                                       Terminal toggle wired to body class
│   │
│   ├── views/                              HTML partials — loaded once, show/hide
│   │   ├── journal.html                    Does NOT import marked.min.js
│   │   ├── tasks.html                      Imports flatpickr.min.js
│   │   ├── ai.html
│   │   ├── reports.html                    Imports chart.min.js + marked.min.js (D-42)
│   │   └── settings.html
│   │
│   └── assets/
│       ├── icons/                          SVG nav icons (journal, tasks, ai, reports, settings)
│       │                                   Energy level icons (4 SVG)
│       │                                   Context tag icons (5 SVG)
│       └── libs/                           Vendored offline — no CDN
│           ├── chart.min.js                4.x
│           ├── marked.min.js               9.x
│           ├── highlight.min.js            11.x
│           └── flatpickr.min.js            4.x
│
├── data/                                   Created at first run
│   └── personallifeos.db
├── backups/                                Created at first run
├── attachments/                            Created at first run
│   └── <task_id>/
│       └── <filename>
└── exports/                                Created at first run
    ├── plo_export_YYYY-MM-DD.json
    └── plo_export_YYYY-MM-DD.zip

── Project Root Files ──────────────────────────────────────────────────

├── AGENTS.md                               AI session memory — update every session
│                                           Contains: purpose, file structure, tech stack,
│                                           conventions, immutable rules, known decisions
├── CHANGELOG.md                            Session-by-session change log
└── README.md                               Setup, prerequisites, first run, security advisory
```

---

## 20. RISK REGISTER

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| R-01 | Ollama not running at launch | High | Low | App opens fully (D-86); amber dot; retry in popover |
| R-02 | `rusqlite bundled-full` fails Windows compile | Medium | Critical | **Verify Phase 0 step 1 before any code** |
| R-03 | Model doesn't support tool-calling | High | High | JSON fallback parser (D-38) |
| R-04 | AI malformed tool JSON | Medium | Medium | ToolExecutor validates; returns error to model (D-36) |
| R-05 | Context overflow on heavy user | Medium | Medium | Compact format (D-94), 16384 default (D-93) |
| R-06 | `fetch_url` blocked by Cloudflare | High | Low | AI informs user; documented in README (D-46) |
| R-07 | SQLite corruption on hard kill | Low | High | WAL + auto-backup covers recovery |
| R-08 | Backup dir fills disk | Low | Medium | Configurable keep count (default 10) |
| R-09 | Migration partial-apply on crash | Low | High | `BEGIN/COMMIT` wrap on every migration file (D-97) |
| R-10 | Backup misses WAL pages | Eliminated | — | `rusqlite::backup::Backup` API (D-98) |
| R-11 | Auto-backup never fires (Exit mode) | Eliminated | — | On-open + on-close triggers (D-99) |
| R-12 | FTS5 surfaces deleted entries | Eliminated | — | soft_delete + restore triggers (D-100) |
| R-13 | AI confirm blocks OllamaClient indefinitely | Eliminated | — | 300s timeout auto-cancel (D-95) |
| R-14 | Analysis queue depth invisible | Eliminated | — | Latest-only dedup + immediate queued event (D-59, D-106) |
| R-15 | `task_restore` doesn't restore subtasks | Eliminated | — | `WHERE id = ? OR parent_task_id = ?` (D-102) |
| R-16 | Report 6 shows no data | High | Low | Graceful empty state |
| R-17 | Model deleted at runtime | Medium | Medium | 404 detection → model list refresh → block + prompt (D-37) |
| R-18 | Pre-restore backup also corrupted | Low | High | `PRAGMA integrity_check` before restore (D-31) |
| R-19 | Layout compressed < 1200px | Medium | Low | minWidth enforced in tauri.conf.json |

---

## 21. FEATURE BACKLOG

### B-01: AI Training Zone
Structured Q&A where the AI learns user values, habits, and patterns. Stores in `user_profile` model that feeds into `PromptComposer` as Block 9. All data local-only. Sub-tab under AI tab.

**Blockers before specification:** Question generation method (AI vs static), `user_profile` schema, confidence decay mechanism, Mutex priority for Training Zone Ollama calls, build phase assignment.

### B-02: AI Insights Tab
Synthesised observations about user patterns derived from journal, tasks, and (when built) Training Zone data. Sub-tab under AI tab alongside Conversations and Training Zone.

**Blockers before specification:** Distinction from Report 5 (AI Suggestions), refresh cadence, minimum data threshold.

---

## 22. VERIFY BEFORE USE

### Phase 0 — Before any app code

| Item | URL |
|------|-----|
| `rusqlite 0.31 bundled-full` compiles on Windows MSVC | https://github.com/rusqlite/rusqlite#notes-on-building |
| `bundled-full` enables FTS5 — check CHANGELOG | https://github.com/rusqlite/rusqlite/blob/master/CHANGELOG.md |
| `rusqlite::backup::Backup` API — confirm available in v0.31 | https://docs.rs/rusqlite/latest/rusqlite/backup/index.html |
| Tauri v2 `AppHandle::emit()` callable from `tokio::spawn` | https://v2.tauri.app/reference/javascript/event/ |
| Tauri v2 plugin APIs (fs, dialog, shell, notification, autostart) | https://v2.tauri.app/plugin/ |
| `sysinfo 0.30` — `System::total_memory()` API | https://docs.rs/sysinfo/latest/sysinfo/ |
| `OnceLock<Mutex<...>>` replaces `lazy_static` (Rust 1.70+) | https://doc.rust-lang.org/std/sync/struct.OnceLock.html |

### Phase 1 — Before journal module

| Item | URL |
|------|-----|
| FTS5 `snippet()` in `rusqlite bundled-full` | https://www.sqlite.org/fts5.html#the_snippet_function |
| FTS5 `unicode61 remove_diacritics` syntax | https://www.sqlite.org/fts5.html#unicode61_tokenizer |
| SQLite `json_each()` in `rusqlite bundled-full` | https://www.sqlite.org/json1.html |
| `DROP COLUMN` requires SQLite 3.35+ — run `SELECT sqlite_version()` | https://www.sqlite.org/lang_altertable.html |

### Phase 2 — Before tasks module

| Item | URL |
|------|-----|
| Ollama `/api/chat` tool-calling payload for llama3.2 | https://github.com/ollama/ollama/blob/main/docs/api.md |
| Which Ollama models support native tool-calling | https://ollama.com/library |

### Phase 5 — Before system module

| Item | URL |
|------|-----|
| `tauri-plugin-autostart` v2 Windows Registry method | https://v2.tauri.app/plugin/autostart/ |
| `tauri-plugin-notification` v2 Windows toast | https://v2.tauri.app/plugin/notification/ |
| Tauri v2 `TrayIconBuilder` | https://v2.tauri.app/learn/system-tray/ |
| `WindowEvent::CloseRequested` + `api.prevent_close()` | https://v2.tauri.app/reference/javascript/window/ |

---

## APPENDIX — SECURITY ADVISORY (D-28, D-48)

*Text for README.md and Settings > About*

```
PRIVACY NOTICE — Your Data Security

PersonalLifeOS stores all data in a plain SQLite file at:
[path shown dynamically in Settings > Database]

This file is NOT encrypted. Anyone with access to your Windows account
can read its contents with any SQLite browser.

RECOMMENDED PROTECTIONS:
1. Windows BitLocker (strongest) — Settings → Privacy & Security → Device Encryption
2. Folder permissions — Right-click data folder → Properties → Security → restrict access
3. Windows Hello PIN/password — locks your session when unattended

Exported files (JSON/ZIP) are also unencrypted. Store them securely.
```

---

*PersonalLifeOS Master Specification v1.6*
*Decisions: D-01 through D-107 (107 total) — all locked*
*Versions consolidated: v1.1 through v1.6 + Engineering Review fixes*
*Backlog: B-01 AI Training Zone · B-02 AI Insights*
*Ready for Phase 0 commencement*
