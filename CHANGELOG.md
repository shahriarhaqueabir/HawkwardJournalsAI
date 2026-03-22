# Changelog

## 2026-03-22

### AI Tool & Reporting Refinement

- **AI Tool Context Association**:
  - Updated `execute_ai_tool` in `src-tauri/src/db/tasks.rs` and `execute_tool_call` in `src-tauri/src/ai/tools.rs` to accept and persist `ai_conversation_id`.
  - **AI Task Name Prioritization**: Updated the system prompt in `src-tauri/src/ai/prompt.rs` to prioritize human-readable task titles and project names over raw UUIDs or ID prefixes in AI responses.
  - **Project Name Resolution**: Enhanced `execute_ai_tool` in `src-tauri/src/db/tasks.rs` with `resolve_project_reference`, allowing the AI to understand and link tasks to projects using their names (e.g., "Work") instead of requiring UUIDs.
  - Wired `ai_chat` in `src-tauri/src/lib.rs` to pass the correct `conversation_id` through both structured and fallback tool execution paths.
  - Aligned AI-generated task creation and updates with their source conversation for better traceability.
- **Reporting UI Finalization**:
  - Implemented full analytical dashboard in `src/js/tabs/reports.js` with 11 specialized chart/metric types.
  - Added support for: Productivity (Created vs Completed), Journal Consistency (Entries/Words), Emotion/Mood distribution, Project Health, Energy levels, Time Allocation, Task Status, and Due-Date Buckets.
  - Implemented high-fidelity SVG fallbacks for every chart type to ensure premium aesthetics without external dependencies.
- **Frontend AI Sidebar Hardening**:
  - Added event listeners for `ai_tool_pending`, `ai_tool_result`, and `ai_confirm_timeout` in `src/js/ai-sidebar.js`.
  - Implemented interactive tool confirmation cards and timeout handling in the sidebar UI.
- **Bug Fixes**:
  - Resolved a startup crash caused by redundant `ALTER TABLE` statements in Migration 006. Migration 006 has been removed as its functionality is redundant to Migration 004.
  - Fixed a database constraint error in `proposed_task_log` by adding Migration 009, which adds the `'proposed'` status to the `outcome` check constraint.
  - Corrected `save_analysis_result` to log valid status `"proposed"` and actual task titles instead of generic strings.
- **Verification**:
  - `cargo check` (Passed)
  - `node --check src/js/ai-sidebar.js` (Passed)
  - `node --check src/js/tabs/reports.js` (Passed)
  - **Comprehensive Backend Testing**: Implemented and verified 39 unit tests across `db`, `ai`, `backup`, and `scheduler` modules with a 100% pass rate.

- **Infrastructure & Visibility**:
  - **Global Event-Driven Logging**: Expanded `src-tauri/src/logger.rs` into a full `tracing` provider that emits `LogEvent` Tauri events for real-time frontend visibility.
  - **Lifecycle Integration**: Wired `logger::init()` and `AppHandle` registration into `src-tauri/src/lib.rs`.
  - **AI Memory Schema Alignment**: Fixed missing `ai_pinned_memory` table in `ai/memory.rs` test setup.
  - **Bug Fixes**: Resolved borrow checker error (`E0502`) in database reset flow and corrected AI prompt renderer to align with verification tests.


## 2026-03-21

### AI & Data Engine Hardening

- **Generous AI Token Limits**:
  - Increased AI context window (`num_ctx`) to **32,768 tokens** across all pipelines.
  - Expanded journal analysis content truncation to **25,000 characters** and web fetch truncation to **15,000 characters**.
  - Aligned frontend `settings.js` defaults with the new context window spec.
- **Mutation Auditing System**:
  - Implemented a comprehensive `audit_log` system in `src-tauri/src/db/audit.rs`.
  - Integrated auditing into all journal, task, and project mutations, including AI attribution and conversation linking.
- **Trash Management**:
  - Added native soft-delete management with `trash_list` and `trash_empty` Tauri commands.
  - Implemented `src-tauri/src/db/trash.rs` to aggregate deleted items across the database.
