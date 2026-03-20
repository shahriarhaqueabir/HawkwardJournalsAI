use crate::error::AppError;
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct Task {
    pub id: String,
    pub parent_task_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub notes: Option<String>,
    pub status: String,
    pub priority: String,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub reminder_at: Option<String>,
    pub reminder_fired: bool,
    pub recurrence: Option<String>,
    pub next_occurrence: Option<String>,
    pub time_estimate: Option<i32>,
    pub time_logged: i32,
    pub actual_start_date: Option<String>,
    pub tags: String,
    pub labels: String,
    pub category: Option<String>,
    pub project: Option<String>,
    pub project_id: Option<String>,
    pub energy_level: Option<String>,
    pub context_tag: Option<String>,
    pub linked_url: Option<String>,
    pub sort_order: i32,
    pub ai_created: bool,
    pub ai_conversation_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
#[serde(rename_all = "snake_case")]
pub struct TaskListFilters {
    pub statuses: Option<Vec<String>>,
    pub exclude_statuses: Option<Vec<String>>,
    pub priorities: Option<Vec<String>>,
    pub project_id: Option<String>,
    pub category: Option<String>,
    pub energy_levels: Option<Vec<String>>,
    pub context_tags: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub due_before: Option<String>,
    pub due_after: Option<String>,
}

pub fn create_task(conn: &Connection, task: &Task) -> Result<String, AppError> {
    conn.execute(
        "INSERT INTO tasks (
            id, parent_task_id, title, description, status, priority, 
            due_date, due_time, reminder_at, recurrence, next_occurrence,
            time_estimate, time_logged, tags, labels, category, project, project_id,
            energy_level, context_tag, linked_url, ai_created, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)",
        params![
            task.id,
            task.parent_task_id,
            task.title,
            task.description,
            task.status,
            task.priority,
            task.due_date,
            task.due_time,
            task.reminder_at,
            task.recurrence,
            task.next_occurrence,
            task.time_estimate,
            task.time_logged,
            task.tags,
            task.labels,
            task.category,
            task.project,
            task.project_id,
            task.energy_level,
            task.context_tag,
            task.linked_url,
            task.ai_created as i32,
            task.created_at,
            task.updated_at,
        ],
    )?;
    Ok(task.id.clone())
}

