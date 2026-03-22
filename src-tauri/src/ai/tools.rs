use crate::error::AppError;
use crate::events::{emit, AppEvent};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tokio::sync::{oneshot, Mutex};

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

pub struct ToolConfirmation {
    pub tx: oneshot::Sender<bool>,
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
    json!(get_tool_definitions()
        .into_iter()
        .map(|tool| json!({
            "type": "function",
            "function": {
                "name": tool.name,
                "description": tool.description,
                "parameters": tool.parameters,
            }
        }))
        .collect::<Vec<_>>())
}

pub fn get_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "create_task".into(),
            description: "Propose a new task to be added to the user's local task list. Use only when the user explicitly asks to create, add, or capture a concrete actionable task. Do not use for hypothetical brainstorming or vague intentions. Requires user confirmation before any database write.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "Short, clear, actionable task title",
                        "minLength": 1,
                        "maxLength": 100
                    },
                    "priority": { "type": "string", "enum": ["low", "medium", "high", "urgent"] },
                    "due_date": {
                        "type": "string",
                        "description": "ISO 8601 date (YYYY-MM-DD)",
                        "pattern": "^\\d{4}-\\d{2}-\\d{2}$"
                    },
                    "project_id": { "type": "string", "description": "Optional project ID (default: 'inbox')" },
                    "energy_level": { "type": "string", "enum": ["deep_focus", "light", "admin", "errand"] },
                    "context_tag": { "type": "string", "enum": ["computer", "phone", "errands", "home", "anywhere"] }
                },
                "required": ["title"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "update_task".into(),
            description: "Modify an existing task when the user explicitly asks to change a real task. Requires a specific task ID. Only include fields that should change. Prefer complete_task when the user is marking work done. Requires user confirmation before any database write.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Task reference: full UUID, short ID prefix like [a1b2c3], or a task title/keyword that can be resolved",
                        "minLength": 1,
                        "maxLength": 200
                    },
                    "title": { "type": "string", "maxLength": 100 },
                    "status": { "type": "string", "enum": ["todo", "in_progress", "done", "cancelled"] },
                    "priority": { "type": "string", "enum": ["low", "medium", "high", "urgent"] },
                    "due_date": {
                        "type": "string",
                        "description": "ISO 8601 date (YYYY-MM-DD)",
                        "pattern": "^\\d{4}-\\d{2}-\\d{2}$"
                    }
                },
                "required": ["id"],
                "additionalProperties": false,
                "minProperties": 2
            }),
        },
        ToolDefinition {
            name: "complete_task".into(),
            description: "Mark a task as completed when the user explicitly says they finished or completed it. Do not use for planned future work or ambiguous references. Requires user confirmation before any database write.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Task reference: full UUID, short ID prefix like [a1b2c3], or a task title/keyword that can be resolved",
                        "minLength": 1,
                        "maxLength": 200
                    }
                },
                "required": ["id"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "delete_task".into(),
            description: "Soft-delete a task (and its subtasks) by ID when the user explicitly asks to delete or remove a task. Requires user confirmation before any database write.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Task reference: full UUID, short ID prefix like [a1b2c3], or a task title/keyword that can be resolved",
                        "minLength": 1,
                        "maxLength": 200
                    }
                },
                "required": ["id"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "list_tasks".into(),
            description: "Retrieve current tasks from the local database when the answer depends on current task state not already present in context. Use filters when helpful. Read-only tool, so no confirmation is needed.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Optional title/keyword filter." },
                    "limit": { "type": "integer", "description": "Max results." },
                    "statuses": { "type": "array", "items": { "type": "string", "enum": ["todo", "in_progress", "done", "cancelled"] } },
                    "project_id": { "type": "string" },
                    "due_before": { "type": "string", "description": "YYYY-MM-DD" },
                    "due_after": { "type": "string", "description": "YYYY-MM-DD" }
                },
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "search_journal".into(),
            description: "Search past journal entries by keyword, optionally limited by a date range. Use when the user asks about prior thoughts, events, or themes and the answer requires historical journal content. Read-only tool, so no confirmation is needed.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search keyword",
                        "minLength": 3,
                        "maxLength": 100
                    },
                    "date_from": {
                        "type": "string",
                        "description": "ISO 8601 date (YYYY-MM-DD)",
                        "pattern": "^\\d{4}-\\d{2}-\\d{2}$"
                    },
                    "date_to": {
                        "type": "string",
                        "description": "ISO 8601 date (YYYY-MM-DD)",
                        "pattern": "^\\d{4}-\\d{2}-\\d{2}$"
                    }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "fetch_url".into(),
            description: "Fetch and read the content of a specific public HTTPS URL when the user explicitly asks you to inspect that exact URL. This is not a web search tool. It is read-only and should not be used for localhost, private-network, or non-HTTPS addresses.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The full HTTPS URL",
                        "pattern": "^https://.+$",
                        "maxLength": 2048
                    }
                },
                "required": ["url"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "search_conversations".into(),
            description: "Search across all past AI chat history for a keyword or phrase. Use when the user asks 'what did we talk about...' or 'did I tell you already...'. Read-only tool.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "minLength": 2, "maxLength": 100 }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "pin_memory".into(),
            description: "Save a key fact about the user for long-term personalization (e.g., 'prefers deep work in mornings', 'has a dog named Rex'). Only use when the user explicitly asks to remember a fact or when a significant preference is revealed. Requires user confirmation.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "content": { "type": "string", "minLength": 5, "maxLength": 300, "description": "The fact to remember (concise third-person statement)." },
                    "importance": { "type": "integer", "minimum": 1, "maximum": 5, "description": "1=casual, 3=important, 5=critical" }
                },
                "required": ["content"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "delete_memory".into(),
            description: "Remove a fact from the long-term pinned memory. Requires the exact text of the fact to find and remove it. Requires user confirmation.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "content": { "type": "string", "description": "The text of the pinned memory to delete." }
                },
                "required": ["content"],
                "additionalProperties": false
            }),
        },
    ]
}