- **Keyword Cloud Analytics**:
  - Implemented high-performance SQL-based keyword aggregation in `src-tauri/src/db/reports.rs` using `json_each`.
  - Exposed keyword stats to the analytical dashboard.

### Frontend Audit & Alignment

- **Comprehensive Frontend Audit**:
  - Conducted a full audit of the styles, CSS, and JS logic.
  - Verified Tauri v2 IPC naming compliance and single-event channel architecture.
  - Evaluated aesthetic implementation against the "Premium WOW" spec, identifying areas for glassmorphism and transition upgrades.
  - Identified missing vendored libraries in `src/assets/libs`.
- **Consistency Fixes**:
  - Updated `create_task` to return the full Task object as per Spec §8A.
  - Resolved settings mismatches between frontend defaults and backend expanded limits.

- **Verification**:
  - `cargo check` (Passed)
  - `node --check src/js/app.js` (Passed)


## 2026-03-20

### Documentation Refresh For Current Progress

- **Project Status Docs Updated**:
  - Updated `AGENTS.md` so the current build status now reflects active Phase 3 hardening instead of describing AI tool execution as still unimplemented.
  - Refreshed the file map in `AGENTS.md` to include the newer `db/projects.rs`, `tasks.css`, and `reports.css` entries, clarified the AI tool count, and corrected the database-object summary text.
  - Rewrote `README.md` to describe the current delivered feature set, remaining gaps, realistic local run instructions, and present Phase 3 / Phase 4 status.
  - Rewrote `AgentDocs/HandoverState.md` to replace the stale Phase 2-era handoff summary with the current HawkwardJournalAI status, recent Phase 3 progress, remaining gaps, and recommended next step.

### Kanban Drag-And-Drop Hardening

- **Tasks Tab UX Fixes**:
  - Updated `src/js/tabs/tasks.js` to make kanban drag/drop more reliable.
  - Prevented accidental task-detail opening immediately after a drag gesture.
  - Added move semantics to the native drag payload and a fallback to the in-memory dragged task ID.
  - Prevented unnecessary `task_update_status` writes when a task is dropped back into the same column.
  - Stabilized the column hover state during drag with `dragenter` depth tracking to reduce flicker and premature `drag-over` removal.
- **Verification**:
  - `node --check src/js/tabs/tasks.js`

### Project, Task, And Subtask CRUD Repairs

- **Backend CRUD Integrity**:
  - Added project update/delete support in `src-tauri/src/db/projects.rs` and exposed new Tauri commands in `src-tauri/src/lib.rs` (`project_get`, `project_update`, `project_delete`).
  - Hardened task status updates and full task updates in `src-tauri/src/db/tasks.rs` so missing-task writes now fail instead of silently succeeding.
  - Updated `src-tauri/src/lib.rs` so `task_update` emits the typed `TaskUpdated` event after successful saves.
- **Project/Task Data Consistency**:
  - Added task project normalization in `src-tauri/src/db/tasks.rs` so task records consistently store both `project_id` and the human-readable `project` name.
  - Project renames now propagate the new project name to linked tasks, and project deletion moves linked tasks back to Inbox instead of leaving stale project references.
- **Frontend CRUD Surface Improvements**:
  - Expanded the existing project modal in `src/index.html` and `src/js/tabs/tasks.js` into a basic project management surface with create, edit, delete, status, color, and goal-date handling.
  - Added a subtask section to the task detail panel with list/read support plus add, status-toggle, open, and delete actions, making subtask CRUD reachable from the UI.
  - New tasks now respect the active project filter when created from the tasks tab.
- **Verification**:
  - `cargo check`
  - `node --check src/js/tabs/tasks.js`

### Phase 3 AI Tool Review And Scoped Hardening