pub fn list_tasks(conn: &Connection, filters: TaskListFilters) -> Result<Vec<Task>, AppError> {
    let mut query = "SELECT * FROM tasks WHERE is_deleted = 0".to_string();
    let mut params_vals: Vec<rusqlite::types::Value> = Vec::new();

    if let Some(statuses) = filters.statuses {
        if !statuses.is_empty() {
            let placeholders = statuses.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            query.push_str(&format!(" AND status IN ({})", placeholders));
            for s in statuses { params_vals.push(rusqlite::types::Value::Text(s)); }
        }
    }

    if let Some(exclude) = filters.exclude_statuses {
        if !exclude.is_empty() {
            let placeholders = exclude.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            query.push_str(&format!(" AND status NOT IN ({})", placeholders));
            for s in exclude { params_vals.push(rusqlite::types::Value::Text(s)); }
        }
    }

    if let Some(priorities) = filters.priorities {
        if !priorities.is_empty() {
            let placeholders = priorities.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            query.push_str(&format!(" AND priority IN ({})", placeholders));
            for p in priorities { params_vals.push(rusqlite::types::Value::Text(p)); }
        }
    }

    if let Some(pid) = filters.project_id {
        query.push_str(" AND project_id = ?");
        params_vals.push(rusqlite::types::Value::Text(pid));
    }

    if let Some(cat) = filters.category {
        query.push_str(" AND category = ?");
        params_vals.push(rusqlite::types::Value::Text(cat));
    }

    if let Some(energy) = filters.energy_levels {
        if !energy.is_empty() {
            let placeholders = energy.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            query.push_str(&format!(" AND energy_level IN ({})", placeholders));
            for e in energy { params_vals.push(rusqlite::types::Value::Text(e)); }
        }
    }

    if let Some(context) = filters.context_tags {
        if !context.is_empty() {
            let placeholders = context.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            query.push_str(&format!(" AND context_tag IN ({})", placeholders));
            for c in context { params_vals.push(rusqlite::types::Value::Text(c)); }
        }
    }

    if let Some(before) = filters.due_before {
        query.push_str(" AND due_date <= ?");
        params_vals.push(rusqlite::types::Value::Text(before));
    }

    if let Some(after) = filters.due_after {
        query.push_str(" AND due_date >= ?");
        params_vals.push(rusqlite::types::Value::Text(after));
    }

    if let Some(tags_list) = filters.tags {
        for t in tags_list {
            query.push_str(" AND EXISTS (SELECT 1 FROM json_each(tags) WHERE value = ?)");
            params_vals.push(rusqlite::types::Value::Text(t));
        }
    }

    query.push_str(" ORDER BY priority DESC, due_date ASC");

    let mut stmt = conn.prepare(&query)?;
    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vals.iter().map(|v| v as &dyn rusqlite::ToSql).collect();

    let rows = stmt.query_map(&*params_refs, |row| {
        Ok(Task {
            id: row.get("id")?,
            parent_task_id: row.get("parent_task_id")?,
            title: row.get("title")?,
            description: row.get("description")?,
            notes: row.get("notes")?,
            status: row.get("status")?,
            priority: row.get("priority")?,
            due_date: row.get("due_date")?,
            due_time: row.get("due_time")?,
            reminder_at: row.get("reminder_at")?,
            reminder_fired: row.get::<_, i32>("reminder_fired")? != 0,
            recurrence: row.get("recurrence")?,
            next_occurrence: row.get("next_occurrence")?,
            time_estimate: row.get("time_estimate")?,
            time_logged: row.get("time_logged")?,
            actual_start_date: row.get("actual_start_date")?,
            tags: row.get("tags")?,
            labels: row.get("labels")?,
            category: row.get("category")?,
            project: row.get("project")?,
            project_id: row.get("project_id")?,
            energy_level: row.get("energy_level")?,
            context_tag: row.get("context_tag")?,
            linked_url: row.get("linked_url")?,
            sort_order: row.get("sort_order")?,
            ai_created: row.get::<_, i32>("ai_created")? != 0,
            ai_conversation_id: row.get("ai_conversation_id")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            completed_at: row.get("completed_at")?,
        })
    })?;

    let mut tasks = Vec::new();
    for task in rows {
        tasks.push(task?);
    }
    Ok(tasks)
}

pub fn update_task_status(conn: &Connection, id: &str, status: &str) -> Result<(), AppError> {
    let now = Utc::now().to_rfc3339();
    let completed_at = if status == "done" { Some(now.clone()) } else { None };
    
    conn.execute(
        "UPDATE tasks SET status = ?1, updated_at = ?2, completed_at = ?3 WHERE id = ?4",
        params![status, now, completed_at, id],
    )?;
    Ok(())
}

pub fn soft_delete(conn: &Connection, id: &str) -> Result<bool, AppError> {
    let now = Utc::now().to_rfc3339();
    let result = conn.execute(
        "UPDATE tasks SET is_deleted = 1, updated_at = ?1 WHERE id = ?2 OR parent_task_id = ?2",
        params![now, id],
    )?;
    Ok(result > 0)
}

