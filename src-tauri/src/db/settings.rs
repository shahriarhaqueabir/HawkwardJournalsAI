use crate::error::AppError;
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct SettingItem {
    pub key: String,
    pub value: String,
    pub updated_at: String,
}

pub fn seed_defaults(conn: &Connection) -> Result<(), AppError> {
    let now = Utc::now().to_rfc3339();
    let defaults = [
        ("ai_model", "llama3.2:latest"),
        ("ai_context_window", "32768"), // Expanded to 32k for richer context
        ("ai_ollama_url", "http://localhost:11434"),
        ("theme", "dark"),
        ("terminal_visible", "false"),
        ("weekly_review_last_run", ""), // D-109 tracking
        ("ollama_models_dir", ""),      // D-108 override path
    ];

    for (key, value) in defaults {
        conn.execute(
            "INSERT OR IGNORE INTO app_settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
            params![key, value, now],
        )?;
    }
    Ok(())
}

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<SettingItem>, AppError> {
    let mut stmt =
        conn.prepare("SELECT key, value, updated_at FROM app_settings WHERE key = ?1")?;

    let row = stmt.query_row(params![key], |row| {
        Ok(SettingItem {
            key: row.get(0)?,
            value: row.get(1)?,
            updated_at: row.get(2)?,
        })
    });

    match row {
        Ok(setting) => Ok(Some(setting)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(AppError::Database(e.to_string())),
    }
}

pub fn list_settings(conn: &Connection) -> Result<Vec<SettingItem>, AppError> {
    let mut stmt =
        conn.prepare("SELECT key, value, updated_at FROM app_settings ORDER BY key ASC")?;

    let rows = stmt.query_map([], |row| {
        Ok(SettingItem {
            key: row.get(0)?,
            value: row.get(1)?,
            updated_at: row.get(2)?,
        })
    })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<SettingItem, AppError> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO app_settings (key, value, updated_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        params![key, value, now],
    )?;

    Ok(SettingItem {
        key: key.to_string(),
        value: value.to_string(),
        updated_at: now,
    })
}
