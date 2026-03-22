use crate::error::AppError;
use rusqlite::Connection;
use serde_json::json;
use std::fs;
use chrono::Utc;

use std::path::PathBuf;

pub fn export_to_json(conn: &Connection) -> Result<String, AppError> {
    let data_dir = crate::db::paths::resolve_data_dir();
    let export_dir = data_dir.join("exports");
    export_to_json_to_dir(conn, export_dir)
}

pub fn export_to_json_to_dir(conn: &Connection, export_dir: PathBuf) -> Result<String, AppError> {
    if !export_dir.exists() {
        fs::create_dir_all(&export_dir).map_err(|e| AppError::Io(e.to_string()))?;
    }

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let export_path = export_dir.join(format!("hawkward_export_{}.json", timestamp));

    let journal = crate::db::journal::list_recent_entries(conn, 1000, None)?;
    let tasks = crate::db::tasks::list_tasks(conn, crate::db::tasks::TaskListFilters::default())?;
    // Note: projects might not exist or list_projects might return empty
    let projects = crate::db::projects::list_projects(conn).unwrap_or_default();

    let data = json!({
        "version": "1.0",
        "exported_at": Utc::now().to_rfc3339(),
        "journal": journal,
        "tasks": tasks,
        "projects": projects
    });

    let json_str = serde_json::to_string_pretty(&data).map_err(|e| AppError::InvalidInput(e.to_string()))?;
    fs::write(&export_path, json_str).map_err(|e| AppError::Io(e.to_string()))?;

    Ok(export_path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_export_to_json_creates_file() {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrations::run_pending(&mut conn).unwrap();
        
        let dir = tempdir().unwrap();
        let path = export_to_json_to_dir(&conn, dir.path().to_path_buf()).unwrap();
        
        assert!(fs::metadata(path).is_ok());
    }
}
