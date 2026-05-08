mod ai;
mod backup;
mod db;
mod error;
mod events;
mod logger;
mod scheduler;

use tauri::{Manager, Emitter, Listener};
use chrono::Utc;
use db::paths::resolve_data_dir;
use error::AppError;
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct AppState {
    pub conn: Arc<Mutex<Connection>>,
    pub graph: Arc<Mutex<graphqlite::Graph>>,
    pub data_dir: std::path::PathBuf,
    pub ai_state: Arc<ai::analysis::AnalysisState>,
    pub ollama: Arc<ai::client::OllamaClient>,
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
) -> Result<String, AppError> {
    let now = Utc::now().to_rfc3339();
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
        reminder_fired: false,
        time_estimate: None,
        time_logged: 0,
        tags: "[]".into(),
        labels: "[]".into(),
        category: None,
        project: None,
        energy_level: None,
        context_tag: None,
        linked_url: None,
        ai_created: false,
        is_blocked: false,
        created_at: now.clone(),
        updated_at: now,
        completed_at: None,
    };

    let conn = state.conn.lock().await;

    // Enforce 2-level nesting limit (D-40)
    if let Some(ref pid) = parent_task_id {
        if let Some(parent) = db::tasks::get_task(&conn, pid)? {
            if parent.parent_task_id.is_some() {
                return Err(AppError::Database("Subtask depth limit exceeded (max 2 levels)".into()));
            }
        }
    }

    db::tasks::create_task(&conn, &task)
}

#[tauri::command]
async fn task_list(
    state: tauri::State<'_, AppState>,
    include_completed: Option<bool>,
) -> Result<Vec<db::tasks::Task>, AppError> {
    let conn = state.conn.lock().await;
    db::tasks::list_tasks(&conn, include_completed.unwrap_or(false))
}

#[tauri::command]
async fn task_update_status(
    state: tauri::State<'_, AppState>,
    id: String,
    status: String,
) -> Result<(), AppError> {
    let conn = state.conn.lock().await;
    db::tasks::update_task_status(&conn, &id, &status)
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
async fn task_add_dependency(
    state: tauri::State<'_, AppState>,
    blocked_id: String,
    blocking_id: String,
) -> Result<(), AppError> {
    let conn = state.conn.lock().await;
    db::tasks::add_dependency(&conn, &blocked_id, &blocking_id)
}

#[tauri::command]
async fn task_remove_dependency(
    state: tauri::State<'_, AppState>,
    blocked_id: String,
    blocking_id: String,
) -> Result<(), AppError> {
    let conn = state.conn.lock().await;
    db::tasks::remove_dependency(&conn, &blocked_id, &blocking_id)
}

#[tauri::command]
async fn task_get_dependencies(
    state: tauri::State<'_, AppState>,
    task_id: String,
) -> Result<Vec<db::tasks::Task>, AppError> {
    let conn = state.conn.lock().await;
    db::tasks::get_dependencies(&conn, &task_id)
}

#[tauri::command]
async fn graph_query(
    state: tauri::State<'_, AppState>,
    cypher: String,
) -> Result<serde_json::Value, AppError> {
    let graph = state.graph.lock().await;
    let results = graph.query(&cypher)
        .map_err(|e| AppError::AiError(format!("Graph Query Error: {}", e)))?;
    
    // Convert results to JSON
    Ok(serde_json::to_value(results).unwrap_or(serde_json::Value::Null))
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
    let db_path = data_dir.join("hawkward.db");
    let conn = db::init::initialise().expect("Failed to initialise database");
    let graph = graphqlite::Graph::open(&db_path).expect("Failed to initialise graph database");

    let ollama_client = Arc::new(ai::client::OllamaClient::new("llama3.2".into()));
    let ai_state = Arc::new(ai::analysis::AnalysisState::new());

    let app_state = AppState {
        conn: Arc::new(Mutex::new(conn)),
        graph: Arc::new(Mutex::new(graph)),
        data_dir,
        ai_state,
        ollama: ollama_client,
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            task_create,
            task_list,
            task_update_status,
            task_get,
            task_update,
            task_search,
            task_delete,
            task_add_dependency,
            task_remove_dependency,
            task_get_dependencies,
            graph_query,
            ollama_health_check
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            
            // Start Schedulers
            crate::scheduler::reminders::spawn_reminder_worker(handle.clone());
            
            // WORKER LOOP (Step 4 & 5)
            let worker_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                    // 1. Lift state once per loop (Optimization)
                    let state = worker_handle.state::<AppState>();
                    
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
                                    {
                                        let mut status_map = state.ai_state.status.lock().await;
                                        status_map.insert(id.clone(), crate::ai::AnalysisStatus::Processing);
                                    }
                                    crate::events::emit(&worker_handle, crate::events::AppEvent::JournalAnalysisProcessing { entry_id: id.clone() });

                                    // 5. Call Ollama (Dropped lock above)
                                    match state.ollama.analyze_journal(&entry.content, id.clone()).await {
                                        Ok(result) => {
                                            // 5.5 Update Knowledge Graph
                                            {
                                                let graph = state.graph.lock().await;
                                                for (s, p, o) in &result.triplets {
                                                    // Simple upsert of entities and their relationship
                                                    let _ = graph.upsert_node(s, Vec::<(String, String)>::new(), "Entity");
                                                    let _ = graph.upsert_node(o, Vec::<(String, String)>::new(), "Entity");
                                                    let _ = graph.upsert_edge(s, o, Vec::<(String, String)>::new(), p);
                                                }
                                            }

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

            // EVENT LISTENER (Step 2 & 6)
            let listener_handle = handle.clone();
            app.listen_any("journal_analysis_queued", move |event| {
                if let Ok(payload) = serde_json::from_str::<serde_json::Value>(event.payload()) {
                    if let Some(id) = payload["id"].as_str() {
                        let task_handle = listener_handle.clone();
                        let id_clone = id.to_string();
                        tauri::async_runtime::spawn(async move {
                            let state = task_handle.state::<AppState>();
                            let mut queue = state.ai_state.queue.lock().await;
                            let mut queued_ids = state.ai_state.queued_ids.lock().await;

                            // Memory Safety: Check MAX_QUEUE limit
                            if queue.len() >= crate::ai::analysis::MAX_QUEUE {
                                return;
                            }

                            // Step 6: Backpressure - Move to back of queue if exists, or add new
                            queue.retain(|existing_id| existing_id != &id_clone);
                            queue.push_back(id_clone.clone());
                            queued_ids.insert(id_clone.clone());

                            // Emit back to frontend so UI can show "Queued" status
                            crate::events::emit(&task_handle, crate::events::AppEvent::JournalAnalysisQueued { 
                                entry_id: id_clone 
                            });
                        });
                    }
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
            graph_query,
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
