use crate::db::{journal, tasks};
use crate::error::AppError;
use chrono::{Duration, Local};
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};

pub struct PromptMemoryContext {
    pub semantic_memory: Vec<String>,
    pub recent_patterns: Vec<String>,
    pub related_journal: Vec<String>,
    pub current_entry: Option<String>,
    pub pinned_points: Vec<String>,
}

pub fn build_prompt_memory(
    conn: &Connection,
    source_entry_id: Option<&str>,
) -> Result<PromptMemoryContext, AppError> {
    let now = Local::now().date_naive();
    let since_7d = (now - Duration::days(6)).format("%Y-%m-%d").to_string();
    let since_30d = (now - Duration::days(29)).format("%Y-%m-%d").to_string();
    let today_str = now.format("%Y-%m-%d").to_string();

    let current_entry = match source_entry_id {
        Some(id) => journal::get_entry(conn, id)?,
        None => None,
    };

    let recent_entries = journal::list_recent_entries(
        conn,
        6,
        current_entry.as_ref().map(|entry| entry.id.as_str()),
    )?;
    let entries_7d = journal::list_entries_since(conn, &since_7d)?;
    let entries_30d = journal::list_entries_since(conn, &since_30d)?;
    let active_tasks = tasks::list_tasks(
        conn,
        tasks::TaskListFilters {
            exclude_statuses: Some(vec!["done".into(), "cancelled".into()]),
            ..Default::default()
        },
    )?;

    let pinned = crate::db::ai::list_pinned_memory(conn)?;
    let pinned_points = pinned
        .into_iter()
        .map(|p| format!("Pinned Point: {}", p.content))
        .collect::<Vec<_>>();

    Ok(PromptMemoryContext {
        semantic_memory: build_semantic_memory(
            &entries_7d,
            &entries_30d,
            &active_tasks,
            &today_str,
        ),
        recent_patterns: build_recent_patterns(&recent_entries),
        related_journal: build_related_journal_memory(&recent_entries),
        current_entry: current_entry.as_ref().map(build_current_entry_context),
        pinned_points,
    })
}

fn build_semantic_memory(
    entries_7d: &[journal::JournalEntry],
    entries_30d: &[journal::JournalEntry],
    active_tasks: &[tasks::Task],
    today_str: &str,
) -> Vec<String> {
    let mut items = Vec::new();

    items.push(format!(
        "Journal cadence: {} entries in the last 7 days, {} entries in the last 30 days.",
        entries_7d.len(),
        entries_30d.len()
    ));

    let streak = compute_journal_streak(entries_30d);
    items.push(format!("Current journal streak signal: {} day(s).", streak));

    if let Some((mood, count)) = dominant_mood(entries_7d) {
        items.push(format!(
            "Dominant mood over the last 7 days: {} ({} entr{}).",
            mood,
            count,
            if count == 1 { "y" } else { "ies" }
        ));
    }

    if let Some((mood, count)) = dominant_mood(entries_30d) {
        items.push(format!(
            "Dominant mood over the last 30 days: {} ({} entr{}).",
            mood,
            count,
            if count == 1 { "y" } else { "ies" }
        ));
    }

    let recurring_tags = top_repeated_strings(
        entries_30d
            .iter()
            .flat_map(|entry| parse_json_string_array(&entry.tags))
            .collect(),
        3,
        2,
    );
    if !recurring_tags.is_empty() {
        items.push(format!(
            "Recurring journal tags: {}.",
            recurring_tags.join(", ")
        ));
    }

    let recurring_emotions = top_repeated_strings(
        entries_30d
            .iter()
            .flat_map(|entry| parse_json_string_array(&entry.emotions))
            .collect(),
        3,
        2,
    );
    if !recurring_emotions.is_empty() {
        items.push(format!(
            "Recurring emotions across recent analysis: {}.",
            recurring_emotions.join(", ")
        ));
    }

    let recurring_insights = top_repeated_strings(
        entries_30d
            .iter()
            .flat_map(|entry| parse_json_string_array(&entry.analysis_insights))
            .collect(),
        3,
        2,
    );
    if !recurring_insights.is_empty() {
        items.push(format!(
            "Recurring insights in recent journal analysis: {}.",
            recurring_insights.join(" | ")
        ));
    }

    let overdue_count = active_tasks
        .iter()
        .filter(|task| {
            task.due_date
                .as_deref()
                .is_some_and(|date| date < today_str)
        })
        .count();
    let open_loop_examples = active_tasks
        .iter()
        .take(3)
        .map(|task| task.title.clone())
        .collect::<Vec<_>>();
    if !active_tasks.is_empty() {
        items.push(format!(
            "Open loops in tasks: {} active, {} overdue. Examples: {}.",
            active_tasks.len(),
            overdue_count,
            open_loop_examples.join(", ")
        ));
    }

    items.truncate(8);
    items
}

