use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::error::AppError;
use tokio::sync::{Mutex, oneshot};
use std::collections::HashMap;
use crate::events::{AppEvent, emit};
use tauri::{AppHandle, Manager};

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

pub struct ToolConfirmation {
    pub tx: oneshot::Sender<bool>,
    pub call_id: String,
}

pub struct AiToolState {
    pub pending_confirmations: Mutex<HashMap<String, ToolConfirmation>>,
}

impl AiToolState {
    pub fn new() -> Self {
        Self {
            pending_confirmations: Mutex::new(HashMap::new()),
        }
    }
}

pub fn get_tools_for_ollama() -> serde_json::Value {
    serde_json::json!([
        {
            "type": "function",
            "function": {
                "name": "create_task",
                "description": "Propose a new task. ONLY call when user explicitly asks to create a task. Requires confirmation.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "title":        { "type": "string" },
                        "priority":     { "type": "string", "enum": ["low","medium","high","urgent"] },
                        "due_date":     { "type": "string", "description": "YYYY-MM-DD" },
                        "energy_level": { "type": "string", "enum": ["deep_focus","light","admin","errand"] },
                        "context_tag":  { "type": "string", "enum": ["computer","phone","errands","home","anywhere"] }
                    },
                    "required": ["title"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "update_task",
                "description": "Modify an existing task. ONLY call when user explicitly asks to update a task. Requires confirmation.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "id":       { "type": "string" },
                        "title":    { "type": "string" },
                        "status":   { "type": "string", "enum": ["idea","todo","in_progress","done","cancelled"] },
                        "priority": { "type": "string", "enum": ["low","medium","high","urgent"] },
                        "due_date": { "type": "string" }
                    },
                    "required": ["id"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "complete_task",
                "description": "Mark a task as done. ONLY call when user explicitly asks to complete a task. Requires confirmation.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" }
                    },
                    "required": ["id"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_tasks",
                "description": "Retrieve tasks. Use when user asks what tasks they have.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "status":   { "type": "string" },
                        "priority": { "type": "string" }
                    }
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "search_journal",
                "description": "Search journal entries. Use when user asks about past journal entries.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query":     { "type": "string" },
                        "date_from": { "type": "string" },
                        "date_to":   { "type": "string" }
                    },
                    "required": ["query"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "fetch_url",
                "description": "Fetch content from a URL. Use only when user provides a specific URL.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "url": { "type": "string" }
                    },
                    "required": ["url"]
                }
            }
        }
    ])
}

pub fn get_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "create_task".into(),
            description: "Propose a new task to be added to the user's task list. Requires user confirmation.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Short, clear title of the task" },
                    "priority": { "type": "string", "enum": ["low", "medium", "high", "urgent"] },
                    "due_date": { "type": "string", "description": "ISO 8601 date (YYYY-MM-DD)" },
                    "project_id": { "type": "string", "description": "Optional project ID (default: 'inbox')" },
                    "energy_level": { "type": "string", "enum": ["deep_focus", "light", "admin", "errand"] },
                    "context_tag": { "type": "string", "enum": ["computer", "phone", "errands", "home", "anywhere"] }
                },
                "required": ["title"]
            }),
        },
        ToolDefinition {
            name: "update_task".into(),
            description: "Modify an existing task. Requires user confirmation.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "The unique ID of the task to update" },
                    "title": { "type": "string" },
                    "status": { "type": "string", "enum": ["todo", "in_progress", "done", "cancelled"] },
                    "priority": { "type": "string", "enum": ["low", "medium", "high", "urgent"] },
                    "due_date": { "type": "string" }
                },
                "required": ["id"]
            }),
        },
        ToolDefinition {
            name: "complete_task".into(),
            description: "Mark a task as completed. Requires user confirmation.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "The unique ID of the task to complete" }
                },
                "required": ["id"]
            }),
        },
        ToolDefinition {
            name: "list_tasks".into(),
            description: "Retrieve a list of tasks matching specific filters. No confirmation needed.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "status": { "type": "string", "enum": ["todo", "in_progress", "done", "cancelled"] },
                    "project_id": { "type": "string" },
                    "priority": { "type": "string" }
                }
            }),
        },
        ToolDefinition {
            name: "search_journal".into(),
            description: "Search the journal for past entries using keyword or date range. No confirmation needed.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search keyword" },
                    "date_from": { "type": "string" },
                    "date_to": { "type": "string" }
                },
                "required": ["query"]
            }),
        },
        ToolDefinition {
            name: "fetch_url".into(),
            description: "Fetch and read the content of a specific public web URL. No confirmation needed.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "The full HTTPS URL" }
                },
                "required": ["url"]
            }),
        },
    ]
}

