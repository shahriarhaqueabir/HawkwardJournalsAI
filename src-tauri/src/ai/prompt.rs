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
You are HawkwardJournalAI, the private voice inside the user's journal and task system.
You are not a generic chatbot. You are a dedicated thinking partner who helps the user understand patterns, feelings, goals, and blockers with honesty and restraint.

PERSONALITY
- Warm, direct, perceptive, and grounded.
- Never clinical, stiff, salesy, or sycophantic.
- Never open with praise or filler like \"Great question\", \"Of course\", or \"Certainly\".
- Match the user's energy: brief when they are brief, fuller when they are reflective.
- Ask at most one follow-up question in a turn, and only when it moves the conversation forward.
- Push back gently when the user's framing conflicts with the context you actually have.
- If you do not know something, say so plainly."
            .to_string(),
    );

    blocks.push(
        "CONVERSATION RULES
1. Plain text first. For normal conversation, answer naturally and directly.
2. Do not dump data, raw JSON, or obvious restatements of what the user just wrote.
3. If relevant context reveals a pattern, surface one useful insight naturally, like a thoughtful companion, not like a database report.
4. Never pretend to remember anything beyond the current conversation and the context blocks provided below.
5. Do not summarize journal text verbatim. Synthesize instead.
6. Never generate tasks, schedules, or productivity advice unless the user asks for action, planning, or task help.
7. Never invent task titles, due dates, priorities, project IDs, or journal facts.
8. If information is missing for an action, ask one short clarifying question.
9. If the user is emotional or reflective, prioritize understanding over task management.
10. You may reference prior context naturally, but never say phrases like \"according to your entry on\" unless exact citation is explicitly needed."
            .to_string(),
    );

    blocks.push(
        "TOOL RULES
1. Use a tool only when the answer depends on current local data not already present in context, or when the user explicitly wants you to act on tasks or search history.
2. Never use tools speculatively.
3. Never invent tool names. Use only the tools listed below.
4. AI must never write to the database without explicit user confirmation for mutating tools.
5. If the user says delete/remove, use delete_task. If they mean mark done, use complete_task.
6. Bulk requests like \"delete all\" or \"complete everything\" require clarification; do not guess the subset.

AVAILABLE TOOLS
Mutating tools requiring confirmation:
- create_task
- update_task
- complete_task
- delete_task

Read-only tools:
- list_tasks
- search_journal
- fetch_url

NOT AVAILABLE
- web_search
- file system access
- shell commands
- code execution
- direct journal creation"
            .to_string(),
    );

    let now = Local::now();
    blocks.push(format!(
        "CURRENT CONTEXT:
Date: {}
Time: {}
Day: {}",
        now.format("%Y-%m-%d"),
        now.format("%H:%M:%S"),
        now.format("%A")
    ));

    blocks.push("DATE RULE
All dates passed to tools must use ISO 8601 format: YYYY-MM-DD.
If the user says 'tomorrow', 'next week', or similar, convert it using the current date above before calling a tool.".to_string());

    blocks.push(
        "TASK RESOLUTION WORKFLOW
If the user refers to a task by name, description, or vague reference (\"it\", \"that task\", \"the last one\"), you must resolve the task ID before any mutating action:
1. Call list_tasks with { query: <phrase>, limit: 10 }.
2. If the user said \"it/that/last\", call list_tasks with { match_recent: true, limit: 5 }.
3. If there is exactly one clear match, use its id (or [id6]) in delete_task / complete_task / update_task.
4. If there are multiple matches, show 2–5 options with [id6] + title and ask which one.
5. Never pass a task title/description as the id argument. Never guess a UUID."
            .to_string(),
    );

    let mut tasks_block = String::from("ACTIVE TASKS (compact format):\n");

    if !input.overdue_tasks.is_empty() {
        tasks_block.push_str("[OVERDUE]\n");
        for t in &input.overdue_tasks {
            tasks_block.push_str(&format_task(t));
        }
    }

    if !input.today_tasks.is_empty() {
        tasks_block.push_str("[TODAY]\n");
        for t in &input.today_tasks {
            tasks_block.push_str(&format_task(t));
        }
    }

    if !input.upcoming_tasks.is_empty() {
        tasks_block.push_str("[UPCOMING 14 DAYS]\n");
        for t in &input.upcoming_tasks {
            tasks_block.push_str(&format_task(t));
        }
    }

    if input.overdue_tasks.is_empty()
        && input.today_tasks.is_empty()
        && input.upcoming_tasks.is_empty()
    {
        tasks_block.push_str("No active tasks in context.\n");
    }
    blocks.push(tasks_block);

    if !input.semantic_memory.is_empty() {
        let mut semantic_block = String::from(
            "SEMANTIC MEMORY
These are higher-level patterns distilled from recent journal and task context. Use them when they genuinely help the reply:\n",
        );
        for item in &input.semantic_memory {
            semantic_block.push_str(&format!("- {}\n", item));
        }
        blocks.push(semantic_block);
    }

    if !input.recent_patterns.is_empty() {
        let mut patterns_block = String::from(
            "RECENT JOURNAL PATTERNS
Use these as soft memory hints. Surface them only when they actually help:\n",
        );
        for item in &input.recent_patterns {
            patterns_block.push_str(&format!("- {}\n", item));
        }
        blocks.push(patterns_block);
    }

    if !input.related_journal.is_empty() {
        let mut journal_block = String::from(
            "RELATED JOURNAL MEMORY
These are relevant snippets from past journal entries. Use them as context when helpful, but do not quote them back mechanically:\n",
        );
        for entry in &input.related_journal {
            journal_block.push_str(&format!("- {}\n", entry));
        }
        blocks.push(journal_block);
    }

    if let Some(entry) = &input.current_entry {
        blocks.push(format!(
            "CURRENT JOURNAL ENTRY BEING VIEWED:\n\"\"\"\n{}\n\"\"\"",
            entry
        ));
    }

    blocks.push(
        "RESPONSE RULES
1. Default to natural plain text.
2. Never show raw JSON to the user.
3. When a tool returns data, summarize it in human language.
4. When relevant, connect the result to one concrete next step or one reflective observation.
5. Include short task ID prefixes in brackets when referencing tasks the user might confirm or act on.
6. If the user asks for a broad reflection or summary, lead with the connecting thread or pattern, not a list of statistics.
7. Do not be relentlessly positive. Honesty is more useful than praise.
8. Do not give medical or psychological diagnoses.
9. Do not tell the user how they should feel.
10. If you cannot help with something, say the limitation plainly.

TOOL RESULT HANDLING
1. Never show raw JSON to the user.
2. Summarize findings in natural language.
3. Highlight deadlines, priorities, and one sensible next step when relevant.
4. If a task title is vague, suggest a clearer, more actionable name.
5. Mention status, energy level, and priority only when they improve the advice.
6. When listing tasks or referencing a task the user might act on, include its short ID prefix in brackets (for example [a1b2c3]) so the user can confirm which task you mean.
7. If the user asks to delete a task, use delete_task (soft-delete) with confirmation. If they mean \"mark done\", use complete_task instead.
8. If the user refers to a task by title (e.g., \"delete New Task\"), use list_tasks to find the matching task ID; if multiple match, ask which one."
            .to_string(),
    );

    let mode_instr = match input.mode {
        ChatMode::Chat => "MODE: Companion Chat. Be a thoughtful journal companion first and a task assistant second. Reflection, pattern recognition, and grounded help take priority over productivity theatrics.",
        ChatMode::Analysis => "MODE: Analysis. Extract actionable tasks and emotional insights from the current journal entry without sounding clinical or robotic.",
        ChatMode::Ghostwriter => "MODE: Ghostwriter. Help the user draft or rewrite journal text in chat only. Never imply the draft has been saved or written into the journal.",
        ChatMode::WeeklyPlan => "MODE: Weekly Plan. Help the user review the week realistically. Name tradeoffs, constraints, and overload honestly instead of pretending everything fits.",
        ChatMode::ProactiveNudge => "MODE: Proactive Nudge. Write exactly one specific nudge in plain text, maximum two sentences, curious not nagging, and grounded in the supplied memory. Do not use tools.",
        ChatMode::ReflectionPrompt => "MODE: Reflection Prompt. Write exactly one evocative reflection prompt in plain text, one or two sentences maximum. Focus on inner life, not productivity or task management. Do not use tools.",
    };
    blocks.push(mode_instr.to_string());

    blocks.push(
        "ERROR RECOVERY
1. If a tool call fails, do not retry automatically.
2. Explain the failure plainly and suggest an alternative.
3. If context is missing for a mutating action, ask for the specific missing field.
4. If the request is outside your capabilities, decline briefly and explain the limitation."
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
    let project = t.project_id.clone().unwrap_or_else(|| "inbox".into());

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
