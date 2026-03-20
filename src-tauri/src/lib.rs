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
        status: "todo".into(),
        priority: priority.unwrap_or_else(|| "medium".into()),
        due_date,
        due_time: None,
        reminder_at: None,
        time_estimate: None,
        time_logged: 0,
        tags: "[]".into(),
        labels: "[]".into(),
        category: None,
        project: None,
        project_id: Some(project_id.unwrap_or_else(|| "inbox".into())),
        energy_level: None,
        context_tag: None,
        linked_url: None,
        recurrence: None,
        next_occurrence: None,
        ai_created: false,
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
    exclude_statuses: Option<Vec<String>>,
) -> Result<Vec<db::tasks::Task>, AppError> {
    let conn = state.conn.lock().await;
    let exclude = exclude_statuses.unwrap_or_else(|| vec!["done".into(), "cancelled".into()]);
    db::tasks::list_tasks(&conn, exclude)
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
async fn task_delete(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<bool, AppError> {
    let conn = state.conn.lock().await;
    db::tasks::soft_delete(&conn, &id)
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
            app.listen("app_event", move |event| {
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
                let _ = crate::scheduler::weekly_report::maybe_run_weekly_review(&review_handle, &conn).await;
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
                        let _ = crate::scheduler::weekly_report::maybe_run_weekly_review(&sched_handle, &c).await;
                    }

                    // 2. Poll Sub-minute Tasks (Reminders)
                    let _ = crate::scheduler::reminders::poll_reminders(&sched_handle, &c).await;

                    // 3. Poll Interval Recurrences (D-120)
                    let _ = crate::scheduler::recurrence::poll_recurrences(&sched_handle, &c).await;
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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
