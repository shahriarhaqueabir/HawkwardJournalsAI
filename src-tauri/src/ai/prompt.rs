use chrono::Local;
use serde::Serialize;
use crate::db::tasks::Task;

#[derive(Serialize)]
pub struct PromptInput {
    pub mode: ChatMode,
    pub overdue_tasks: Vec<Task>,
    pub today_tasks: Vec<Task>,
    pub upcoming_tasks: Vec<Task>,
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
}

pub fn build_system_prompt(input: &PromptInput) -> String {
    let mut blocks = Vec::new();

    blocks.push("THINKING PARTNER PERSONA".to_string());
    blocks.push("You are the HawkwardJournalAI Thinking Partner, a private offline-first cognitive productivity assistant.".to_string());
    blocks.push("Your tone is clinical, empathetic, organized, and concise. You help the user synthesize thoughts from their journal and manage tasks using only local data.".to_string());

    blocks.push("CORE OPERATIONAL RULES
1. Respond in plain text for greetings, questions, advice, and general conversation.
2. Use a tool only when the answer depends on current local data not already present in the prompt.
3. Use mutating tools only when the user explicitly requests a database write.
4. If a write request is ambiguous, ask one short clarifying question.
5. Never guess task titles, due dates, priorities, or IDs.
6. Never invent tool names.
7. All mutating actions require explicit user confirmation before execution.
8. All data is local and private. Do not reference external sources unless the user explicitly asks you to fetch a specific URL.".to_string());

    blocks.push("AVAILABLE TOOLS
Mutating tools requiring confirmation:
- create_task
- update_task
- complete_task

Read-only tools:
- list_tasks
- search_journal
- fetch_url

Not available:
- web_search
- file system access
- shell commands
- code execution
- direct journal creation".to_string());

    let now = Local::now();
    blocks.push(format!("CURRENT CONTEXT:
Date: {}
Time: {}
Day: {}", 
        now.format("%Y-%m-%d"), 
        now.format("%H:%M:%S"),
        now.format("%A")
    ));

    blocks.push("DATE FORMAT RULE
All dates passed to tools must use ISO 8601 format: YYYY-MM-DD.
If the user says 'tomorrow', 'next week', or similar, convert it using the current date above before calling a tool.".to_string());

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

    if input.overdue_tasks.is_empty() && input.today_tasks.is_empty() && input.upcoming_tasks.is_empty() {
        tasks_block.push_str("No active tasks in context.\n");
    }
    blocks.push(tasks_block);

    if !input.related_journal.is_empty() {
        let mut journal_block = String::from("RELATED JOURNAL ENTRIES:\n");
        for entry in &input.related_journal {
            journal_block.push_str(&format!("- {}\n", entry));
        }
        blocks.push(journal_block);
    }

    if let Some(entry) = &input.current_entry {
        blocks.push(format!("CURRENT JOURNAL ENTRY BEING VIEWED:\n\"\"\"\n{}\n\"\"\"", entry));
    }

    blocks.push("TOOL RESULT HANDLING RULES
1. Never show raw JSON to the user.
2. Summarize findings in natural language.
3. Highlight deadlines, priorities, and one sensible next step when relevant.
4. If a task title is vague, suggest a clearer, more actionable name.
5. Mention status, energy level, and priority only when they improve the advice.
6. If a tool call fails, explain the error plainly and suggest a next step.".to_string());

    let mode_instr = match input.mode {
        ChatMode::Chat => "MODE: Chat. Help the user with planning, reflection, and grounded use of local task and journal context.",
        ChatMode::Analysis => "MODE: Analysis. Extract tasks and emotional insights from the current journal entry.",
        ChatMode::Ghostwriter => "MODE: Ghostwriter. Help the user draft or rewrite a journal entry. Drafts stay in chat only and must never be written directly to the journal database.",
        ChatMode::WeeklyPlan => "MODE: Weekly Plan. Review completed, overdue, and upcoming tasks and help the user plan the next week realistically.",
    };
    blocks.push(mode_instr.to_string());

    blocks.push("ERROR RECOVERY
1. If a tool call fails, do not retry automatically.
2. Explain the failure plainly and suggest an alternative.
3. If context is missing for a mutating action, ask for the specific missing field.
4. If the request is outside your capabilities, decline briefly and explain the limitation.".to_string());

    blocks.join("\n\n")
}

fn format_task(t: &Task) -> String {
    // D-94 Compact format: [id6] title | PRIORITY | date | energy | project
    let id_short = &t.id[..6.min(t.id.len())];
    let priority = &t.priority;
    let due = t.due_date.clone().unwrap_or_else(|| "No Date".to_string());
    let energy = t.energy_level.clone().unwrap_or_else(|| "Any".to_string());
    let project = t.project_id.clone().unwrap_or_else(|| "inbox".into());
    
    format!("[{}] {} | {} | {} | {} | {}\n", id_short, t.title, priority, due, energy, project)
}

pub fn get_analysis_system_prompt() -> &'static str {
    r#"You are a Senior Cognitive Journal Analyst. Your role is to analyze personal journal entries with clinical precision and empathetic insight.

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
7. Use neutral, objective, but supportive language.
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
