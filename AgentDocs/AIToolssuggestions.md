Verbesserter tools.rs Vorschlag
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::error::AppError;
use tokio::sync::{Mutex, oneshot};
use std::collections::HashMap;
use std::time::Duration;
use crate::events::{AppEvent, emit};
use tauri::{AppHandle, Manager};
use chrono::NaiveDate;

// ═══════════════════════════════════════════════════════════════
// STRUCTS & STATE
// ═══════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════
// TOOL DEFINITIONS — Enhanced with Guardrails
// ═══════════════════════════════════════════════════════════════

/// Returns tool definitions formatted for Ollama's function calling API.
/// 
/// Each tool description includes:
/// - When to use it (trigger conditions)
/// - When NOT to use it (constraints)
/// - Required vs optional parameters
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
        // ─────────────────────────────────────────────────────────
        // MUTATING TOOLS (Require Confirmation)
        // ─────────────────────────────────────────────────────────
        ToolDefinition {
            name: "create_task".into(),
            description: r#"Propose a new task to be added to the user's task list.

WHEN TO USE:
- User explicitly asks to create, add, or schedule a new task.
- User describes an actionable item during conversation.
- Journal analysis extracts a concrete, user-actionable task.

WHEN NOT TO USE:
- User is brainstorming hypothetically ("What if I had to...").
- Task details are ambiguous (missing title or unclear intent).
- User is venting emotions without requesting action.

REQUIRED PARAMETERS:
- title: Must be specific, actionable, and ≤100 characters.

OPTIONAL PARAMETERS (use defaults if unspecified):
- priority: Default "medium". Use "urgent" only for same-day deadlines.
- due_date: ISO 8601 (YYYY-MM-DD). Convert relative dates ("tomorrow") using current context.
- project_id: Default "inbox". Only set if user explicitly names a project.
- energy_level: Default "any". Match task demands to user's energy states.
- context_tag: Default "anywhere". Use for location-specific tasks only.

USER CONFIRMATION: Required before execution."#.into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "title": { 
                        "type": "string", 
                        "description": "Short, clear, actionable task title (≤100 chars)",
                        "maxLength": 100
                    },
                    "priority": { 
                        "type": "string", 
                        "enum": ["low", "medium", "high", "urgent"],
                        "description": "Default: medium. Use 'urgent' only for same-day deadlines."
                    },
                    "due_date": { 
                        "type": "string", 
                        "description": "ISO 8601 date (YYYY-MM-DD). Convert relative dates yourself.",
                        "pattern": "^\\d{4}-\\d{2}-\\d{2}$"
                    },
                    "project_id": { 
                        "type": "string", 
                        "description": "Default: inbox. Only set if user explicitly names a project."
                    },
                    "energy_level": { 
                        "type": "string", 
                        "enum": ["deep_focus", "light", "admin", "errand"],
                        "description": "Default: any. Match task to required energy state."
                    },
                    "context_tag": { 
                        "type": "string", 
                        "enum": ["computer", "phone", "errands", "home", "anywhere"],
                        "description": "Default: anywhere. Use for location-specific tasks only."
                    }
                },
                "required": ["title"],
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "update_task".into(),
            description: r#"Modify an existing task's fields.

WHEN TO USE:
- User explicitly asks to change, edit, or update a task.
- User provides a valid task ID (or you can infer from context).
- User wants to change priority, due date, status, or title.

WHEN NOT TO USE:
- User is asking about a task (use list_tasks instead).
- Task ID is ambiguous and cannot be inferred.
- User wants to mark complete (use complete_task instead).

REQUIRED PARAMETERS:
- id: The unique ID of the task (6+ characters from start of UUID).

OPTIONAL PARAMETERS:
- title, status, priority, due_date: Only include fields the user wants to change.
  Do NOT send unchanged fields.

USER CONFIRMATION: Required before execution.

IMPORTANT: Never update a task that is already 'done' or 'cancelled'."#.into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": { 
                        "type": "string", 
                        "description": "The unique task ID (first 6+ chars of UUID)",
                        "minLength": 6
                    },
                    "title": { "type": "string", "maxLength": 100 },
                    "status": { 
                        "type": "string", 
                        "enum": ["todo", "in_progress", "done", "cancelled"],
                        "description": "Do NOT use 'done' here — use complete_task instead."
                    },
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
            description: r#"Mark a task as completed (status = 'done').

WHEN TO USE:
- User explicitly says they finished, completed, or did a task.
- User indicates accomplishment ("checked off", "crossed out").

WHEN NOT TO USE:
- User is planning to do something (task not yet complete).
- User is unsure which task they finished.