- **Reviewed `AgentDocs/AIToolssuggestions.md` Against Scope**:
  - Accepted the parts that improve the existing Phase 3 tool engine without expanding the product surface: clearer tool contracts, safer input handling, confirmation-path hardening, and safer URL fetching.
  - Rejected or skipped suggestion details that would add unsupported scope or misrepresent current capabilities, such as introducing unimplemented tool parameters, implying broader tool inventory, or pulling in unrelated execution/logging architecture.
- **Tool Definition Hardening**:
  - Updated `src-tauri/src/ai/tools.rs` so all 6 built tools expose stricter JSON schemas with `additionalProperties: false`, stronger date patterns, tighter string bounds, and more explicit usage guidance aligned to the project's actual capabilities.
  - Kept tool descriptions consistent with the locked scope: no web search, no shell access, no file-system tools, and no extra Phase 3 features beyond the current tool set.
- **Tool Execution Validation & Safety**:
  - Added pre-execution validation for mutating tools and structured validation errors before confirmation prompts, preventing invalid AI proposals from reaching the user confirmation step.
  - Added validation for `list_tasks` and `search_journal` date filters.
  - Hardened `fetch_url` with HTTPS-only validation, local/private-network blocking, redirect limits, a 10-second timeout, HTTP/read error reporting, and explicit truncation metadata.
- **Confirmation / Result Accuracy**:
  - Updated `src-tauri/src/lib.rs` so mutating AI tool results are no longer always recorded as confirmed.
  - Cancelled, timed-out, or validation/error results now persist with the correct confirmation state in `ai_messages` and emit more accurate `AiToolResult.confirmed` values to the frontend.
- **AI Tab Feedback Polish**:
  - Updated `src/js/tabs/ai.js` so tool cards now distinguish successful execution from cancellation, timeout, and error states instead of always showing success messaging.
- **Verification**:
  - `cargo check`
  - `node --check src/js/tabs/ai.js`
  - `cargo test ai::tools --lib`

### Phase 3 Runtime Validation Pass

- **Live Desktop Runtime Checks**:
  - Launched the real Tauri desktop app from `src-tauri` and verified a live `hawkward-journal-ai.exe` process with a responsive `HawkwardJournalAI` window title.
  - Confirmed the app was resolving and using the expected roaming data directory at `%APPDATA%\\HawkwardJournals` with the live `hawkward.db`/WAL files present.
  - Confirmed local Ollama availability at `http://127.0.0.1:11434/api/tags` with `llama3.2:latest` present, so the AI runtime dependency is healthy.
- **Focused Tool Flow Validation**:
  - Added targeted unit tests in `src-tauri/src/ai/tools.rs` for the new guardrails covering blank task titles, missing update fields, invalid date filters, short journal queries, localhost URL blocking, public HTTPS URL acceptance, and URL truncation behavior.
- **Known Limitation During Validation**:
  - Full click-through UI automation inside the live Tauri/WebView2 window was not available in this environment, so the validation pass covered actual app launch/runtime prerequisites plus backend tool-contract behavior rather than claiming manual in-window interaction that was not possible here.

### AI Prompt Upgrade From Design Review

- **Chat Prompt Rewrite**:
  - Reworked `src-tauri/src/ai/prompt.rs` to adopt a more structured prompt format with clearer persona, operational rules, tool whitelist, date handling, tool-result narration rules, mode instructions, and failure behavior.
  - Preserved accuracy by correcting the design note's tool-count mismatch: the prompt now reflects the 6 currently built tools rather than claiming unavailable capabilities.
- **Analysis Prompt Hardening**:
  - Expanded the journal-analysis system prompt in `src-tauri/src/ai/prompt.rs` with stricter schema language, stronger task-extraction constraints, and a valid example payload for the analysis model.
- **Verification**:
  - `cargo check`

### Frontend Tab Repair & Settings Wiring

- **Settings Tab Activation**:
  - Replaced the Settings placeholder in `src/index.html` with a usable settings form for AI model, Ollama URL, context window, theme, and terminal visibility.
  - Added `src/js/tabs/settings.js` and wired it in `src/js/app.js`.
  - Added backend settings commands in `src-tauri/src/lib.rs` plus CRUD helpers in `src-tauri/src/db/settings.rs` (`settings_list`, `setting_get`, `setting_set`).