pub async fn execute_tool_call(
    app: &AppHandle,
    tool_state: &AiToolState,
    name: &str,
    args: Value,
) -> Result<(String, Value), AppError> {
    let call_id = uuid::Uuid::new_v4().to_string();

    match name {
        // --- Confirmable Tools ---
        "create_task" | "update_task" | "complete_task" => {
            let (tx, rx) = oneshot::channel();
            
            {
                let mut pending = tool_state.pending_confirmations.lock().await;
                pending.insert(call_id.clone(), ToolConfirmation { tx, call_id: call_id.clone() });
            }

            // Emit pending event for UI
            emit(app, AppEvent::AiToolPending {
                call_id: call_id.clone(),
                name: name.to_string(),
                args: args.clone(),
                description: format!("AI wants to {}...", name.replace("_", " ")),
            });

            // Wait for confirmation with 300s timeout (D-95)
            let confirmed = tokio::select! {
                res = rx => res.unwrap_or(false),
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(300)) => {
                    let mut pending = tool_state.pending_confirmations.lock().await;
                    pending.remove(&call_id);
                    emit(app, AppEvent::AiConfirmTimeout {
                        call_id: call_id.clone(),
                        tool_name: name.to_string(),
                    });
                    false
                }
            };

            if !confirmed {
                return Ok((call_id, json!({ "status": "error", "message": "User declined the operation or it timed out." })));
            }

            // Execute actual logic
            let res = crate::db::tasks::execute_ai_tool(app, name, args).await?;
            Ok((call_id, res))
        }

        // --- Read-only Tools ---
        "list_tasks" => {
            emit(app, AppEvent::AiStatus("Searching for tasks...".into()));
            let conn_arc = app.state::<crate::AppState>().conn.clone();
            let conn = conn_arc.lock().await;
            
            let filters = serde_json::from_value(args).unwrap_or_default();
            let tasks = crate::db::tasks::list_tasks(&conn, filters)?;
            Ok((call_id, json!(tasks)))
        }
        "search_journal" => {
            emit(app, AppEvent::AiStatus("Searching journal entries...".into()));
            let conn_arc = app.state::<crate::AppState>().conn.clone();
            let conn = conn_arc.lock().await;
            
            let query = args["query"].as_str().ok_or_else(|| AppError::InvalidInput("Missing query".into()))?;
            let results = crate::db::journal::search_entries(&conn, query)?;
            Ok((call_id, json!(results)))
        }
        "fetch_url" => {
            emit(app, AppEvent::AiStatus(format!("Fetching {}...", args["url"].as_str().unwrap_or("URL"))));
            let url = args["url"].as_str().ok_or_else(|| AppError::InvalidInput("Missing URL".into()))?;
            let client = reqwest::Client::new();
            let res = client.get(url).send().await.map_err(|e| AppError::AiError(e.to_string()))?;
            let text = res.text().await.map_err(|e| AppError::AiError(e.to_string()))?;
            Ok((call_id, json!({ "content": text.chars().take(5000).collect::<String>() })))
        }
        _ => Err(AppError::InvalidInput(format!("Unknown tool: {}", name))),
    }
}
