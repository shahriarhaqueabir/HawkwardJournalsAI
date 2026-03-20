-- Migration 007: Analytical View (Phase 4 Ready)
-- Decision: D-129 (Emotional Trend View)
BEGIN;

-- 1. Explode emotions into a flat list for SQL aggregations
-- Usage: SELECT emotion, count(*) FROM journal_emotions_flat GROUP BY emotion
CREATE VIEW IF NOT EXISTS journal_emotions_flat AS
SELECT 
    je.id as entry_id, 
    je.created_at,
    json_each.value as emotion
FROM journal_entries je, json_each(je.emotions)
WHERE je.is_deleted = 0;

-- 2. Project Performance Index (Optional but useful for UI)
CREATE VIEW IF NOT EXISTS project_status_distribution AS
SELECT 
    p.id as project_id,
    p.name as project_name,
    t.status,
    count(t.id) as task_count
FROM projects p
LEFT JOIN tasks t ON t.project_id = p.id
WHERE p.is_deleted = 0 AND t.is_deleted = 0
GROUP BY p.id, t.status;

COMMIT;
