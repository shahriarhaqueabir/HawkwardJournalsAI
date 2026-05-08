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
use serde_json::json;

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
        recurrence_rule: None,
        is_blocked: false,
        created_at: now.clone(),
        updated_at: now,
        completed_at: None,
    };

    let conn = state.conn.lock().await;

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
async fn timer_start(
    state: tauri::State<'_, AppState>,
    task_id: String,
) -> Result<String, AppError> {
    let conn = state.conn.lock().await;
    db::tasks::timer_start(&conn, &task_id)
}

#[tauri::command]
async fn timer_stop(
    state: tauri::State<'_, AppState>,
    log_id: String,
    duration: i32,
    note: Option<String>,
) -> Result<(), AppError> {
    let conn = state.conn.lock().await;
    db::tasks::timer_stop(&conn, &log_id, duration, note.as_deref())
}

#[tauri::command]
async fn timer_get_logs(
    state: tauri::State<'_, AppState>,
    task_id: String,
) -> Result<Vec<db::tasks::TimeLog>, AppError> {
    let conn = state.conn.lock().await;
    db::tasks::timer_get_logs(&conn, &task_id)
}

#[tauri::command]
async fn attachment_add(
    state: tauri::State<'_, AppState>,
    task_id: String,
    file_name: String,
    file_path: String,
    mime_type: Option<String>,
    size_bytes: Option<i32>,
) -> Result<String, AppError> {
    let conn = state.conn.lock().await;
    db::tasks::attachment_add(&conn, &task_id, &file_name, &file_path, mime_type.as_deref(), size_bytes)
}