REQUIRED PARAMETERS:
- id: The unique task ID.

USER CONFIRMATION: Required before execution.

NOTE: This is a convenience wrapper around update_task with status='done'."#.into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": { 
                        "type": "string", 
                        "description": "The unique task ID (first 6+ chars of UUID)",
                        "minLength": 6
                    }
                },
                "required": ["id"],
                "additionalProperties": false
            }),
        },

        // ─────────────────────────────────────────────────────────
        // READ-ONLY TOOLS (No Confirmation)
        // ─────────────────────────────────────────────────────────
        ToolDefinition {
            name: "list_tasks".into(),
            description: r#"Retrieve current tasks from the local database.

WHEN TO USE:
- User asks "What are my tasks?", "What's due?", etc.
- Answer depends on current task data not already in context.
- User wants to filter by status, priority, date range, or project.

WHEN NOT TO USE:
- Task list is already provided in the prompt context.
- User asks about a specific task by ID (you have enough info).

FILTER PARAMETERS (all optional):
- statuses: Filter by status array. Default: ["todo", "in_progress"].
- exclude_statuses: Exclude certain statuses.
- priorities: Filter by priority array.
- due_before / due_after: Date range filtering (ISO 8601).
- project_id: Filter by specific project.
- energy_levels, context_tags, tags: Additional filters.
- limit: Max results to return. Default: 50, Max: 200.

TIP: Always set a reasonable limit (20-50) to avoid overwhelming responses."#.into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "statuses": { 
                        "type": "array", 
                        "items": { "type": "string", "enum": ["todo", "in_progress", "done", "cancelled"] },
                        "description": "Default: ['todo', 'in_progress']"
                    },
                    "exclude_statuses": { 
                        "type": "array", 
                        "items": { "type": "string", "enum": ["todo", "in_progress", "done", "cancelled"] }
                    },
                    "priorities": { 
                        "type": "array", 
                        "items": { "type": "string", "enum": ["low", "medium", "high", "urgent"] }
                    },
                    "project_id": { "type": "string" },
                    "category": { "type": "string" },
                    "energy_levels": { 
                        "type": "array", 
                        "items": { "type": "string", "enum": ["deep_focus", "light", "admin", "errand"] }
                    },
                    "context_tags": { 
                        "type": "array", 
                        "items": { "type": "string", "enum": ["computer", "phone", "errands", "home", "anywhere"] }
                    },
                    "tags": { "type": "array", "items": { "type": "string" } },
                    "due_before": { 
                        "type": "string", 
                        "description": "ISO 8601 date (YYYY-MM-DD)",
                        "pattern": "^\\d{4}-\\d{2}-\\d{2}$"
                    },
                    "due_after": { 
                        "type": "string", 
                        "description": "ISO 8601 date (YYYY-MM-DD)",
                        "pattern": "^\\d{4}-\\d{2}-\\d{2}$"
                    },
                    "limit": { 
                        "type": "integer", 
                        "description": "Max results. Default: 50, Max: 200.",
                        "minimum": 1,
                        "maximum": 200
                    }
                },
                "additionalProperties": false
            }),
        },
        ToolDefinition {
            name: "search_journal".into(),
            description: r#"Search the journal for past entries using a keyword and optional date range.

WHEN TO USE:
- User asks about past thoughts, feelings, or events.
- User wants to find entries mentioning a specific topic.
- Answer requires searching historical journal content.

WHEN NOT TO USE:
- Query is empty or too vague ("search for something about work").
- Journal entries are already provided in the prompt context.

REQUIRED PARAMETERS:
- query: Search keyword (≥3 characters, ≤100 characters).

OPTIONAL PARAMETERS:
- date_from / date_to: ISO 8601 dates to limit search range.

TIP: If search returns no results, suggest broader keywords or wider date range."#.into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": { 
                        "type": "string", 
                        "description": "Search keyword (3-100 chars)",
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

        // ─────────────────────────────────────────────────────────
        // EXTERNAL READ TOOLS (No Confirmation)
        // ─────────────────────────────────────────────────────────
        ToolDefinition {
            name: "fetch_url".into(),
            description: r#"Fetch and read the content of a specific public web URL.

WHEN TO USE:
- User provides a specific URL and asks you to read it.
- User wants to summarize an article, blog post, or document.
- Content behind URL is relevant to conversation.

WHEN NOT TO USE:
- User asks for web search (not available — use search_journal instead).
- URL is not provided or is malformed.
- URL requires authentication (login walls, paywalls).

REQUIRED PARAMETERS:
- url: Full HTTPS URL (must start with https://).

LIMITATIONS:
- Max 5000 characters returned.
- 10-second timeout for slow servers.
- Cannot access localhost, private IPs, or blocked domains.
- Cannot execute JavaScript or render dynamic content.

SECURITY: URLs are validated to prevent SSRF attacks."#.into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "url": { 
                        "type": "string", 
                        "description": "Full HTTPS URL (must start with https://)",
                        "pattern": "^https://.+$",
                        "maxLength": 2048
                    }
                },
                "required": ["url"],
                "additionalProperties": false
            }),
        },
    ]
}

