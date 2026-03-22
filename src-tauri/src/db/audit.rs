use crate::error::AppError;
use rusqlite::{params, Connection};
use chrono::Utc;

pub fn log_action(
    conn: &Connection,
    action: &str,
    entity: &str,
    entity_id: &str,
    actor: &str,
    changes: Option<String>,
    ai_conversation_id: Option<String>,
) -> Result<(), AppError> {
    let now = Utc::now().to_rfc3339();
    
    conn.execute(
        "INSERT INTO audit_log (
            action, entity, entity_id, actor, changes, ai_conv_id, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            action,
            entity,
            entity_id,
            actor,
            changes,
            ai_conversation_id,
            now
        ],
    ).map_err(|e| AppError::Database(format!("Failed to write audit log: {}", e)))?;

    Ok(())
}
pub fn get_recent_logs(conn: &Connection, limit: u32) -> Result<Vec<AuditEntry>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, action, entity, entity_id, actor, changes, ai_conv_id, created_at 
         FROM audit_log ORDER BY id DESC LIMIT ?1"
    )?;
    let rows = stmt.query_map(params![limit], |row| {
        Ok(AuditEntry {
            id: row.get(0)?,
            action: row.get(1)?,
            entity: row.get(2)?,
            entity_id: row.get(3)?,
            actor: row.get(4)?,
            changes: row.get(5)?,
            ai_conversation_id: row.get(6)?,
            created_at: row.get(7)?,
        })
    })?;

    let mut logs = Vec::new();
    for log in rows {
        logs.push(log?);
    }
    Ok(logs)
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct AuditEntry {
    pub id: i64,
    pub action: String,
    pub entity: String,
    pub entity_id: String,
    pub actor: String,
    pub changes: Option<String>,
    pub ai_conversation_id: Option<String>,
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_audit_logging_and_retrieval() {
        let conn = Connection::open_in_memory().unwrap();
        // Create table manually for unit test (usually handled by migrations)
        conn.execute(
            "CREATE TABLE audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                action TEXT NOT NULL,
                entity TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                actor TEXT NOT NULL,
                changes TEXT,
                ai_conv_id TEXT,
                created_at TEXT NOT NULL
            )",
            [],
        ).unwrap();

        // Log one action
        log_action(&conn, "test_action", "test_entity", "123", "test_actor", Some("change".into()), None).unwrap();
        
        // Retrieve logs
        let logs = get_recent_logs(&conn, 10).unwrap();
        assert_eq!(logs.len(), 1) ;
        assert_eq!(logs[0].action, "test_action");
        assert_eq!(logs[0].actor, "test_actor");
    }
}
