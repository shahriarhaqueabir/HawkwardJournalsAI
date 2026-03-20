use crate::error::AppError;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub color: Option<String>,
    pub goal_date: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub fn create_project(conn: &Connection, project: &Project) -> Result<String, AppError> {
    conn.execute(
        "INSERT INTO projects (id, name, description, status, color, goal_date, created_at, updated_at) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            project.id,
            project.name,
            project.description,
            project.status,
            project.color,
            project.goal_date,
            project.created_at,
            project.updated_at,
        ],
    )?;
    Ok(project.id.clone())
}

pub fn list_projects(conn: &Connection) -> Result<Vec<Project>, AppError> {
    let mut stmt = conn.prepare("SELECT id, name, description, status, color, goal_date, created_at, updated_at FROM projects WHERE is_deleted = 0 ORDER BY name ASC")?;
    let rows = stmt.query_map([], |row| {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            status: row.get(3)?,
            color: row.get(4)?,
            goal_date: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?;

    let mut projects = Vec::new();
    for project in rows {
        projects.push(project?);
    }
    Ok(projects)
}

pub fn get_project(conn: &Connection, id: &str) -> Result<Option<Project>, AppError> {
    let mut stmt = conn.prepare("SELECT id, name, description, status, color, goal_date, created_at, updated_at FROM projects WHERE id = ?1 AND is_deleted = 0")?;
    let project = stmt.query_row(params![id], |row| {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            status: row.get(3)?,
            color: row.get(4)?,
            goal_date: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    });

    match project {
        Ok(p) => Ok(Some(p)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(AppError::Database(e.to_string())),
    }
}

pub fn update_project(conn: &Connection, project: &Project) -> Result<(), AppError> {
    let rows = conn.execute(
        "UPDATE projects
         SET name = ?1, description = ?2, status = ?3, color = ?4, goal_date = ?5, updated_at = ?6
         WHERE id = ?7 AND is_deleted = 0",
        params![
            project.name,
            project.description,
            project.status,
            project.color,
            project.goal_date,
            project.updated_at,
            project.id,
        ],
    )?;

    if rows == 0 {
        return Err(AppError::NotFound("Project not found".into()));
    }

    conn.execute(
        "UPDATE tasks
         SET project = ?1, updated_at = ?2
         WHERE project_id = ?3 AND is_deleted = 0",
        params![project.name, project.updated_at, project.id],
    )?;

    Ok(())
}

pub fn soft_delete_project(conn: &Connection, id: &str, updated_at: &str) -> Result<bool, AppError> {
    if id == "inbox" {
        return Err(AppError::InvalidInput("Inbox project cannot be deleted".into()));
    }

    conn.execute(
        "UPDATE tasks
         SET project_id = 'inbox', project = 'Inbox', updated_at = ?1
         WHERE project_id = ?2 AND is_deleted = 0",
        params![updated_at, id],
    )?;

    let rows = conn.execute(
        "UPDATE projects SET is_deleted = 1, updated_at = ?1 WHERE id = ?2 AND is_deleted = 0",
        params![updated_at, id],
    )?;

    Ok(rows > 0)
}
