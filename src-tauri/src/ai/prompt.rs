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

    // BLOCK 1 — THINKING PARTNER PERSONA
    blocks.push("You are the HawkwardJournalAI Thinking Partner. You are a private, offline-first productivity assistant.".to_string());
    blocks.push("Your tone is clinical, empathetic, and highly organized. You help the user synthesize thoughts from their journal and manage tasks.".to_string());

    blocks.push("CORE RULES:
- You are the HawkwardJournalAI Thinking Partner. You have access to the user's task and journal database.
- Resolve any general knowledge questions using your internal weights.
- RULES FOR TOOL RESULTS:
  1. NEVER show raw JSON to the user.
  2. INTERPRET the data: If you see many 'todo' tasks, suggest a priority or a next step.
  3. BE CONCISE: Group small tasks. Mention specifically if something is 'In Progress'.
  4. IDENTIFY VAGUENESS: If a task is named 'New Task' or 'test', ask if they'd like to rename it based on their recent entries.
  5. CONTEXTUALIZE: Use 'energy_level' and 'priority' fields to give better advice (e.g., 'Since you have a Deep Focus task...').
- TOOL USAGE: 
  * Use 'list_tasks' for ANY question about what needs to be done.
  * Use 'create_task' if the user mentions a new intent.
- Respond in plain text for greetings, questions, advice, and general conversation.
- ONLY use a tool call when the user explicitly asks you to create, update, complete, or search.
- NEVER invent tool names.
- User confirmation is REQUIRED before any database write.
- All data is local and private. Keep responses concise.".to_string());

    // BLOCK 3 — DATE/TIME
    let now = Local::now();
    blocks.push(format!("CURRENT CONTEXT:
Date: {}
Time: {}
Day: {}", 
        now.format("%Y-%m-%d"), 
        now.format("%H:%M:%S"),
        now.format("%A")
    ));

    // BLOCK 4 — GUARANTEED MINIMUMS (D-56, D-61, D-94)
    let mut tasks_block = String::from("ACTIVE TASKS:\n");
    
    if !input.overdue_tasks.is_empty() {
        tasks_block.push_str("--- OVERDUE ---\n");
        for t in &input.overdue_tasks {
            tasks_block.push_str(&format_task(t));
        }
    }
    
    if !input.today_tasks.is_empty() {
        tasks_block.push_str("--- TODAY ---\n");
        for t in &input.today_tasks {
            tasks_block.push_str(&format_task(t));
        }
    }
    
    if !input.upcoming_tasks.is_empty() {
        tasks_block.push_str("--- UPCOMING (14d) ---\n");
        for t in &input.upcoming_tasks {
            tasks_block.push_str(&format_task(t));
        }
    }
    blocks.push(tasks_block);

    // BLOCK 5 — SMART INJECT (Journal)
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

    // BLOCK 7 — MODE INSTRUCTIONS
    let mode_instr = match input.mode {
        ChatMode::Chat => "MODE: Chat. Help the user with general planning or reflection.",
        ChatMode::Analysis => "MODE: Analysis. Extract tasks and emotional insights from the current journal entry.",
        ChatMode::Ghostwriter => "MODE: Ghostwriter. Help the user draft a journal entry or rewrite thoughts.",
        ChatMode::WeeklyPlan => "MODE: Weekly Plan. Look at the completed/overdue/upcoming tasks and help the user plan their next week.",
    };
    blocks.push(mode_instr.to_string());

    // BLOCK 8 — ISO 8601 DATE RULE
    blocks.push("DATE RULE: Always use ISO 8601 (YYYY-MM-DD) for tool arguments.".to_string());

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
    "You are a Senior Cognitive Journal Analyst. Your role is to analyze personal journal entries with clinical precision and empathetic insight. 

    Analyze the provided text and return a JSON object that MUST strictly adhere to this schema:
    {
      \"summary\": \"A 1-2 sentence high-level summary (max 30 words).\",
      \"mood\": \"One-word sentiment (e.g., joyful, anxious, reflective, frustrated).\",
      \"emotions\": [\"List of identified emotions (max 5)\"],
      \"tasks\": [
        {
          \"title\": \"Extract actionable task title\",
          \"project_suggestion\": \"inbox\"
        }
      ],
      \"insights\": [\"Synthesis of patterns or realizations (max 3)\"]
    }

    Constraints:
    - Output MUST be a single valid JSON object.
    - NO markdown, NO backticks (```json), NO conversational text before or after the JSON.
    - If no data for a field, return an empty array [] or empty string \"\".
    - Use neutral, objective but supportive language."
}