- **Journal UX Fixes**:
  - Implemented journal search flow in `src/js/tabs/journal.js` using the new `journal_search` Tauri command.
  - Added manual re-analysis support via `journal_request_analysis` instead of the previous dead Analyze button.
  - Removed the raw frontend event emission path so journal analysis now stays on the typed `app_event` architecture.
- **AI Surface Fixes**:
  - Fixed the right sidebar selector in `src/js/ai-sidebar.js` so the always-mounted Thinking Partner panel initializes correctly.
  - Added sidebar message sending and repaired `jumpToAiChat` to use the valid `ai_tab` source instead of an unsupported conversation source.
  - Removed markdown/HTML rendering from AI chat bubbles in `src/js/tabs/ai.js`, keeping responses plain-text and aligned with project rules.
  - Added conversation deletion controls to the AI conversation list.
- **Reports & CSS Hardening**:
  - Updated `src/js/tabs/reports.js` to refresh on actual emitted event types (`journal_saved`, `task_updated`, `task_created`, `task_completed`, `task_deleted`).
  - Added graceful chart fallbacks when Chart.js is unavailable rather than silently rendering blank canvases.
  - Normalized missing CSS variables in `src/styles/base.css` and added layout/settings/report responsiveness and fallback styling across `layout.css`, `components.css`, `ai-chat.css`, `tasks.css`, and `reports.css`.
- **Verification**:
  - `cargo check`
  - `node --check src/js/app.js`
  - `node --check src/js/tabs/journal.js`
  - `node --check src/js/tabs/settings.js`
  - `node --check src/js/tabs/ai.js`
  - `node --check src/js/tabs/reports.js`
  - `node --check src/js/ai-sidebar.js`

### Phase 3 Prompt, Tool, and Analysis Contract Cleanup

- **Prompt Refinement**:
  - Reworked `src-tauri/src/ai/prompt.rs` to remove contradictory tool-use instructions and make the assistant's decision rules explicit.
  - Tightened the analysis prompt to forbid vague/non-actionable tasks and reduce duplicate task extraction.
- **Tool Contract Alignment**:
  - Refactored `src-tauri/src/ai/tools.rs` so Ollama tool payloads are derived from a single `get_tool_definitions()` source of truth.
  - Aligned `list_tasks` tool arguments with `TaskListFilters` (`statuses`, `exclude_statuses`, `priorities`, `project_id`, date ranges, energy/context filters).
  - Updated `search_journal` to accept real date filters and validate structured arguments instead of silently ignoring unsupported fields.
  - Reduced `list_tasks` tool responses to compact task summaries to improve narration quality and preserve context budget.
- **Analysis Reliability**:
  - Made `AnalysisResult::from_raw` in `src-tauri/src/ai/mod.rs` validate required fields and filter empty task/insight entries.
  - Changed `src-tauri/src/ai/client.rs` to fail loudly on invalid analysis JSON/schema mismatches instead of silently fabricating a neutral fallback result.
- **Analysis Persistence**:
  - Extended `journal_entries` with persisted AI analysis fields (`analysis_summary`, `analysis_mood`, `analysis_insights`) via `src-tauri/migrations/008_ai_analysis_fields.sql`.
  - Updated journal read/write paths in `src-tauri/src/db/journal.rs`, `src-tauri/src/db/ai.rs`, and `src-tauri/src/lib.rs` to store and retrieve richer AI analysis output.
- **Verification**:
  - `cargo check`
  - `cargo test ai::analysis --lib`
  - `cargo test db::migrations --lib`

### Phase 3 Audit & Scheduler Fixes

