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
    pub status: String,
    pub priority: String,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub reminder_at: Option<String>,
    pub recurrence: Option<String>,
    pub next_occurrence: Option<String>,
    pub time_estimate: Option<i32>,
    pub time_logged: i32,
    pub tags: String, // JSON array
    pub labels: String, // JSON array
    pub category: Option<String>,
    pub project: Option<String>, // Legacy text label
    pub project_id: Option<String>, // New FK to projects table
    pub energy_level: Option<String>,
    pub context_tag: Option<String>,
    pub linked_url: Option<String>,
    pub ai_created: bool,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
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

pub fn list_tasks(conn: &Connection, exclude_statuses: Vec<String>) -> Result<Vec<Task>, AppError> {
    let mut query = "SELECT * FROM tasks WHERE is_deleted = 0".to_string();
    
    if !exclude_statuses.is_empty() {
        let placeholders: Vec<String> = exclude_statuses.iter().map(|_| "?".to_string()).collect();
        query.push_str(&format!(" AND status NOT IN ({})", placeholders.join(",")));
    }
    
    query.push_str(" ORDER BY priority DESC, due_date ASC");

    let mut stmt = conn.prepare(&query)?;
    
    // Convert Vec<String> to Vec<Value> for params
    let params: Vec<rusqlite::types::Value> = exclude_statuses.into_iter().map(|s| rusqlite::types::Value::Text(s)).collect();
    let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|v| v as &dyn rusqlite::ToSql).collect();

    let rows = stmt.query_map(&*params_refs, |row| {
        Ok(Task {
            id: row.get(0)?,
            parent_task_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            status: row.get(4)?,
            priority: row.get(5)?,
            due_date: row.get(6)?,
            due_time: row.get(7)?,
            reminder_at: row.get(8)?,
            recurrence: row.get(21)?,
            next_occurrence: row.get(22)?,
            time_estimate: row.get(10)?,
            time_logged: row.get(11)?,
            tags: row.get(13)?,
            labels: row.get(14)?,
            category: row.get(15)?,
            project: row.get(16)?,
            project_id: row.get(17)?,
            energy_level: row.get(23)?,
            context_tag: row.get(24)?,
            linked_url: row.get(25)?,
            ai_created: row.get::<_, i32>(27)? != 0,
            created_at: row.get(31)?,
            updated_at: row.get(32)?,
            completed_at: row.get(33)?,
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
            id: row.get(0)?,
            parent_task_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            status: row.get(4)?,
            priority: row.get(5)?,
            due_date: row.get(6)?,
            due_time: row.get(7)?,
            reminder_at: row.get(8)?,
            recurrence: row.get(21)?,
            next_occurrence: row.get(22)?,
            time_estimate: row.get(10)?,
            time_logged: row.get(11)?,
            tags: row.get(13)?,
            labels: row.get(14)?,
            category: row.get(15)?,
            project: row.get(16)?,
            project_id: row.get(17)?,
            energy_level: row.get(23)?,
            context_tag: row.get(24)?,
            linked_url: row.get(25)?,
            ai_created: row.get::<_, i32>(27)? != 0,
            created_at: row.get(31)?,
            updated_at: row.get(32)?,
            completed_at: row.get(33)?,
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
            title = ?1, description = ?2, status = ?3, priority = ?4,
            due_date = ?5, due_time = ?6, reminder_at = ?7, recurrence = ?8,
            next_occurrence = ?9, time_estimate = ?10, category = ?11, 
            project = ?12, project_id = ?13, energy_level = ?14, context_tag = ?15,
            linked_url = ?16, updated_at = ?17
         WHERE id = ?18",
        params![
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
            task.category,
            task.project,
            task.project_id,
            task.energy_level,
            task.context_tag,
            task.linked_url,
            now,
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
            id: row.get(0)?,
            parent_task_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            status: row.get(4)?,
            priority: row.get(5)?,
            due_date: row.get(6)?,
            due_time: row.get(7)?,
            reminder_at: row.get(8)?,
            recurrence: row.get(21)?,
            next_occurrence: row.get(22)?,
            time_estimate: row.get(10)?,
            time_logged: row.get(11)?,
            tags: row.get(13)?,
            labels: row.get(14)?,
            category: row.get(15)?,
            project: row.get(16)?,
            project_id: row.get(17)?,
            energy_level: row.get(23)?,
            context_tag: row.get(24)?,
            linked_url: row.get(25)?,
            ai_created: row.get::<_, i32>(27)? != 0,
            created_at: row.get(31)?,
            updated_at: row.get(32)?,
            completed_at: row.get(33)?,
        })
    })?;

    let mut tasks = Vec::new();
    for task in rows {
        tasks.push(task?);
    }
    Ok(tasks)
}