pub async fn execute_tool_call(
    app: &AppHandle,
    conversation_id: Option<String>,
    tool_state: &AiToolState,
    name: &str,
    mut args: Value,
) -> Result<(String, Value), AppError> {
    let call_id = uuid::Uuid::new_v4().to_string();

    match name {
        // --- Confirmable Tools ---
        "create_task" | "update_task" | "complete_task" | "delete_task" => {
            if matches!(name, "create_task" | "update_task") {
                strip_blank_optional_strings(
                    &mut args,
                    &[
                        "due_date",
                        "project_id",
                        "energy_level",
                        "context_tag",
                        "status",
                        "priority",
                    ],
                );
            }
            if let Err(message) = validate_mutating_tool_args(name, &args) {
                return Ok((
                    call_id,
                    json!({
                        "status": "error",
                        "code": "validation_failed",
                        "message": message
                    }),
                ));
            }

            let (tx, rx) = oneshot::channel();

            {
                let mut pending = tool_state.pending_confirmations.lock().await;
                pending.insert(call_id.clone(), ToolConfirmation { tx });
            }

            // Emit pending event for UI
            emit(
                app,
                AppEvent::AiToolPending {
                    call_id: call_id.clone(),
                    name: name.to_string(),
                    args: args.clone(),
                    description: format!("AI wants to {}...", name.replace("_", " ")),
                },
            );

            // Wait for confirmation with 300s timeout (D-95)
            let confirmed = tokio::select! {
                res = rx => res.unwrap_or(false),
                _ = tokio::time::sleep(Duration::from_secs(300)) => {
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
                return Ok((
                    call_id,
                    json!({
                        "status": "cancelled",
                        "message": "User declined the operation or it timed out after 300 seconds."
                    }),
                ));
            }

            // Execute actual logic
            match crate::db::tasks::execute_ai_tool(app, conversation_id, name, args).await {
                Ok(res) => Ok((call_id, res)),
                Err(e) => Ok((
                    call_id,
                    json!({
                        "status": "error",
                        "code": "execution_failed",
                        "message": e.to_string()
                    }),
                )),
            }
        }

        // --- Read-only Tools ---
        "list_tasks" => {
            args = normalize_readonly_tool_args(name, args).map_err(AppError::InvalidInput)?;
            strip_blank_optional_strings(
                &mut args,
                &["query", "limit", "project_id", "category", "due_before", "due_after"],
            );
            coerce_string_to_integer(&mut args, &["limit"]);
            if let Err(message) = validate_list_tasks_args(&args) {
                return Ok((
                    call_id,
                    json!({
                        "status": "error",
                        "code": "validation_failed",
                        "message": message
                    }),
                ));
            }

            emit(app, AppEvent::AiStatus("Searching for tasks...".into()));
            let conn_arc = app.state::<crate::AppState>().conn.clone();
            let conn = conn_arc.lock().await;

            let filters: crate::db::tasks::TaskListFilters = match serde_json::from_value(args) {
                Ok(v) => v,
                Err(e) => {
                    return Ok((
                        call_id,
                        json!({
                            "status": "error",
                            "code": "invalid_arguments",
                            "message": format!("Invalid list_tasks arguments: {}", e)
                        }),
                    ));
                }
            };
            let tasks = crate::db::tasks::list_tasks(&conn, filters)?;
            let compact = tasks
                .into_iter()
                .map(|task| {
                    json!({
                        "id": task.id,
                        "title": task.title,
                        "status": task.status,
                        "priority": task.priority,
                        "due_date": task.due_date,
                        "project_id": task.project_id,
                        "energy_level": task.energy_level,
                    })
                })
                .collect::<Vec<_>>();
            Ok((
                call_id,
                json!({
                    "status": "success",
                    "count": compact.len(),
                    "tasks": compact
                }),
            ))
        }
        "search_journal" => {
            args = normalize_readonly_tool_args(name, args).map_err(AppError::InvalidInput)?;
            strip_blank_optional_strings(&mut args, &["date_from", "date_to"]);
            if let Err(message) = validate_search_journal_args(&args) {
                return Ok((
                    call_id,
                    json!({
                        "status": "error",
                        "code": "validation_failed",
                        "message": message
                    }),
                ));
            }

            emit(
                app,
                AppEvent::AiStatus("Searching journal entries...".into()),
            );
            let conn_arc = app.state::<crate::AppState>().conn.clone();
            let conn = conn_arc.lock().await;

            let filters: crate::db::journal::JournalSearchFilters =
                match serde_json::from_value(args) {
                    Ok(v) => v,
                    Err(e) => {
                        return Ok((
                            call_id,
                            json!({
                                "status": "error",
                                "code": "invalid_arguments",
                                "message": format!("Invalid search_journal arguments: {}", e)
                            }),
                        ));
                    }
                };
            let results = crate::db::journal::search_entries(&conn, &filters)?;
            Ok((
                call_id,
                json!({
                    "status": "success",
                    "count": results.len(),
                    "results": results
                }),
            ))
        }
        "fetch_url" => {
            let url = args["url"]
                .as_str()
                .ok_or_else(|| AppError::InvalidInput("Missing URL".into()))?;

            if let Err(message) = validate_url(url) {
                return Ok((
                    call_id,
                    json!({
                        "status": "error",
                        "code": "invalid_url",
                        "message": message
                    }),
                ));
            }

            emit(
                app,
                AppEvent::AiStatus(format!("Fetching {}...", truncate_url(url))),
            );

            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .redirect(reqwest::redirect::Policy::limited(5))
                .build()
                .map_err(|e| AppError::AiError(e.to_string()))?;

            match client.get(url).send().await {
                Ok(response) => {
                    if !response.status().is_success() {
                        return Ok((
                            call_id,
                            json!({
                                "status": "error",
                                "code": "http_error",
                                "message": format!("Server returned {}", response.status())
                            }),
                        ));
                    }

                    match response.text().await {
                        Ok(text) => {
                            let truncated = text.chars().take(5000).collect::<String>();
                            Ok((
                                call_id,
                                json!({
                                    "status": "success",
                                    "url": url,
                                    "content": truncated,
                                    "truncated": text.chars().count() > 5000
                                }),
                            ))
                        }
                        Err(e) => Ok((
                            call_id,
                            json!({
                                "status": "error",
                                "code": "read_error",
                                "message": format!("Could not read response body: {}", e)
                            }),
                        )),
                    }
                }
                Err(e) => {
                    let code = if e.is_timeout() {
                        "timeout"
                    } else if e.is_connect() {
                        "connection_failed"
                    } else {
                        "request_failed"
                    };

                    Ok((
                        call_id,
                        json!({
                            "status": "error",
                            "code": code,
                            "message": format!("Request failed: {}", e)
                        }),
                    ))
                }
            }
        }
        "pin_memory" | "delete_memory" => {
            if name == "pin_memory" {
                coerce_string_to_integer(&mut args, &["importance"]);
            }
            let (tx, rx) = oneshot::channel();

            {
                let mut pending = tool_state.pending_confirmations.lock().await;
                pending.insert(call_id.clone(), ToolConfirmation { tx });
            }

            emit(
                app,
                AppEvent::AiToolPending {
                    call_id: call_id.clone(),
                    name: name.to_string(),
                    args: args.clone(),
                    description: if name == "pin_memory" {
                        "AI wants to remember a specific fact about you...".into()
                    } else {
                        "AI wants to forget a specific pinned fact...".into()
                    },
                },
            );

            let confirmed = tokio::select! {
                res = rx => res.unwrap_or(false),
                _ = tokio::time::sleep(Duration::from_secs(300)) => {
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
                return Ok((
                    call_id,
                    json!({
                        "status": "cancelled",
                        "message": "User declined the operation or it timed out after 300 seconds."
                    }),
                ));
            }

            let conn_arc = app.state::<crate::AppState>().conn.clone();
            let conn = conn_arc.lock().await;

            if name == "pin_memory" {
                let content = args["content"].as_str().unwrap_or_default().to_string();
                let importance = args["importance"].as_i64().unwrap_or(1) as i32;
                let now = Utc::now().to_rfc3339();
                let mem = crate::db::ai::AiPinnedMemory {
                    id: uuid::Uuid::new_v4().to_string(),
                    content,
                    importance,
                    metadata: None,
                    created_at: now.clone(),
                    updated_at: now,
                };
                crate::db::ai::upsert_pinned_memory(&conn, &mem)?;
                Ok((call_id, json!({ "status": "success", "message": "Memory pinned successfully." })))
            } else {
                let content = args["content"].as_str().unwrap_or_default();
                // Find by content
                let pinned = crate::db::ai::list_pinned_memory(&conn)?;
                if let Some(p) = pinned.iter().find(|m| m.content.contains(content)) {
                    crate::db::ai::delete_pinned_memory(&conn, &p.id)?;
                    Ok((call_id, json!({ "status": "success", "message": "Memory deleted successfully." })))
                } else {
                    Ok((call_id, json!({ "status": "error", "message": "Memory not found." })))
                }
            }
        }
        "search_conversations" => {
            let query = args["query"].as_str().unwrap_or_default();
            emit(app, AppEvent::AiStatus("Searching conversations...".into()));
            
            let conn_arc = app.state::<crate::AppState>().conn.clone();
            let conn = conn_arc.lock().await;

            let results = crate::db::ai::search_messages(&conn, query)?;
            let compact = results.into_iter().map(|m| {
                json!({
                    "role": m.role,
                    "content": m.content,
                    "created_at": m.created_at,
                    "conversation_id": m.conversation_id
                })
            }).collect::<Vec<_>>();

            Ok((call_id, json!({ "status": "success", "results": compact })))
        }
        _ => Err(AppError::InvalidInput(format!("Unknown tool: {}", name))),
    }
}

fn normalize_readonly_tool_args(tool_name: &str, args: Value) -> Result<Value, String> {
    // Ollama tool calls sometimes emit `arguments` as `[]`, `null`, or even as a JSON string.
    // For read-only tools, we can safely normalize common malformed shapes.
    match args {
        Value::Null => Ok(json!({})),
        Value::Array(arr) if arr.is_empty() => Ok(json!({})),
        Value::Array(_) => Err(format!(
            "{} expects an object argument like {{...filters}}, not an array",
            tool_name
        )),
        Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return Ok(json!({}));
            }

            if trimmed == "[]" {
                return Ok(json!({}));
            }

            if !(trimmed.starts_with('{') || trimmed.starts_with('[')) {
                return Err(format!(
                    "{} arguments must be a JSON object (or omitted).",
                    tool_name
                ));
            }

            serde_json::from_str::<Value>(trimmed)
                .map_err(|e| format!("{} arguments JSON could not be parsed: {}", tool_name, e))
        }
        other => Ok(other),
    }
}

