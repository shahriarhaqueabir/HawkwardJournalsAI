use crate::error::AppError;
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AiConversation {
    pub id: String,
    pub title: Option<String>,
    pub model: String,
    pub source: String, // 'sidebar', 'ai_tab', 'analysis', 'weekly_plan'
    pub source_entry_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub is_deleted: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AiMessage {
    pub id: String,
    pub conversation_id: String,
    pub role: String, // 'user', 'assistant', 'tool', 'system'
    pub content: String,
    pub tool_name: Option<String>,
    pub tool_args: Option<String>,
    pub tool_result: Option<String>,
    pub confirmed: Option<i32>, // NULL=N/A, 1=confirmed, 0=cancelled
    pub model: Option<String>,
    pub created_at: String,
}

pub fn create_conversation(
    conn: &Connection,
    source: &str,
    entry_id: Option<&str>,
) -> Result<String, AppError> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    
    conn.execute(
        "INSERT INTO ai_conversations (id, title, model, source, source_entry_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![id, "New Chat", "llama3.2", source, entry_id, now, now],
    )?;
    
    Ok(id)
}

pub fn list_conversations(
    conn: &Connection,
    source_filter: Option<&str>,
) -> Result<Vec<AiConversation>, AppError> {
    let mut query = "SELECT id, title, model, source, source_entry_id, created_at, updated_at, is_deleted 
                     FROM ai_conversations WHERE is_deleted = 0".to_string();
    
    if source_filter.is_some() {
        query.push_str(" AND source = ?1");
    }
    query.push_str(" ORDER BY updated_at DESC");

    let mut stmt = conn.prepare(&query)?;
    let rows = if let Some(s) = source_filter {
        stmt.query_map(params![s], map_conversation)?
    } else {
        stmt.query_map([], map_conversation)?
    };

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

fn map_conversation(row: &rusqlite::Row) -> rusqlite::Result<AiConversation> {
    Ok(AiConversation {
        id: row.get(0)?,
        title: row.get(1)?,
        model: row.get(2)?,
        source: row.get(3)?,
        source_entry_id: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
        is_deleted: row.get::<_, i32>(7)? != 0,
    })
}

pub fn add_message(
    conn: &Connection,
    msg: &AiMessage,
) -> Result<(), AppError> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO ai_messages (id, conversation_id, role, content, tool_name, tool_args, tool_result, confirmed, model, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            msg.id, msg.conversation_id, msg.role, msg.content, 
            msg.tool_name, msg.tool_args, msg.tool_result, 
            msg.confirmed, msg.model, now
        ],
    )?;
    
    // Update conversation timestamp
    conn.execute(
        "UPDATE ai_conversations SET updated_at = ?1 WHERE id = ?2",
        params![now, msg.conversation_id],
    )?;
    
    Ok(())
}

pub fn get_messages(
    conn: &Connection,
    conversation_id: &str,
) -> Result<Vec<AiMessage>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, conversation_id, role, content, tool_name, tool_args, tool_result, confirmed, model, created_at
         FROM ai_messages WHERE conversation_id = ?1 ORDER BY created_at ASC",
    )?;
    
    let rows = stmt.query_map(params![conversation_id], |row| {
        Ok(AiMessage {
            id: row.get(0)?,
            conversation_id: row.get(1)?,
            role: row.get(2)?,
            content: row.get(3)?,
            tool_name: row.get(4)?,
            tool_args: row.get(5)?,
            tool_result: row.get(6)?,
            confirmed: row.get(7)?,
            model: row.get(8)?,
            created_at: row.get(9)?,
        })
    })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

pub fn soft_delete_conversation(conn: &Connection, id: &str) -> Result<bool, AppError> {
    let rows = conn.execute(
        "UPDATE ai_conversations SET is_deleted = 1 WHERE id = ?1",
        params![id],
    )?;
    Ok(rows > 0)
}

pub fn log_ai_feedback(
    conn: &Connection,
    conv_id: &str,
    entry_id: Option<&str>,
    action: &str,
    entity_type: &str,
    data: &str,
) -> Result<(), AppError> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO proposed_task_log (id, conversation_id, source_entry_id, outcome, proposed_title, proposed_data, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![id, conv_id, entry_id, action, entity_type, data, now],
    )?;
    Ok(())
}
