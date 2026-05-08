use crate::error::AppError;
use chrono::{Utc, Datelike};
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
    pub reminder_fired: bool,
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
    pub recurrence_rule: Option<String>,
    pub is_blocked: bool,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct TimeLog {
    pub id: String,
    pub task_id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration: Option<i32>,
    pub note: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct TaskAttachment {
    pub id: String,
    pub task_id: String,
    pub file_name: String,
    pub file_path: String,
    pub mime_type: Option<String>,
    pub size_bytes: Option<i32>,
    pub file_missing: bool,
    pub created_at: String,
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
        "SELECT *, 
         (SELECT COUNT(*) FROM task_dependencies d JOIN tasks bt ON d.blocking_task_id = bt.id WHERE d.blocked_task_id = tasks.id AND bt.status != 'done') > 0 as is_blocked
         FROM tasks WHERE is_deleted = 0 ORDER BY priority DESC, due_date ASC"
    } else {
        "SELECT *, 
         (SELECT COUNT(*) FROM task_dependencies d JOIN tasks bt ON d.blocking_task_id = bt.id WHERE d.blocked_task_id = tasks.id AND bt.status != 'done') > 0 as is_blocked
         FROM tasks WHERE is_deleted = 0 AND status != 'done' AND status != 'cancelled' ORDER BY priority DESC, due_date ASC"
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
            reminder_fired: row.get::<_, i32>(9)? != 0,
            time_estimate: row.get(10)?,
            time_logged: row.get(11)?,
            tags: row.get(13)?,
            labels: row.get(14)?,
            category: row.get(15)?,
            project: row.get(16)?,
            recurrence_rule: row.get(18)?,
            energy_level: row.get(20)?,
            context_tag: row.get(21)?,
            linked_url: row.get(22)?,
            ai_created: row.get::<_, i32>(24)? != 0,
            is_blocked: row.get::<_, i32>(row.column_count() - 1)? != 0,
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

    // Handle Recurrence (D-48)
    if status == "done" {
        if let Some(task) = get_task(conn, id)? {
            if let Some(rule) = task.recurrence_rule.clone() {
                let next_due = calculate_next_due(&task.due_date, &rule);
                if let Some(due) = next_due {
                    let mut new_task = task.clone();
                    new_task.id = Uuid::new_v4().to_string();
                    new_task.status = "todo".to_string();
                    new_task.due_date = Some(due);
                    new_task.completed_at = None;
                    new_task.created_at = now.clone();
                    new_task.updated_at = now;
                    create_task(conn, &new_task)?;
                }
            }
        }
    }

    Ok(())
}

fn calculate_next_due(current_due: &Option<String>, rule: &str) -> Option<String> {
    let base_date = if let Some(d) = current_due {
        chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()?
    } else {
        chrono::Local::now().date_naive()
    };

    let next = match rule.to_lowercase().as_str() {
        "daily" => base_date + chrono::Duration::days(1),
        "weekly" => base_date + chrono::Duration::weeks(1),
        "monthly" => {
            let month = base_date.month();
            let year = base_date.year();
            if month == 12 {
                chrono::NaiveDate::from_ymd_opt(year + 1, 1, base_date.day())
                    .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(year + 1, 1, 28).unwrap())
            } else {
                chrono::NaiveDate::from_ymd_opt(year, month + 1, base_date.day())
                    .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(year, month + 1, 28).unwrap())
            }
        },
        _ => return None,
    };

    Some(next.format("%Y-%m-%d").to_string())
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
    let mut stmt = conn.prepare(
        "SELECT *, 
         (SELECT COUNT(*) FROM task_dependencies d JOIN tasks bt ON d.blocking_task_id = bt.id WHERE d.blocked_task_id = tasks.id AND bt.status != 'done') > 0 as is_blocked
         FROM tasks WHERE id = ?1 AND is_deleted = 0"
    )?;
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
            reminder_fired: row.get::<_, i32>(9)? != 0,
            time_estimate: row.get(10)?,
            time_logged: row.get(11)?,
            tags: row.get(13)?,
            labels: row.get(14)?,
            category: row.get(15)?,
            project: row.get(16)?,
            recurrence_rule: row.get(18)?,
            energy_level: row.get(20)?,
            context_tag: row.get(21)?,
            linked_url: row.get(22)?,
            ai_created: row.get::<_, i32>(24)? != 0,
            is_blocked: row.get::<_, i32>(row.column_count() - 1)? != 0,
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
        "SELECT *, 
         (SELECT COUNT(*) FROM task_dependencies d JOIN tasks bt ON d.blocking_task_id = bt.id WHERE d.blocked_task_id = tasks.id AND bt.status != 'done') > 0 as is_blocked
         FROM tasks 
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
            reminder_fired: row.get::<_, i32>(9)? != 0,
            time_estimate: row.get(10)?,
            time_logged: row.get(11)?,
            tags: row.get(13)?,
            labels: row.get(14)?,
            category: row.get(15)?,
            project: row.get(16)?,
            recurrence_rule: row.get(18)?,
            energy_level: row.get(20)?,
            context_tag: row.get(21)?,
            linked_url: row.get(22)?,
            ai_created: row.get::<_, i32>(24)? != 0,
            is_blocked: row.get::<_, i32>(row.column_count() - 1)? != 0,
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
pub fn add_dependency(conn: &Connection, blocked_id: &str, blocking_id: &str) -> Result<(), AppError> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO task_dependencies (id, blocked_task_id, blocking_task_id, created_at) 
         VALUES (?1, ?2, ?3, ?4)",
        params![Uuid::new_v4().to_string(), blocked_id, blocking_id, now],
    )?;
    Ok(())
}

