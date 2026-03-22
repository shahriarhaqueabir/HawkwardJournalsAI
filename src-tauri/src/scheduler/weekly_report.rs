use crate::error::AppError;
use chrono::{Datelike, Local, NaiveDate};
use rusqlite::{params, Connection};
use tauri::{AppHandle, Manager};

/// Checks if the weekly review has already run for the current week.
pub fn has_review_run_this_week(conn: &Connection) -> bool {
    let mut stmt =
        match conn.prepare("SELECT value FROM app_settings WHERE key = 'weekly_review_last_run'") {
            Ok(s) => s,
            Err(_) => return false,
        };

    let last_run_str: String = match stmt.query_row([], |r| r.get(0)) {
        Ok(s) => s,
        Err(_) => return false,
    };

    if last_run_str.is_empty() {
        return false;
    }

    let last_run = match NaiveDate::parse_from_str(&last_run_str, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => return false,
    };

    let today = Local::now().date_naive();

    // Compare ISO week and year (D-109)
    last_run.iso_week() == today.iso_week() && last_run.year() == today.year()
}

pub async fn maybe_run_weekly_review(app: &AppHandle) -> Result<(), AppError> {
    let today = Local::now().date_naive();

    // Decision D-109: Only run on Mondays
    if today.weekday() != chrono::Weekday::Mon {
        return Ok(());
    }

    {
        let state = app.state::<crate::AppState>();
        let conn = state.conn.lock().await;
        if has_review_run_this_week(&conn) {
            return Ok(());
        }
    }

    println!("[SCHEDULER] Running weekly review for week of {}", today);

    // 1. Fetch data summary for the last 7 days
    let (report_data, prompt_memory) = {
        let state = app.state::<crate::AppState>();
        let conn = state.conn.lock().await;
        (
            crate::db::reports::get_report_data(&conn, 7)?,
            crate::ai::memory::build_prompt_memory(&conn, None)?,
        )
    };

    // 2. Format a prompt for the AI
    let stats_text = format!(
        "WEEKLY STATS:
        - Tasks Completed: {}
        - Journal Entries: {}
        - Emotions: {}
        - Projects: {} active
        - Time Allocation: {}
        ",
        report_data.total_tasks_completed,
        report_data.total_journal_entries,
        report_data
            .emotions
            .iter()
            .map(|e| e.emotion.clone())
            .collect::<Vec<_>>()
            .join(", "),
        report_data.projects.len(),
        report_data
            .time_allocation
            .iter()
            .map(|t| format!("{}: {}m", t.category, t.total_minutes))
            .collect::<Vec<_>>()
            .join(", ")
    );

    // 3. Initiate AI Chat in WeeklyPlan mode
    let state = app.state::<crate::AppState>();
    let client = state.ollama.clone();

    let conv_id = uuid::Uuid::new_v4().to_string();
    let now_ts = Local::now().to_rfc3339();

    // Create a special conversation for this review
    {
        let conn = state.conn.lock().await;
        conn.execute(
            "INSERT INTO ai_conversations (id, title, model, source, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                conv_id,
                format!("Weekly Review: {}", today.format("%b %d, %Y")),
                crate::ai::client::DEFAULT_MODEL,
                "weekly_plan",
                now_ts.clone(),
                now_ts.clone()
            ],
        )?;
    }

    // Request the AI insight (Non-streaming for background)
    let prompt = format!(
        "Help me plan my next week based on these stats from last week:\n{}\n\nProvide 3-4 actionable insights or suggestions.",
        stats_text
    );

    // We use a simple chat call (non-streaming)
    let prompt_input = crate::ai::prompt::PromptInput {
        mode: crate::ai::prompt::ChatMode::WeeklyPlan,
        overdue_tasks: vec![],
        today_tasks: vec![],
        upcoming_tasks: vec![],
        semantic_memory: prompt_memory.semantic_memory,
        recent_patterns: prompt_memory.recent_patterns,
        related_journal: prompt_memory.related_journal,
        current_entry: None,
        pinned_points: prompt_memory.pinned_points,
    };

    let generated = match client.chat_single_with_input(&prompt, prompt_input).await {
        Ok(ai_response) => {
            // Save AI message
            let conn = state.conn.lock().await;
            conn.execute(
                "INSERT INTO ai_messages (id, conversation_id, role, content, model, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    uuid::Uuid::new_v4().to_string(),
                    conv_id,
                    "assistant",
                    ai_response,
                    crate::ai::client::DEFAULT_MODEL,
                    Local::now().to_rfc3339()
                ],
            )?;
            true
        }
        Err(e) => {
            println!("[SCHEDULER] Weekly AI review failed: {:?}", e);
            false
        }
    };

    if !generated {
        return Ok(());
    }

    let now_date = today.format("%Y-%m-%d").to_string();
    let now_ts = Local::now().to_rfc3339();

    {
        let conn = state.conn.lock().await;
        conn.execute(
            "UPDATE app_settings SET value = ?1, updated_at = ?2 WHERE key = 'weekly_review_last_run'",
            params![now_date, now_ts],
        )?;
    }

    crate::events::emit(
        app,
        crate::events::AppEvent::WeeklyReviewGenerated {
            date: now_date.clone(),
        },
    );
    crate::events::emit(
        app,
        crate::events::AppEvent::SystemStatus {
            message: "Weekly AI review generated! Check the AI tab.".into(),
        },
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_review_run_this_week_returns_false_without_setting() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )
        .unwrap();

        assert!(!has_review_run_this_week(&conn));
    }

    #[test]
    fn has_review_run_this_week_returns_true_for_current_week() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )
        .unwrap();

        let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
        conn.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2)",
            params!["weekly_review_last_run", today],
        )
        .unwrap();

        assert!(has_review_run_this_week(&conn));
    }

    #[test]
    fn has_review_run_this_week_returns_false_for_invalid_date() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )
        .unwrap();

        conn.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2)",
            params!["weekly_review_last_run", "not-a-date"],
        )
        .unwrap();

        assert!(!has_review_run_this_week(&conn));
    }
}