#[tauri::command]
async fn attachment_remove(
    state: tauri::State<'_, AppState>,
    attachment_id: String,
) -> Result<(), AppError> {
    let conn = state.conn.lock().await;
    db::tasks::attachment_remove(&conn, &attachment_id)
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
async fn graph_query(
    state: tauri::State<'_, AppState>,
    cypher: String,
) -> Result<serde_json::Value, AppError> {
    let graph = state.graph.lock().await;
    let results = graph.query(&cypher)
        .map_err(|e| AppError::AiError(format!("Graph Query Error: {}", e)))?;
    
    Ok(serde_json::to_value(results).unwrap_or(serde_json::Value::Null))
}

#[tauri::command]
async fn profile_upsert_fact(
    state: tauri::State<'_, AppState>,
    fact: db::profile::ProfileFact,
) -> Result<(), AppError> {
    let conn = state.conn.lock().await;
    db::profile::upsert_fact(&conn, &fact)
}

#[tauri::command]
async fn profile_get_facts(
    state: tauri::State<'_, AppState>,
    category: Option<String>,
) -> Result<Vec<db::profile::ProfileFact>, AppError> {
    let conn = state.conn.lock().await;
    db::profile::get_facts(&conn, category.as_deref())
}

#[tauri::command]
async fn ai_chat(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    conversation_id: Option<String>,
    message: String,
) -> Result<String, AppError> {
    let conv_id = conversation_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    
    // 1. Fetch Memory Bank facts
    let conn = state.conn.lock().await;
    let facts = db::profile::get_facts(&conn, None)?;
    drop(conn); // Drop lock as soon as possible

    // 2. Prepare system prompt
    let system_prompt = ai::prompt::get_chat_system_prompt(&facts);

    // 3. Prepare messages (for now just the current message, history can be added later)
    // In a real app, you'd fetch history from db::ai::get_messages(conv_id)
    let messages = vec![
        json!({"role": "user", "content": message})
    ];

    // 4. Start streaming in background
    let ollama = state.ollama.clone();
    let app_clone = app.clone();
    let conv_id_clone = conv_id.clone();
    
    tauri::async_runtime::spawn(async move {
        if let Err(e) = ollama.chat_stream(app_clone, conv_id_clone, system_prompt, messages).await {
            eprintln!("[AI] Chat error: {:?}", e);
        }
    });

    Ok(conv_id)
}

#[tauri::command]
async fn profile_delete_fact(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let conn = state.conn.lock().await;
    db::profile::delete_fact(&conn, &id)
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

#[tauri::command]
async fn report_bug(
    state: tauri::State<'_, AppState>,
    user_feedback: String,
) -> Result<String, AppError> {
    let log_path = state.data_dir.join("hawkward-debug.log");
    let export_dir = state.data_dir.join("exports");
    
    if !export_dir.exists() {
        std::fs::create_dir_all(&export_dir)
            .map_err(|e| AppError::Io(format!("Failed to create exports dir: {}", e)))?;
    }

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let report_filename = format!("bug_report_{}.md", timestamp);
    let report_path = export_dir.join(&report_filename);

    let logs = std::fs::read_to_string(&log_path)
        .unwrap_or_else(|_| "No debug log found or could not be read.".into());

    let report_content = format!(
        "# HawkwardJournalsAI Bug Report\n\
        Date: {}\n\
        \n\
        ## User Feedback\n\
        {}\n\
        \n\
        ## System Context\n\
        OS: {}\n\
        \n\
        ## Debug Logs (Last 500 lines)\n\
        ```log\n\
        {}\n\
        ```\n",
        chrono::Local::now().to_rfc2822(),
        user_feedback,
        std::env::consts::OS,
        logs.lines().rev().take(500).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n")
    );

    std::fs::write(&report_path, report_content)
        .map_err(|e| AppError::Io(format!("Failed to write bug report: {}", e)))?;

    tracing::info!("Bug report generated at {:?}", report_path);
    Ok(report_path.to_string_lossy().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let data_dir = resolve_data_dir();
    crate::logger::init(&data_dir);
    
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
        .setup(|app| {
            let handle = app.handle().clone();
            
            crate::scheduler::reminders::spawn_reminder_worker(handle.clone());
            
            let worker_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                    let state = worker_handle.state::<AppState>();
                    
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
                        let db_result = {
                            let conn = state.conn.lock().await;
                            db::journal::get_entry_by_id(&conn, &id)
                        };

                        match db_result {
                            Ok(Some(entry)) => {
                                if state.ai_state.should_analyze(&id, &entry.content).await {
                                    {
                                        let mut status_map = state.ai_state.status.lock().await;
                                        status_map.insert(id.clone(), crate::ai::AnalysisStatus::Processing);
                                    }
                                    crate::events::emit(&worker_handle, crate::events::AppEvent::JournalAnalysisProcessing { entry_id: id.clone() });

                                    match state.ollama.analyze_journal(&entry.content, id.clone()).await {
                                        Ok(result) => {
                                            {
                                                let graph = state.graph.lock().await;
                                                for (s, p, o) in &result.triplets {
                                                    let _ = graph.upsert_node(s, Vec::<(String, String)>::new(), "Entity");
                                                    let _ = graph.upsert_node(o, Vec::<(String, String)>::new(), "Entity");
                                                    let _ = graph.upsert_edge(s, o, Vec::<(String, String)>::new(), p);
                                                }
                                            }

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
                                                state.ai_state.status.lock().await.remove(&id);
                                            }
                                        }
                                        Err(e) => {
                                            let error_msg = e.to_string();
                                            if error_msg.contains("not found") {
                                                crate::events::emit(&worker_handle, crate::events::AppEvent::AiModelMissing { model: "llama3.2".into() });
                                            }
                                            
                                            state.ai_state.status.lock().await.insert(id.clone(), crate::ai::AnalysisStatus::Failed);
                                            crate::events::emit(&worker_handle, crate::events::AppEvent::JournalAnalysisError { 
                                                entry_id: id.clone(), 
                                                error: error_msg 
                                            });
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            });

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

                            if queue.len() >= crate::ai::analysis::MAX_QUEUE {
                                return;
                            }

                            queue.retain(|existing_id| existing_id != &id_clone);
                            queue.push_back(id_clone.clone());
                            queued_ids.insert(id_clone.clone());

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
            save_journal_entry,
            journal_get,
            journal_list,
            journal_delete,
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
            timer_start,
            timer_stop,
            timer_get_logs,
            attachment_add,
            attachment_remove,
            attachment_list,
            profile_upsert_fact,
            profile_get_facts,
            profile_delete_fact,
            graph_query,
            ollama_health_check,
            ai_chat,
            report_bug
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
