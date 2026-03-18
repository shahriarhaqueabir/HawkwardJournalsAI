use crate::error::AppError;
use chrono::Utc;
use rusqlite::{params, Connection};

pub fn seed_defaults(conn: &Connection) -> Result<(), AppError> {
    let now = Utc::now().to_rfc3339();
    let defaults = [
        ("ai_model", "llama3.2:latest"),
        ("ai_context_window", "16384"), // D-93: Default raised to 16k
        ("ai_ollama_url", "http://localhost:11434"),
        ("theme", "dark"),
        ("weekly_review_last_run", ""), // D-109 tracking
    ];

    for (key, value) in defaults {
        conn.execute(
            "INSERT OR IGNORE INTO app_settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
            params![key, value, now],
        )?;
    }
    Ok(())
}
