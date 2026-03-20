use crate::error::AppError;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductivityStat {
    pub date: String,
    pub created: i64,
    pub completed: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmotionStat {
    pub emotion: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectHealthStat {
    pub name: String,
    pub total_tasks: i64,
    pub completed_tasks: i64,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnergyFocusStat {
    pub energy_level: Option<String>,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeywordStat {
    pub word: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimeAllocationStat {
    pub category: String,
    pub total_minutes: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportData {
    pub productivity: Vec<ProductivityStat>,
    pub emotions: Vec<EmotionStat>,
    pub projects: Vec<ProjectHealthStat>,
    pub energy: Vec<EnergyFocusStat>,
    pub keywords: Vec<KeywordStat>,
    pub time_allocation: Vec<TimeAllocationStat>,
    pub total_journal_entries: i64,
    pub total_tasks_completed: i64,
}

pub fn get_report_data(
    conn: &Connection,
    days: i32,
) -> Result<ReportData, AppError> {
    // 1. Productivity Stats (Daily Created/Completed)
    let prod_sql = "
        WITH RECURSIVE days(d) AS (
            SELECT date('now', '-' || ?1 || ' days')
            UNION ALL
            SELECT date(d, '+1 day') FROM days WHERE d < date('now')
        )
        SELECT 
            days.d,
            (SELECT COUNT(*) FROM tasks WHERE date(created_at) = days.d AND is_deleted = 0) as created,
            (SELECT COUNT(*) FROM tasks WHERE date(completed_at) = days.d AND is_deleted = 0 AND status = 'done') as completed
        FROM days;
    ";
    let mut stmt = conn.prepare(prod_sql)?;
    let productivity = stmt.query_map(params![days], |row| {
        Ok(ProductivityStat {
            date: row.get(0)?,
            created: row.get(1)?,
            completed: row.get(2)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    // 2. Emotion Stats (Distribution from View)
    let emotion_sql = "
        SELECT emotion, COUNT(*) as count
        FROM journal_emotions_flat
        WHERE date(created_at) >= date('now', '-' || ?1 || ' days')
        GROUP BY emotion
        ORDER BY count DESC
        LIMIT 10;
    ";
    let mut stmt = conn.prepare(emotion_sql)?;
    let emotions = stmt.query_map(params![days], |row| {
        Ok(EmotionStat {
            emotion: row.get(0)?,
            count: row.get(1)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    // 3. Project Health Stats
    let project_sql = "
        SELECT 
            p.name,
            COUNT(t.id) as total,
            SUM(CASE WHEN t.status = 'done' THEN 1 ELSE 0 END) as completed,
            p.status
        FROM projects p
        LEFT JOIN tasks t ON t.project_id = p.id AND t.is_deleted = 0
        WHERE p.is_deleted = 0
        GROUP BY p.id
        ORDER BY total DESC;
    ";
    let mut stmt = conn.prepare(project_sql)?;
    let projects = stmt.query_map([], |row| {
        Ok(ProjectHealthStat {
            name: row.get(0)?,
            total_tasks: row.get(1)?,
            completed_tasks: row.get(2)?,
            status: row.get(3)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    // 4. Energy Stats
    let energy_sql = "
        SELECT energy_level, COUNT(*) as count
        FROM tasks
        WHERE status = 'done' AND is_deleted = 0 AND energy_level IS NOT NULL
        GROUP BY energy_level;
    ";
    let mut stmt = conn.prepare(energy_sql)?;
    let energy = stmt.query_map([], |row| {
        Ok(EnergyFocusStat {
            energy_level: row.get(0)?,
            count: row.get(1)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    // 5. Keyword Cloud (FTS5 tokenization mockup via simple count for now)
    // Placeholder logic for keyword analysis
    let keywords = vec![]; 

    // 6. Time Allocation
    let time_sql = "
        SELECT COALESCE(t.category, 'uncategorized'), SUM(tl.duration) / 60 as total
        FROM time_logs tl
        JOIN tasks t ON t.id = tl.task_id
        WHERE tl.started_at >= date('now', '-' || ?1 || ' days')
        GROUP BY t.category
        ORDER BY total DESC;
    ";
    let mut stmt = conn.prepare(time_sql)?;
    let time_allocation = stmt.query_map(params![days], |row| {
        Ok(TimeAllocationStat {
            category: row.get(0)?,
            total_minutes: row.get(1)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;

    // 7. Totals
    let total_journal_entries = if let Ok(res) = conn.query_row(
        "SELECT COUNT(*) FROM journal_entries WHERE is_deleted = 0", [], |r| r.get(0)
    ) { res } else { 0 };
    
    let total_tasks_completed = if let Ok(res) = conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE status = 'done' AND is_deleted = 0", [], |r| r.get(0)
    ) { res } else { 0 };

    Ok(ReportData {
        productivity,
        emotions,
        projects,
        energy,
        keywords,
        time_allocation,
        total_journal_entries,
        total_tasks_completed,
    })
}