fn compute_journal_streak(entries: &[journal::JournalEntry]) -> i64 {
    let mut days = entries
        .iter()
        .filter_map(|entry| entry.created_at.get(..10).map(str::to_string))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    days.sort();

    let Some(mut cursor) = days.last().cloned() else {
        return 0;
    };

    let mut streak = 0;
    for day in days.into_iter().rev() {
        if day == cursor {
            streak += 1;
            let parsed = chrono::NaiveDate::parse_from_str(&cursor, "%Y-%m-%d")
                .unwrap_or_else(|_| Local::now().date_naive());
            cursor = (parsed - Duration::days(1)).format("%Y-%m-%d").to_string();
        } else if streak > 0 {
            break;
        }
    }
    streak
}

fn dominant_mood(entries: &[journal::JournalEntry]) -> Option<(String, usize)> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for entry in entries {
        if let Some(mood) = &entry.analysis_mood {
            let normalized = mood.trim().to_lowercase();
            if !normalized.is_empty() {
                *counts.entry(normalized).or_insert(0) += 1;
            }
        }
    }

    counts
        .into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
}

fn build_recent_patterns(entries: &[journal::JournalEntry]) -> Vec<String> {
    let mut patterns = Vec::new();

    let analyzed_count = entries
        .iter()
        .filter(|entry| {
            entry.analysis_summary.is_some()
                || entry.analysis_mood.is_some()
                || !parse_json_string_array(&entry.analysis_insights).is_empty()
        })
        .count();
    if analyzed_count > 0 {
        patterns.push(format!(
            "{} recent journal entr{} had analysis context available.",
            analyzed_count,
            if analyzed_count == 1 { "y" } else { "ies" }
        ));
    }

    if let Some((mood, count)) = dominant_mood(entries) {
        patterns.push(format!(
            "Dominant recent mood signal: {} ({} entr{}).",
            mood,
            count,
            if count == 1 { "y" } else { "ies" }
        ));
    }

    let recurring_insights = top_repeated_strings(
        entries
            .iter()
            .flat_map(|entry| parse_json_string_array(&entry.analysis_insights))
            .collect(),
        3,
        2,
    );
    for insight in recurring_insights {
        patterns.push(format!(
            "Recurring recent insight: {}",
            truncate_for_prompt(&insight, 140)
        ));
    }

    patterns.truncate(5);
    patterns
}

fn build_related_journal_memory(entries: &[journal::JournalEntry]) -> Vec<String> {
    entries
        .iter()
        .take(6)
        .map(|entry| {
            let title = entry.title.as_deref().unwrap_or("Untitled");
            let date = entry.created_at.get(..10).unwrap_or(&entry.created_at);
            let mood = entry
                .analysis_mood
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("unknown");
            let summary = entry
                .analysis_summary
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(|value| truncate_for_prompt(value, 180))
                .unwrap_or_else(|| truncate_for_prompt(&entry.content, 180));
            let insights = parse_json_string_array(&entry.analysis_insights);
            let insight_suffix = insights
                .first()
                .map(|insight| format!(" | insight: {}", truncate_for_prompt(insight, 120)))
                .unwrap_or_default();

            format!(
                "{} | {} | mood: {} | {}{}",
                date, title, mood, summary, insight_suffix
            )
        })
        .collect()
}

