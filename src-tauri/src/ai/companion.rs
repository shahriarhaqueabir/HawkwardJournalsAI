use crate::ai::client::OllamaClient;
use crate::ai::memory::PromptMemoryContext;
use crate::ai::prompt::{ChatMode, PromptInput};
use crate::db::{journal, settings, tasks};
use crate::error::AppError;
use chrono::{Datelike, Local, NaiveDate};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

const NUDGE_HISTORY_KEY: &str = "companion_nudge_history";
const REFLECTION_HISTORY_KEY: &str = "companion_reflection_history";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProactiveTrigger {
    AppOpen,
    EmptyEntry,
    TaskCompleted,
}

impl ProactiveTrigger {
    pub fn from_str(value: &str) -> Result<Self, AppError> {
        match value {
            "app_open" => Ok(Self::AppOpen),
            "empty_entry" => Ok(Self::EmptyEntry),
            "task_completed" => Ok(Self::TaskCompleted),
            _ => Err(AppError::InvalidInput(format!(
                "Unknown proactive trigger: {}",
                value
            ))),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AppOpen => "app_open",
            Self::EmptyEntry => "empty_entry",
            Self::TaskCompleted => "task_completed",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReflectionPromptResponse {
    pub content: String,
    pub suggested_tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct HistoryEntry {
    content: String,
    created_at: String,
}

#[derive(Debug, Clone)]
pub struct ProactiveDecision {
    pub trigger: ProactiveTrigger,
    pub reason: String,
    pub details: Vec<String>,
}

pub fn decide_proactive_nudge(
    conn: &Connection,
    trigger: ProactiveTrigger,
) -> Result<Option<ProactiveDecision>, AppError> {
    let recent_entries = journal::list_recent_entries(conn, 1, None)?;
    let active_tasks = tasks::list_tasks(
        conn,
        tasks::TaskListFilters {
            exclude_statuses: Some(vec!["done".into(), "cancelled".into()]),
            ..Default::default()
        },
    )?;

    let today = Local::now().date_naive();
    let days_since_last_entry = recent_entries
        .first()
        .and_then(|entry| entry.created_at.get(..10))
        .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
        .map(|date| (today - date).num_days());

    let stale_open_loops = active_tasks
        .iter()
        .filter_map(|task| {
            task.created_at
                .get(..10)
                .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
                .map(|date| (task, (today - date).num_days()))
        })
        .filter(|(_, age_days)| *age_days >= 5)
        .take(3)
        .map(|(task, age_days)| format!("{} (open {} days)", task.title, age_days))
        .collect::<Vec<_>>();

    match trigger {
        ProactiveTrigger::AppOpen => {
            if let Some(days) = days_since_last_entry {
                if days >= 2 {
                    return Ok(Some(ProactiveDecision {
                        trigger,
                        reason: "reengagement".into(),
                        details: vec![format!("The user has not written for {} day(s).", days)],
                    }));
                }
            }

            if !stale_open_loops.is_empty() {
                return Ok(Some(ProactiveDecision {
                    trigger,
                    reason: "open_loop".into(),
                    details: stale_open_loops,
                }));
            }

            Ok(None)
        }
        ProactiveTrigger::EmptyEntry => {
            if !stale_open_loops.is_empty() {
                return Ok(Some(ProactiveDecision {
                    trigger,
                    reason: "open_loop".into(),
                    details: stale_open_loops,
                }));
            }

            if let Some(days) = days_since_last_entry {
                if days >= 2 {
                    return Ok(Some(ProactiveDecision {
                        trigger,
                        reason: "reengagement".into(),
                        details: vec![format!("The user has not written for {} day(s).", days)],
                    }));
                }
            }

            Ok(Some(ProactiveDecision {
                trigger,
                reason: "blank_page".into(),
                details: vec!["The user has been idle in a new blank entry for 30 seconds.".into()],
            }))
        }
        ProactiveTrigger::TaskCompleted => {
            // Pick the most recently completed task that was older than 3 days
            let conn_ref = conn;
            let mut stmt = conn_ref.prepare(
                "SELECT title, created_at FROM tasks 
                 WHERE status = 'done' AND is_deleted = 0 
                 ORDER BY completed_at DESC LIMIT 1"
            )?;
            let row = stmt.query_row([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)));
            
            if let Ok((title, created_at)) = row {
                if let Some(created_date) = created_at.get(..10).and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()) {
                    let age = (today - created_date).num_days();
                    if age >= 3 {
                        return Ok(Some(ProactiveDecision {
                            trigger,
                            reason: "task_completion_reflection".into(),
                            details: vec![format!("Finished '{}' which was open for {} day(s).", title, age)],
                        }));
                    }
                }
            }
            Ok(None)
        }
    }
}

pub async fn generate_proactive_nudge(
    client: &OllamaClient,
    memory: PromptMemoryContext,
    recent_nudges: &[String],
    decision: &ProactiveDecision,
) -> Result<Option<String>, AppError> {
    let prompt_input = PromptInput {
        mode: ChatMode::ProactiveNudge,
        overdue_tasks: vec![],
        today_tasks: vec![],
        upcoming_tasks: vec![],
        semantic_memory: memory.semantic_memory,
        recent_patterns: memory.recent_patterns,
        related_journal: memory.related_journal,
        current_entry: memory.current_entry,
        pinned_points: memory.pinned_points,
    };

    let mut prompt = format!(
        "Generate one proactive nudge.\nTrigger: {}\nReason: {}\nDetails:\n- {}",
        decision.trigger.as_str(),
        decision.reason,
        decision.details.join("\n- ")
    );
    if !recent_nudges.is_empty() {
        prompt.push_str("\nAvoid repeating these recent nudges from this week:\n- ");
        prompt.push_str(&recent_nudges.join("\n- "));
    }

    let first = sanitize_companion_text(
        &client
            .chat_single_with_input(&prompt, prompt_input.clone())
            .await?,
        220,
    );
    if first.is_empty() {
        return Ok(None);
    }
    if !recent_nudges.iter().any(|item| item == &first) {
        return Ok(Some(first));
    }

    let retry_prompt = format!(
        "{}\nUse a different angle and wording than the blocked nudges above.",
        prompt
    );
    let second = sanitize_companion_text(
        &client
            .chat_single_with_input(&retry_prompt, prompt_input)
            .await?,
        220,
    );
    if second.is_empty() || recent_nudges.iter().any(|item| item == &second) {
        return Ok(None);
    }

    Ok(Some(second))
}

pub async fn generate_reflection_prompt(
    client: &OllamaClient,
    memory: PromptMemoryContext,
    draft_context: Option<String>,
    recent_prompts: &[String],
    try_another: bool,
) -> Result<Option<ReflectionPromptResponse>, AppError> {
    let prompt_input = PromptInput {
        mode: ChatMode::ReflectionPrompt,
        overdue_tasks: vec![],
        today_tasks: vec![],
        upcoming_tasks: vec![],
        semantic_memory: memory.semantic_memory,
        recent_patterns: memory.recent_patterns,
        related_journal: memory.related_journal,
        current_entry: draft_context.or(memory.current_entry),
        pinned_points: memory.pinned_points,
    };

    let mut prompt = String::from(
        "Generate one reflection prompt for the journal editor. It must be personal, evocative, and focused on inner life.",
    );
    if try_another {
        prompt.push_str(" The user clicked try another, so it must feel clearly different.");
    }
    if !recent_prompts.is_empty() {
        prompt.push_str("\nAvoid repeating these prompts from the last 7 days:\n- ");
        prompt.push_str(&recent_prompts.join("\n- "));
    }

    let first = sanitize_companion_text(
        &client
            .chat_single_with_input(&prompt, prompt_input.clone())
            .await?,
        220,
    );
    if first.is_empty() {
        return Ok(None);
    }
    if !recent_prompts.iter().any(|item| item == &first) {
        return Ok(Some(ReflectionPromptResponse {
            content: first,
            suggested_tags: vec![],
        }));
    }

    let retry_prompt = format!(
        "{}\nUse a different emotional angle and different wording than the blocked prompts above.",
        prompt
    );
    let second = sanitize_companion_text(
        &client
            .chat_single_with_input(&retry_prompt, prompt_input)
            .await?,
        220,
    );
    if second.is_empty() || recent_prompts.iter().any(|item| item == &second) {
        return Ok(None);
    }

    Ok(Some(ReflectionPromptResponse {
        content: second,
        suggested_tags: vec![],
    }))
}

pub fn load_recent_nudges(conn: &Connection) -> Result<Vec<String>, AppError> {
    let week_key = current_week_key();
    Ok(load_history(conn, NUDGE_HISTORY_KEY)?
        .into_iter()
        .filter(|entry| week_key_for(&entry.created_at).is_some_and(|key| key == week_key))
        .map(|entry| entry.content)
        .collect())
}

pub fn save_nudge(conn: &Connection, content: &str) -> Result<(), AppError> {
    let mut history = load_history(conn, NUDGE_HISTORY_KEY)?;
    history.push(HistoryEntry {
        content: content.to_string(),
        created_at: Local::now().to_rfc3339(),
    });
    prune_history(&mut history, 20);
    save_history(conn, NUDGE_HISTORY_KEY, &history)
}

pub fn load_recent_reflection_prompts(conn: &Connection) -> Result<Vec<String>, AppError> {
    let today = Local::now().date_naive();
    Ok(load_history(conn, REFLECTION_HISTORY_KEY)?
        .into_iter()
        .filter(|entry| {
            parse_history_date(&entry.created_at)
                .map(|date| (today - date).num_days() <= 7)
                .unwrap_or(false)
        })
        .map(|entry| entry.content)
        .collect())
}

pub fn save_reflection_prompt(conn: &Connection, content: &str) -> Result<(), AppError> {
    let mut history = load_history(conn, REFLECTION_HISTORY_KEY)?;
    history.push(HistoryEntry {
        content: content.to_string(),
        created_at: Local::now().to_rfc3339(),
    });
    prune_history(&mut history, 30);
    save_history(conn, REFLECTION_HISTORY_KEY, &history)
}

pub fn format_draft_context(title: Option<&str>, body: Option<&str>) -> Option<String> {
    let title = title.unwrap_or_default().trim();
    let body = body.unwrap_or_default().trim();
    if title.is_empty() && body.is_empty() {
        return None;
    }

    let preview = body.chars().take(600).collect::<String>();
    Some(format!(
        "Draft title: {}\nDraft body so far:\n{}",
        if title.is_empty() { "Untitled" } else { title },
        preview
    ))
}

fn sanitize_companion_text(text: &str, max_chars: usize) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = compact.trim_matches('"').trim();
    if trimmed.chars().count() <= max_chars {
        trimmed.to_string()
    } else {
        let shortened = trimmed.chars().take(max_chars).collect::<String>();
        format!("{}...", shortened.trim_end())
    }
}

fn load_history(conn: &Connection, key: &str) -> Result<Vec<HistoryEntry>, AppError> {
    let raw = settings::get_setting(conn, key)?
        .map(|setting| setting.value)
        .unwrap_or_default();
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }

