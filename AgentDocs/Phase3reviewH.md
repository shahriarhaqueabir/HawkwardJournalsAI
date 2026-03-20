CONTEXT:
Project: HawkwardJournalAI
Stack: Tauri v2 · Rust · rusqlite · Ollama llama3.2 · Vanilla JS
Location: E:\Abir\LocalCodeRepo\HawkwardJournalAI\src-tauri\src\ai\

PROBLEM:
The AI chat is broken in four ways:
1. The model responds to "hi" and "how are you" with invented tool calls (len(), tell_joke)
   because the system prompt says "Do NOT perform any action without a tool call" —
   this tells the model it MUST always use a tool.
2. Tool definitions are sent to Ollama without the required "type": "function" wrapper.
   Ollama silently ignores malformed tools and the model guesses instead.
3. ChatMessage.tool_calls is typed as Option<Value> instead of Option<Vec<OllamaToolCall>>.
   Ollama returns tool calls as a typed array — the current struct cannot deserialise them.
4. fallback.rs pattern 3 matches analysis JSON responses (which have a "name" key from
   task objects) and misidentifies them as tool calls, producing "Unknown tool: " errors.

TASK:
Apply the following four targeted fixes. Do not change anything else.

═══════════════════════════════════════════════════════════════
FIX 1 — src\ai\prompt.rs
═══════════════════════════════════════════════════════════════

Find BLOCK 2 — CORE RULES. Replace the entire push() call with:

    blocks.push("CORE RULES:
- You are a conversational assistant first. Respond in plain text for
  greetings, questions, advice, and general conversation.
- ONLY use a tool call when the user explicitly asks you to create,
  update, complete, or search tasks or journal entries.
- NEVER call a tool for greetings, small talk, or general questions.
  Just reply in plain text.
- NEVER invent tool names. Your only tools are:
  create_task, update_task, complete_task, list_tasks,
  search_journal, fetch_url.
- User confirmation is REQUIRED before any database write.
- All data is local and private. Never mention cloud or sync.
- Keep responses concise and direct.".to_string());

═══════════════════════════════════════════════════════════════
FIX 2 — src\ai\tools.rs
═══════════════════════════════════════════════════════════════

Add this new function anywhere in the file. Do NOT remove
get_tool_definitions() — it may be used elsewhere.

pub fn get_tools_for_ollama() -> serde_json::Value {
    serde_json::json!([
        {
            "type": "function",
            "function": {
                "name": "create_task",
                "description": "Propose a new task. ONLY call when user explicitly asks to create a task. Requires confirmation.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "title":        { "type": "string" },
                        "priority":     { "type": "string", "enum": ["low","medium","high","urgent"] },
                        "due_date":     { "type": "string", "description": "YYYY-MM-DD" },
                        "energy_level": { "type": "string", "enum": ["deep_focus","light","admin","errand"] },
                        "context_tag":  { "type": "string", "enum": ["computer","phone","errands","home","anywhere"] }
                    },
                    "required": ["title"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "update_task",
                "description": "Modify an existing task. ONLY call when user explicitly asks to update a task. Requires confirmation.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "id":       { "type": "string" },
                        "title":    { "type": "string" },
                        "status":   { "type": "string", "enum": ["idea","todo","in_progress","done","cancelled"] },
                        "priority": { "type": "string", "enum": ["low","medium","high","urgent"] },
                        "due_date": { "type": "string" }
                    },
                    "required": ["id"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "complete_task",
                "description": "Mark a task as done. ONLY call when user explicitly asks to complete a task. Requires confirmation.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" }
                    },
                    "required": ["id"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_tasks",
                "description": "Retrieve tasks. Use when user asks what tasks they have.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "status":   { "type": "string" },
                        "priority": { "type": "string" }
                    }
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "search_journal",
                "description": "Search journal entries. Use when user asks about past journal entries.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query":     { "type": "string" },
                        "date_from": { "type": "string" },
                        "date_to":   { "type": "string" }
                    },
                    "required": ["query"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "fetch_url",
                "description": "Fetch content from a URL. Use only when user provides a specific URL.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "url": { "type": "string" }
                    },
                    "required": ["url"]
                }
            }
        }
    ])
}

Then find every call to get_tool_definitions() in the codebase
(search lib.rs, client.rs, and any command handler).
Replace each call with get_tools_for_ollama().
The old function can stay — just stop calling it.

═══════════════════════════════════════════════════════════════
FIX 3 — src\ai\client.rs
═══════════════════════════════════════════════════════════════

Replace the existing ChatMessage struct and add two new structs.
Find:

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct ChatMessage {
        pub role: String,
        pub content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub tool_calls: Option<Value>,
    }

Replace with:

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct OllamaToolFunction {
        pub name: String,
        pub arguments: serde_json::Value,
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct OllamaToolCall {
        pub function: OllamaToolFunction,
    }

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct ChatMessage {
        pub role: String,
        #[serde(default)]
        pub content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub tool_calls: Option<Vec<OllamaToolCall>>,
    }

The #[serde(default)] on content is required — Ollama omits the
content field entirely when returning a tool call response.
Without the default, deserialisation panics on tool call chunks.

═══════════════════════════════════════════════════════════════
FIX 4 — src\ai\fallback.rs
═══════════════════════════════════════════════════════════════

Replace the val_to_call function at the bottom of the file:

    fn val_to_call(v: &Value) -> Option<ExtractedToolCall> {
        let name = v.get("name")?.as_str()?.to_string();

        // Guard 1: empty name = malformed response, never a valid tool call
        if name.is_empty() {
            return None;
        }

        // Guard 2: analysis JSON contains "summary" or "mood" —
        // these are journal analysis responses, not tool calls.
        // Pattern 3 (raw JSON match) would otherwise misidentify them.
        if v.get("summary").is_some() || v.get("mood").is_some() {
            return None;
        }

        // Guard 3: only match known tool names to prevent false positives
        let known_tools = [
            "create_task", "update_task", "complete_task",
            "list_tasks", "search_journal", "fetch_url",
        ];
        if !known_tools.contains(&name.as_str()) {
            return None;
        }

        let arguments = v.get("arguments").cloned()
            .or_else(|| v.get("parameters").cloned())
            .unwrap_or(Value::Object(serde_json::Map::new()));

        Some(ExtractedToolCall { name, arguments })
    }

═══════════════════════════════════════════════════════════════
VERIFICATION
═══════════════════════════════════════════════════════════════

After applying all four fixes:

1. Run: cargo build
   Expected: compiles with warnings only, zero errors.

2. Run: cargo tauri dev
   Expected: app opens, AI sidebar loads.

3. Test in AI sidebar: type "hi"
   Expected: plain text greeting response, no tool call.

4. Test: type "how are you"
   Expected: plain text response, no JSON output.

5. Test: type "create a task called buy milk"
   Expected: tool call card appears with confirm/cancel buttons.

6. Check terminal output — you should NOT see:
   [AI] Tool execution failed: InvalidInput("Unknown tool: ")

CONSTRAINTS:
- Only modify the four files listed above.
- Do not refactor anything else.
- Do not add new dependencies.
- Do not change the analysis pipeline (analyze_journal uses
  api/generate, not api/chat — leave it completely untouched).