fn strip_blank_optional_strings(args: &mut Value, keys: &[&str]) {
    let Some(obj) = args.as_object_mut() else {
        return;
    };

    for key in keys {
        let should_remove = obj
            .get(*key)
            .and_then(|v| v.as_str())
            .is_some_and(|s| s.trim().is_empty());

        if should_remove {
            obj.remove(*key);
        }
    }
}

fn coerce_string_to_integer(args: &mut Value, keys: &[&str]) {
    let Some(obj) = args.as_object_mut() else {
        return;
    };

    for key in keys {
        if let Some(val) = obj.get(*key) {
            if let Some(s) = val.as_str() {
                if let Ok(num) = s.parse::<i64>() {
                    obj.insert(key.to_string(), json!(num));
                }
            }
        }
    }
}

fn validate_mutating_tool_args(tool_name: &str, args: &Value) -> Result<(), String> {
    match tool_name {
        "create_task" => {
            let title = args["title"]
                .as_str()
                .ok_or_else(|| "Missing required field: title".to_string())?;
            if title.trim().is_empty() {
                return Err("Task title cannot be empty".into());
            }
            if title.len() > 100 {
                return Err("Task title must be 100 characters or fewer".into());
            }
            if let Some(due_date) = args["due_date"].as_str() {
                validate_iso_date("due_date", due_date)?;
            }
            Ok(())
        }
        "update_task" => {
            let id = args["id"]
                .as_str()
                .ok_or_else(|| "Missing required field: id".to_string())?;
            validate_task_reference_field("id", id)?;
            if args.as_object().map(|obj| obj.len()).unwrap_or_default() <= 1 {
                return Err("At least one field to update must be provided".into());
            }
            if let Some(title) = args["title"].as_str() {
                if title.trim().is_empty() {
                    return Err("Updated task title cannot be empty".into());
                }
                if title.len() > 100 {
                    return Err("Updated task title must be 100 characters or fewer".into());
                }
            }
            if let Some(due_date) = args["due_date"].as_str() {
                validate_iso_date("due_date", due_date)?;
            }
            Ok(())
        }
        "complete_task" => {
            let id = args["id"]
                .as_str()
                .ok_or_else(|| "Missing required field: id".to_string())?;
            validate_task_reference_field("id", id)?;
            Ok(())
        }
        "delete_task" => {
            let id = args["id"]
                .as_str()
                .ok_or_else(|| "Missing required field: id".to_string())?;
            validate_task_reference_field("id", id)?;
            Ok(())
        }
        _ => Ok(()),
    }
}

