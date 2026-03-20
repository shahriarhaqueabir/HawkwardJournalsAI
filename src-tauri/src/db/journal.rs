use crate::error::AppError;
use chrono::Utc;
use rusqlite::{params, named_params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JournalEntry {
    pub id: String,
    pub title: Option<String>,
    pub content: String,
    pub analysis_summary: Option<String>,
    pub analysis_mood: Option<String>,
    pub analysis_insights: String,
    pub emotions: String,
    pub tags: String,
    pub last_analysis_conv_id: Option<String>,
    pub last_analysed_at: Option<String>,
    pub word_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

pub fn upsert_entry(conn: &Connection, entry: &JournalEntry) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO journal_entries (id, title, content, analysis_summary, analysis_mood, analysis_insights, emotions, tags, last_analysis_conv_id, last_analysed_at, word_count, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
         ON CONFLICT(id) DO UPDATE SET
            title = excluded.title,
            content = excluded.content,
            analysis_summary = excluded.analysis_summary,
            analysis_mood = excluded.analysis_mood,
            analysis_insights = excluded.analysis_insights,
            emotions = excluded.emotions,
            tags = excluded.tags,
            last_analysis_conv_id = excluded.last_analysis_conv_id,
            last_analysed_at = excluded.last_analysed_at,
            word_count = excluded.word_count,
            updated_at = excluded.updated_at",
        params![
            entry.id,
            entry.title,
            entry.content,
            entry.analysis_summary,
            entry.analysis_mood,
            entry.analysis_insights,
            entry.emotions,
            entry.tags,
            entry.last_analysis_conv_id,
            entry.last_analysed_at,
            entry.word_count,
            entry.created_at,
            entry.updated_at
        ],
    )?;
    Ok(())
}

pub fn get_entry(conn: &Connection, id: &str) -> Result<Option<JournalEntry>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, title, content, analysis_summary, analysis_mood, analysis_insights, emotions, tags, last_analysis_conv_id, last_analysed_at, word_count, created_at, updated_at
         FROM journal_entries
         WHERE id = ?1 AND is_deleted = 0",
    )?;

    let entry = stmt.query_row(params![id], |row| {
        Ok(JournalEntry {
            id: row.get(0)?,
            title: row.get(1)?,
            content: row.get(2)?,
            analysis_summary: row.get(3)?,
            analysis_mood: row.get(4)?,
            analysis_insights: row.get(5)?,
            emotions: row.get(6)?,
            tags: row.get(7)?,
            last_analysis_conv_id: row.get(8)?,
            last_analysed_at: row.get(9)?,
            word_count: row.get(10)?,
            created_at: row.get(11)?,
            updated_at: row.get(12)?,
        })
    });

    match entry {
        Ok(e) => Ok(Some(e)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(AppError::Database(e.to_string())),
    }
}

pub fn get_entry_by_id(conn: &Connection, id: &str) -> Result<Option<JournalEntry>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, title, content, analysis_summary, analysis_mood, analysis_insights, emotions, tags, last_analysis_conv_id, last_analysed_at, word_count, created_at, updated_at 
         FROM journal_entries WHERE id = ?1 AND is_deleted = 0"
    )?;
    let entry = stmt.query_row(params![id], |row| {
        Ok(JournalEntry {
            id: row.get(0)?,
            title: row.get(1)?,
            content: row.get(2)?,
            analysis_summary: row.get(3)?,
            analysis_mood: row.get(4)?,
            analysis_insights: row.get(5)?,
            emotions: row.get(6)?,
            tags: row.get(7)?,
            last_analysis_conv_id: row.get(8)?,
            last_analysed_at: row.get(9)?,
            word_count: row.get(10)?,
            created_at: row.get(11)?,
            updated_at: row.get(12)?,
        })
    });

    match entry {
        Ok(e) => Ok(Some(e)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(AppError::Database(e.to_string())),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JournalEntrySummary {
    pub id: String,
    pub title: Option<String>,
    pub word_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

pub fn list_entries(
    conn: &Connection,
    cursor: Option<&str>,
    limit: u32,
) -> Result<Vec<JournalEntrySummary>, AppError> {
    let mut result = Vec::new();

    if let Some(c) = cursor {
        let mut stmt = conn.prepare(
            "SELECT id, title, word_count, created_at, updated_at
             FROM journal_entries
             WHERE is_deleted = 0 AND created_at < :cursor
             ORDER BY created_at DESC
             LIMIT :limit",
        )?;

        let entries = stmt.query_map(named_params! { ":cursor": c, ":limit": limit }, |row| {
            Ok(JournalEntrySummary {
                id: row.get(0)?,
                title: row.get(1)?,
                word_count: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;

        for entry in entries {
            result.push(entry?);
        }
    } else {
        let mut stmt = conn.prepare(
            "SELECT id, title, word_count, created_at, updated_at
             FROM journal_entries
             WHERE is_deleted = 0
             ORDER BY created_at DESC
             LIMIT :limit",
        )?;

        let entries = stmt.query_map(named_params! { ":limit": limit }, |row| {
            Ok(JournalEntrySummary {
                id: row.get(0)?,
                title: row.get(1)?,
                word_count: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;

        for entry in entries {
            result.push(entry?);
        }
    }

    Ok(result)
}

pub fn soft_delete(conn: &Connection, id: &str) -> Result<bool, AppError> {
    let result = conn.execute(
        "UPDATE journal_entries SET is_deleted = 1, updated_at = ?1 WHERE id = ?2 AND is_deleted = 0",
        params![Utc::now().to_rfc3339(), id],
    )?;

    Ok(result > 0)
}

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
#[serde(rename_all = "snake_case")]
pub struct JournalSearchFilters {
    pub query: String,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

pub fn search_entries(conn: &Connection, filters: &JournalSearchFilters) -> Result<Vec<serde_json::Value>, AppError> {
    let mut query = String::from(
        "SELECT f.id, e.title, snippet(journal_fts, 2, '...', '...', '...', 10) as match, e.created_at
         FROM journal_fts f
         JOIN journal_entries e ON f.id = e.id
         WHERE f.content MATCH ? AND e.is_deleted = 0"
    );
    let mut params_vals: Vec<rusqlite::types::Value> = vec![rusqlite::types::Value::Text(filters.query.clone())];

    if let Some(date_from) = &filters.date_from {
        query.push_str(" AND date(e.created_at) >= date(?)");
        params_vals.push(rusqlite::types::Value::Text(date_from.clone()));
    }

    if let Some(date_to) = &filters.date_to {
        query.push_str(" AND date(e.created_at) <= date(?)");
        params_vals.push(rusqlite::types::Value::Text(date_to.clone()));
    }

    query.push_str(" ORDER BY rank LIMIT 10");

    let mut stmt = conn.prepare(&query)?;
    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vals.iter().map(|v| v as &dyn rusqlite::ToSql).collect();

    let rows = stmt.query_map(&*params_refs, |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, String>(0)?,
            "title": row.get::<_, Option<String>>(1)?,
            "snippet": row.get::<_, String>(2)?,
            "created_at": row.get::<_, String>(3)?
        }))
    })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}
