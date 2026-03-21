use crate::error::AppError;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DayCountStat {
    pub date: String,
    pub count: i64,
    pub total_words: i64,
}

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
pub struct MoodStat {
    pub mood: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusStat {
    pub status: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DueBucketStat {
    pub bucket: String,
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
    pub journal_by_day: Vec<DayCountStat>,
    pub productivity: Vec<ProductivityStat>,
    pub emotions: Vec<EmotionStat>,
    pub moods: Vec<MoodStat>,
    pub projects: Vec<ProjectHealthStat>,
    pub task_status: Vec<StatusStat>,
    pub due_buckets: Vec<DueBucketStat>,
    pub energy: Vec<EnergyFocusStat>,
    pub keywords: Vec<KeywordStat>,
    pub time_allocation: Vec<TimeAllocationStat>,
    pub total_journal_entries: i64,
    pub total_tasks_completed: i64,
    pub completion_rate: f64,
    pub journal_streak_days: i64,
    pub insights: Vec<String>,
}

pub fn get_report_data(conn: &Connection, days: i32) -> Result<ReportData, AppError> {
    // 0. Journal consistency (entries and words per day)
    let journal_sql = "
        WITH RECURSIVE days(d) AS (
            SELECT date('now', '-' || ?1 || ' days')
            UNION ALL
            SELECT date(d, '+1 day') FROM days WHERE d < date('now')
        )
        SELECT
            days.d,
            COALESCE((SELECT COUNT(*) FROM journal_entries je WHERE je.is_deleted = 0 AND date(je.created_at) = days.d), 0) as entry_count,
            COALESCE((SELECT SUM(je.word_count) FROM journal_entries je WHERE je.is_deleted = 0 AND date(je.created_at) = days.d), 0) as total_words
        FROM days;
    ";
    let journal_by_day = match conn.prepare(journal_sql) {
        Ok(mut stmt) => stmt
            .query_map(params![days], |row| {
                Ok(DayCountStat {
                    date: row.get(0)?,
                    count: row.get(1)?,
                    total_words: row.get(2)?,
                })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    };

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
    let productivity = match conn.prepare(prod_sql) {
        Ok(mut stmt) => stmt
            .query_map(params![days], |row| {
                Ok(ProductivityStat {
                    date: row.get(0)?,
                    created: row.get(1)?,
                    completed: row.get(2)?,
                })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    // 2. Emotion Stats (Distribution from View)
    let emotion_sql = "
        SELECT emotion, COUNT(*) as count
        FROM journal_emotions_flat
        WHERE date(created_at) >= date('now', '-' || ?1 || ' days')
        GROUP BY emotion
        ORDER BY count DESC
        LIMIT 10;
    ";
    let emotions = match conn.prepare(emotion_sql) {
        Ok(mut stmt) => stmt
            .query_map(params![days], |row| {
                Ok(EmotionStat {
                    emotion: row.get(0)?,
                    count: row.get(1)?,
                })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    // 2b. Mood distribution (from analysis_mood)
    let mood_sql = "
        SELECT LOWER(TRIM(COALESCE(analysis_mood, ''))) as mood, COUNT(*) as count
        FROM journal_entries
        WHERE is_deleted = 0
          AND analysis_mood IS NOT NULL
          AND TRIM(analysis_mood) != ''
          AND date(created_at) >= date('now', '-' || ?1 || ' days')
        GROUP BY mood
        ORDER BY count DESC
        LIMIT 10;
    ";
    let moods = match conn.prepare(mood_sql) {
        Ok(mut stmt) => stmt
            .query_map(params![days], |row| {
                Ok(MoodStat {
                    mood: row.get(0)?,
                    count: row.get(1)?,
                })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    };

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
    let projects = match conn.prepare(project_sql) {
        Ok(mut stmt) => stmt
            .query_map([], |row| {
                Ok(ProjectHealthStat {
                    name: row.get(0)?,
                    total_tasks: row.get(1)?,
                    completed_tasks: row.get(2)?,
                    status: row.get(3)?,
                })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    // 3b. Task status breakdown (within range, by current status)
    let status_sql = "
        SELECT status, COUNT(*) as count
        FROM tasks
        WHERE is_deleted = 0
          AND date(created_at) >= date('now', '-' || ?1 || ' days')
        GROUP BY status
        ORDER BY count DESC;
    ";
    let task_status = match conn.prepare(status_sql) {
        Ok(mut stmt) => stmt
            .query_map(params![days], |row| {
                Ok(StatusStat {
                    status: row.get(0)?,
                    count: row.get(1)?,
                })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    // 3c. Due-date buckets (open tasks only, relative to today)
    let due_sql = "
        SELECT bucket, COUNT(*) as count
        FROM (
            SELECT
                CASE
                    WHEN due_date IS NULL OR TRIM(due_date) = '' THEN 'no_date'
                    WHEN date(due_date) < date('now') THEN 'overdue'
                    WHEN date(due_date) = date('now') THEN 'today'
                    WHEN date(due_date) <= date('now', '+7 days') THEN 'next_7'
                    WHEN date(due_date) <= date('now', '+30 days') THEN 'next_30'
                    ELSE 'later'
                END as bucket
            FROM tasks
            WHERE is_deleted = 0
              AND status NOT IN ('done', 'cancelled')
        )
        GROUP BY bucket
        ORDER BY
          CASE bucket
            WHEN 'overdue' THEN 1
            WHEN 'today' THEN 2
            WHEN 'next_7' THEN 3
            WHEN 'next_30' THEN 4
            WHEN 'later' THEN 5
            WHEN 'no_date' THEN 6
            ELSE 99
          END;
    ";
    let due_buckets = match conn.prepare(due_sql) {
        Ok(mut stmt) => stmt
            .query_map([], |row| {
                Ok(DueBucketStat {
                    bucket: row.get(0)?,
                    count: row.get(1)?,
                })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    // 4. Energy Stats
    let energy_sql = "
        SELECT energy_level, COUNT(*) as count
        FROM tasks
        WHERE status = 'done' AND is_deleted = 0 AND energy_level IS NOT NULL
        GROUP BY energy_level;
    ";
    let energy = match conn.prepare(energy_sql) {
        Ok(mut stmt) => stmt
            .query_map([], |row| {
                Ok(EnergyFocusStat {
                    energy_level: row.get(0)?,
                    count: row.get(1)?,
                })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    };

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
    let time_allocation = match conn.prepare(time_sql) {
        Ok(mut stmt) => stmt
            .query_map(params![days], |row| {
                Ok(TimeAllocationStat {
                    category: row.get(0)?,
                    total_minutes: row.get(1)?,
                })
            })
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    // 7. Totals
    let total_journal_entries = if let Ok(res) = conn.query_row(
        "SELECT COUNT(*) FROM journal_entries WHERE is_deleted = 0",
        [],
        |r| r.get(0),
    ) {
        res
    } else {
        0
    };

    let total_tasks_completed = if let Ok(res) = conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE status = 'done' AND is_deleted = 0",
        [],
        |r| r.get(0),
    ) {
        res
    } else {
        0
    };

    let tasks_created_in_range: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE is_deleted = 0 AND date(created_at) >= date('now', '-' || ?1 || ' days')",
            params![days],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let tasks_completed_in_range: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE is_deleted = 0 AND status = 'done' AND completed_at IS NOT NULL AND date(completed_at) >= date('now', '-' || ?1 || ' days')",
            params![days],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let completion_rate = if tasks_created_in_range > 0 {
        (tasks_completed_in_range as f64) / (tasks_created_in_range as f64)
    } else {
        0.0
    };

    let journal_days_with_entries: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT date(created_at)) FROM journal_entries WHERE is_deleted = 0 AND date(created_at) >= date('now', '-' || ?1 || ' days')",
            params![days],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let journal_streak_days = compute_journal_streak(&journal_by_day);

    let mut insights = Vec::new();
    insights.push(format!(
        "You wrote on {} of the last {} days (current streak: {}).",
        journal_days_with_entries, days, journal_streak_days
    ));
    insights.push(format!(
        "Tasks completed: {} in the selected range (completion rate: {}%).",
        tasks_completed_in_range,
        (completion_rate * 100.0).round() as i64
    ));
    if let Some(top) = time_allocation.first() {
        let hours = (top.total_minutes as f64) / 60.0;
        insights.push(format!(
            "Most time logged: {} ({:.1}h).",
            top.category, hours
        ));
    }
    if let Some(top_mood) = moods.first() {
        insights.push(format!(
            "Most common mood in analyzed entries: {} ({}).",
            top_mood.mood, top_mood.count
        ));
    }

    Ok(ReportData {
        journal_by_day,
        productivity,
        emotions,
        moods,
        projects,
        task_status,
        due_buckets,
        energy,
        keywords,
        time_allocation,
        total_journal_entries,
        total_tasks_completed,
        completion_rate,
        journal_streak_days,
        insights,
    })
}

fn compute_journal_streak(by_day: &[DayCountStat]) -> i64 {
    // Streak ending today (or last day in the list): count consecutive days with >=1 entry.
    let mut streak = 0;
    for day in by_day.iter().rev() {
        if day.count > 0 {
            streak += 1;
        } else if streak > 0 {
            break;
        }
    }
    streak
}
