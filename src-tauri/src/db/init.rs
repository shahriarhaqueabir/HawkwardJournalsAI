use crate::db::{migrations, paths, settings};
use crate::error::AppError;
use rusqlite::Connection;
use graphqlite::Graph;
use std::time::Duration;

pub fn initialise() -> Result<Connection, AppError> {
    let data_dir = paths::resolve_data_dir();
    let db_path = data_dir.join("hawkward.db");

    let mut conn = Connection::open(&db_path)?;

    // PRAGMA Fix for journal_mode WAL (D-01)
    conn.pragma_update_and_check(None, "journal_mode", "WAL", |row| {
        let mode: String = row.get(0)?;
        if mode.to_uppercase() != "WAL" {
            return Err(rusqlite::Error::ExecuteReturnedResults);
        }
        Ok(())
    })?;

    conn.pragma_update(None, "foreign_keys", 1)?;
    conn.pragma_update(None, "synchronous", 1)?;
    conn.pragma_update(None, "temp_store", 2)?;
    conn.busy_timeout(Duration::from_millis(5000))?;

    migrations::run_pending(&mut conn)?;
    settings::seed_defaults(&conn)?;

    println!("[DB] Initialised at {:?}", db_path);
    Ok(conn)
}
