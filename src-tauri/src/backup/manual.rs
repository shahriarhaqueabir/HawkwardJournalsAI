use crate::error::AppError;
use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;
use chrono::Utc;

pub fn perform_backup(conn: &Connection) -> Result<String, AppError> {
    let data_dir = crate::db::paths::resolve_data_dir();
    let backup_dir = data_dir.join("backups");
    perform_backup_to_dir(conn, backup_dir)
}

pub fn perform_backup_to_dir(conn: &Connection, backup_dir: PathBuf) -> Result<String, AppError> {
    if !backup_dir.exists() {
        fs::create_dir_all(&backup_dir).map_err(|e| AppError::Io(e.to_string()))?;
    }

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let backup_path = backup_dir.join(format!("manual_backup_{}.db", timestamp));

    let mut backup_conn = Connection::open(&backup_path)?;
    let backup = rusqlite::backup::Backup::new(conn, &mut backup_conn)?;
    
    backup.run_to_completion(5, std::time::Duration::from_millis(10), None)?;

    Ok(backup_path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_perform_backup_creates_file() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute("CREATE TABLE t (id TEXT)", []).unwrap();
        
        let dir = tempdir().unwrap();
        let path = perform_backup_to_dir(&conn, dir.path().to_path_buf()).unwrap();
        
        assert!(fs::metadata(path).is_ok());
    }
}