fn validate_list_tasks_args(args: &Value) -> Result<(), String> {
    if let Some(due_before) = args["due_before"].as_str() {
        validate_iso_date("due_before", due_before)?;
    }
    if let Some(due_after) = args["due_after"].as_str() {
        validate_iso_date("due_after", due_after)?;
    }
    Ok(())
}

fn validate_search_journal_args(args: &Value) -> Result<(), String> {
    let query = args["query"]
        .as_str()
        .ok_or_else(|| "Missing required field: query".to_string())?;
    let trimmed = query.trim();
    if trimmed.len() < 3 {
        return Err("Search query must be at least 3 characters".into());
    }
    if trimmed.len() > 100 {
        return Err("Search query must be 100 characters or fewer".into());
    }

    if let Some(date_from) = args["date_from"].as_str() {
        validate_iso_date("date_from", date_from)?;
    }
    if let Some(date_to) = args["date_to"].as_str() {
        validate_iso_date("date_to", date_to)?;
    }
    Ok(())
}

fn validate_iso_date(field_name: &str, value: &str) -> Result<(), String> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map(|_| ())
        .map_err(|_| format!("Invalid {} format. Use YYYY-MM-DD", field_name))
}

fn validate_task_reference_field(field_name: &str, value: &str) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("Missing required field: {}", field_name));
    }
    if trimmed.len() > 200 {
        return Err(format!(
            "Invalid {}: must be 200 characters or fewer",
            field_name
        ));
    }
    Ok(())
}

