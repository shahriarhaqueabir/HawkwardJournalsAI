use serde_json::json;

pub fn get_tools() -> Vec<serde_json::Value> {
    vec![
        json!({
            "type": "function",
            "function": {
                "name": "create_task",
                "description": "Create a new task in the user's task manager.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "title": { "type": "string", "description": "The title of the task" },
                        "priority": { "type": "string", "description": "Priority level: low, medium, high, urgent", "enum": ["low", "medium", "high", "urgent"] },
                        "due_date": { "type": "string", "description": "Optional due date in YYYY-MM-DD format" }
                    },
                    "required": ["title"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "update_task",
                "description": "Update an existing task.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "The ID of the task to update" },
                        "title": { "type": "string", "description": "New title for the task" },
                        "status": { "type": "string", "description": "New status: todo, in_progress, done", "enum": ["todo", "in_progress", "done"] },
                        "priority": { "type": "string", "description": "New priority: low, medium, high, urgent", "enum": ["low", "medium", "high", "urgent"] }
                    },
                    "required": ["id"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "complete_task",
                "description": "Mark a specific task as done.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "The ID of the task to mark as completed" }
                    },
                    "required": ["id"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "list_tasks",
                "description": "List existing tasks.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "status": { "type": "string", "description": "Filter by status: todo, in_progress, done. Omit for all pending tasks." }
                    }
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "search_journal",
                "description": "Search the user's past journal entries.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "The search query or keyword" }
                    },
                    "required": ["query"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "fetch_url",
                "description": "Fetch text content from a web URL.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "url": { "type": "string", "description": "The full HTTP/HTTPS URL to fetch" }
                    },
                    "required": ["url"]
                }
            }
        }),
    ]
}