pub fn get_task(conn: &Connection, id: &str) -> Result<Option<Task>, AppError> {
    let mut stmt = conn.prepare("SELECT * FROM tasks WHERE id = ?1 AND is_deleted = 0")?;
    let task = stmt.query_row(params![id], |row| {
        Ok(Task {
            id: row.get("id")?,
            parent_task_id: row.get("parent_task_id")?,
            title: row.get("title")?,
            description: row.get("description")?,
            notes: row.get("notes")?,
            status: row.get("status")?,
            priority: row.get("priority")?,
            due_date: row.get("due_date")?,
            due_time: row.get("due_time")?,
            reminder_at: row.get("reminder_at")?,
            reminder_fired: row.get::<_, i32>("reminder_fired")? != 0,
            recurrence: row.get("recurrence")?,
            next_occurrence: row.get("next_occurrence")?,
            time_estimate: row.get("time_estimate")?,
            time_logged: row.get("time_logged")?,
            actual_start_date: row.get("actual_start_date")?,
            tags: row.get("tags")?,
            labels: row.get("labels")?,
            category: row.get("category")?,
            project: row.get("project")?,
            project_id: row.get("project_id")?,
            energy_level: row.get("energy_level")?,
            context_tag: row.get("context_tag")?,
            linked_url: row.get("linked_url")?,
            sort_order: row.get("sort_order")?,
            ai_created: row.get::<_, i32>("ai_created")? != 0,
            ai_conversation_id: row.get("ai_conversation_id")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            completed_at: row.get("completed_at")?,
        })
    });

    match task {
        Ok(t) => Ok(Some(t)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(AppError::Database(e.to_string())),
    }
}

pub fn update_task(conn: &Connection, task: &Task) -> Result<(), AppError> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE tasks SET 
            title = ?1, description = ?2, notes = ?3, status = ?4, priority = ?5,
            due_date = ?6, due_time = ?7, reminder_at = ?8, reminder_fired = ?9, recurrence = ?10,
            next_occurrence = ?11, time_estimate = ?12, time_logged = ?13, actual_start_date = ?14,
            category = ?15, project = ?16, project_id = ?17, energy_level = ?18, 
            context_tag = ?19, linked_url = ?20, sort_order = ?21, 
            ai_conversation_id = ?22, updated_at = ?23, completed_at = ?24
         WHERE id = ?25",
        params![
            task.title,
            task.description,
            task.notes,
            task.status,
            task.priority,
            task.due_date,
            task.due_time,
            task.reminder_at,
            if task.reminder_fired { 1 } else { 0 },
            task.recurrence,
            task.next_occurrence,
            task.time_estimate,
            task.time_logged,
            task.actual_start_date,
            task.category,
            task.project,
            task.project_id,
            task.energy_level,
            task.context_tag,
            task.linked_url,
            task.sort_order,
            task.ai_conversation_id,
            now,
            task.completed_at,
            task.id,
        ],
    )?;
    Ok(())
}

pub fn search_tasks(conn: &Connection, query: &str) -> Result<Vec<Task>, AppError> {
    let sql_query = format!("%{}%", query);
    let mut stmt = conn.prepare(
        "SELECT * FROM tasks 
         WHERE is_deleted = 0 AND (title LIKE ?1 OR description LIKE ?1 OR project LIKE ?1)
         ORDER BY priority DESC, created_at DESC"
    )?;
    
    let rows = stmt.query_map(params![sql_query], |row| {
        Ok(Task {
            id: row.get("id")?,
            parent_task_id: row.get("parent_task_id")?,
            title: row.get("title")?,
            description: row.get("description")?,
            notes: row.get("notes")?,
            status: row.get("status")?,
            priority: row.get("priority")?,
            due_date: row.get("due_date")?,
            due_time: row.get("due_time")?,
            reminder_at: row.get("reminder_at")?,
            reminder_fired: row.get::<_, i32>("reminder_fired")? != 0,
            recurrence: row.get("recurrence")?,
            next_occurrence: row.get("next_occurrence")?,
            time_estimate: row.get("time_estimate")?,
            time_logged: row.get("time_logged")?,
            actual_start_date: row.get("actual_start_date")?,
            tags: row.get("tags")?,
            labels: row.get("labels")?,
            category: row.get("category")?,
            project: row.get("project")?,
            project_id: row.get("project_id")?,
            energy_level: row.get("energy_level")?,
            context_tag: row.get("context_tag")?,
            linked_url: row.get("linked_url")?,
            sort_order: row.get("sort_order")?,
            ai_created: row.get::<_, i32>("ai_created")? != 0,
            ai_conversation_id: row.get("ai_conversation_id")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            completed_at: row.get("completed_at")?,
        })
    })?;

    let mut tasks = Vec::new();
    for task in rows {
        tasks.push(task?);
    }
    Ok(tasks)
}

