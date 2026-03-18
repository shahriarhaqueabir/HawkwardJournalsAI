# PersonalLifeOS — Specification Addendum v1.7
## Ollama Model Discovery · Weekly Review Trigger · AI Tools Scope
**Date:** 2026-03-15 | **Addendum to:** v1.6 | **Status:** All Decisions Locked

---

## WHAT CHANGED IN v1.7

| # | Change | Sections affected |
|---|--------|------------------|
| D-108 | Ollama models directory: auto-detect silently, show + override in Settings | §3D, §10, §13 |
| D-109 | Weekly review: fires Monday 08:00 OR first app launch on Monday (whichever comes first) | §13E, §18 Phase 4 |
| D-110 | Web search deferred to backlog (B-03) | §13, §21 Backlog |
| D-111 | AI tools scope confirmed: web fetch only for now | §13D |

---

## NEW DECISIONS

| # | Topic | Decision |
|---|-------|----------|
| D-108 | Ollama models directory | Auto-detect at startup: check `OLLAMA_MODELS` env var first, fall back to `%USERPROFILE%\.ollama\models`. Resolved path shown in Settings > AI. User can override by entering a custom path. Override stored in `app_settings`. |
| D-109 | Weekly review trigger | Fires at Monday 08:00 local time if app is running. Also fires on first app launch on any Monday if this week's review has not yet run. Whichever condition is met first. |
| D-110 | Web search | Deferred — added to feature backlog as B-03. `fetch_url` (specific URL) remains the only web tool. |
| D-111 | AI tools scope | Two tools only: `fetch_url` (read a specific URL) and the five task/journal tools. File system, shell, and code execution tools are not built. |

---

## 1. OLLAMA MODEL DIRECTORY DETECTION (D-108)

### 1A. Detection Logic (Rust — `src-tauri/src/ai/client.rs`)

```rust
pub fn resolve_ollama_models_dir() -> PathBuf {
    // Step 1: Check OLLAMA_MODELS environment variable (user-set override)
    if let Ok(env_path) = std::env::var("OLLAMA_MODELS") {
        let p = PathBuf::from(env_path);
        if p.exists() {
            return p;
        }
    }

    // Step 2: Check app_settings for user-overridden path (set in Settings > AI)
    if let Some(setting_path) = get_setting_sync("ollama_models_dir") {
        if let Some(s) = setting_path.as_str() {
            let p = PathBuf::from(s);
            if p.exists() {
                return p;
            }
        }
    }

    // Step 3: Fall back to Windows default
    // %USERPROFILE%\.ollama\models
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("C:\\Users\\Default"))
        .join(".ollama")
        .join("models")
}
```