- **AI Analysis Resilience**:
  - Hardened `src-tauri/src/ai/mod.rs` so journal analysis can recover when Ollama returns a blank `summary` or `mood`, using insight/source-text fallbacks instead of failing the sidebar analysis outright.
  - Updated `src-tauri/src/ai/client.rs` to pass the journal content into analysis normalization so fallback summaries can be derived deterministically from the saved entry text.
- **Formatting Cleanup**:
  - Removed trailing whitespace in `src-tauri/src/lib.rs` and `src-tauri/src/db/ai.rs` so `cargo fmt` runs cleanly again.
- **Build Fixes**:
  - Fixed `src-tauri/src/scheduler/weekly_report.rs` imports for `rusqlite::params` and `tauri::Manager`.
  - Removed the unused `AnalysisResult` import in `src-tauri/src/ai/analysis.rs`.
- **Weekly Review Runtime Hardening**:
  - Refactored `maybe_run_weekly_review` in `src-tauri/src/scheduler/weekly_report.rs` to acquire the SQLite mutex only for short DB operations instead of holding it across the async Ollama call.
  - Fixed the background scheduler in `src-tauri/src/lib.rs` to actually await the Monday weekly review future.
  - Weekly review completion is now recorded only after a successful AI response, preventing false "already ran" state after failures.
  - Emitted the typed `WeeklyReviewGenerated` event in addition to the existing system status toast.
- **Task Tab UX Fixes**:
  - Updated `src/index.html`, `src/js/tabs/tasks.js`, and `src/styles/tasks.css` to add explicit task view switching with `Kanban`, `List`, and `Calendar` views.
  - Added list/calendar rendering in `src/js/tabs/tasks.js` so due-dated tasks are now visible in a real calendar-style grouped view instead of the previously missing calendar surface.
  - Hardened drag/drop status changes in `src/js/tabs/tasks.js` by reading the destination status from explicit `data-status` attributes on drop columns.
  - Added an explicit `Manage` projects button in the task header and made project rows in the project modal clearly clickable for editing existing projects.
  - Updated modal sizing in `src/styles/components.css` so the project management list is scrollable and accessible when many projects exist.
- **Verification**:
  - `node --check src/js/tabs/tasks.js`
  - `node --check src/js/ai-sidebar.js`
  - `cargo test -q` (src-tauri)

- **Companion Nudge + Reflection Runtime**:
  - Added `src-tauri/src/ai/companion.rs` to drive proactive nudges and reflection prompts on top of the shared semantic-memory layer, including app-settings-backed dedupe history for weekly nudges and 7-day reflection prompt reuse protection.
  - Extended `src-tauri/src/ai/prompt.rs` with `ProactiveNudge` and `ReflectionPrompt` modes so Ollama can generate short companion nudges and entry prompts without tool use.
  - Added `ai_maybe_emit_proactive_nudge` and `ai_generate_reflection_prompt` commands in `src-tauri/src/lib.rs`, and new `AiProactiveNudge` / `AiReflectionPrompt` events in `src-tauri/src/events.rs`.
  - Updated `src/js/ai-sidebar.js` and `src/index.html` so the always-mounted right sidebar now renders proactive companion cards and reflection prompts, supports dismiss, supports “Try Another,” and requests an app-open nudge automatically.
  - Updated `src/js/tabs/journal.js` so new blank entries schedule a reflection prompt after 1.2s and an empty-entry proactive nudge after 30s of inactivity.
  - Added sidebar companion card styling in `src/styles/ai-chat.css`.
- **Tests & Verification**:
  - Added companion history/decision tests in `src-tauri/src/ai/companion.rs`.
  - `cargo fmt` (src-tauri)
  - `cargo test -q` (src-tauri)
  - `node --check src/js/ai-sidebar.js`
  - `node --check src/js/tabs/journal.js`

- **AI Companion Test Coverage**:
  - Added in-memory DB tests in `src-tauri/src/ai/memory.rs` covering semantic-memory assembly, recent-pattern generation, current-entry binding, entry exclusion from related-memory snippets, and empty-database fallback behavior.
  - Added prompt rendering coverage in `src-tauri/src/ai/prompt.rs` for semantic memory, recent patterns, related journal memory, and current-entry blocks.
  - Added weekly scheduler tests in `src-tauri/src/scheduler/weekly_report.rs` for `has_review_run_this_week` across missing, current-week, and invalid-date settings values.
