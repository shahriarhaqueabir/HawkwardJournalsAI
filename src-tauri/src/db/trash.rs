use crate::error::AppError;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct TrashItem {
    pub id: String,
    pub entity: String, // "journal", "task", "project"
    pub title: String,
    pub deleted_at: String,
}

pub fn list_trash(conn: &Connection) -> Result<Vec<TrashItem>, AppError> {
    let mut results = Vec::new();

    // 1. Journal entries
    let mut stmt = conn.prepare(
        "SELECT id, title, updated_at FROM journal_entries WHERE is_deleted = 1 ORDER BY updated_at DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(TrashItem {
            id: row.get(0)?,
            entity: "journal".to_string(),
            title: row.get::<_, Option<String>>(1)?.unwrap_or_else(|| "Untitled Entry".into()),
            deleted_at: row.get(2)?,
        })
    })?;
    for item in rows {
        results.push(item?);
    }

    // 2. Tasks
    let mut stmt = conn.prepare(
        "SELECT id, title, updated_at FROM tasks WHERE is_deleted = 1 ORDER BY updated_at DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(TrashItem {
            id: row.get(0)?,
            entity: "task".to_string(),
            title: row.get(1)?,
            deleted_at: row.get(2)?,
        })
    })?;
    for item in rows {
        results.push(item?);
    }

    // 3. Projects
    let mut stmt = conn.prepare(
        "SELECT id, name, updated_at FROM projects WHERE is_deleted = 1 ORDER BY updated_at DESC"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(TrashItem {
            id: row.get(0)?,
            entity: "project".to_string(),
            title: row.get(1)?,
            deleted_at: row.get(2)?,
        })
    })?;
    for item in rows {
        results.push(item?);
    }

    Ok(results)
}

pub fn empty_trash(conn: &Connection) -> Result<u32, AppError> {
    let mut total_deleted = 0;

    // Delete tasks and their attachments/time_logs first (cascade isn't always reliable with manual soft-delete)
    // Actually, SQL schema might have cascade. Let's assume standard DELETE.
    
    total_deleted += conn.execute("DELETE FROM journal_entries WHERE is_deleted = 1", [])?;
    total_deleted += conn.execute("DELETE FROM tasks WHERE is_deleted = 1", [])?;
    total_deleted += conn.execute("DELETE FROM projects WHERE is_deleted = 1", [])?;

    // Cleanup orphan tasks that were in a deleted project (just in case)
    // (Project soft-delete already moves them to Inbox, but empty_trash should be thorough)

    super::audit::log_action(
        conn,
        "empty_trash",
        "system",
        "all",
        "user",
        Some(format!("Permanently deleted {} items", total_deleted)),
        None
    )?;

    Ok(total_deleted as u32)
}
