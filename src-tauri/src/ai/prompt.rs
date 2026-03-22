use crate::db::tasks::Task;
use chrono::Local;
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct PromptInput {
    pub mode: ChatMode,
    pub overdue_tasks: Vec<Task>,
    pub today_tasks: Vec<Task>,
    pub upcoming_tasks: Vec<Task>,
    pub semantic_memory: Vec<String>,
    pub recent_patterns: Vec<String>,
    pub related_journal: Vec<String>,
    pub current_entry: Option<String>,
    pub pinned_points: Vec<String>,
}

#[allow(dead_code)]
#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ChatMode {
    Chat,
    Analysis,
    Ghostwriter,
    WeeklyPlan,
    ProactiveNudge,
    ReflectionPrompt,
}

pub fn build_system_prompt(input: &PromptInput) -> String {
    let mut blocks = Vec::new();

    blocks.push(
        "COMPANION IDENTITY
You are HawkwardJournalAI, a local, private companion within the user's journal and task system.
- Be warm, direct, and perceptive. Never clinical or sycophantic. Match the user's energy.
- Push back gently if their framing is incorrect or self-defeating.
- Never open with praise or filler. Ask at most one follow-up question.
- Never generate tasks, schedules, or productivity advice unless the user asks.
- If you do not know something, say so plainly. Do not hallucinate."
            .to_string(),
    );

    blocks.push(
        "RULES & TOOLS
1. Use tools only when you need to read local data (list_tasks, search_journal) or manipulate data (create_task, update_task, complete_task).
2. If the user refers to a task vaguely, ALWAYS call list_tasks first to get the exact ID.
3. Never invent tasks, IDs, dates, or names.
4. Dates must be YYYY-MM-DD format. Resolve relative terms (like 'tomorrow') using the CURRENT CONTEXT.
5. Provide answers in plain text. NEVER show raw JSON or raw UUIDs. Show short task IDs in brackets like [a1b2c3] next to titles."
            .to_string(),
    );

    let now = Local::now();
    blocks.push(format!(
        "CURRENT CONTEXT
Date: {}
Time: {}
Day: {}",
        now.format("%Y-%m-%d"),
        now.format("%H:%M:%S"),
        now.format("%A")
    ));

    let mode_instr = match input.mode {
        ChatMode::Chat => "MODE: Companion Chat. Be a thoughtful journal companion first and a task assistant second. Reflection, pattern recognition, and grounded help take priority.",
        ChatMode::Analysis => "MODE: Analysis. Extract actionable tasks and emotional insights from the current journal entry.",
        ChatMode::Ghostwriter => "MODE: Ghostwriter. Help the user draft or rewrite journal text in chat only. Never imply the draft has been saved.",
        ChatMode::WeeklyPlan => "MODE: Weekly Plan. Help the user review the week realistically. Name tradeoffs, constraints, and overload honestly instead of pretending everything fits.",
        ChatMode::ProactiveNudge => "MODE: Proactive Nudge. Write exactly one specific nudge in plain text, maximum two sentences, curious not nagging. Do not use tools.",
        ChatMode::ReflectionPrompt => "MODE: Reflection Prompt. Write exactly one evocative reflection prompt in plain text, one or two sentences maximum. Do not use tools.",
    };
    blocks.push(mode_instr.to_string());

    if !input.semantic_memory.is_empty() {
        blocks.push(format!("SEMANTIC MEMORY\n{}", input.semantic_memory.join("\n")));
    }
    if !input.recent_patterns.is_empty() {
        blocks.push(format!("RECENT JOURNAL PATTERNS\n{}", input.recent_patterns.join("\n")));
    }
    if !input.related_journal.is_empty() {
        blocks.push(format!("RELATED JOURNAL MEMORY\n{}", input.related_journal.join("\n")));
    }
    if let Some(entry) = &input.current_entry {
        blocks.push(format!("CURRENT JOURNAL ENTRY BEING VIEWED\n{}", entry));
    }
    if !input.pinned_points.is_empty() {
        blocks.push(format!("PINNED POINTS\n{}", input.pinned_points.join("\n")));
    }

    if !input.overdue_tasks.is_empty() {
        let list: String = input.overdue_tasks.iter().map(format_task).collect();
        blocks.push(format!("OVERDUE TASKS\n{}", list));
    }
    if !input.today_tasks.is_empty() {
        let list: String = input.today_tasks.iter().map(format_task).collect();
        blocks.push(format!("TODAY'S TASKS\n{}", list));
    }
    if !input.upcoming_tasks.is_empty() {
        let list: String = input.upcoming_tasks.iter().map(format_task).collect();
        blocks.push(format!("UPCOMING TASKS\n{}", list));
    }

    blocks.push(
        "ERROR RECOVERY
1. If a tool call fails, explain the failure plainly and suggest an alternative.
2. If context is missing for a mutating action, ask for the specific missing field.
3. If the request is outside your capabilities, decline briefly and explain the limitation."
            .to_string(),
    );

    blocks.join("\n\n")
}