- **Verification**:
  - `cargo fmt` (src-tauri)
  - `cargo test -q` (src-tauri)

- **AI Semantic Memory Layer**:
  - Added `src-tauri/src/ai/memory.rs` to build reusable prompt memory from existing journal/task data: semantic memory bullets (cadence, streak, dominant moods, recurring tags/emotions/insights, task open loops), recent journal patterns, related journal snippets, and optional bound entry context.
  - Updated `src-tauri/src/lib.rs` so `ai_chat` now uses the shared memory builder instead of ad hoc recent-entry injection.
  - Extended `src-tauri/src/ai/prompt.rs` with a dedicated `semantic_memory` prompt block so the companion can reason from higher-level patterns instead of only raw snippets.
  - Updated `src-tauri/src/ai/client.rs` with `chat_single_with_input(...)` so non-chat AI flows can reuse the same prompt-memory contract.
  - Updated `src-tauri/src/scheduler/weekly_report.rs` so the weekly review now receives the same semantic journal memory layer as AI chat.
  - Added `list_entries_since` in `src-tauri/src/db/journal.rs` and reused `get_conversation` in `src-tauri/src/db/ai.rs` to support semantic-memory assembly without schema changes.
- **Verification**:
  - `cargo fmt` (src-tauri)
  - `cargo test -q` (src-tauri)

- **AI Chat Journal Memory Injection**:
  - Updated `src-tauri/src/lib.rs` so `ai_chat` now injects compact journal memory into the system prompt: recent analyzed entry patterns, recent journal memory snippets, and bound `source_entry_id` entry context when the chat is opened from a journal entry.
  - Added prompt-safe truncation and lightweight pattern synthesis in `src-tauri/src/lib.rs` to keep companion context useful without overloading the token budget.
  - Extended `PromptInput` in `src-tauri/src/ai/prompt.rs` with a `recent_patterns` layer and rendered it as a dedicated prompt block.
  - Added `get_conversation` in `src-tauri/src/db/ai.rs` so existing AI chats retain their linked journal entry context across later turns.
  - Added `list_recent_entries` in `src-tauri/src/db/journal.rs` for recent-memory prompt injection without changing persistence.
- **Verification**:
  - `cargo fmt` (src-tauri)
  - `cargo test -q` (src-tauri)

- **AI Companion Prompt Alignment**:
  - Updated `src-tauri/src/ai/prompt.rs` to align the live chat persona with `AgentDocs/HowAiShouldBehave.md`: stronger companion identity, anti-sycophancy rules, one-question limit, natural memory/pattern use, no unprompted task generation, and more reflective mode guidance without breaking the existing plain-text/tool-call runtime.
  - Softened the journal analysis prompt language in `src-tauri/src/ai/prompt.rs` to avoid a clinical tone while keeping the strict JSON extraction contract intact.
  - Added prompt-focused tests in `src-tauri/src/ai/prompt.rs` covering companion identity and realistic weekly-plan guidance.
- **Verification**:
  - `cargo fmt` (src-tauri)
  - `cargo test -q prompt` (src-tauri)

- **AI Chat Context Safety**:
  - Fixed `src-tauri/src/lib.rs` so historical assistant `tool_calls` are no longer re-injected into later chat turns, preventing stale tool replays from prior messages.

### Phase 3 AI Chat & Tool Engine Hardening

- **AI Stream Logic Fixes**: 
  - Refactored `ai_chat` in `lib.rs` to use typed `OllamaToolCall` structs instead of dynamic JSON access, ensuring type safety and compatibility with updated Ollama API responses.
  - Modified history loading to clear `tool_calls` from past messages, preventing unintended tool re-triggering during context injection (D-94).
  - Fixed syntax corruption in `lib.rs` assistant token stream and nested block logic.