fn build_current_entry_context(entry: &journal::JournalEntry) -> String {
    let title = entry.title.as_deref().unwrap_or("Untitled");
    let mood = entry
        .analysis_mood
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("unknown");
    let summary = entry
        .analysis_summary
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("No saved analysis summary yet.");
    let content_preview = truncate_for_prompt(&entry.content, 800);

    format!(
        "Title: {}\nCreated: {}\nAnalysis mood: {}\nAnalysis summary: {}\nBody preview:\n{}",
        title, entry.created_at, mood, summary, content_preview
    )
}

fn top_repeated_strings(items: Vec<String>, max_items: usize, min_count: usize) -> Vec<String> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for item in items {
        let normalized = item.trim();
        if normalized.is_empty() {
            continue;
        }
        *counts.entry(normalized.to_string()).or_insert(0) += 1;
    }

    let mut ranked = counts
        .into_iter()
        .filter(|(_, count)| *count >= min_count)
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    ranked
        .into_iter()
        .take(max_items)
        .map(|(value, _)| value)
        .collect()
}

fn truncate_for_prompt(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }

    let truncated = trimmed.chars().take(max_chars).collect::<String>();
    format!("{}...", truncated.trim_end())
}

fn parse_json_string_array(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_memory_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();

        conn.execute_batch(
            "
            CREATE TABLE journal_entries (
                id TEXT PRIMARY KEY,
                title TEXT,
                content TEXT NOT NULL,
                analysis_summary TEXT,
                analysis_mood TEXT,
                analysis_insights TEXT NOT NULL,
                emotions TEXT NOT NULL,
                tags TEXT NOT NULL,
                last_analysis_conv_id TEXT,
                last_analysed_at TEXT,
                word_count INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                is_deleted INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE tasks (
                id TEXT PRIMARY KEY,
                parent_task_id TEXT,
                title TEXT NOT NULL,
                description TEXT,
                notes TEXT,
                status TEXT NOT NULL,
                priority TEXT NOT NULL,
                due_date TEXT,
                due_time TEXT,
                reminder_at TEXT,
                reminder_fired INTEGER NOT NULL DEFAULT 0,
                recurrence TEXT,
                next_occurrence TEXT,
                time_estimate INTEGER,
                time_logged INTEGER NOT NULL DEFAULT 0,
                actual_start_date TEXT,
                tags TEXT NOT NULL,
                labels TEXT NOT NULL,
                category TEXT,
                project TEXT,
                project_id TEXT,
                energy_level TEXT,
                context_tag TEXT,
                linked_url TEXT,
                sort_order INTEGER NOT NULL DEFAULT 0,
                ai_created INTEGER NOT NULL DEFAULT 0,
                ai_conversation_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                completed_at TEXT,
                is_deleted INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE ai_pinned_memory (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                importance INTEGER NOT NULL DEFAULT 1,
                metadata TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            ",
        )
        .unwrap();

        conn
    }

    fn insert_journal_entry(
        conn: &Connection,
        id: &str,
        title: &str,
        mood: &str,
        summary: &str,
        insights: &[&str],
        emotions: &[&str],
        tags: &[&str],
        created_at: &str,
    ) {
        conn.execute(
            "INSERT INTO journal_entries (
                id, title, content, analysis_summary, analysis_mood, analysis_insights,
                emotions, tags, last_analysis_conv_id, last_analysed_at, word_count,
                created_at, updated_at, is_deleted
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, NULL, ?9, ?10, ?10, 0)",
            rusqlite::params![
                id,
                title,
                format!("{} body text", title),
                summary,
                mood,
                serde_json::to_string(&insights).unwrap(),
                serde_json::to_string(&emotions).unwrap(),
                serde_json::to_string(&tags).unwrap(),
                100_i64,
                created_at,
            ],
        )
        .unwrap();
    }

    fn insert_task(conn: &Connection, id: &str, title: &str, due_date: Option<&str>, status: &str) {
        conn.execute(
            "INSERT INTO tasks (
                id, parent_task_id, title, description, notes, status, priority, due_date,
                due_time, reminder_at, reminder_fired, recurrence, next_occurrence,
                time_estimate, time_logged, actual_start_date, tags, labels, category,
                project, project_id, energy_level, context_tag, linked_url, sort_order,
                ai_created, ai_conversation_id, created_at, updated_at, completed_at, is_deleted
            ) VALUES (
                ?1, NULL, ?2, NULL, NULL, ?3, 'medium', ?4, NULL, NULL, 0, NULL, NULL,
                NULL, 0, NULL, '[]', '[]', NULL, 'Inbox', 'inbox', NULL, NULL, NULL, 0,
                0, NULL, '2026-03-01T09:00:00Z', '2026-03-01T09:00:00Z', NULL, 0
            )",
            rusqlite::params![id, title, status, due_date],
        )
        .unwrap();
    }

    #[test]
    fn top_repeated_strings_returns_ranked_values() {
        let values = vec![
            "sleep".to_string(),
            "focus".to_string(),
            "sleep".to_string(),
            "sleep".to_string(),
            "work".to_string(),
            "work".to_string(),
        ];

        let result = top_repeated_strings(values, 2, 2);

        assert_eq!(result, vec!["sleep".to_string(), "work".to_string()]);
    }

    #[test]
    fn truncate_for_prompt_appends_ellipsis_when_needed() {
        let truncated = truncate_for_prompt("abcdefghijklmnopqrstuvwxyz", 10);
        assert_eq!(truncated, "abcdefghij...");
    }

    #[test]
    fn build_prompt_memory_collects_semantic_and_entry_context() {
        let conn = setup_memory_test_db();
        let today = Local::now().date_naive();
        let day0 = today.format("%Y-%m-%dT08:00:00Z").to_string();
        let day1 = (today - Duration::days(1))
            .format("%Y-%m-%dT08:00:00Z")
            .to_string();
        let day2 = (today - Duration::days(2))
            .format("%Y-%m-%dT08:00:00Z")
            .to_string();

        insert_journal_entry(
            &conn,
            "entry-1",
            "Morning Reset",
            "reflective",
            "Needed a slower start.",
            &["Sleep matters", "Protect mornings"],
            &["tired", "hopeful"],
            &["morning", "routine"],
            &day0,
        );
        insert_journal_entry(
            &conn,
            "entry-2",
            "Work Spiral",
            "reflective",
            "Work kept bleeding into the evening.",
            &["Sleep matters"],
            &["tired", "anxious"],
            &["work", "routine"],
            &day1,
        );
        insert_journal_entry(
            &conn,
            "entry-3",
            "Quiet Win",
            "calm",
            "A calmer day with better pacing.",
            &["Protect mornings"],
            &["calm"],
            &["morning"],
            &day2,
        );

        insert_task(
            &conn,
            "task-1",
            "Call the contractor",
            Some("2020-01-01"),
            "todo",
        );
        insert_task(&conn, "task-2", "Draft section 3", None, "in_progress");

        let memory = build_prompt_memory(&conn, Some("entry-1")).unwrap();

        assert!(memory
            .semantic_memory
            .iter()
            .any(|item| item.contains("Journal cadence")));
        assert!(memory
            .semantic_memory
            .iter()
            .any(|item| item.contains("Dominant mood over the last 7 days: reflective")));
        assert!(memory
            .semantic_memory
            .iter()
            .any(|item| item.contains("Recurring journal tags")));
        assert!(memory
            .semantic_memory
            .iter()
            .any(|item| item.contains("Open loops in tasks: 2 active")));
        assert!(memory
            .recent_patterns
            .iter()
            .any(|item| item.contains("Dominant recent mood signal")));
        assert_eq!(memory.related_journal.len(), 2);
        assert!(memory
            .related_journal
            .iter()
            .all(|item| !item.contains("Morning Reset")));
        assert!(memory
            .current_entry
            .as_deref()
            .is_some_and(|item| item.contains("Morning Reset")));
    }

    #[test]
    fn build_prompt_memory_handles_empty_database() {
        let conn = setup_memory_test_db();

        let memory = build_prompt_memory(&conn, None).unwrap();

        assert!(memory
            .semantic_memory
            .iter()
            .any(|item| item.contains("Journal cadence: 0 entries")));
        assert!(memory
            .semantic_memory
            .iter()
            .any(|item| item.contains("Current journal streak signal: 0")));
        assert!(memory.recent_patterns.is_empty());
        assert!(memory.related_journal.is_empty());
        assert!(memory.current_entry.is_none());
    }
}