pub fn timer_start(conn: &Connection, task_id: &str) -> Result<String, AppError> {
    let task = get_task(conn, task_id)?.ok_or_else(|| AppError::InvalidInput("Task not found".into()))?;
    if task.status == "done" || task.status == "cancelled" {
        return Err(AppError::InvalidInput("Cannot start timer on completed or cancelled task".into()));
    }
    
    let mut stmt = conn.prepare("SELECT id FROM time_logs WHERE task_id = ?1 AND ended_at IS NULL")?;
    let open_timer = match stmt.query_row(rusqlite::params![task_id], |row| row.get::<_, String>(0)) {
        Ok(id) => Some(id),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => return Err(AppError::Database(e.to_string())),
    };

    if open_timer.is_some() {
        return Err(AppError::InvalidInput("Task already has an open timer".into()));
    }

    let now = Utc::now().to_rfc3339();
    let log_id = uuid::Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO time_logs (id, task_id, started_at, created_at) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![log_id, task_id, now, now],
    )?;

    if task.status == "todo" {
        update_task_status(conn, task_id, "in_progress")?;
    }

    Ok(log_id)
}

pub fn timer_stop(conn: &Connection, task_id: &str) -> Result<(), AppError> {
    let mut stmt = conn.prepare("SELECT id, started_at FROM time_logs WHERE task_id = ?1 AND ended_at IS NULL")?;
    
    let timer_info = stmt.query_row(rusqlite::params![task_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    });

    let (log_id, started_at) = match timer_info {
        Ok(info) => info,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Err(AppError::InvalidInput("No open timer found for this task".into())),
        Err(e) => return Err(AppError::Database(e.to_string())),
    };

    let now = Utc::now();
    let started = chrono::DateTime::parse_from_rfc3339(&started_at)
        .map_err(|e| AppError::InvalidInput(e.to_string()))?.with_timezone(&Utc);
    
    let duration_secs = (now - started).num_seconds() as i32;

    conn.execute(
        "UPDATE time_logs SET ended_at = ?1, duration = ?2 WHERE id = ?3",
        rusqlite::params![now.to_rfc3339(), duration_secs, log_id],
    )?;

    conn.execute(
        "UPDATE tasks SET time_logged = time_logged + ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![duration_secs, now.to_rfc3339(), task_id],
    )?;

    Ok(())
}

pub fn task_add_dependency(conn: &Connection, blocked_task_id: &str, blocking_task_id: &str) -> Result<(), AppError> {
    if blocked_task_id == blocking_task_id {
        return Err(AppError::InvalidInput("A task cannot depend on itself".into()));
    }

    let now = Utc::now().to_rfc3339();
    let dep_id = uuid::Uuid::new_v4().to_string();

    let mut stmt = conn.prepare("SELECT id FROM task_dependencies WHERE blocked_task_id = ?1 AND blocking_task_id = ?2")?;
    let circular_exists = stmt.exists(rusqlite::params![blocking_task_id, blocked_task_id])?;
    if circular_exists {
        return Err(AppError::InvalidInput("Circular dependency detected".into()));
    }

    let count: i64 = conn.query_row("SELECT COUNT(*) FROM task_dependencies WHERE blocked_task_id = ?1", rusqlite::params![blocked_task_id], |row| row.get(0))?;
    if count >= 10 {
        return Err(AppError::InvalidInput("Maximum 10 dependencies allowed per task".into()));
    }

    conn.execute(
        "INSERT INTO task_dependencies (id, blocked_task_id, blocking_task_id, created_at) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![dep_id, blocked_task_id, blocking_task_id, now],
    )?;

    Ok(())
}