- **Tool Engine Enhancements**:
  - Updated `execute_tool_call` in `tools.rs` to return a unique `call_id` for every tool invocation.
  - Implemented automatic UI feedback for read-only tools (search, list) by creating "Silent Action" cards in the AI chat tab (`src/js/tabs/ai.js`).
  - Hardened fallback tool parser in `fallback.rs` with empty name guards and known-tool filters to prevent false positives from conversational text.
- **API & Schema Alignment**:
  - Resolved `E0063` compilation errors by adding missing fields to `JournalEntry` (`emotions`, `tags`, `last_analysed_at`) and `Task` (`recurrence`, `next_occurrence`) initializers in `lib.rs`.
  - Standardized JSON macro usage in `lib.rs` with `use serde_json::json`.
- **System Prompt Refinement**: Updated thinking partner rules in `prompt.rs` to enforce strict plain-text-first conversational behavior and restricted tool usage.

### Project Review & Documentation Handover

- **Agent Alignment Review**: Evaluated current branch state against `PersonalLifeOS_MASTER_SPEC_v1.6` and `ADDENDUM_v1.7`.
- **Created Handover Document**: Recorded current milestones, target features, and spec deviations into `AgentDocs/HandoverState.md`.
- **Status Update**: Updated `AGENTS.md` to reflect Phase 2 conclusion and pivot into Phase 3/4. Noted the active architectural deviation pertaining to Project objects (D-13).

### Reports IPC + AI Sidebar Chat Fixes

- **Reports Backend Hardening**:
  - Added `days` input validation to `get_report_summary` in `src-tauri/src/lib.rs` (1–365) to avoid invalid IPC payloads.
  - Made `src-tauri/src/db/reports.rs` fail-soft so missing/partial report schema (views/tables) returns empty sections instead of failing the entire IPC call.
- **AI Sidebar Chat Reliability**:
  - Updated `src-tauri/src/ai/tools.rs` to strip blank optional string fields (e.g., `due_before: ""`) before validating/deserializing tool arguments, preventing repeated ISO-date validation loops.
  - Updated `src/js/ai-sidebar.js` to persist `conversationId` across sidebar messages (D-105), instead of creating a new conversation on every send.
- **Verification**:
  - `cargo test -q` (src-tauri)

### AI Bulk-Intent Understanding (Task IDs)

- **Tool ID Validation**:
  - Tightened `update_task` / `complete_task` tool schemas and validation in `src-tauri/src/ai/tools.rs` so human phrases like `"all tasks"` can’t be misinterpreted as a task ID.
- **ID Prefix Resolution**:
  - Updated `src-tauri/src/db/tasks.rs` AI tool executor to resolve short task-id prefixes (from compact context injection like `[a1b2c3]`) to full UUIDs, with clear errors for not-found or ambiguous prefixes.
- **Prompt Rule**:
  - Updated `src-tauri/src/ai/prompt.rs` to treat phrases like “all tasks / everything / delete all” as bulk requests and ask a clarifying question or explain limitations instead of inventing a task name.
- **Verification**:
  - `cargo test -q` (src-tauri)

### Reports Upgrade: More Insights + More Charts

- **Backend Report Enrichment**:
  - Extended `get_report_summary` payload in `src-tauri/src/db/reports.rs` with journal consistency (entries/words per day), mood distribution, task status breakdown, due-date buckets, completion rate, streak days, and an `insights` list.
- **Frontend Report Visuals**:
  - Expanded the Reports tab layout in `src/index.html` with new cards (Highlights, Journal Consistency, Mood Distribution, Task Status, Due Buckets).
  - Updated `src/js/tabs/reports.js` to render the new sections and added lightweight SVG chart fallbacks so charts still render even when Chart.js is not vendored yet.
  - Added SVG chart styling in `src/styles/reports.css` for consistent dark-theme presentation.
- **Verification**:
  - `cargo test -q` (src-tauri)
  - `node --check src/js/tabs/reports.js`

