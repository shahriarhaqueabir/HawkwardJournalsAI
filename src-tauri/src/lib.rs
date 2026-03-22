mod ai;
mod backup;
mod db;
mod error;
mod events;
mod logger;
mod scheduler;

use chrono::Utc;
use db::paths::resolve_data_dir;
use error::AppError;
use rusqlite::Connection;
use std::sync::Arc;
use tauri::{Listener, Manager};
use tokio::sync::Mutex;
use uuid::Uuid;

fn ai_tool_confirmed_flag(tool_name: &str, result: &serde_json::Value) -> Option<i32> {
    let is_mutating = matches!(
        tool_name,
        "create_task" | "update_task" | "complete_task" | "delete_task"
    );
    if !is_mutating {
        return None;
    }

    match result.get("status").and_then(|value| value.as_str()) {
        Some("cancelled") | Some("error") => Some(0),
        _ => Some(1),
    }
}

#[derive(Clone)]
pub struct AppState {
    pub conn: Arc<Mutex<Connection>>,
    pub data_dir: std::path::PathBuf,
    pub ai_state: Arc<ai::analysis::AnalysisState>,
    pub ai_tool_state: Arc<ai::tools::AiToolState>,
    pub ollama: Arc<ai::client::OllamaClient>,
    pub handle: tauri::AppHandle,
}

#[tauri::command]
async fn save_journal_entry(
    state: tauri::State<'_, AppState>,
    id: String,
    title: Option<String>,
    content: String,
) -> Result<String, AppError> {
    let now = Utc::now().to_rfc3339();
    // If id is empty, it's a new entry
    let entry_id = if id.is_empty() {
        Uuid::new_v4().to_string()
    } else {
        id
    };

    let title_clone = title.clone();
    let word_count = content.split_whitespace().count() as i64;

    // D-112: Trigger Background Analysis (Check BEFORE moving content)
    let should_analyze = state.ai_state.should_analyze(&entry_id, &content).await;

    let conn = state.conn.lock().await;

    // Preserve analysis data if updating an existing entry
    let existing = db::journal::get_entry_by_id(&conn, &entry_id).unwrap_or(None);

    let mut entry = db::journal::JournalEntry {
        id: entry_id.clone(),
        title: title_clone,
        content,
        analysis_summary: None,
        analysis_mood: None,
        analysis_insights: "".into(),
        emotions: "[]".into(),
        tags: "[]".into(),
        last_analysis_conv_id: None,
        last_analysed_at: None,
        word_count,
        created_at: now.clone(),
        updated_at: now.clone(),
    };

    if let Some(existing) = existing {
        entry.created_at = existing.created_at.clone();
        db::journal::merge_analysis_data(&existing, &mut entry);
    }

    db::journal::upsert_entry(&conn, &entry)?;
    drop(conn);

    if should_analyze {
        let mut queue = state.ai_state.queue.lock().await;
        let mut queued_ids = state.ai_state.queued_ids.lock().await;

        if queue.len() < ai::analysis::MAX_QUEUE && !queued_ids.contains(&entry_id) {
            queue.push_back(entry_id.clone());
            queued_ids.insert(entry_id.clone());

            // Wake up the background loop
            state.ai_state.notify.notify_one();

            crate::events::emit(
                &state.handle,
                crate::events::AppEvent::JournalAnalysisQueued {
                    entry_id: entry_id.clone(),
                },
            );
        }
    }

    crate::events::emit(
        &state.handle,
        crate::events::AppEvent::JournalSaved {
            entry_id: entry_id.clone(),
        },
    );

    Ok(entry_id)
}

#[tauri::command]
async fn get_report_summary(
    state: tauri::State<'_, AppState>,
    days: i32,
) -> Result<db::reports::ReportData, AppError> {
    if !(1..=365).contains(&days) {
        return Err(AppError::InvalidInput(
            "days must be between 1 and 365".into(),
        ));
    }
    let conn = state.conn.lock().await;
    db::reports::get_report_data(&conn, days)
}

