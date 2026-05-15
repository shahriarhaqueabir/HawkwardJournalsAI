use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use crate::error::AppError;
use chrono::Utc;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProfileFact {
    pub id: String,
    pub fact_key: String,
    pub content: String,
    pub category: String,
    pub confidence: f64,
    pub source_entry_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub fn upsert_fact(conn: &Connection, fact: &ProfileFact) -> Result<(), AppError> {
    let now = Utc::now().to_rfc3339();
    
    conn.execute(
        "INSERT INTO user_profile (id, fact_key, content, category, confidence, source_entry_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(fact_key) DO UPDATE SET
            content = excluded.content,
            category = excluded.category,
            confidence = excluded.confidence,
            source_entry_id = excluded.source_entry_id,
            updated_at = excluded.updated_at",
        params![
            fact.id,
            fact.fact_key,
            fact.content,
            fact.category,
            fact.confidence,
            fact.source_entry_id,
            fact.created_at,
            now
        ],
    )?;
    Ok(())
}

pub fn get_facts(conn: &Connection, category: Option<&str>) -> Result<Vec<ProfileFact>, AppError> {
    let mut stmt = if category.is_some() {
        conn.prepare("SELECT * FROM user_profile WHERE category = ?1 ORDER BY confidence DESC")?
    } else {
        conn.prepare("SELECT * FROM user_profile ORDER BY category, confidence DESC")?
    };

    let params = if let Some(cat) = category {
        vec![cat]
    } else {
        vec![]
    };

    let rows = stmt.query_map(rusqlite::params_from_iter(params), |row| {
        Ok(ProfileFact {
            id: row.get(0)?,
            fact_key: row.get(1)?,
            content: row.get(2)?,
            category: row.get(3)?,
            confidence: row.get(4)?,
            source_entry_id: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn delete_fact(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM user_profile WHERE id = ?1", params![id])?;
    Ok(())
}
