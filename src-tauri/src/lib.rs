mod ai;
mod backup;
mod db;
mod error;
mod events;
mod logger;
mod scheduler;

use tauri::{Manager, Listener};
use chrono::Utc;
use db::paths::resolve_data_dir;
use error::AppError;
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

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

    let title = title
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty());
    let word_count = content.split_whitespace().count() as i64;

    let entry = db::journal::JournalEntry {
        id: entry_id.clone(),
        title,
        content,
        emotions: "".into(),
        tags: "".into(),
        last_analysis_conv_id: None,
        last_analysed_at: None,
        word_count,
        created_at: now.clone(),
        updated_at: now,
    };

    let conn = state.conn.lock().await;
    db::journal::upsert_entry(&conn, &entry)?;

    crate::events::emit(&state.handle, crate::events::AppEvent::JournalSaved { 
        entry_id: entry_id.clone() 
    });

    Ok(entry_id)
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
async fn journal_delete(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<bool, AppError> {
    let conn = state.conn.lock().await;
    db::journal::soft_delete(&conn, &id)
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
                return Err(AppError::InvalidInput("Maximum subtask depth (2 levels) reached.".into()));
            }
        }
    }

    let task = db::tasks::Task {
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

    db::tasks::create_task(&conn, &task)?;
    
    // §8A: Return full Task object
    crate::events::emit(&state.handle, crate::events::AppEvent::TaskCreated { 
        id: task.id.clone(), 
        title: task.title.clone() 
    });

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
            } else {
                crate::events::emit(&state.handle, crate::events::AppEvent::TaskUpdated { id });
            }
            Ok(())
        },
        Err(e) => {
            crate::events::emit(&state.handle, crate::events::AppEvent::DatabaseError { 
                operation: "task_update_status".into(), 
                error: e.to_string() 
            });
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
    db::tasks::update_task(&conn, &task)
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
async fn task_add_dependency(state: tauri::State<'_, AppState>, blocked_task_id: String, blocking_task_id: String) -> Result<(), AppError> {
    let conn = state.conn.lock().await;
    db::tasks::task_add_dependency(&conn, &blocked_task_id, &blocking_task_id)
}

#[tauri::command]
async fn task_remove_dependency(state: tauri::State<'_, AppState>, blocked_task_id: String, blocking_task_id: String) -> Result<(), AppError> {
    let conn = state.conn.lock().await;
    db::tasks::task_remove_dependency(&conn, &blocked_task_id, &blocking_task_id)
}

#[tauri::command]
async fn timer_start(state: tauri::State<'_, AppState>, task_id: String) -> Result<String, AppError> {
    let conn = state.conn.lock().await;
    match db::tasks::timer_start(&conn, &task_id) {
        Ok(log_id) => {
            crate::events::emit(&state.handle, crate::events::AppEvent::TaskUpdated { id: task_id });
            Ok(log_id)
        },
        Err(e) => Err(e),
    }
}

#[tauri::command]
async fn timer_stop(state: tauri::State<'_, AppState>, task_id: String) -> Result<(), AppError> {
    let conn = state.conn.lock().await;
    match db::tasks::timer_stop(&conn, &task_id) {
        Ok(_) => {
            crate::events::emit(&state.handle, crate::events::AppEvent::TaskUpdated { id: task_id });
            Ok(())
        },
        Err(e) => Err(e),
    }
}

#[tauri::command]
async fn attachment_add(state: tauri::State<'_, AppState>, task_id: String, source_path: String) -> Result<db::tasks::TaskAttachment, AppError> {
    let conn = state.conn.lock().await;
    match db::tasks::attachment_add(&conn, &task_id, &source_path) {
        Ok(attachment) => {
            crate::events::emit(&state.handle, crate::events::AppEvent::TaskUpdated { id: task_id.clone() });
            Ok(attachment)
        },
        Err(e) => Err(e),
    }
}

#[tauri::command]
async fn attachment_list(state: tauri::State<'_, AppState>, task_id: String) -> Result<Vec<db::tasks::TaskAttachment>, AppError> {
    let conn = state.conn.lock().await;
    db::tasks::attachment_list(&conn, &task_id)
}

#[tauri::command]
async fn attachment_remove(state: tauri::State<'_, AppState>, id: String, task_id: String) -> Result<(), AppError> {
    let conn = state.conn.lock().await;
    match db::tasks::attachment_remove(&conn, &id) {
        Ok(_) => {
            crate::events::emit(&state.handle, crate::events::AppEvent::TaskUpdated { id: task_id });
            Ok(())
        },
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
            
            let app_state = AppState {
                conn: conn.clone(),
                data_dir: data_dir.clone(),
                ai_state: ai_state.clone(),
                ai_tool_state: ai_tool_state.clone(),
                ollama: ollama_client.clone(),
                handle: handle.clone(),
            };
            app.manage(app_state);

            // WORKER LOOP (Step 4 & 5)
            let worker_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                let state = worker_handle.state::<AppState>();
                loop {
                    // Wait for notification OR continue if items exist
                    let items_in_queue = {
                        let queue = state.ai_state.queue.lock().await;
                        !queue.is_empty()
                    };

                    if !items_in_queue {
                        state.ai_state.notify.notified().await;
                    }
                    
                    // 2. Pop from queue with explicit type
                    let entry_id: Option<String> = {
                        let mut queue = state.ai_state.queue.lock().await;
                        let mut queued_ids = state.ai_state.queued_ids.lock().await;
                        
                        if let Some(id) = queue.pop_front() {
                            queued_ids.remove(&id);
                            Some(id)
                        } else {
                            None
                        }
                    };

                    if let Some(id) = entry_id {
                        // 3. Fetch latest from DB (Source of Truth)
                        let db_result = {
                            let conn = state.conn.lock().await;
                            db::journal::get_entry_by_id(&conn, &id)
                        };

                        match db_result {
                            Ok(Some(entry)) => {
                                // 4. Deduplication
                                if state.ai_state.should_analyze(&id, &entry.content).await {
                                    crate::events::emit(&worker_handle, crate::events::AppEvent::JournalAnalysisProcessing { entry_id: id.clone() });

                                    // 5. Call Ollama (Dropped lock above)
                                    match state.ollama.analyze_journal(&entry.content, id.clone()).await {
                                        Ok(result) => {
                                            // 6. Ghost Entry Check: Verify entry still exists before emitting
                                            let exists = {
                                                let conn = state.conn.lock().await;
                                                db::journal::get_entry_by_id(&conn, &id).ok().flatten().is_some()
                                            };

                                            if exists {
                                                crate::events::emit(&worker_handle, crate::events::AppEvent::JournalAnalysisCompleted {
                                                    entry_id: id.clone(),
                                                    result: result,
                                                });
                                            } else {
                                                println!("[AI] Ghost Entry detected: {} was deleted during analysis.", id);
                                                state.ai_state.status.lock().await.remove(&id);
                                            }
                                        }
                                        Err(e) => {
                                            // Handle "Model Not Found" or other fatal errors
                                            let error_msg = e.to_string();
                                            if error_msg.contains("not found") {
                                                crate::events::emit(&worker_handle, crate::events::AppEvent::AiModelMissing { model: "llama3.2".into() });
                                            }
                                            
                                            eprintln!("[AI] Analysis error for {}: {:?}", id, e);
                                            state.ai_state.status.lock().await.insert(id.clone(), crate::ai::AnalysisStatus::Failed);
                                            crate::events::emit(&worker_handle, crate::events::AppEvent::JournalAnalysisError { 
                                                entry_id: id.clone(), 
                                                error: error_msg 
                                            });
                                        }
                                    }
                                }
                            }
                            Ok(None) => println!("[AI] Warning: Entry {} disappeared before analysis", id),
                            Err(e) => eprintln!("[AI] Database error during fetch: {:?}", e),
                        }
                    }
                }
            });

            // EVENT LISTENER (D-96: Unified app_event channel)
            let listener_handle = handle.clone();
            app.listen("app_event", move |event: tauri::Event| {
                if let Ok(payload) = serde_json::from_str::<serde_json::Value>(event.payload()) {
                    // Trigger AI Analysis on JournalSaved
                    if payload["type"] == "journal_saved" {
                        if let Some(id) = payload["entry_id"].as_str() {
                            let task_handle = listener_handle.clone();
                            let id_clone = id.to_string();
                            tauri::async_runtime::spawn(async move {
                                let state = task_handle.state::<AppState>();
                                let mut queue = state.ai_state.queue.lock().await;
                                let mut queued_ids = state.ai_state.queued_ids.lock().await;

                                if queue.len() >= crate::ai::analysis::MAX_QUEUE { return; }

                                queue.retain(|existing_id| existing_id != &id_clone);
                                queue.push_back(id_clone.clone());
                                queued_ids.insert(id_clone);
                                state.ai_state.notify.notify_one();
                            });
                        }
                    }
                }
            });

            // Step 8 (D-109): Check for missed Monday weekly review
            let review_handle = handle.clone();
            let review_conn = conn.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                let conn = review_conn.lock().await;
                let _ = crate::scheduler::weekly_report::maybe_run_weekly_review(&review_handle, &conn);
            });

            // Step 9: Main application background scheduler worker
            let sched_handle = handle.clone();
            let sched_conn = conn.clone();
            tauri::async_runtime::spawn(async move {
                use chrono::{Datelike, Timelike};
                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
                loop {
                    interval.tick().await;
                    let c = sched_conn.lock().await;

                    // 1. Weekly Review (Scheduled time check: Mon >= 08:00)
                    let now = chrono::Local::now();
                    if now.weekday() == chrono::Weekday::Mon && now.hour() >= 8 {
                        let _ = crate::scheduler::weekly_report::maybe_run_weekly_review(&sched_handle, &c);
                    }

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
            journal_get,
            journal_list,
            journal_delete,
            project_create,
            project_list,
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
            ai_conversation_list,
            ai_message_list,
            ai_conversation_delete,
            ai_confirm_tool,
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
    let conv_id = match conversation_id {
        Some(id) if !id.is_empty() => id,
        _ => db::ai::create_conversation(&conn, &source, entry_id.as_deref())?,
    };

    // 2. Persist User Message
    let user_msg = db::ai::AiMessage {
        id: Uuid::new_v4().to_string(),
        conversation_id: conv_id.clone(),
        role: "user".into(),
        content: message.clone(),
        tool_name: None, tool_args: None, tool_result: None,
        confirmed: None, model: None, created_at: "".into(),
    };
    db::ai::add_message(&conn, &user_msg)?;

    // 3. Gather Context & Build System Prompt (D-56, D-61, D-94)
    let now = chrono::Local::now();
    let today_str = now.format("%Y-%m-%d").to_string();
    let fourteen_days_later = (now + chrono::Duration::days(14)).format("%Y-%m-%d").to_string();

    let overdue_tasks = db::tasks::list_tasks(&conn, db::tasks::TaskListFilters {
        due_before: Some(today_str.clone()),
        exclude_statuses: Some(vec!["done".to_string(), "cancelled".to_string()]),
        ..Default::default()
    })?;

    let today_tasks = db::tasks::list_tasks(&conn, db::tasks::TaskListFilters {
        due_after: Some(today_str.clone()),
        due_before: Some(today_str.clone()),
        exclude_statuses: Some(vec!["done".to_string(), "cancelled".to_string()]),
        ..Default::default()
    })?;

    let upcoming_tasks = db::tasks::list_tasks(&conn, db::tasks::TaskListFilters {
        due_after: Some(today_str.clone()),
        due_before: Some(fourteen_days_later),
        exclude_statuses: Some(vec!["done".to_string(), "cancelled".to_string()]),
        ..Default::default()
    })?;

    let prompt_input = ai::prompt::PromptInput {
        mode: ai::prompt::ChatMode::Chat, // Default for AI Tab
        overdue_tasks,
        today_tasks,
        upcoming_tasks,
        related_journal: vec![], // TODO: Injected via FTS5 later
        current_entry: None,
    };

    let system_prompt = ai::prompt::build_system_prompt(&prompt_input);

    // 4. Fetch History (including this user message)
    let history = db::ai::get_messages(&conn, &conv_id)?;
    let mut chat_history: Vec<ai::client::ChatMessage> = history.into_iter().map(|m| {
        ai::client::ChatMessage {
            role: m.role,
            content: m.content,
            tool_calls: m.tool_args.and_then(|a| serde_json::from_str(&a).ok()),
        }
    }).collect();

    // Prepend System Prompt
    chat_history.insert(0, ai::client::ChatMessage {
        role: "system".into(),
        content: system_prompt,
        tool_calls: None,
    });

    // 5. Call Ollama Chat Stream
    let tools_json = Some(ai::tools::get_tools_for_ollama());
    crate::events::emit(&state.handle, crate::events::AppEvent::AiStatus("Thinking...".into()));
    
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
            
            match ollama.chat_stream(current_history.clone(), tools_for_this_turn.clone()).await {
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
                                            tool_args: Some(serde_json::to_string(&tool_calls).unwrap_or_default()), 
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
                                            
                                            match ai::tools::execute_tool_call(&handle, &tool_state, name, args.clone()).await {
                                                Ok((call_id, result)) => {
                                                    crate::events::emit(&handle, crate::events::AppEvent::AiToolResult {
                                                        call_id,
                                                        name: name.to_string(),
                                                        result: result.clone(),
                                                        confirmed: true,
                                                    });
                                                    
                                                    let tool_msg = db::ai::AiMessage {
                                                        id: Uuid::new_v4().to_string(),
                                                        conversation_id: conv_id_clone.clone(),
                                                        role: "tool".into(),
                                                        content: serde_json::to_string(&result).unwrap_or_default(),
                                                        tool_name: Some(name.to_string()),
                                                        tool_args: None,
                                                        tool_result: Some(serde_json::to_string(&result).unwrap_or_default()),
                                                        confirmed: Some(1), model: None, created_at: "".into(),
                                                    };
                                                    {
                                                        let conn = conn_arc.lock().await;
                                                        let _ = db::ai::add_message(&conn, &tool_msg);
                                                    }

                                                    // Push to context for narration
                                                    current_history.push(ai::client::ChatMessage {
                                                        role: "tool".into(),
                                                        content: serde_json::to_string(&result).unwrap_or_default(),
                                                        tool_calls: None,
                                                    });
                                                }
                                                Err(e) => {
                                                    eprintln!("[AI] Tool failed: {:?}", e);
                                                    let res = serde_json::json!({"error": e.to_string()});
                                                    current_history.push(ai::client::ChatMessage {
                                                        role: "tool".into(),
                                                        content: serde_json::to_string(&res).unwrap_or_default(),
                                                        tool_calls: None,
                                                    });
                                                }
                                            }
                                        }
                                    } else {
                                        full_assistant_content.push_str(&msg.content);
                                        crate::events::emit(&handle, crate::events::AppEvent::AiToken {
                                            token: msg.content.clone(),
                                            done: part.done && turns_remaining == 0,
                                            source: crate::events::AiTokenSource::Chat,
                                        });
                                    }
                                }
                                if part.done && !tool_invoked {
                                    // Fallback Check
                                    let fallback_calls = ai::fallback::extract_tool_calls(&full_assistant_content);
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
                                            { let conn = conn_arc.lock().await; let _ = db::ai::add_message(&conn, &assistant_msg); }
                                            current_history.push(ai::client::ChatMessage { role: "assistant".into(), content: full_assistant_content.clone(), tool_calls: None });

                                            match ai::tools::execute_tool_call(&handle, &tool_state, &name, args.clone()).await {
                                                Ok((call_id, result)) => {
                                                    crate::events::emit(&handle, crate::events::AppEvent::AiToolResult { call_id, name: name.clone(), result: result.clone(), confirmed: true });
                                                    let t_msg = db::ai::AiMessage {
                                                        id: Uuid::new_v4().to_string(), conversation_id: conv_id_clone.clone(), role: "tool".into(), content: serde_json::to_string(&result).unwrap_or_default(),
                                                        tool_name: Some(name.clone()), tool_args: None, tool_result: Some(serde_json::to_string(&result).unwrap_or_default()), confirmed: Some(1), model: None, created_at: "".into(),
                                                    };
                                                    { let conn = conn_arc.lock().await; let _ = db::ai::add_message(&conn, &t_msg); }
                                                    current_history.push(ai::client::ChatMessage { role: "tool".into(), content: serde_json::to_string(&result).unwrap_or_default(), tool_calls: None });
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
                                            tool_name: None, tool_args: None, tool_result: None,
                                            confirmed: None, model: Some("llama3.2".into()), created_at: "".into(),
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
                            Err(e) => { eprintln!("[AI] Stream error: {:?}", e); break; }
                        }
                    }
                }
                Err(e) => { eprintln!("[AI] ollama.chat_stream error: {:?}", e); break; }
            }

            if tool_invoked && turns_remaining > 0 {
                // We used a tool! Turn 2 will be narration.
                crate::events::emit(&handle, crate::events::AppEvent::AiStatus("Interpreting results...".into()));
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
async fn ai_conversation_list(state: tauri::State<'_, AppState>, source: Option<String>) -> Result<Vec<db::ai::AiConversation>, AppError> {
    let conn = state.conn.lock().await;
    db::ai::list_conversations(&conn, source.as_deref())
}

#[tauri::command]
async fn ai_message_list(state: tauri::State<'_, AppState>, conversation_id: String) -> Result<Vec<db::ai::AiMessage>, AppError> {
    let conn = state.conn.lock().await;
    db::ai::get_messages(&conn, &conversation_id)
}

#[tauri::command]
async fn ai_conversation_delete(state: tauri::State<'_, AppState>, id: String) -> Result<bool, AppError> {
    let conn = state.conn.lock().await;
    db::ai::soft_delete_conversation(&conn, &id)
}

#[tauri::command]
async fn ai_confirm_tool(state: tauri::State<'_, AppState>, call_id: String, confirmed: bool) -> Result<(), AppError> {
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
async fn task_delete(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<bool, AppError> {
    let conn = state.conn.lock().await;
    match db::tasks::soft_delete(&conn, &id) {
        Ok(res) => {
            crate::events::emit(&state.handle, crate::events::AppEvent::TaskDeleted { id });
            Ok(res)
        },
        Err(e) => {
            crate::events::emit(&state.handle, crate::events::AppEvent::DatabaseError { 
                operation: "task_delete".into(), 
                error: e.to_string() 
            });
            Err(e)
        }
    }
}