pub fn task_remove_dependency(conn: &Connection, blocked_task_id: &str, blocking_task_id: &str) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM task_dependencies WHERE blocked_task_id = ?1 AND blocking_task_id = ?2",
        rusqlite::params![blocked_task_id, blocking_task_id],
    )?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct TaskAttachment {
    pub id: String,
    pub task_id: String,
    pub file_name: String,
    pub file_path: String,
    pub mime_type: Option<String>,
    pub size_bytes: Option<i64>,
    pub file_missing: i32,
    pub created_at: String,
}

pub fn attachment_add(
    conn: &Connection,
    task_id: &str,
    source_path_str: &str,
) -> Result<TaskAttachment, AppError> {
    let source_path = std::path::Path::new(source_path_str);
    let id = uuid::Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    
    // Ensure attachments directory exists
    let data_dir = crate::db::paths::resolve_data_dir();
    let attach_dir = data_dir.join("attachments");
    if !attach_dir.exists() {
        std::fs::create_dir_all(&attach_dir)
            .map_err(|e| AppError::Io(format!("Failed to create attachments dir: {}", e)))?;
    }
    
    let original_name = source_path.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| format!("attachment_{}.bin", id));
        
    let new_file_name = format!("{}_{}", id, original_name);
    let dest_path = attach_dir.join(&new_file_name);
    
    std::fs::copy(source_path, &dest_path)
        .map_err(|e| AppError::Io(format!("Failed to copy attachment: {}", e)))?;
    
    let size_bytes = std::fs::metadata(&dest_path).map(|m| m.len() as i64).ok();
    
    let mime_type = source_path.extension()
        .map(|e| e.to_string_lossy().to_lowercase());
    
    let relative_path = format!("attachments/{}", new_file_name); // D-47
    
    conn.execute(
        "INSERT INTO task_attachments (id, task_id, file_name, file_path, mime_type, size_bytes, file_missing, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7)",
         rusqlite::params![id, task_id, original_name, relative_path, mime_type, size_bytes, created_at]
    )?;
    
    Ok(TaskAttachment {
        id,
        task_id: task_id.to_string(),
        file_name: original_name,
        file_path: relative_path,
        mime_type,
        size_bytes,
        file_missing: 0,
        created_at,
    })
}

pub fn attachment_list(conn: &Connection, task_id: &str) -> Result<Vec<TaskAttachment>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, task_id, file_name, file_path, mime_type, size_bytes, file_missing, created_at 
         FROM task_attachments WHERE task_id = ?1 ORDER BY created_at DESC"
    )?;
    
    let rows = stmt.query_map(rusqlite::params![task_id], |row| {
        Ok(TaskAttachment {
            id: row.get(0)?,
            task_id: row.get(1)?,
            file_name: row.get(2)?,
            file_path: row.get(3)?,
            mime_type: row.get(4)?,
            size_bytes: row.get(5)?,
            file_missing: row.get(6)?,
            created_at: row.get(7)?,
        })
    })?;
    
    let mut results = Vec::new();
    let data_dir = crate::db::paths::resolve_data_dir();
    
    for row in rows {
        let mut att = row?;
        let absolute_path = data_dir.join(&att.file_path);
        let actual_missing = if absolute_path.exists() { 0 } else { 1 };
        
        if att.file_missing != actual_missing {
            let _ = conn.execute(
                "UPDATE task_attachments SET file_missing = ?1 WHERE id = ?2", 
                rusqlite::params![actual_missing, att.id]
            );
            att.file_missing = actual_missing;
        }
        results.push(att);
    }
    
    Ok(results)
}