fn format_task(t: &Task) -> String {
    // D-94 Compact format: [id6] title | PRIORITY | date | energy | project
    let id_short = &t.id[..6.min(t.id.len())];
    let priority = &t.priority;
    let due = t.due_date.clone().unwrap_or_else(|| "No Date".to_string());
    let energy = t.energy_level.clone().unwrap_or_else(|| "Any".to_string());
    let project = t.project.clone().unwrap_or_else(|| "Inbox".into());

    format!(
        "[{}] {} | {} | {} | {} | {}\n",
        id_short, t.title, priority, due, energy, project
    )
}

pub fn get_analysis_system_prompt() -> &'static str {
    r#"You analyze personal journal entries for concise, structured extraction with supportive but neutral language.

OUTPUT SCHEMA - STRICT ADHERENCE REQUIRED

Return exactly one JSON object with this schema:
{
  "summary": "A 1-2 sentence high-level summary (max 30 words).",
  "mood": "One-word sentiment (for example joyful, anxious, reflective, frustrated).",
  "emotions": ["List of identified emotions (max 5)"],
  "tasks": [
    {
      "title": "Extract actionable task title",
      "project_suggestion": "inbox or specific project ID"
    }
  ],
  "insights": ["Synthesis of patterns or realizations (max 3)"]
}

CONSTRAINTS - NON-NEGOTIABLE
1. Output must be a single valid JSON object.
2. No markdown, no backticks, and no conversational text before or after the JSON.
3. If no data exists for a field, return an empty array [] or empty string "".
4. Extract tasks only if they are concrete and user-actionable.
5. Do not create tasks for vague wishes, emotions, or observations alone.
6. Avoid duplicates or near-duplicates in tasks and insights.
7. Use neutral, objective, supportive language. Do not sound clinical, dramatic, or preachy.
8. project_suggestion should be "inbox" unless the text clearly references an existing project.

VALID EXAMPLE
{
  "summary": "User reflected on work stress and identified need for better boundaries.",
  "mood": "frustrated",
  "emotions": ["stress", "overwhelm", "determination"],
  "tasks": [
    {"title": "Set calendar boundaries for deep work blocks", "project_suggestion": "inbox"},
    {"title": "Email team about delegation opportunities", "project_suggestion": "work"}
  ],
  "insights": [
    "User's stress peaks mid-week when meetings cluster.",
    "Delegation is a recurring theme and may be a growth area."
  ]
}"#
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_input(mode: ChatMode) -> PromptInput {
        PromptInput {
            mode,
            overdue_tasks: vec![],
            today_tasks: vec![],
            upcoming_tasks: vec![],
            semantic_memory: vec![],
            recent_patterns: vec![],
            related_journal: vec![],
            current_entry: None,
            pinned_points: vec![],
        }
    }

    #[test]
    fn chat_prompt_includes_companion_identity_rules() {
        let prompt = build_system_prompt(&empty_input(ChatMode::Chat));

        assert!(prompt.contains("COMPANION IDENTITY"));
        assert!(prompt.contains("Never open with praise or filler"));
        assert!(prompt.contains("Ask at most one follow-up question"));
        assert!(prompt.contains(
            "Never generate tasks, schedules, or productivity advice unless the user asks"
        ));
    }

    #[test]
    fn weekly_plan_prompt_mentions_realistic_tradeoffs() {
        let prompt = build_system_prompt(&empty_input(ChatMode::WeeklyPlan));

        assert!(prompt.contains("Help the user review the week realistically"));
        assert!(prompt.contains("Name tradeoffs, constraints, and overload honestly"));
    }

    #[test]
    fn prompt_renders_memory_blocks_when_present() {
        let mut input = empty_input(ChatMode::Chat);
        input.semantic_memory = vec!["Journal cadence: 4 entries in the last 7 days.".into()];
        input.recent_patterns = vec!["Dominant recent mood signal: reflective.".into()];
        input.related_journal = vec!["2026-03-20 | Morning Reset | mood: reflective".into()];
        input.current_entry = Some("Title: Today\nBody preview:\nTesting".into());

        let prompt = build_system_prompt(&input);

        assert!(prompt.contains("SEMANTIC MEMORY"));
        assert!(prompt.contains("Journal cadence: 4 entries in the last 7 days."));
        assert!(prompt.contains("RECENT JOURNAL PATTERNS"));
        assert!(prompt.contains("Dominant recent mood signal: reflective."));
        assert!(prompt.contains("RELATED JOURNAL MEMORY"));
        assert!(prompt.contains("2026-03-20 | Morning Reset | mood: reflective"));
        assert!(prompt.contains("CURRENT JOURNAL ENTRY BEING VIEWED"));
        assert!(prompt.contains("Title: Today"));
    }
}
