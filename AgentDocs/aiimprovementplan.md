This is a context and resolution problem. The AI needs to resolve what the user means before calling a mutating tool.

## Project-aligned solution (no new tools)
HawkwardJournalAI intentionally keeps a small fixed tool surface (D-111). Instead of adding a new `find_task` tool, extend the existing read-only `list_tasks` tool so the model can safely resolve tasks by **name**, **keyword**, or **recency** before calling `delete_task`, `complete_task`, or `update_task`.

## The core problem
When the user says “delete that shopping task”, the model may call `delete_task` with `id: "shopping task"` (a title string, not an ID). The backend correctly rejects it.

## Fix 1 — Extend `list_tasks` for task resolution
Add optional arguments to `list_tasks`:
- `query` (string): case-insensitive substring match against `title`, `description`, and `notes`.
- `match_recent` (boolean): if true, sort by `created_at DESC` (useful for “that one” / “the last task”).
- `limit` (integer): cap results for disambiguation (recommended 5–10).

## Fix 2 — Teach a strict resolution workflow in the system prompt
When the user refers to a task by name, description, or vague reference (“it”, “that task”, “the last one”), resolve the task **before** any mutating tool call:
1. Call `list_tasks` with `{ query: <phrase>, limit: 10 }`.
2. If the user said “it/that/last”, call `list_tasks` with `{ match_recent: true, limit: 5 }`.
3. If exactly one clear match → use its `id` (or `[id6]` prefix) in `delete_task` / `complete_task` / `update_task`.
4. If multiple matches → show 2–5 options with `[id6] + title` and ask which one.
5. Never pass a title/description as the `id` argument. Never guess a UUID.

## Fix 3 — Fuzzy matching (LIKE) in the list query
Implement `query` filtering as:
- `LOWER(title) LIKE %query%`
- `LOWER(COALESCE(description,'')) LIKE %query%`
- `LOWER(COALESCE(notes,'')) LIKE %query%`

## Example conversation
User: delete that shopping task  
AI: (calls `list_tasks` `{ query: "shopping", limit: 10 }`)  
AI: I found 2 matches: `[a1b2c3] Buy groceries`, `[d4e5f6] Order pantry staples`. Which one?  
User: the groceries one  
AI: (calls `delete_task` with `id: "a1b2c3"`) → confirmation card appears  
User: yes  
AI: Deleted.