pub fn attachment_remove(conn: &Connection, id: &str) -> Result<(), AppError> {
    let mut stmt = conn.prepare("SELECT file_path FROM task_attachments WHERE id = ?1")?;
    let path: Option<String> = stmt.query_row(rusqlite::params![id], |r| r.get(0)).ok();
    
    if let Some(rel_path) = path {
        let absolute_path = crate::db::paths::resolve_data_dir().join(rel_path);
        if absolute_path.exists() {
            let _ = std::fs::remove_file(absolute_path);
        }
    }
    
    conn.execute("DELETE FROM task_attachments WHERE id = ?1", rusqlite::params![id])?;
    Ok(())
}

use tauri::Manager;

pub async fn execute_ai_tool(app: &tauri::AppHandle, name: &str, args: serde_json::Value) -> Result<serde_json::Value, AppError> {
    let conn_arc = app.state::<crate::AppState>().conn.clone();
    
    match name {
        "create_task" => {
            let title = args["title"].as_str().ok_or_else(|| AppError::InvalidInput("Missing title".into()))?.to_string();
            let priority = args["priority"].as_str().map(|s| s.to_string());
            let due_date = args["due_date"].as_str().map(|s| s.to_string());
            let project_id = args["project_id"].as_str().map(|s| s.to_string());
            let energy_level = args["energy_level"].as_str().map(|s| s.to_string());
            let context_tag = args["context_tag"].as_str().map(|s| s.to_string());

            let now = Utc::now().to_rfc3339();
            let task = Task {
                id: uuid::Uuid::new_v4().to_string(),
                parent_task_id: None,
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
                energy_level,
                context_tag,
                linked_url: None,
                recurrence: None,
                next_occurrence: None,
                sort_order: 0,
                ai_created: true,
                ai_conversation_id: None,
                created_at: now.clone(),
                updated_at: now,
                completed_at: None,
            };

            let conn = conn_arc.lock().await;
            create_task(&conn, &task)?;
            
            crate::events::emit(app, crate::events::AppEvent::TaskCreated { 
                id: task.id.clone(), 
                title: task.title.clone() 
            });

            Ok(serde_json::to_value(task).unwrap())
        }
        "update_task" => {
            let id = args["id"].as_str().ok_or_else(|| AppError::InvalidInput("Missing task ID".into()))?.to_string();
            let conn = conn_arc.lock().await;
            
            let mut task = get_task(&conn, &id)?.ok_or_else(|| AppError::InvalidInput("Task not found".into()))?;
            
            if let Some(t) = args["title"].as_str() { task.title = t.to_string(); }
            if let Some(s) = args["status"].as_str() { 
                task.status = s.to_string(); 
                if task.status == "done" {
                    task.completed_at = Some(Utc::now().to_rfc3339());
                } else {
                    task.completed_at = None;
                }
            }
            if let Some(p) = args["priority"].as_str() { task.priority = p.to_string(); }
            if let Some(d) = args["due_date"].as_str() { task.due_date = Some(d.to_string()); }
            
            task.updated_at = Utc::now().to_rfc3339();
            update_task(&conn, &task)?;

            crate::events::emit(app, crate::events::AppEvent::TaskUpdated { id: id.clone() });
            Ok(serde_json::to_value(task).unwrap())
        }
        "complete_task" => {
            let id = args["id"].as_str().ok_or_else(|| AppError::InvalidInput("Missing task ID".into()))?.to_string();
            update_task_status_internal(app, &id, "done").await?;
            Ok(serde_json::json!({ "status": "success", "id": id }))
        }
        _ => Err(AppError::InvalidInput(format!("Tool {} not supported in task executor", name)))
    }
}

async fn update_task_status_internal(app: &tauri::AppHandle, id: &str, status: &str) -> Result<(), AppError> {
    let conn_arc = app.state::<crate::AppState>().conn.clone();
    let conn = conn_arc.lock().await;
    update_task_status(&conn, id, status)?;
    if status == "done" {
        crate::events::emit(app, crate::events::AppEvent::TaskCompleted { id: id.to_string() });
    } else {
        crate::events::emit(app, crate::events::AppEvent::TaskUpdated { id: id.to_string() });
    }
    Ok(())
}
