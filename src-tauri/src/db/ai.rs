use crate::error::AppError;
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Feedback {
    pub id: String,
    pub conversation_id: String,
    pub source_entry_id: Option<String>,
    pub action: String, // 'accepted', 'rejected', 'ignored'
    pub entity_type: String, // 'task', 'insight'
    pub entity_data: String,
    pub created_at: String,
}

pub fn log_ai_feedback(
    conn: &Connection,
    conv_id: &str,
    entry_id: Option<&str>,
    action: &str,
    entity_type: &str,
    data: &str,
) -> Result<(), AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO proposed_task_log (id, conversation_id, source_entry_id, outcome, proposed_title, proposed_data, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![id, conv_id, entry_id, action, entity_type, data, now],
    )?;
    Ok(())
}