#[tauri::command]
async fn settings_list(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<db::settings::SettingItem>, AppError> {
    let conn = state.conn.lock().await;
    db::settings::list_settings(&conn)
}

#[tauri::command]
async fn setting_get(
    state: tauri::State<'_, AppState>,
    key: String,
) -> Result<Option<db::settings::SettingItem>, AppError> {
    let conn = state.conn.lock().await;
    db::settings::get_setting(&conn, &key)
}

#[tauri::command]
async fn setting_set(
    state: tauri::State<'_, AppState>,
    key: String,
    value: String,
) -> Result<db::settings::SettingItem, AppError> {
    let conn = state.conn.lock().await;
    db::settings::set_setting(&conn, &key, &value)
}

#[tauri::command]
async fn journal_get(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<Option<db::journal::JournalEntry>, AppError> {
    let conn = state.conn.lock().await;
    db::journal::get_entry(&conn, &id)
}

#[tauri::command]
async fn journal_list(
    state: tauri::State<'_, AppState>,
    cursor: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<db::journal::JournalEntrySummary>, AppError> {
    let conn = state.conn.lock().await;
    let limit = limit.unwrap_or(20);
    db::journal::list_entries(&conn, cursor.as_deref(), limit)
}

#[tauri::command]
async fn journal_delete(state: tauri::State<'_, AppState>, id: String) -> Result<bool, AppError> {
    let conn = state.conn.lock().await;
    db::journal::soft_delete(&conn, &id)
}

#[tauri::command]
async fn journal_search(
    state: tauri::State<'_, AppState>,
    query: String,
    date_from: Option<String>,
    date_to: Option<String>,
) -> Result<Vec<serde_json::Value>, AppError> {
    let conn = state.conn.lock().await;
    db::journal::search_entries(
        &conn,
        &db::journal::JournalSearchFilters {
            query,
            date_from,
            date_to,
        },
    )
}

#[tauri::command]
async fn journal_request_analysis(
    state: tauri::State<'_, AppState>,
    entry_id: String,
) -> Result<(), AppError> {
    let conn = state.conn.lock().await;
    let exists = db::journal::get_entry(&conn, &entry_id)?.is_some();
    drop(conn);

    if !exists {
        return Err(AppError::NotFound("Journal entry not found".into()));
    }

    let mut queue = state.ai_state.queue.lock().await;
    let mut queued_ids = state.ai_state.queued_ids.lock().await;

    if queue.len() >= ai::analysis::MAX_QUEUE {
        return Err(AppError::AiError("Analysis queue is full".into()));
    }

    if !queued_ids.contains(&entry_id) {
        queue.push_back(entry_id.clone());
        queued_ids.insert(entry_id.clone());
    }

    state.ai_state.notify.notify_one();
    crate::events::emit(
        &state.handle,
        crate::events::AppEvent::JournalAnalysisQueued { entry_id },
    );

    Ok(())
}

#[tauri::command]
async fn task_create(
    state: tauri::State<'_, AppState>,
    title: String,
    parent_task_id: Option<String>,
    due_date: Option<String>,
    priority: Option<String>,
    project_id: Option<String>,
) -> Result<db::tasks::Task, AppError> {
    let now = Utc::now().to_rfc3339();
    let conn = state.conn.lock().await;

    // --- D-40: Enforce Subtask Depth (2 levels max) ---
    if let Some(ref pid) = parent_task_id {
        if let Some(parent) = db::tasks::get_task(&conn, pid)? {
            if parent.parent_task_id.is_some() {
                return Err(AppError::InvalidInput(
                    "Maximum subtask depth (2 levels) reached.".into(),
                ));
            }
        }
    }

    let mut task = db::tasks::Task {
        id: Uuid::new_v4().to_string(),
        parent_task_id,
        title,
        description: None,
        notes: None,
        status: "todo".into(),
        priority: priority.unwrap_or_else(|| "medium".into()),
        due_date,
        due_time: None,
        reminder_at: None,
        reminder_fired: false,
        time_estimate: None,
        time_logged: 0,
        actual_start_date: None,
        tags: "[]".into(),
        labels: "[]".into(),
        category: None,
        project: None,
        project_id: Some(project_id.unwrap_or_else(|| "inbox".into())),
        energy_level: None,
        context_tag: None,
        linked_url: None,
        sort_order: 0,
        recurrence: None,
        next_occurrence: None,
        ai_created: false,
        ai_conversation_id: None,
        created_at: now.clone(),
        updated_at: now,
        completed_at: None,
    };

    db::tasks::normalize_task_project(&conn, &mut task)?;
    db::tasks::create_task(&conn, &task)?;

    // §8A: Return full Task object
    crate::events::emit(
        &state.handle,
        crate::events::AppEvent::TaskCreated {
            id: task.id.clone(),
            title: task.title.clone(),
        },
    );

    Ok(task)
}

#[tauri::command]
async fn task_list(
    state: tauri::State<'_, AppState>,
    filters: Option<db::tasks::TaskListFilters>,
    exclude_statuses: Option<Vec<String>>,
) -> Result<Vec<db::tasks::Task>, AppError> {
    let conn = state.conn.lock().await;

    let mut actual_filters = filters.unwrap_or_default();

    if exclude_statuses.is_some() && actual_filters.exclude_statuses.is_none() {
        actual_filters.exclude_statuses = exclude_statuses;
    } else if actual_filters.exclude_statuses.is_none() && actual_filters.statuses.is_none() {
        actual_filters.exclude_statuses = Some(vec!["done".into(), "cancelled".into()]);
    }

    db::tasks::list_tasks(&conn, actual_filters)
}

#[tauri::command]
async fn task_update_status(
    state: tauri::State<'_, AppState>,
    id: String,
    status: String,
) -> Result<(), AppError> {
    let conn = state.conn.lock().await;

    match db::tasks::update_task_status(&conn, &id, &status) {
        Ok(_) => {
            if status == "done" {
                crate::events::emit(&state.handle, crate::events::AppEvent::TaskCompleted { id });

                // D-121: Trigger proactive reflection in background
                let state_clone = state.inner().clone();
                tauri::async_runtime::spawn(async move {
                    let _ =
                        ai_maybe_emit_proactive_nudge_internal(state_clone, "task_completed".into())
                            .await;
                });
            } else {
                crate::events::emit(&state.handle, crate::events::AppEvent::TaskUpdated { id });
            }
            Ok(())
        }
        Err(e) => {
            crate::events::emit(
                &state.handle,
                crate::events::AppEvent::DatabaseError {
                    operation: "task_update_status".into(),
                    error: e.to_string(),
                },
            );
            Err(e)
        }
    }
}

#[tauri::command]
async fn task_get(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<Option<db::tasks::Task>, AppError> {
    let conn = state.conn.lock().await;
    db::tasks::get_task(&conn, &id)
}

#[tauri::command]
async fn task_update(
    state: tauri::State<'_, AppState>,
    task: db::tasks::Task,
) -> Result<(), AppError> {
    let conn = state.conn.lock().await;
    let mut task = task;
    db::tasks::normalize_task_project(&conn, &mut task)?;
    db::tasks::update_task(&conn, &task)?;
    crate::events::emit(
        &state.handle,
        crate::events::AppEvent::TaskUpdated {
            id: task.id.clone(),
        },
    );
    Ok(())
}

#[tauri::command]
async fn task_search(
    state: tauri::State<'_, AppState>,
    query: String,
) -> Result<Vec<db::tasks::Task>, AppError> {
    let conn = state.conn.lock().await;
    db::tasks::search_tasks(&conn, &query)
}

#[tauri::command]
async fn task_add_dependency(
    state: tauri::State<'_, AppState>,
    blocked_task_id: String,
    blocking_task_id: String,
) -> Result<(), AppError> {
    let conn = state.conn.lock().await;
    db::tasks::task_add_dependency(&conn, &blocked_task_id, &blocking_task_id)
}

#[tauri::command]
async fn task_remove_dependency(
    state: tauri::State<'_, AppState>,
    blocked_task_id: String,
    blocking_task_id: String,
) -> Result<(), AppError> {
    let conn = state.conn.lock().await;
    db::tasks::task_remove_dependency(&conn, &blocked_task_id, &blocking_task_id)
}

#[tauri::command]
async fn timer_start(
    state: tauri::State<'_, AppState>,
    task_id: String,
) -> Result<String, AppError> {
    let conn = state.conn.lock().await;
    match db::tasks::timer_start(&conn, &task_id) {
        Ok(log_id) => {
            crate::events::emit(
                &state.handle,
                crate::events::AppEvent::TaskUpdated { id: task_id },
            );
            Ok(log_id)
        }
        Err(e) => Err(e),
    }
}

#[tauri::command]
async fn timer_stop(state: tauri::State<'_, AppState>, task_id: String) -> Result<(), AppError> {
    let conn = state.conn.lock().await;
    match db::tasks::timer_stop(&conn, &task_id) {
        Ok(_) => {
            crate::events::emit(
                &state.handle,
                crate::events::AppEvent::TaskUpdated { id: task_id },
            );
            Ok(())
        }
        Err(e) => Err(e),
    }
}

#[tauri::command]
async fn attachment_add(
    state: tauri::State<'_, AppState>,
    task_id: String,
    source_path: String,
) -> Result<db::tasks::TaskAttachment, AppError> {
    let conn = state.conn.lock().await;
    match db::tasks::attachment_add(&conn, &task_id, &source_path) {
        Ok(attachment) => {
            crate::events::emit(
                &state.handle,
                crate::events::AppEvent::TaskUpdated {
                    id: task_id.clone(),
                },
            );
            Ok(attachment)
        }
        Err(e) => Err(e),
    }
}

#[tauri::command]
async fn attachment_list(
    state: tauri::State<'_, AppState>,
    task_id: String,
) -> Result<Vec<db::tasks::TaskAttachment>, AppError> {
    let conn = state.conn.lock().await;
    db::tasks::attachment_list(&conn, &task_id)
}

#[tauri::command]
async fn attachment_remove(
    state: tauri::State<'_, AppState>,
    id: String,
    task_id: String,
) -> Result<(), AppError> {
    let conn = state.conn.lock().await;
    match db::tasks::attachment_remove(&conn, &id) {
        Ok(_) => {
            crate::events::emit(
                &state.handle,
                crate::events::AppEvent::TaskUpdated { id: task_id },
            );
            Ok(())
        }
        Err(e) => Err(e),
    }
}

#[tauri::command]
async fn ollama_health_check() -> Result<bool, AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .map_err(|e| AppError::AiError(e.to_string()))?;

    match client.get("http://127.0.0.1:11434/api/tags").send().await {
        Ok(r) => Ok(r.status().is_success()),
        Err(_) => Ok(false),
    }
}

/// Count the number of tokens in `text` using the cl100k_base (GPT-4 / LLaMA 3) vocabulary.
/// Returns the raw token count. Suitable for the Settings page token meter.
#[tauri::command]
fn ai_count_tokens(text: String) -> Result<usize, AppError> {
    Ok(ai::tokens::count_tokens(&text))
}

/// Return a full `TokenBudget` snapshot for `text` within a given `context_window`.
/// If `context_window` is 0, uses the default (16384, D-93).
#[tauri::command]
fn ai_get_token_budget(text: String, context_window: usize) -> Result<ai::tokens::TokenBudget, AppError> {
    let window = if context_window == 0 {
        ai::tokens::DEFAULT_CONTEXT_TOKENS
    } else {
        context_window
    };
    Ok(ai::tokens::TokenBudget::calculate(&text, window))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    crate::logger::init();
    let data_dir = resolve_data_dir();
    let conn_raw = db::init::initialise().expect("Failed to initialise database");
    let conn = Arc::new(Mutex::new(conn_raw));

    let ollama_client = Arc::new(ai::client::OllamaClient::new("llama3.2".into()));
    let ai_state = Arc::new(ai::analysis::AnalysisState::new());
    let ai_tool_state = Arc::new(crate::ai::tools::AiToolState::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .setup(move |app| {
            let handle = app.handle().clone();
            crate::logger::set_handle(handle.clone());

            let app_state = AppState {
                conn: conn.clone(),
                data_dir: data_dir.clone(),
                ai_state: ai_state.clone(),
                ai_tool_state: ai_tool_state.clone(),
                ollama: ollama_client.clone(),
                handle: handle.clone(),
            };
            app.manage(app_state.clone());

            // AI ANALYSIS WORKER (D-112: Deterministic Background Loop)
            let state_arc = Arc::new(app_state);
            tauri::async_runtime::spawn(async move {
                ai::analysis::start_analysis_worker(state_arc).await;
            });

            // Unified app_event listener
            app.listen("app_event", move |event: tauri::Event| {
                // Future event handlers can go here
                let _ = event;
            });

            // Step 8 (D-109): Check for missed Monday weekly review
            let review_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                let _ =
                    crate::scheduler::weekly_report::maybe_run_weekly_review(&review_handle).await;
            });

            // Step 9: Main application background scheduler worker
            let sched_handle = handle.clone();
            let sched_conn = conn.clone();
            tauri::async_runtime::spawn(async move {
                use chrono::{Datelike, Timelike};
                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
                loop {
                    interval.tick().await;
                    // 1. Weekly Review (Scheduled time check: Mon >= 08:00)
                    let now = chrono::Local::now();
                    if now.weekday() == chrono::Weekday::Mon && now.hour() >= 8 {
                        let _ =
                            crate::scheduler::weekly_report::maybe_run_weekly_review(&sched_handle)
                                .await;
                    }

                    let c = sched_conn.lock().await;

                    // 2. Poll Sub-minute Tasks (Reminders)
                    let _ = crate::scheduler::reminders::poll_reminders(&sched_handle, &c);

                    // 3. Poll Interval Recurrences (D-120)
                    let _ = crate::scheduler::recurrence::poll_recurrences(&sched_handle, &c);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ollama_health_check,
            save_journal_entry,
            journal_search,
            journal_request_analysis,
            get_report_summary,
            settings_list,
            setting_get,
            setting_set,
            journal_get,
            journal_list,
            journal_delete,
            project_create,
            project_get,
            project_list,
            project_update,
            project_delete,
            task_create,
            task_list,
            task_update_status,
            task_get,
            task_update,
            task_search,
            task_delete,
            task_add_dependency,
            task_remove_dependency,
            timer_start,
            timer_stop,
            attachment_add,
            attachment_list,
            attachment_remove,
            ai_chat,
            ai_maybe_emit_proactive_nudge,
            ai_generate_reflection_prompt,
            ai_conversation_list,
            ai_message_list,
            ai_conversation_delete,
            ai_confirm_tool,
            ai_list_pinned_memory,
            ai_upsert_pinned_memory,
            ai_delete_pinned_memory,
            trash_list,
            trash_empty,
            db_manual_backup,
            db_export_json,
            db_reset,
            db_get_audit_log,
            ai_get_queue_status,
            db_get_path,
            ai_count_tokens,
            ai_get_token_budget,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
async fn ai_chat(
    state: tauri::State<'_, AppState>,
    conversation_id: Option<String>,
    message: String,
    source: String,
    entry_id: Option<String>,
) -> Result<String, AppError> {
    let conn = state.conn.lock().await;

    // 1. Resolve or Create Conversation
    let existing_conversation = match conversation_id.as_deref() {
        Some(id) if !id.is_empty() => db::ai::get_conversation(&conn, id)?,
        _ => None,
    };
    let source_entry_id = entry_id.clone().or_else(|| {
        existing_conversation
            .as_ref()
            .and_then(|conv| conv.source_entry_id.clone())
    });

    let conv_id = match conversation_id {
        Some(id) if !id.is_empty() => id,
        _ => db::ai::create_conversation(&conn, &source, source_entry_id.as_deref())?,
    };

    // 2. Persist User Message
    let user_msg = db::ai::AiMessage {
        id: Uuid::new_v4().to_string(),
        conversation_id: conv_id.clone(),
        role: "user".into(),
        content: message.clone(),
        tool_name: None,
        tool_args: None,
        tool_result: None,
        confirmed: None,
        model: None,
        created_at: "".into(),
    };
    db::ai::add_message(&conn, &user_msg)?;

    // 3. Gather Context & Build System Prompt (D-56, D-61, D-94)
    let now = chrono::Local::now();
    let today_str = now.format("%Y-%m-%d").to_string();
    let fourteen_days_later = (now + chrono::Duration::days(14))
        .format("%Y-%m-%d")
        .to_string();

    let overdue_tasks = db::tasks::list_tasks(
        &conn,
        db::tasks::TaskListFilters {
            due_before: Some(today_str.clone()),
            exclude_statuses: Some(vec!["done".to_string(), "cancelled".to_string()]),
            ..Default::default()
        },
    )?;

    let today_tasks = db::tasks::list_tasks(
        &conn,
        db::tasks::TaskListFilters {
            due_after: Some(today_str.clone()),
            due_before: Some(today_str.clone()),
            exclude_statuses: Some(vec!["done".to_string(), "cancelled".to_string()]),
            ..Default::default()
        },
    )?;

    let upcoming_tasks = db::tasks::list_tasks(
        &conn,
        db::tasks::TaskListFilters {
            due_after: Some(today_str.clone()),
            due_before: Some(fourteen_days_later),
            exclude_statuses: Some(vec!["done".to_string(), "cancelled".to_string()]),
            ..Default::default()
        },
    )?;
    let memory_context = ai::memory::build_prompt_memory(&conn, source_entry_id.as_deref())?;

    let prompt_input = ai::prompt::PromptInput {
        mode: ai::prompt::ChatMode::Chat, // Default for AI Tab
        overdue_tasks,
        today_tasks,
        upcoming_tasks,
        semantic_memory: memory_context.semantic_memory,
        recent_patterns: memory_context.recent_patterns,
        related_journal: memory_context.related_journal,
        current_entry: memory_context.current_entry,
        pinned_points: memory_context.pinned_points,
    };

    let system_prompt = ai::prompt::build_system_prompt(&prompt_input);

    // 4. Fetch History (including this user message)
    let history = db::ai::get_messages(&conn, &conv_id)?;
    #[cfg(debug_assertions)]
    println!(
        "[AI] Loaded {} messages from history for conversation {}",
        history.len(),
        conv_id
    );
    let mut chat_history: Vec<ai::client::ChatMessage> = history
        .into_iter()
        .map(|m| {
            ai::client::ChatMessage {
                role: m.role,
                content: m.content,
                // D-94: never replay historical tool calls back into the next model turn
                tool_calls: None,
            }
        })
        .collect();
    #[cfg(debug_assertions)]
    println!(
        "[AI] Sending {} messages to Ollama (including system)",
        chat_history.len() + 1
    );

    // Prepend System Prompt
    chat_history.insert(
        0,
        ai::client::ChatMessage {
            role: "system".into(),
            content: system_prompt,
            tool_calls: None,
        },
    );

    // 5. Call Ollama Chat Stream
    let tools_json = Some(ai::tools::get_tools_for_ollama());
    crate::events::emit(
        &state.handle,
        crate::events::AppEvent::AiStatus("Thinking...".into()),
    );

    let handle = state.handle.clone();
    let ollama = state.ollama.clone();
    let tool_state = state.ai_tool_state.clone();
    let conn_arc = state.conn.clone();
    let conv_id_clone = conv_id.clone();

    tauri::async_runtime::spawn(async move {
        use futures_util::StreamExt;
        let mut current_history = chat_history;
        let mut tools_for_this_turn = tools_json;
        let mut turns_remaining = 2; // Turn 1: tools/chat, Turn 2: narration if tools used

        while turns_remaining > 0 {
            turns_remaining -= 1;
            let mut full_assistant_content = String::new();
            let mut tool_invoked = false;

            match ollama
                .chat_stream(current_history.clone(), tools_for_this_turn.clone())
                .await
            {
                Ok(mut stream) => {
                    while let Some(part_res) = stream.next().await {
                        match part_res {
                            Ok(part) => {
                                if let Some(msg) = part.message {
                                    if let Some(tool_calls) = msg.tool_calls {
                                        tool_invoked = true;

                                        // 1. Persist assistant message with tool calls
                                        let assistant_msg = db::ai::AiMessage {
                                            id: Uuid::new_v4().to_string(),
                                            conversation_id: conv_id_clone.clone(),
                                            role: "assistant".into(),
                                            content: full_assistant_content.clone(),
                                            tool_name: None,
                                            tool_args: Some(
                                                serde_json::to_string(&tool_calls)
                                                    .unwrap_or_default(),
                                            ),
                                            tool_result: None,
                                            confirmed: None,
                                            model: Some("llama3.2".into()),
                                            created_at: "".into(),
                                        };
                                        {
                                            let conn = conn_arc.lock().await;
                                            let _ = db::ai::add_message(&conn, &assistant_msg);
                                        }

                                        // Push to current history for the narration turn
                                        current_history.push(ai::client::ChatMessage {
                                            role: "assistant".into(),
                                            content: full_assistant_content.clone(),
                                            tool_calls: Some(tool_calls.clone()),
                                        });

                                        // 2. Execute tools & Persist results
                                        for call in &tool_calls {
                                            let name = &call.function.name;
                                            let args = call.function.arguments.clone();

                                            match ai::tools::execute_tool_call(
                                                &handle,
                                                Some(conv_id_clone.clone()),
                                                &tool_state,
                                                name,
                                                args.clone(),
                                            )
                                            .await
                                            {
                                                Ok((call_id, result)) => {
                                                    let confirmed_flag =
                                                        ai_tool_confirmed_flag(name, &result);
                                                    crate::events::emit(
                                                        &handle,
                                                        crate::events::AppEvent::AiToolResult {
                                                            call_id,
                                                            name: name.to_string(),
                                                            result: result.clone(),
                                                            confirmed: confirmed_flag.unwrap_or(1)
                                                                == 1,
                                                        },
                                                    );

                                                    let tool_msg = db::ai::AiMessage {
                                                        id: Uuid::new_v4().to_string(),
                                                        conversation_id: conv_id_clone.clone(),
                                                        role: "tool".into(),
                                                        content: serde_json::to_string(&result)
                                                            .unwrap_or_default(),
                                                        tool_name: Some(name.to_string()),
                                                        tool_args: None,
                                                        tool_result: Some(
                                                            serde_json::to_string(&result)
                                                                .unwrap_or_default(),
                                                        ),
                                                        confirmed: confirmed_flag,
                                                        model: None,
                                                        created_at: "".into(),
                                                    };
                                                    {
                                                        let conn = conn_arc.lock().await;
                                                        let _ =
                                                            db::ai::add_message(&conn, &tool_msg);
                                                    }

                                                    // Push to context for narration
                                                    current_history.push(ai::client::ChatMessage {
                                                        role: "tool".into(),
                                                        content: serde_json::to_string(&result)
                                                            .unwrap_or_default(),
                                                        tool_calls: None,
                                                    });
                                                }
                                                Err(e) => {
                                                    eprintln!("[AI] Tool failed: {:?}", e);
                                                    let res =
                                                        serde_json::json!({"error": e.to_string()});
                                                    current_history.push(ai::client::ChatMessage {
                                                        role: "tool".into(),
                                                        content: serde_json::to_string(&res)
                                                            .unwrap_or_default(),
                                                        tool_calls: None,
                                                    });
                                                }
                                            }
                                        }
                                    } else {
                                        full_assistant_content.push_str(&msg.content);
                                        crate::events::emit(
                                            &handle,
                                            crate::events::AppEvent::AiToken {
                                                token: msg.content.clone(),
                                                done: part.done && turns_remaining == 0,
                                                source: crate::events::AiTokenSource::Chat,
                                            },
                                        );
                                    }
                                }
                                if part.done && !tool_invoked {
                                    // Fallback Check
                                    let fallback_calls =
                                        ai::fallback::extract_tool_calls(&full_assistant_content);
                                    if !fallback_calls.is_empty() {
                                        tool_invoked = true;
                                        for call in fallback_calls {
                                            // Handle fallback exactly like structured tools
                                            let name = call.name;
                                            let args = call.arguments;
                                            // [Persist Assistant / Execute / Persist Tool Result / Push to History omitted for brevity, but logically same as above]
                                            // actually I'll implement it for completeness
                                            let assistant_msg = db::ai::AiMessage {
                                                id: Uuid::new_v4().to_string(),
                                                conversation_id: conv_id_clone.clone(),
                                                role: "assistant".into(),
                                                content: full_assistant_content.clone(),
                                                tool_name: None,
                                                tool_args: Some(serde_json::to_string(&serde_json::json!([{"function": {"name": name, "arguments": args}}])).unwrap_or_default()), 
                                                tool_result: None, confirmed: None, model: Some("llama3.2".into()), created_at: "".into(),
                                            };
                                            {
                                                let conn = conn_arc.lock().await;
                                                let _ = db::ai::add_message(&conn, &assistant_msg);
                                            }
                                            current_history.push(ai::client::ChatMessage {
                                                role: "assistant".into(),
                                                content: full_assistant_content.clone(),
                                                tool_calls: None,
                                            });

                                            match ai::tools::execute_tool_call(
                                                &handle,
                                                Some(conv_id_clone.clone()),
                                                &tool_state,
                                                &name,
                                                args.clone(),
                                            )
                                            .await
                                            {
                                                Ok((call_id, result)) => {
                                                    let confirmed_flag =
                                                        ai_tool_confirmed_flag(&name, &result);
                                                    crate::events::emit(
                                                        &handle,
                                                        crate::events::AppEvent::AiToolResult {
                                                            call_id,
                                                            name: name.clone(),
                                                            result: result.clone(),
                                                            confirmed: confirmed_flag.unwrap_or(1)
                                                                == 1,
                                                        },
                                                    );
                                                    let t_msg = db::ai::AiMessage {
                                                        id: Uuid::new_v4().to_string(),
                                                        conversation_id: conv_id_clone.clone(),
                                                        role: "tool".into(),
                                                        content: serde_json::to_string(&result)
                                                            .unwrap_or_default(),
                                                        tool_name: Some(name.clone()),
                                                        tool_args: None,
                                                        tool_result: Some(
                                                            serde_json::to_string(&result)
                                                                .unwrap_or_default(),
                                                        ),
                                                        confirmed: confirmed_flag,
                                                        model: None,
                                                        created_at: "".into(),
                                                    };
                                                    {
                                                        let conn = conn_arc.lock().await;
                                                        let _ = db::ai::add_message(&conn, &t_msg);
                                                    }
                                                    current_history.push(ai::client::ChatMessage {
                                                        role: "tool".into(),
                                                        content: serde_json::to_string(&result)
                                                            .unwrap_or_default(),
                                                        tool_calls: None,
                                                    });
                                                }
                                                Err(_) => {}
                                            }
                                        }
                                    } else {
                                        // Finalize text-only message
                                        let assistant_msg = db::ai::AiMessage {
                                            id: Uuid::new_v4().to_string(),
                                            conversation_id: conv_id_clone.clone(),
                                            role: "assistant".into(),
                                            content: full_assistant_content.clone(),
                                            tool_name: None,
                                            tool_args: None,
                                            tool_result: None,
                                            confirmed: None,
                                            model: Some("llama3.2".into()),
                                            created_at: "".into(),
                                        };
                                        let conn = conn_arc.lock().await;
                                        let _ = db::ai::add_message(&conn, &assistant_msg);
                                        // Push to context to be safe for Turn 2 if Turn 1 was pure chat
                                        current_history.push(ai::client::ChatMessage {
                                            role: "assistant".into(),
                                            content: full_assistant_content.clone(),
                                            tool_calls: None,
                                        });
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("[AI] Stream error: {:?}", e);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[AI] ollama.chat_stream error: {:?}", e);
                    break;
                }
            }

            if tool_invoked && turns_remaining > 0 {
                // We used a tool! Turn 2 will be narration.
                crate::events::emit(
                    &handle,
                    crate::events::AppEvent::AiStatus("Interpreting results...".into()),
                );
                tools_for_this_turn = None; // Disable tools for narration turn

                // Option B: Map "Tool" to "User" to prevent 400 Bad Request when tools is None
                for msg in current_history.iter_mut() {
                    if msg.role == "tool" {
                        msg.role = "user".to_string();
                        msg.content = format!("[DATABASE_RESULT]: {}", msg.content);
                    } else if msg.role == "assistant" {
                        msg.tool_calls = None;
                    }
                }

                // Add a hidden "Narrator" instruction to nudge the model towards a natural summary
                current_history.push(ai::client::ChatMessage {
                    role: "user".into(),
                    content: "Based on the data above, please provide a natural, helpful summary to the user. Do not show raw JSON. If a task was created, confirm it.".into(),
                    tool_calls: None,
                });
            } else {
                break; // No tools used or already finished Turn 2
            }
        }
    });

    Ok(conv_id)
}

#[tauri::command]
async fn ai_maybe_emit_proactive_nudge(
    state: tauri::State<'_, AppState>,
    trigger: String,
) -> Result<bool, AppError> {
    ai_maybe_emit_proactive_nudge_internal(state.inner().clone(), trigger).await
}

async fn ai_maybe_emit_proactive_nudge_internal(
    state: AppState,
    trigger: String,
) -> Result<bool, AppError> {
    let trigger = ai::companion::ProactiveTrigger::from_str(&trigger)?;
    let client = state.ollama.clone();
    let handle = state.handle.clone();

    let (memory, recent_nudges, decision) = {
        let conn = state.conn.lock().await;
        let decision = ai::companion::decide_proactive_nudge(&conn, trigger)?;
        let Some(decision) = decision else {
            return Ok(false);
        };
        let memory = ai::memory::build_prompt_memory(&conn, None)?;
        let recent_nudges = ai::companion::load_recent_nudges(&conn)?;
        (memory, recent_nudges, decision)
    };

    let generated =
        ai::companion::generate_proactive_nudge(&client, memory, &recent_nudges, &decision).await?;
    let Some(content) = generated else {
        return Ok(false);
    };

    {
        let conn = state.conn.lock().await;
        ai::companion::save_nudge(&conn, &content)?;
    }

    crate::events::emit(
        &handle,
        crate::events::AppEvent::AiProactiveNudge {
            content,
            trigger: decision.trigger.as_str().to_string(),
        },
    );

    Ok(true)
}

#[tauri::command]
async fn ai_list_pinned_memory(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<db::ai::AiPinnedMemory>, AppError> {
    let conn = state.conn.lock().await;
    db::ai::list_pinned_memory(&conn)
}

#[tauri::command]
async fn ai_upsert_pinned_memory(
    state: tauri::State<'_, AppState>,
    id: Option<String>,
    content: String,
    importance: i32,
) -> Result<(), AppError> {
    let conn = state.conn.lock().await;
    let now = chrono::Utc::now().to_rfc3339();
    let memory = db::ai::AiPinnedMemory {
        id: id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        content,
        importance,
        metadata: None,
        created_at: now.clone(),
        updated_at: now,
    };
    db::ai::upsert_pinned_memory(&conn, &memory)
}

#[tauri::command]
async fn ai_delete_pinned_memory(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<bool, AppError> {
    let conn = state.conn.lock().await;
    db::ai::delete_pinned_memory(&conn, &id)
}

#[tauri::command]
async fn ai_generate_reflection_prompt(
    state: tauri::State<'_, AppState>,
    title: Option<String>,
    body_so_far: Option<String>,
    try_another: Option<bool>,
) -> Result<Option<ai::companion::ReflectionPromptResponse>, AppError> {
    let client = state.ollama.clone();
    let handle = state.handle.clone();
    let try_another = try_another.unwrap_or(false);

    let (memory, recent_prompts, draft_context) = {
        let conn = state.conn.lock().await;
        let memory = ai::memory::build_prompt_memory(&conn, None)?;
        let recent_prompts = ai::companion::load_recent_reflection_prompts(&conn)?;
        let draft_context =
            ai::companion::format_draft_context(title.as_deref(), body_so_far.as_deref());
        (memory, recent_prompts, draft_context)
    };

    let generated = ai::companion::generate_reflection_prompt(
        &client,
        memory,
        draft_context,
        &recent_prompts,
        try_another,
    )
    .await?;

    let Some(response) = generated else {
        return Ok(None);
    };

    {
        let conn = state.conn.lock().await;
        ai::companion::save_reflection_prompt(&conn, &response.content)?;
    }

    crate::events::emit(
        &handle,
        crate::events::AppEvent::AiReflectionPrompt {
            content: response.content.clone(),
            suggested_tags: response.suggested_tags.clone(),
        },
    );

    Ok(Some(response))
}

#[tauri::command]
async fn ai_conversation_list(
    state: tauri::State<'_, AppState>,
    source: Option<String>,
) -> Result<Vec<db::ai::AiConversation>, AppError> {
    let conn = state.conn.lock().await;
    db::ai::list_conversations(&conn, source.as_deref())
}

#[tauri::command]
async fn ai_message_list(
    state: tauri::State<'_, AppState>,
    conversation_id: String,
) -> Result<Vec<db::ai::AiMessage>, AppError> {
    let conn = state.conn.lock().await;
    db::ai::get_messages(&conn, &conversation_id)
}

#[tauri::command]
async fn ai_conversation_delete(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<bool, AppError> {
    let conn = state.conn.lock().await;
    db::ai::soft_delete_conversation(&conn, &id)
}

#[tauri::command]
async fn ai_confirm_tool(
    state: tauri::State<'_, AppState>,
    call_id: String,
    confirmed: bool,
) -> Result<(), AppError> {
    let mut pending = state.ai_tool_state.pending_confirmations.lock().await;
    if let Some(conf) = pending.remove(&call_id) {
        let _ = conf.tx.send(confirmed);
    }
    Ok(())
}
#[tauri::command]
async fn project_create(
    state: tauri::State<'_, AppState>,
    project: db::projects::Project,
) -> Result<String, AppError> {
    let conn = state.conn.lock().await;
    db::projects::create_project(&conn, &project)
}

#[tauri::command]
async fn project_list(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<db::projects::Project>, AppError> {
    let conn = state.conn.lock().await;
    db::projects::list_projects(&conn)
}

#[tauri::command]
async fn project_get(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<Option<db::projects::Project>, AppError> {
    let conn = state.conn.lock().await;
    db::projects::get_project(&conn, &id)
}

#[tauri::command]
async fn project_update(
    state: tauri::State<'_, AppState>,
    project: db::projects::Project,
) -> Result<(), AppError> {
    let conn = state.conn.lock().await;
    db::projects::update_project(&conn, &project)
}

#[tauri::command]
async fn project_delete(state: tauri::State<'_, AppState>, id: String) -> Result<bool, AppError> {
    let conn = state.conn.lock().await;
    db::projects::soft_delete_project(&conn, &id, &Utc::now().to_rfc3339())
}

#[tauri::command]
async fn task_delete(state: tauri::State<'_, AppState>, id: String) -> Result<bool, AppError> {
    let conn = state.conn.lock().await;
    match db::tasks::soft_delete(&conn, &id) {
        Ok(res) => {
            crate::events::emit(&state.handle, crate::events::AppEvent::TaskDeleted { id });
            Ok(res)
        }
        Err(e) => {
            crate::events::emit(
                &state.handle,
                crate::events::AppEvent::DatabaseError {
                    operation: "task_delete".into(),
                    error: e.to_string(),
                },
            );
            Err(e)
        }
    }
}

#[tauri::command]
async fn trash_list(state: tauri::State<'_, AppState>) -> Result<Vec<db::trash::TrashItem>, AppError> {
    let conn = state.conn.lock().await;
    db::trash::list_trash(&conn)
}

#[tauri::command]
async fn trash_empty(state: tauri::State<'_, AppState>) -> Result<u32, AppError> {
    let conn = state.conn.lock().await;
    db::trash::empty_trash(&conn)
}

#[tauri::command]
async fn db_manual_backup(state: tauri::State<'_, AppState>) -> Result<String, AppError> {
    let conn = state.conn.lock().await;
    crate::backup::manual::perform_backup(&conn)
}

#[tauri::command]
async fn db_export_json(state: tauri::State<'_, AppState>) -> Result<String, AppError> {
    let conn = state.conn.lock().await;
    crate::backup::export::export_to_json(&conn)
}

#[tauri::command]
async fn db_reset(state: tauri::State<'_, AppState>) -> Result<(), AppError> {
    let mut conn = state.conn.lock().await;
    db::migrations::reset_database(&mut conn)
}

#[tauri::command]
async fn db_get_audit_log(state: tauri::State<'_, AppState>, limit: u32) -> Result<Vec<db::audit::AuditEntry>, AppError> {
    let conn = state.conn.lock().await;
    db::audit::get_recent_logs(&conn, limit)
}

#[tauri::command]
async fn ai_get_queue_status(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, AppError> {
    let queue = state.ai_state.queue.lock().await;
    let status = state.ai_state.status.lock().await;
    
    Ok(serde_json::json!({
        "queue_length": queue.len(),
        "processing_count": status.values().filter(|s| match s {
            crate::ai::AnalysisStatus::Processing => true,
            _ => false
        }).count(),
    }))
}

#[tauri::command]
async fn db_get_path() -> Result<String, AppError> {
    let data_dir = crate::db::paths::resolve_data_dir();
    let db_path = data_dir.join("hawkward.db");
    Ok(db_path.to_string_lossy().into_owned())
}