// ═══════════════════════════════════════════════════════════════
// TOOL EXECUTION — Enhanced with Validation & Logging
// ═══════════════════════════════════════════════════════════════

/// Execute a tool call with full validation, confirmation handling, and logging.
/// 
/// Returns: (call_id, result_json)
/// - call_id: UUID for tracking confirmations
/// - result_json: Tool-specific response or error object
/// 
/// Errors are returned as structured JSON, not thrown as Rust errors,
/// so the AI can gracefully handle and explain failures to the user.
pub async fn execute_tool_call(
    app: &AppHandle,
    tool_state: &AiToolState,
    name: &str,
    args: Value,
) -> Result<(String, Value), AppError> {
    let call_id = uuid::Uuid::new_v4().to_string();
    
    // Log tool invocation for audit trail
    tracing::info!(
        target: "ai_tools",
        call_id = %call_id,
        tool = name,
        args = %args,
        "Tool call initiated"
    );

    match name {
        // ─────────────────────────────────────────────────────────
        // MUTATING TOOLS — Require Confirmation
        // ─────────────────────────────────────────────────────────
        "create_task" | "update_task" | "complete_task" => {
            // Validate arguments BEFORE requesting confirmation
            if let Err(e) = validate_mutating_tool_args(name, &args) {
                tracing::warn!(
                    target: "ai_tools",
                    call_id = %call_id,
                    tool = name,
                    error = %e,
                    "Tool call validation failed"
                );
                return Ok((call_id, json!({
                    "status": "error",
                    "code": "VALIDATION_FAILED",
                    "message": e
                })));
            }

            let (tx, rx) = oneshot::channel();

            {
                let mut pending = tool_state.pending_confirmations.lock().await;
                pending.insert(call_id.clone(), ToolConfirmation { tx });
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
                _ = tokio::time::sleep(Duration::from_secs(300)) => {
                    let mut pending = tool_state.pending_confirmations.lock().await;
                    pending.remove(&call_id);
                    emit(app, AppEvent::AiConfirmTimeout {
                        call_id: call_id.clone(),
                        tool_name: name.to_string(),
                    });
                    tracing::warn!(
                        target: "ai_tools",
                        call_id = %call_id,
                        tool = name,
                        "Tool call timed out (300s)"
                    );
                    false
                }
            };

            if !confirmed {
                tracing::info!(
                    target: "ai_tools",
                    call_id = %call_id,
                    tool = name,
                    "Tool call declined by user or timed out"
                );
                return Ok((call_id, json!({
                    "status": "cancelled",
                    "reason": "User declined the operation or it timed out after 300 seconds."
                })));
            }

            // Execute actual logic
            match crate::db::tasks::execute_ai_tool(app, name, args).await {
                Ok(result) => {
                    tracing::info!(
                        target: "ai_tools",
                        call_id = %call_id,
                        tool = name,
                        "Tool call completed successfully"
                    );
                    Ok((call_id, result))
                }
                Err(e) => {
                    tracing::error!(
                        target: "ai_tools",
                        call_id = %call_id,
                        tool = name,
                        error = %e,
                        "Tool call execution failed"
                    );
                    Ok((call_id, json!({
                        "status": "error",
                        "code": "EXECUTION_FAILED",
                        "message": e.to_string()
                    })))
                }
            }
        }

        // ─────────────────────────────────────────────────────────
        // READ-ONLY TOOLS — No Confirmation
        // ─────────────────────────────────────────────────────────
        "list_tasks" => {
            // Validate arguments
            if let Err(e) = validate_list_tasks_args(&args) {
                return Ok((call_id, json!({
                    "status": "error",
                    "code": "VALIDATION_FAILED",
                    "message": e
                })));
            }

            emit(app, AppEvent::AiStatus("Searching for tasks...".into()));
            let conn_arc = app.state::<crate::AppState>().conn.clone();
            let conn = conn_arc.lock().await;

            let filters: crate::db::tasks::TaskListFilters = serde_json::from_value(args)
                .map_err(|e| {
                    tracing::error!(
                        target: "ai_tools",
                        call_id = %call_id,
                        tool = "list_tasks",
                        error = %e,
                        "Failed to parse arguments"
                    );
                    AppError::InvalidInput(format!("Invalid list_tasks arguments: {}", e))
                })?;
            
            let tasks = crate::db::tasks::list_tasks(&conn, filters)?;
            
            // Return compact format to save context tokens
            let compact = tasks.into_iter().map(|task| {
                json!({
                    "id": task.id,
                    "title": task.title,
                    "status": task.status,
                    "priority": task.priority,
                    "due_date": task.due_date,
                    "project_id": task.project_id,
                    "energy_level": task.energy_level,
                })
            }).collect::<Vec<_>>();
            
            tracing::info!(
                target: "ai_tools",
                call_id = %call_id,
                tool = "list_tasks",
                result_count = compact.len(),
                "Tool call completed"
            );
            Ok((call_id, json!(compact)))
        }
        "search_journal" => {
            // Validate arguments
            if let Err(e) = validate_search_journal_args(&args) {
                return Ok((call_id, json!({
                    "status": "error",
                    "code": "VALIDATION_FAILED",
                    "message": e
                })));
            }

            emit(app, AppEvent::AiStatus("Searching journal entries...".into()));
            let conn_arc = app.state::<crate::AppState>().conn.clone();
            let conn = conn_arc.lock().await;

            let filters: crate::db::journal::JournalSearchFilters = serde_json::from_value(args)
                .map_err(|e| {
                    tracing::error!(
                        target: "ai_tools",
                        call_id = %call_id,
                        tool = "search_journal",
                        error = %e,
                        "Failed to parse arguments"
                    );
                    AppError::InvalidInput(format!("Invalid search_journal arguments: {}", e))
                })?;
            
            if filters.query.trim().is_empty() {
                return Ok((call_id, json!({
                    "status": "error",
                    "code": "INVALID_QUERY",
                    "message": "Search query cannot be empty"
                })));
            }
            
            let results = crate::db::journal::search_entries(&conn, &filters)?;
            
            tracing::info!(
                target: "ai_tools",
                call_id = %call_id,
                tool = "search_journal",
                result_count = results.len(),
                "Tool call completed"
            );
            Ok((call_id, json!(results)))
        }
        "fetch_url" => {
            // Validate URL before fetching
            let url = args["url"].as_str().ok_or_else(|| {
                AppError::InvalidInput("Missing URL parameter".into())
            })?;
            
            if let Err(e) = validate_url(url) {
                return Ok((call_id, json!({
                    "status": "error",
                    "code": "INVALID_URL",
                    "message": e
                })));
            }

            emit(app, AppEvent::AiStatus(format!("Fetching {}...", truncate_url(url))));
            
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(10))  // 10s timeout (D-111)
                .redirect(reqwest::redirect::Policy::limited(5))
                .build()
                .map_err(|e| AppError::AiError(format!("HTTP client error: {}", e)))?;
            
            match client.get(url).send().await {
                Ok(res) => {
                    if !res.status().is_success() {
                        return Ok((call_id, json!({
                            "status": "error",
                            "code": "HTTP_ERROR",
                            "message": format!("Server returned {}", res.status())
                        })));
                    }
                    
                    match res.text().await {
                        Ok(text) => {
                            let truncated = text.chars().take(5000).collect::<String>();
                            let char_count = truncated.len();
                            
                            tracing::info!(
                                target: "ai_tools",
                                call_id = %call_id,
                                tool = "fetch_url",
                                url = url,
                                content_length = char_count,
                                "Tool call completed"
                            );
                            
                            Ok((call_id, json!({ 
                                "status": "success",
                                "content": truncated,
                                "truncated": char_count >= 5000,
                                "url": url
                            })))
                        }
                        Err(e) => {
                            tracing::error!(
                                target: "ai_tools",
                                call_id = %call_id,
                                tool = "fetch_url",
                                error = %e,
                                "Failed to read response body"
                            );
                            Ok((call_id, json!({
                                "status": "error",
                                "code": "READ_ERROR",
                                "message": format!("Could not read response: {}", e)
                            })))
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        target: "ai_tools",
                        call_id = %call_id,
                        tool = "fetch_url",
                        url = url,
                        error = %e,
                        "HTTP request failed"
                    );
                    
                    let error_code = if e.is_timeout() {
                        "TIMEOUT"
                    } else if e.is_connect() {
                        "CONNECTION_FAILED"
                    } else {
                        "REQUEST_FAILED"
                    };
                    
                    Ok((call_id, json!({
                        "status": "error",
                        "code": error_code,
                        "message": format!("Request failed: {}", e)
                    })))
                }
            }
        }
        _ => {
            tracing::warn!(
                target: "ai_tools",
                call_id = %call_id,
                tool = name,
                "Unknown tool requested"
            );
            Err(AppError::InvalidInput(format!("Unknown tool: {}", name)))
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// VALIDATION HELPERS
// ═══════════════════════════════════════════════════════════════

/// Validate arguments for mutating tools before confirmation request.
/// This prevents wasting user's time on invalid operations.
fn validate_mutating_tool_args(tool_name: &str, args: &Value) -> Result<String, String> {
    match tool_name {
        "create_task" => {
            let title = args["title"].as_str().ok_or("Missing required field: title")?;
            if title.trim().is_empty() {
                return Err("Task title cannot be empty".into());
            }
            if title.len() > 100 {
                return Err("Task title must be ≤100 characters".into());
            }
            
            // Validate due_date format if provided
            if let Some(due_date) = args["due_date"].as_str() {
                NaiveDate::parse_from_str(due_date, "%Y-%m-%d")
                    .map_err(|_| "Invalid due_date format. Use ISO 8601 (YYYY-MM-DD)")?;
            }
            
            Ok(())
        }
        "update_task" => {
            let id = args["id"].as_str().ok_or("Missing required field: id")?;
            if id.len() < 6 {
                return Err("Task ID must be at least 6 characters".into());
            }
            
            // Validate due_date format if provided
            if let Some(due_date) = args["due_date"].as_str() {
                NaiveDate::parse_from_str(due_date, "%Y-%m-%d")
                    .map_err(|_| "Invalid due_date format. Use ISO 8601 (YYYY-MM-DD)")?;
            }
            
            // Ensure at least one field to update is provided
            if args.as_object().map(|o| o.len() <= 1).unwrap_or(true) {
                return Err("No fields provided to update".into());
            }
            
            Ok(())
        }
        "complete_task" => {
            let id = args["id"].as_str().ok_or("Missing required field: id")?;
            if id.len() < 6 {
                return Err("Task ID must be at least 6 characters".into());
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Validate list_tasks arguments
fn validate_list_tasks_args(args: &Value) -> Result<String, String> {
    // Validate date formats
    if let Some(due_before) = args["due_before"].as_str() {
        NaiveDate::parse_from_str(due_before, "%Y-%m-%d")
            .map_err(|_| "Invalid due_before format. Use ISO 8601 (YYYY-MM-DD)")?;
    }
    if let Some(due_after) = args["due_after"].as_str() {
        NaiveDate::parse_from_str(due_after, "%Y-%m-%d")
            .map_err(|_| "Invalid due_after format. Use ISO 8601 (YYYY-MM-DD)")?;
    }
    
    // Validate limit
    if let Some(limit) = args["limit"].as_i64() {
        if limit < 1 || limit > 200 {
            return Err("Limit must be between 1 and 200".into());
        }
    }
    
    Ok(())
}

/// Validate search_journal arguments
fn validate_search_journal_args(args: &Value) -> Result<String, String> {
    let query = args["query"].as_str().ok_or("Missing required field: query")?;
    if query.trim().len() < 3 {
        return Err("Search query must be at least 3 characters".into());
    }
    if query.len() > 100 {
        return Err("Search query must be ≤100 characters".into());
    }
    
    // Validate date formats
    if let Some(date_from) = args["date_from"].as_str() {
        NaiveDate::parse_from_str(date_from, "%Y-%m-%d")
            .map_err(|_| "Invalid date_from format. Use ISO 8601 (YYYY-MM-DD)")?;
    }
    if let Some(date_to) = args["date_to"].as_str() {
        NaiveDate::parse_from_str(date_to, "%Y-%m-%d")
            .map_err(|_| "Invalid date_to format. Use ISO 8601 (YYYY-MM-DD)")?;
    }
    
    Ok(())
}

/// Validate URL for fetch_url to prevent SSRF attacks
fn validate_url(url: &str) -> Result<String, String> {
    // Must start with https://
    if !url.starts_with("https://") {
        return Err("URL must use HTTPS protocol".into());
    }
    
    // Block localhost/private IPs (SSRF prevention)
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
    
    let url_lower = url.to_lowercase();
    for prefix in &blocked_prefixes {
        if url_lower.starts_with(prefix) {
            return Err("Cannot access local or private network URLs".into());
        }
    }
    
    // Max length check
    if url.len() > 2048 {
        return Err("URL must be ≤2048 characters".into());
    }
    
    Ok(())
}

/// Truncate URL for display purposes
fn truncate_url(url: &str) -> String {
    if url.len() > 50 {
        format!("{}...", &url[..47])
    } else {
        url.to_string()
    }
}