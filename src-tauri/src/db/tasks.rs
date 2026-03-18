use crate::error::AppError;
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    pub time_estimate: Option<i32>,
    pub time_logged: i32,
    pub tags: String, // JSON array
    pub labels: String, // JSON array
    pub category: Option<String>,
    pub project: Option<String>,
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
            due_date, due_time, reminder_at, time_estimate, time_logged,
            tags, labels, category, project, energy_level, context_tag, 
            linked_url, ai_created, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
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
            task.time_estimate,
            task.time_logged,
            task.tags,
            task.labels,
            task.category,
            task.project,
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

pub fn list_tasks(conn: &Connection, include_completed: bool) -> Result<Vec<Task>, AppError> {
    let query = if include_completed {
        "SELECT * FROM tasks WHERE is_deleted = 0 ORDER BY priority DESC, due_date ASC"
    } else {
        "SELECT * FROM tasks WHERE is_deleted = 0 AND status != 'done' AND status != 'cancelled' ORDER BY priority DESC, due_date ASC"
    };

    let mut stmt = conn.prepare(query)?;
    let rows = stmt.query_map([], |row| {
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
            time_estimate: row.get(10)?,
            time_logged: row.get(11)?,
            tags: row.get(13)?,
            labels: row.get(14)?,
            category: row.get(15)?,
            project: row.get(16)?,
            energy_level: row.get(20)?,
            context_tag: row.get(21)?,
            linked_url: row.get(22)?,
            ai_created: row.get::<_, i32>(24)? != 0,
            created_at: row.get(26)?,
            updated_at: row.get(27)?,
            completed_at: row.get(28)?,
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
            time_estimate: row.get(10)?,
            time_logged: row.get(11)?,
            tags: row.get(13)?,
            labels: row.get(14)?,
            category: row.get(15)?,
            project: row.get(16)?,
            energy_level: row.get(20)?,
            context_tag: row.get(21)?,
            linked_url: row.get(22)?,
            ai_created: row.get::<_, i32>(24)? != 0,
            created_at: row.get(26)?,
            updated_at: row.get(27)?,
            completed_at: row.get(28)?,
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
            due_date = ?5, due_time = ?6, reminder_at = ?7, time_estimate = ?8,
            category = ?9, project = ?10, energy_level = ?11, context_tag = ?12,
            linked_url = ?13, updated_at = ?14
         WHERE id = ?15",
        params![
            task.title,
            task.description,
            task.status,
            task.priority,
            task.due_date,
            task.due_time,
            task.reminder_at,
            task.time_estimate,
            task.category,
            task.project,
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
            time_estimate: row.get(10)?,
            time_logged: row.get(11)?,
            tags: row.get(13)?,
            labels: row.get(14)?,
            category: row.get(15)?,
            project: row.get(16)?,
            energy_level: row.get(20)?,
            context_tag: row.get(21)?,
            linked_url: row.get(22)?,
            ai_created: row.get::<_, i32>(24)? != 0,
            created_at: row.get(26)?,
            updated_at: row.get(27)?,
            completed_at: row.get(28)?,
        })
    })?;

    let mut tasks = Vec::new();
    for task in rows {
        tasks.push(task?);
    }
    Ok(tasks)
}