### AI Chat Tool Robustness (Task Listing + IDs)

- **Tool Argument Normalization**:
  - Updated `src-tauri/src/ai/tools.rs` so read-only tools normalize common malformed `arguments` (like `[]`, `null`, or JSON strings) instead of erroring silently.
  - Standardized `list_tasks` and `search_journal` tool results to return `{ status, count, ... }` objects.
- **Prompt Guidance**:
  - Updated `src-tauri/src/ai/prompt.rs` to always include short task IDs like `[a1b2c3]` when listing/referencing tasks and to handle “delete” as cancel/complete with clarification.
- **AI Tab UX**:
  - Updated `src/js/tabs/ai.js` to render `list_tasks` results directly in the tool card with `[id6]` prefixes so users can refer to tasks unambiguously.
- **Verification**:
  - `cargo test -q` (src-tauri)
  - `node --check src/js/tabs/ai.js`

### AI Task Deletion + Full-ID Context

- **New Tool: `delete_task` (Confirmed Soft-Delete)**:
  - Added `delete_task` to the AI tool contract in `src-tauri/src/ai/tools.rs` (confirmation required, D-19/D-95).
  - Implemented execution via existing `tasks::soft_delete` (including subtasks) and emits `TaskDeleted` in `src-tauri/src/db/tasks.rs`.
- **Better Task Referencing**:
  - Updated task context injection in `src-tauri/src/ai/prompt.rs` to include both `[id6]` and the full UUID (`id=...`) so the model can reliably call tools without guessing.
  - Updated prompt guidance to use `delete_task` for delete requests and to resolve by title using `list_tasks` when needed.
- **Verification**:
  - `cargo test -q` (src-tauri)

### AI Persona + Memory Debugging (Prompt)

- **Tone & Guidance**:
  - Replaced the “clinical” persona in `src-tauri/src/ai/prompt.rs` with a warmer, more human thinking-partner style while keeping strict tool/confirmation rules.
  - Restored D-94 compact task injection format (`[id6] ...`) and clarified in the prompt that `[id6]` is a resolvable ID prefix for tool calls.
- **Dev Verification Aids**:
  - Added debug-only console logs in `src-tauri/src/lib.rs` showing history/message counts sent to Ollama (no message content).
- **Verification**:
  - `cargo test -q` (src-tauri)

### Task Resolution By Name/Recency (No New Tools)

- **AI Tooling (D-111 aligned)**:
  - Extended `list_tasks` tool and `TaskListFilters` with `query`, `match_recent`, and `limit` so the AI can resolve tasks by name/keyword/recency without adding an extra tool (`src-tauri/src/ai/tools.rs`, `src-tauri/src/db/tasks.rs`).
- **Prompt Workflow**:
  - Added an explicit resolve-first workflow instructing the model to use `list_tasks` before `delete_task` / `complete_task` / `update_task when the user refers to a task vaguely or by name (`src-tauri/src/ai/prompt.rs`).
- **Docs**:
  - Rewrote `AgentDocs/aiimprovementplan.md` to reflect the project-aligned approach (7-tool limit; use `list_tasks` for resolution).
- **Verification**:
  - `cargo test -q` (src-tauri)

### Backend Task Reference Auto-Resolution

- **AI Action Hardening**:
  - Updated `src-tauri/src/db/tasks.rs` so `update_task`, `complete_task`, and `delete_task` can resolve task references from full UUIDs, `[id6]` prefixes, or title/keyword phrases before mutating the database.
  - Improved `search_tasks` in `src-tauri/src/db/tasks.rs` to use case-insensitive fuzzy matching across `title`, `description`, and `notes`, with better ranking for exact and prefix title matches.
- **Validation Alignment**:
  - Relaxed mutating tool `id` validation/schema in `src-tauri/src/ai/tools.rs` so non-empty task references are allowed and resolved server-side instead of being rejected up front.
- **Verification**:
  - `cargo test -q` (src-tauri)

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