    serde_json::from_str(&raw)
        .map_err(|e| AppError::Database(format!("Invalid companion history for {}: {}", key, e)))
}

fn save_history(conn: &Connection, key: &str, history: &[HistoryEntry]) -> Result<(), AppError> {
    let serialized = serde_json::to_string(history)
        .map_err(|e| AppError::Database(format!("Could not serialize {}: {}", key, e)))?;
    settings::set_setting(conn, key, &serialized)?;
    Ok(())
}

fn prune_history(history: &mut Vec<HistoryEntry>, keep: usize) {
    history.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    if history.len() > keep {
        let drain_to = history.len() - keep;
        history.drain(0..drain_to);
    }
}

fn current_week_key() -> String {
    let now = Local::now().date_naive();
    format!("{}-{:02}", now.iso_week().year(), now.iso_week().week())
}

fn week_key_for(value: &str) -> Option<String> {
    parse_history_date(value)
        .map(|date| format!("{}-{:02}", date.iso_week().year(), date.iso_week().week()))
}

fn parse_history_date(value: &str) -> Option<NaiveDate> {
    value
        .get(..10)
        .and_then(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_settings_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT ''
            );",
        )
        .unwrap();
        conn
    }

    fn setup_decision_db() -> Connection {
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

            CREATE TABLE app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT ''
            );
            ",
        )
        .unwrap();
        conn
    }

    fn insert_entry(conn: &Connection, days_ago: i64) {
        let created_at = (Local::now().date_naive() - chrono::Duration::days(days_ago))
            .format("%Y-%m-%dT08:00:00Z")
            .to_string();
        conn.execute(
            "INSERT INTO journal_entries (
                id, title, content, analysis_summary, analysis_mood, analysis_insights,
                emotions, tags, last_analysis_conv_id, last_analysed_at, word_count,
                created_at, updated_at, is_deleted
            ) VALUES (
                'entry-1', 'Entry', 'Body', 'Summary', 'reflective', '[]', '[]', '[]',
                NULL, NULL, 10, ?1, ?1, 0
            )",
            rusqlite::params![created_at],
        )
        .unwrap();
    }

    fn insert_task(conn: &Connection, title: &str, created_days_ago: i64) {
        let created_at = (Local::now().date_naive() - chrono::Duration::days(created_days_ago))
            .format("%Y-%m-%dT08:00:00Z")
            .to_string();
        conn.execute(
            "INSERT INTO tasks (
                id, parent_task_id, title, description, notes, status, priority, due_date,
                due_time, reminder_at, reminder_fired, recurrence, next_occurrence,
                time_estimate, time_logged, actual_start_date, tags, labels, category,
                project, project_id, energy_level, context_tag, linked_url, sort_order,
                ai_created, ai_conversation_id, created_at, updated_at, completed_at, is_deleted
            ) VALUES (
                ?1, NULL, ?2, NULL, NULL, 'todo', 'medium', NULL, NULL, NULL, 0, NULL, NULL,
                NULL, 0, NULL, '[]', '[]', NULL, 'Inbox', 'inbox', NULL, NULL, NULL, 0,
                0, NULL, ?3, ?3, NULL, 0
            )",
            rusqlite::params![format!("task-{}", created_days_ago), title, created_at],
        )
        .unwrap();
    }

    #[test]
    fn format_draft_context_returns_none_for_blank_draft() {
        assert!(format_draft_context(Some(""), Some("   ")).is_none());
    }

    #[test]
    fn history_round_trip_filters_to_current_week() {
        let conn = setup_settings_db();
        save_nudge(&conn, "Test nudge").unwrap();

        let nudges = load_recent_nudges(&conn).unwrap();

        assert_eq!(nudges, vec!["Test nudge".to_string()]);
    }

    #[test]
    fn reflection_history_round_trip_loads_recent_prompts() {
        let conn = setup_settings_db();
        save_reflection_prompt(&conn, "What are you not saying yet?").unwrap();

        let prompts = load_recent_reflection_prompts(&conn).unwrap();

        assert_eq!(prompts, vec!["What are you not saying yet?".to_string()]);
    }

    #[test]
    fn decide_proactive_nudge_prefers_reengagement_on_app_open() {
        let conn = setup_decision_db();
        insert_entry(&conn, 3);

        let decision = decide_proactive_nudge(&conn, ProactiveTrigger::AppOpen)
            .unwrap()
            .unwrap();

        assert_eq!(decision.reason, "reengagement");
    }

    #[test]
    fn decide_proactive_nudge_uses_open_loop_for_blank_entry() {
        let conn = setup_decision_db();
        insert_entry(&conn, 0);
        insert_task(&conn, "Call the contractor", 6);

        let decision = decide_proactive_nudge(&conn, ProactiveTrigger::EmptyEntry)
            .unwrap()
            .unwrap();

        assert_eq!(decision.reason, "open_loop");
        assert!(decision
            .details
            .iter()
            .any(|item| item.contains("Call the contractor")));
    }
}