Resolution priority order:
1. `OLLAMA_MODELS` environment variable (Ollama's own override mechanism)
2. User-set path in `app_settings` (stored from Settings > AI override field)
3. Default Windows path: `%USERPROFILE%\.ollama\models`

### 1B. What the Resolved Path Is Used For

The resolved models directory is used for **two purposes only**:

- **Display**: shown in Settings > AI as an informational field so the user can verify the app is looking in the right place.
- **Disk size calculation**: the app can show total models storage size by scanning the directory. This is optional/cosmetic — not required for any functional operation.

**Model listing itself always uses the Ollama REST API** (`GET /api/tags`), never the filesystem. The directory path is never used to load models or verify their availability. Reason: Ollama manages its own model registry; reading the filesystem directly can produce stale or partial results.

### 1C. New `app_settings` Key

```
ollama_models_dir    String    null    — null = use auto-detected path
```

When null: auto-detection runs every time. When set: the stored path takes precedence over auto-detection (but `OLLAMA_MODELS` env var still wins over both — respects Ollama's own configuration).

### 1D. Settings > AI — Updated UI

```
── AI Engine ────────────────────────────────────────────────────────────
Ollama URL:     [http://localhost:11434        ]  [Test connection]
Active model:   [llama3.2                     ▼]

── Model Storage ────────────────────────────────────────────────────────
Models directory (auto-detected):
  [C:\Users\Abir\.ollama\models               ]  [Browse]
  Storage used: 4.2 GB across 3 models
  ℹ️ Override OLLAMA_MODELS env var or this field to change location.
     App always uses /api/tags to list models — this path is for display only.
```

- **[Browse]** opens a folder picker dialog (`tauri-plugin-dialog`). Selected path saved to `app_settings.ollama_models_dir`.
- **Storage used** is computed by scanning the directory with `std::fs::read_dir` and summing file sizes. Shown in human-readable form (GB/MB).
- If the resolved path does not exist: shows `⚠️ Path not found — models may be in a different location` in amber.

### 1E. New Rust Handler

```rust
#[tauri::command]
pub fn get_ollama_models_dir_info(state: State<DbState>) -> Result<ModelsDirInfo>
  // Resolves path using priority order above
  // Scans directory for total size if path exists
  // Returns: { path: String, exists: bool, size_bytes: Option<u64>, source: String }
  // source = "env_var" | "settings_override" | "default"
  // Frontend: Settings > AI model storage section

pub struct ModelsDirInfo {
    pub path:       String,
    pub exists:     bool,
    pub size_bytes: Option<u64>,  // None if path doesn't exist or scan failed
    pub source:     String,       // where the path came from
}
```

Add `get_ollama_models_dir_info` to the IPC contract table in §8A.

---

## 2. WEEKLY REVIEW TRIGGER UPDATE (D-109)

### 2A. Previous behaviour (v1.6)

The weekly review fired at Monday 08:00 local time via a tokio interval scheduler. If the app was not running at 08:00, the review never fired that week.

### 2B. New behaviour (D-109)

Two independent conditions, either of which triggers the review. Whichever fires first:

**Condition A — Scheduled time:**
App is running → tokio interval checks every minute → if current time is Monday AND current time >= 08:00 AND this week's review has not yet run → fire.

**Condition B — First Monday launch:**
App starts → startup sequence → check if today is Monday AND this week's review has not yet run → fire asynchronously (non-blocking, 5-second delay after startup to let the app finish loading first).

"This week's review has not yet run" is determined by a new `app_settings` key:

```
weekly_review_last_run    String    null    — ISO 8601 date of last review generation
```

The scheduler writes this date after successfully completing the review. The check compares this date to the current ISO week number.

### 2C. Scheduler Logic (Rust — `src-tauri/src/scheduler/weekly_report.rs`)

```rust
pub fn has_review_run_this_week(conn: &Connection) -> bool {
    let last_run = get_setting_sync(conn, "weekly_review_last_run")
        .and_then(|v| NaiveDate::parse_from_str(v.as_str()?, "%Y-%m-%d").ok());

    let today = chrono::Local::now().date_naive();

    match last_run {
        None => false,   // never run
        Some(date) => {
            // Same ISO week number and same year
            date.iso_week() == today.iso_week()
                && date.year() == today.year()
        }
    }
}

// Called by both the interval scheduler and the startup sequence
pub async fn maybe_run_weekly_review(app: &AppHandle, conn: &Connection) {
    let today = chrono::Local::now().date_naive();
    let weekday = today.weekday();

    // Only runs on Monday
    if weekday != chrono::Weekday::Mon {
        return;
    }

    // Only runs once per week
    if has_review_run_this_week(conn) {
        return;
    }

    log::info!("[SCHEDULER] Running weekly review for week of {}", today);
    run_weekly_review_and_plan(app, conn).await;

    // Mark as run for this week
    setting_set_internal(conn, "weekly_review_last_run", today.to_string()).ok();
}

// Interval scheduler: checks every minute while app is running
pub async fn start_weekly_scheduler(app: AppHandle, conn: Arc<Mutex<Connection>>) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        let conn = conn.lock().await;

        // Time check: only fire at or after 08:00 local time
        let now = chrono::Local::now();
        if now.hour() >= 8 {
            maybe_run_weekly_review(&app, &conn).await;
        }
    }
}
```

### 2D. Startup Sequence Addition

In `db/init.rs`, step 7 becomes:

```rust
// Step 7 (existing): emit startup_complete, open Journal tab
// Step 8 (NEW — D-109): check for missed Monday weekly review
tokio::spawn(async move {
    tokio::time::sleep(Duration::from_secs(5)).await;  // let app finish loading
    maybe_run_weekly_review(&app, &conn).await;
});
```

This 5-second delay ensures the UI is fully loaded before the review fires, so the `WeeklyReportReady` toast appears after the user sees the app — not before.

### 2E. User-visible behaviour summary

| Scenario | What happens |
|----------|-------------|
| App running at Monday 08:00 | Review fires at 08:00 |
| App closed at 08:00, user opens app at 10:00 Monday | Review fires 5 seconds after app opens |
| App opened Monday 07:55, left open | Review fires at 08:00 |
| App opened Tuesday (review already ran) | Nothing — `weekly_review_last_run` is this week |
| App opened Monday, review runs, app restarted same day | Nothing — already ran this week |
| User clicks "Regenerate" in Reports | Runs on demand regardless of schedule |

---

## 3. WEB SEARCH — DEFERRED (D-110)

Web search is moved to the feature backlog as **B-03**.

### 3A. Current state (unchanged from v1.6)

`fetch_url` remains the only web tool. The AI can read a specific URL the user provides. It cannot perform a search query.

### 3B. Backlog entry B-03: Web Search

**Summary:** Allow the AI to search the web for information given a query, not just fetch a specific URL. Results are returned to the AI for synthesis.

**Why deferred:** No search provider was acceptable. A configurable search endpoint (user provides their own API URL) is the most flexible approach and does not lock the app to any single provider. This requires a Settings > AI section for "Search endpoint URL + API key" and a new `web_search` tool definition.

**What needs to be decided before implementation:**
- Search provider (the user will specify when ready to build this)
- API key storage (stored in `app_settings`, never in the DB with other data)
- Result count per query (3–10 results recommended)
- Whether results are injected as context or returned to AI for tool-calling

**Recommended approach when ready:** A configurable endpoint field in Settings > AI. The user pastes their own search API URL (compatible with any provider that returns JSON results). The `web_search` Rust handler sends the query to that URL, parses the response, and returns formatted results to the AI.

---

## 4. AI TOOLS SCOPE CONFIRMED (D-111)

### 4A. Current tool set (7 tools total)

```
TASK TOOLS (5):
  create_task       — propose a new task (requires confirmation)
  update_task       — modify task fields (requires confirmation)
  complete_task     — mark a task done (requires confirmation)
  list_tasks        — query tasks with filters (no confirmation needed — read-only)
  search_journal    — search journal entries by keyword/date (read-only)

WEB TOOLS (1):
  fetch_url         — fetch the text content of a specific URL (read-only)

DEFERRED (backlog B-03):
  web_search        — search query across the web (not built yet)
```

### 4B. Tools explicitly not built

| Tool | Reason |
|------|--------|
| File system read | Not requested — not in scope |
| Shell / terminal commands | Not requested — not in scope |
| Calculator / code execution | Not requested — not in scope |
| Web search | Deferred — B-03 |

### 4C. Tool confirmation rules (unchanged from D-19, D-95)

| Tool | Requires confirmation? |
|------|----------------------|
| `create_task` | ✅ Yes — writes to DB |
| `update_task` | ✅ Yes — writes to DB |
| `complete_task` | ✅ Yes — writes to DB |
| `list_tasks` | ✅ No — read only |
| `search_journal` | ✅ No — read only |
| `fetch_url` | ✅ No — read only (network fetch, no DB write) |

Timeout: 300 seconds on all confirmations (D-95). Auto-cancel if user does not respond.

---

## 5. UPDATED SPEC SECTIONS

### Settings > AI — Full Updated Layout

```
── AI Engine ────────────────────────────────────────────────────────────
Ollama URL:     [http://localhost:11434        ]  [Test connection]

── Model ────────────────────────────────────────────────────────────────
Active model:   [llama3.2                     ▼]  (from /api/tags)

── Parameters ───────────────────────────────────────────────────────────
Temperature:    [────●──────────] 0.7   (0.0 – 2.0)
Top-P:          [──────●────────] 0.9   (0.0 – 1.0)
Top-K:          [40]
Context length: [16384          ▼]      ⚠️ 16GB RAM recommended at this setting
System prompt:  [                                    ]
                (empty = use built-in Thinking Partner prompt)

── Model Storage ────────────────────────────────────────────────────────
Models directory:
  [C:\Users\Abir\.ollama\models               ]  [Browse]
  Source: Windows default path
  Storage used: 4.2 GB across 3 models
  ℹ️ Set OLLAMA_MODELS env var or use Browse to change this location.
     Model listing always uses the Ollama API — this is display only.

[Reset AI settings to defaults]
```

### Terminal log messages added

```
[INFO][AI  ] Models directory: C:\Users\Abir\.ollama\models (source: default)
[INFO][AI  ] Models directory: D:\models (source: env_var OLLAMA_MODELS)
[INFO][SCHED] Weekly review check: Monday detected, not yet run this week — starting
[INFO][SCHED] Weekly review check: already ran this week (2026-03-16) — skipping
[INFO][SCHED] Weekly review fired on startup (missed scheduled time)
```

---

## 6. BUILD PHASE ADDITIONS

### Phase 0 additions
- [ ] `resolve_ollama_models_dir()` function in `ai/client.rs` (D-108)
- [ ] Seed `app_settings`: `ollama_models_dir = null`, `weekly_review_last_run = null`

### Phase 3 additions (AI module)
- [ ] `get_ollama_models_dir_info` Tauri command (D-108)
- [ ] Models directory display in Settings > AI (path + source label + storage size)
- [ ] [Browse] folder picker wired to `setting_set("ollama_models_dir", path)`

### Phase 4 additions (Reports + Schedulers)
- [ ] `has_review_run_this_week()` function (D-109)
- [ ] `maybe_run_weekly_review()` called by both interval scheduler AND startup (D-109)
- [ ] `start_weekly_scheduler()` — 60s interval, fires after 08:00 on Monday (D-109)
- [ ] Startup step 8: `tokio::spawn` with 5s delay for Monday check (D-109)
- [ ] `app_settings` key `weekly_review_last_run` written after every successful generation

---

## 7. UPDATED FEATURE BACKLOG

### B-01: AI Training Zone (unchanged)
### B-02: AI Insights Tab (unchanged)

### B-03: Web Search (NEW — D-110)

**Summary:** Give the AI the ability to search the web with a text query and receive ranked results, rather than only being able to fetch a specific URL.

**Recommended implementation:** Configurable search endpoint in Settings > AI. User provides their own search API URL. App sends query, parses JSON results, returns to AI as `web_search` tool result.

**New tool definition when built:**
```json
{
  "name": "web_search",
  "description": "Search the web for information given a query. Returns top results with titles, URLs, and summaries.",
  "parameters": {
    "query": { "type": "string", "description": "The search query" },
    "num_results": { "type": "integer", "default": 5, "max": 10 }
  }
}
```

**Decisions needed before build:**
- Search provider selection (user will specify)
- API key storage approach
- Result format normalisation across providers

---

## 8. COMPLETE DECISION REGISTER — v1.7 ADDITIONS

| # | Topic | Decision |
|---|-------|----------|
| D-108 | Ollama models directory | Auto-detect: `OLLAMA_MODELS` env var → `app_settings` override → `%USERPROFILE%\.ollama\models`. Show resolved path + source in Settings > AI. User can override via Browse button. Model listing always uses `/api/tags` API — directory is display only. |
| D-109 | Weekly review trigger | Monday 08:00 if app is running **OR** first Monday app launch if review hasn't run this week. Tracked via `weekly_review_last_run` in `app_settings`. 5-second startup delay before check fires. |
| D-110 | Web search | **Deferred to backlog B-03.** `fetch_url` (specific URL) is the only web tool. |
| D-111 | AI tools scope | 7 tools total: `create_task`, `update_task`, `complete_task`, `list_tasks`, `search_journal`, `fetch_url`. No file system, shell, or code execution tools. Web search in backlog. |

---

*PersonalLifeOS Specification Addendum v1.7*
*Decisions: D-108 through D-111 added (111 total locked decisions)*
*Backlog: B-01 AI Training Zone · B-02 AI Insights · B-03 Web Search*
*Merges cleanly into Master Spec v1.6*