pub fn remove_dependency(conn: &Connection, blocked_id: &str, blocking_id: &str) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM task_dependencies WHERE blocked_task_id = ?1 AND blocking_task_id = ?2",
        params![blocked_id, blocking_id],
    )?;
    Ok(())
}

pub fn get_dependencies(conn: &Connection, task_id: &str) -> Result<Vec<Task>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT t.*, 
         (SELECT COUNT(*) FROM task_dependencies d2 JOIN tasks bt ON d2.blocking_task_id = bt.id WHERE d2.blocked_task_id = t.id AND bt.status != 'done') > 0 as is_blocked
         FROM tasks t
         JOIN task_dependencies d ON t.id = d.blocking_task_id
         WHERE d.blocked_task_id = ?1 AND t.is_deleted = 0"
    )?;
    
    let rows = stmt.query_map(params![task_id], |row| {
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
            reminder_fired: row.get::<_, i32>(9)? != 0,
            time_estimate: row.get(10)?,
            time_logged: row.get(11)?,
            tags: row.get(13)?,
            labels: row.get(14)?,
            category: row.get(15)?,
            project: row.get(16)?,
            recurrence_rule: row.get(18)?,
            energy_level: row.get(20)?,
            context_tag: row.get(21)?,
            linked_url: row.get(22)?,
            ai_created: row.get::<_, i32>(24)? != 0,
            is_blocked: row.get::<_, i32>(row.column_count() - 1)? != 0,
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

// ==========================================
// Phase 2 Placeholders: Timers
// ==========================================

pub fn timer_start(conn: &Connection, task_id: &str) -> Result<String, AppError> {
    let now = Utc::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO time_logs (id, task_id, started_at, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![id, task_id, now, now],
    )?;
    Ok(id)
}

pub fn timer_stop(conn: &Connection, log_id: &str, duration: i32, note: Option<&str>) -> Result<(), AppError> {
    let now = Utc::now().to_rfc3339();
    
    // Update the time_log
    conn.execute(
        "UPDATE time_logs SET ended_at = ?1, duration = ?2, note = ?3 WHERE id = ?4",
        params![now, duration, note, log_id],
    )?;
    
    // Also update the task time_logged
    conn.execute(
        "UPDATE tasks SET time_logged = time_logged + ?1 WHERE id = (SELECT task_id FROM time_logs WHERE id = ?2)",
        params![duration, log_id],
    )?;

    Ok(())
}

pub fn timer_get_logs(conn: &Connection, task_id: &str) -> Result<Vec<TimeLog>, AppError> {
    let mut stmt = conn.prepare("SELECT * FROM time_logs WHERE task_id = ?1 ORDER BY started_at DESC")?;
    let rows = stmt.query_map(params![task_id], |row| {
        Ok(TimeLog {
            id: row.get(0)?,
            task_id: row.get(1)?,
            started_at: row.get(2)?,
            ended_at: row.get(3)?,
            duration: row.get(4)?,
            note: row.get(5)?,
            created_at: row.get(6)?,
        })
    })?;

    let mut logs = Vec::new();
    for row in rows {
        logs.push(row?);
    }
    Ok(logs)
}

// ==========================================
// Phase 2 Placeholders: Attachments
// ==========================================

pub fn attachment_add(
    conn: &Connection,
    task_id: &str,
    file_name: &str,
    file_path: &str,
    mime_type: Option<&str>,
    size_bytes: Option<i32>,
) -> Result<String, AppError> {
    let now = Utc::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO task_attachments (id, task_id, file_name, file_path, mime_type, size_bytes, file_missing, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7)",
        params![id, task_id, file_name, file_path, mime_type, size_bytes, now],
    )?;
    Ok(id)
}

pub fn attachment_remove(conn: &Connection, attachment_id: &str) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM task_attachments WHERE id = ?1",
        params![attachment_id],
    )?;
    Ok(())
}

pub fn attachment_list(conn: &Connection, task_id: &str) -> Result<Vec<TaskAttachment>, AppError> {
    let mut stmt = conn.prepare("SELECT * FROM task_attachments WHERE task_id = ?1 ORDER BY created_at DESC")?;
    let rows = stmt.query_map(params![task_id], |row| {
        Ok(TaskAttachment {
            id: row.get(0)?,
            task_id: row.get(1)?,
            file_name: row.get(2)?,
            file_path: row.get(3)?,
            mime_type: row.get(4)?,
            size_bytes: row.get(5)?,
            file_missing: row.get::<_, i32>(6)? != 0,
            created_at: row.get(7)?,
        })
    })?;

    let mut attachments = Vec::new();
    for row in rows {
        attachments.push(row?);
    }
    Ok(attachments)
}
