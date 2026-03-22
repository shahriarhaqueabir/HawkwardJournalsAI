use crate::error::AppError;
use chrono::Utc;
use rusqlite::{params, Connection};

struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "initial_schema",
        sql: include_str!("../../migrations/001_initial.sql"),
    },
    Migration {
        version: 2,
        name: "fts_triggers",
        sql: include_str!("../../migrations/002_fts_triggers.sql"),
    },
    Migration {
        version: 3,
        name: "analysis_tracking",
        sql: include_str!("../../migrations/003_analysis_tracking.sql"),
    },
    Migration {
        version: 4,
        name: "project_hierarchy",
        sql: include_str!("../../migrations/004_field_expansion.sql"),
    },
    Migration {
        version: 5,
        name: "settings_v16",
        sql: include_str!("../../migrations/005_settings_v16.sql"),
    },
    Migration {
        version: 7,
        name: "analytical_reports",
        sql: include_str!("../../migrations/007_analytical_reports.sql"),
    },
    Migration {
        version: 8,
        name: "ai_analysis_fields",
        sql: include_str!("../../migrations/008_ai_analysis_fields.sql"),
    },
    Migration {
        version: 9,
        name: "fix_proposed_outcome",
        sql: include_str!("../../migrations/009_fix_proposed_outcome.sql"),
    },
    Migration {
        version: 10,
        name: "ai_pinned_memory",
        sql: include_str!("../../migrations/010_ai_pinned_memory.sql"),
    },
];

fn is_begin_transaction_line(line: &str) -> bool {
    let before_comment = line.split_once("--").map_or(line, |(left, _)| left);
    match before_comment.trim().to_ascii_uppercase().as_str() {
        "BEGIN;" | "BEGIN TRANSACTION;" | "BEGIN DEFERRED;" | "BEGIN IMMEDIATE;"
        | "BEGIN EXCLUSIVE;" => true,
        _ => false,
    }
}

fn is_commit_transaction_line(line: &str) -> bool {
    let before_comment = line.split_once("--").map_or(line, |(left, _)| left);
    before_comment.trim().eq_ignore_ascii_case("COMMIT;")
}

fn strip_outer_transaction(sql: &str) -> String {
    let lines: Vec<&str> = sql.lines().collect();
    let begin_index = lines
        .iter()
        .position(|line| is_begin_transaction_line(line));
    let commit_index = lines
        .iter()
        .rposition(|line| is_commit_transaction_line(line));

    match (begin_index, commit_index) {
        (Some(begin), Some(commit)) if begin < commit => lines
            .into_iter()
            .enumerate()
            .filter(|(idx, _)| *idx != begin && *idx != commit)
            .map(|(_, line)| line)
            .collect::<Vec<&str>>()
            .join("\n"),
        _ => sql.to_string(),
    }
}

fn ensure_no_explicit_transactions(sql: &str, migration_name: &str) -> Result<(), AppError> {
    for line in sql.lines() {
        if is_begin_transaction_line(line) || is_commit_transaction_line(line) {
            return Err(AppError::InvalidInput(format!(
                "Migration '{migration_name}' contains explicit BEGIN/COMMIT; migration runner wraps each migration in a rusqlite transaction so SQL files must not contain BEGIN/COMMIT."
            )));
        }
    }
    Ok(())
}

pub fn run_pending(conn: &mut Connection) -> Result<(), AppError> {
    // 1. Ensure migration tracking table exists
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version     INTEGER PRIMARY KEY,
            name        TEXT NOT NULL,
            applied_at  TEXT NOT NULL
        );",
    )?;

    // 2. Fetch applied versions (Fixed E0597: collect to Vec to drop stmt early)
    let applied: Vec<i64> = {
        let mut stmt = conn.prepare("SELECT version FROM schema_migrations ORDER BY version")?;
        let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
        let mut versions = Vec::new();
        for row in rows {
            versions.push(row?);
        }
        versions
    };

    // 3. Apply pending migrations atomically
    for m in MIGRATIONS {
        if !applied.contains(&m.version) {
            println!("[DB] Applying migration {}: {}", m.version, m.name);
            // We use a transaction to ensure D-97 compliance
            let tx = conn.transaction()?;
            let sql = strip_outer_transaction(m.sql);
            ensure_no_explicit_transactions(&sql, m.name)?;
            tx.execute_batch(&sql)?;
            tx.execute(
                "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
                params![m.version, m.name, Utc::now().to_rfc3339()],
            )?;
            tx.commit()?;
        }
    }
    Ok(())
}

pub fn reset_database(conn: &mut Connection) -> Result<(), AppError> {
    // 1. Get all table names (excluding sqlite_ sequences)
    let table_names: Vec<String> = {
        let mut stmt = conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
        )?;
        let names = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .filter_map(|name| name.ok())
            .collect();
        names
    };

    // 2. Disable foreign keys temporarily to drop tables reliably
    conn.execute("PRAGMA foreign_keys = OFF", [])?;

    // 3. Drop all tables
    for table in table_names {
        conn.execute(&format!("DROP TABLE IF EXISTS \"{}\"", table), [])?;
    }

    // 4. Re-enable foreign keys
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    // 5. Re-run migrations
    run_pending(conn)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn strip_outer_transaction_removes_begin_and_commit_lines() {
        let sql = r#"
-- comment
BEGIN;
CREATE TABLE test (id TEXT);
COMMIT;
"#;
        let stripped = strip_outer_transaction(sql);
        assert!(!stripped.lines().any(is_begin_transaction_line));
        assert!(!stripped.lines().any(is_commit_transaction_line));
        assert!(stripped.contains("CREATE TABLE test"));
    }

    #[test]
    fn strip_outer_transaction_keeps_trigger_begin_block() {
        let sql = r#"
BEGIN;
CREATE TRIGGER tr AFTER INSERT ON t
BEGIN
  SELECT 1;
END;
COMMIT;
"#;
        let stripped = strip_outer_transaction(sql);
        assert!(stripped.contains("CREATE TRIGGER"));
        assert!(stripped.lines().any(|line| line.trim() == "BEGIN"));
        assert!(!stripped.lines().any(is_begin_transaction_line));
        assert!(!stripped.lines().any(is_commit_transaction_line));
    }

    #[test]
    fn run_pending_applies_all_migrations_in_memory() {
        let mut conn = Connection::open_in_memory().unwrap();
        run_pending(&mut conn).unwrap();
        let applied_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(applied_count, MIGRATIONS.len() as i64);
    }

    #[test]
    fn reset_database_wipes_and_restores_schema() {
        let mut conn = Connection::open_in_memory().unwrap();
        run_pending(&mut conn).unwrap();

        // Add some dummy data
        conn.execute("INSERT INTO notebooks (id, name, created_at, updated_at) VALUES ('test', 'Test', '2023-01-01', '2023-01-01')", []).unwrap();

        // Reset
        reset_database(&mut conn).unwrap();

        // Check if dummy data is gone
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM notebooks", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);

        // Check if schema is still there (migrations re-applied)
        let migration_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(migration_count, MIGRATIONS.len() as i64);
    }
}
