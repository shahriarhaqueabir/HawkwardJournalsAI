# HawkwardJournalAI

HawkwardJournalAI is a private, offline-first Windows desktop app built with Tauri v2. It combines a plain-text journal, a task manager, and a local Ollama-powered thinking partner, with all user data stored locally in a single SQLite database.

## Project Status

Current phase: Phase 3 hardening, with Phase 2 complete and Phase 4 partially scaffolded.

What is working now:

- Journal save/load flow with keyset pagination and FTS-backed search
- Background journal analysis with deduplication and persisted summary, mood, insights, and proposed tasks
- Task CRUD, task search, subtasks, reminders, recurrence polling, attachments, and project-aware task organization
- Unified typed Rust-to-frontend event model through `AppEvent`
- AI chat conversations with prompt hardening, built tool definitions, confirmation-gated writes, fallback tool parsing, and safer tool validation
- Settings tab wiring and reports tab event refresh/fallback behavior

What is still incomplete:

- Full manual in-window validation of every AI tool confirmation flow
- The six analytical reports as fully finished report experiences
- Proper vendored frontend library loading for every intended report/chart surface
- Remaining UI coverage for timers, dependencies, fuller settings, and some deeper CRUD paths
- Resolution of the D-13 spec deviation where Projects currently exist as first-class entities

## Architecture

- Desktop shell: Tauri v2
- Backend: Rust + Tokio
- Database: SQLite via `rusqlite` with `bundled-full`
- Frontend: Vanilla HTML, CSS, and JavaScript
- AI runtime: Ollama on `http://127.0.0.1:11434`

Important constraints:

- No cloud sync, no accounts, no hosted AI
- AI may read local data, but mutating AI actions require explicit user confirmation
- Database backups must use `rusqlite::backup::Backup`, not raw file copies
- Journal editor content remains plain text only

## Running Locally

Prerequisites:

- Rust toolchain with Windows/MSVC support
- Tauri v2 Windows prerequisites
- Ollama installed locally
- `llama3.2` pulled in Ollama

Pull the default model:

```bash
ollama pull llama3.2
```

Run the desktop app from the Tauri crate:

```bash
cd src-tauri
cargo run
```

If you prefer the Tauri CLI workflow and already have it installed:

```bash
cd src-tauri
cargo tauri dev
```

## Repository Layout

- `src-tauri/`: Rust backend, DB layer, AI orchestration, schedulers, and migrations
- `src/`: frontend shell, tabs, styles, and UI event handling
- `AgentDocs/`: specs, handoff notes, and architecture/reference docs
- `data/`, `backups/`, `attachments/`, `exports/`: local runtime storage folders

## Notes For Contributors

- Start with [AGENTS.md](e:/Abir/LocalCodeRepo/HawkwardJournalAI/AGENTS.md) before changing code
- Use [CHANGELOG.md](e:/Abir/LocalCodeRepo/HawkwardJournalAI/CHANGELOG.md) as the session-by-session history
- The canonical product/spec references live in `AgentDocs/HawkwardJournalAI_MASTER_SPEC_v1.6.md` and `AgentDocs/HawkwardJournalAI_SPEC_ADDENDUM_v1.7.md`
