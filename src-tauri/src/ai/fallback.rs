use regex::Regex;
use serde_json::Value;

pub struct ExtractedToolCall {
    pub name: String,
    pub arguments: Value,
}

pub fn extract_tool_calls(text: &str) -> Vec<ExtractedToolCall> {
    let mut results = Vec::new();

    // Pattern 1: <tool_call>{...}</tool_call>
    let tag_re = Regex::new(r"<tool_call>(?s)(.*?)</tool_call>").unwrap();
    for cap in tag_re.captures_iter(text) {
        if let Ok(val) = serde_json::from_str::<Value>(&cap[1]) {
            if let Some(call) = val_to_call(&val) {
                results.push(call);
            }
        }
    }
    if !results.is_empty() {
        return results;
    }

    // Pattern 2: ```json {...} ```
    let fence_re = Regex::new(r"```json(?s)(.*?)```").unwrap();
    for cap in fence_re.captures_iter(text) {
        if let Ok(val) = serde_json::from_str::<Value>(&cap[1]) {
            if let Some(call) = val_to_call(&val) {
                results.push(call);
            }
        }
    }
    if !results.is_empty() {
        return results;
    }

    // Pattern 3: First complete JSON object with "name" key
    // This is more complex; we'll look for anything that looks like a JSON object
    let obj_re = Regex::new(r"\{(?s)(.*?)\}").unwrap();
    for cap in obj_re.captures_iter(text) {
        let full_text = format!("{{{}}}", &cap[1]);
        if let Ok(val) = serde_json::from_str::<Value>(&full_text) {
            if let Some(call) = val_to_call(&val) {
                results.push(call);
                return results; // Only take the first one for raw matches (D-38)
            }
        }
    }

    results
}

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
        "create_task",
        "update_task",
        "complete_task",
        "list_tasks",
        "search_journal",
        "fetch_url",
    ];
    if !known_tools.contains(&name.as_str()) {
        return None;
    }

    let arguments = v
        .get("arguments")
        .cloned()
        .or_else(|| v.get("parameters").cloned())
        .unwrap_or(Value::Object(serde_json::Map::new()));

    Some(ExtractedToolCall { name, arguments })
}