fn validate_url(url: &str) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err("URL must start with https://".into());
    }

    if url.len() > 2048 {
        return Err("URL must be 2048 characters or fewer".into());
    }

    let lower = url.to_ascii_lowercase();
    let blocked_prefixes = [
        "https://localhost",
        "https://127.",
        "https://10.",
        "https://192.168.",
        "https://172.16.",
        "https://172.17.",
        "https://172.18.",
        "https://172.19.",
        "https://172.20.",
        "https://172.21.",
        "https://172.22.",
        "https://172.23.",
        "https://172.24.",
        "https://172.25.",
        "https://172.26.",
        "https://172.27.",
        "https://172.28.",
        "https://172.29.",
        "https://172.30.",
        "https://172.31.",
        "https://0.0.0.0",
        "https://[::1]",
        "https://[fc",
        "https://[fd",
        "https://[fe80",
    ];

    if blocked_prefixes
        .iter()
        .any(|prefix| lower.starts_with(prefix))
    {
        return Err("Local and private-network URLs are not allowed".into());
    }

    Ok(())
}

fn truncate_url(url: &str) -> String {
    const MAX_LEN: usize = 60;
    if url.len() <= MAX_LEN {
        return url.to_string();
    }

    format!("{}...", &url[..MAX_LEN - 3])
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn create_task_validation_rejects_blank_title() {
        let result = validate_mutating_tool_args(
            "create_task",
            &json!({
                "title": "   "
            }),
        );

        assert!(result.is_err());
    }

    #[test]
    fn update_task_validation_requires_fields_to_change() {
        let result = validate_mutating_tool_args(
            "update_task",
            &json!({
                "id": "abcdef"
            }),
        );

        assert!(result.is_err());
    }

    #[test]
    fn search_journal_validation_rejects_short_query() {
        let result = validate_search_journal_args(&json!({
            "query": "hi"
        }));

        assert!(result.is_err());
    }

    #[test]
    fn list_tasks_validation_rejects_invalid_dates() {
        let result = validate_list_tasks_args(&json!({
            "due_before": "2026-99-99"
        }));

        assert!(result.is_err());
    }

    #[test]
    fn strip_blank_optional_strings_removes_empty_values() {
        let mut args = json!({
            "due_before": "",
            "due_after": "   ",
            "category": "work"
        });

        strip_blank_optional_strings(&mut args, &["due_before", "due_after"]);

        assert!(args.get("due_before").is_none());
        assert!(args.get("due_after").is_none());
        assert_eq!(args.get("category").and_then(|v| v.as_str()), Some("work"));
    }

    #[test]
    fn task_reference_validation_allows_human_phrases() {
        let result = validate_mutating_tool_args(
            "complete_task",
            &json!({
                "id": "New Task"
            }),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn normalize_readonly_tool_args_accepts_empty_array_as_empty_object() {
        let normalized = normalize_readonly_tool_args("list_tasks", json!([])).unwrap();
        assert!(normalized.is_object());
    }

    #[test]
    fn validate_url_allows_public_https() {
        let result = validate_url("https://example.com/article");

        assert!(result.is_ok());
    }

    #[test]
    fn validate_url_blocks_localhost() {
        let result = validate_url("https://localhost:11434/api/tags");

        assert!(result.is_err());
    }

    #[test]
    fn truncate_url_shortens_long_values() {
        let url =
            "https://example.com/this/is/a/very/long/path/that/should/be/truncated/by/the-helper";
        let truncated = truncate_url(url);

        assert!(truncated.len() <= 60);
        assert!(truncated.ends_with("..."));
    }
}
