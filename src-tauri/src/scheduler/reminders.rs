use chrono::{Local};
use rusqlite::Connection;
use tauri::AppHandle;
use crate::error::AppError;

pub fn poll_reminders(app: &AppHandle, conn: &Connection) -> Result<(), AppError> {
    let now = Local::now().to_rfc3339();
    
    // Find tasks that have a reminder in the past, haven't fired,
    // aren't deleted, and aren't done/cancelled/idea (D-113: idea exclusion).
    let mut stmt = conn.prepare(
        "SELECT id, title, reminder_at FROM tasks 
         WHERE is_deleted = 0 
           AND reminder_fired = 0 
           AND reminder_at IS NOT NULL 
           AND reminder_at <= ?1
           AND status NOT IN ('done', 'cancelled', 'idea')"
    )?;

    let rows = stmt.query_map([&now], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut tasks_to_remind = Vec::new();
    for row in rows {
        if let Ok(task) = row {
            tasks_to_remind.push(task);
        }
    }

    for (id, title) in tasks_to_remind {
        use tauri_plugin_notification::NotificationExt;
        
        // Use Tauri Notification Plugin to show system notification
        let _ = app.notification()
            .builder()
            .title("Task Reminder")
            .body(&title)
            .show();
            
        // Mark as fired
        conn.execute(
            "UPDATE tasks SET reminder_fired = 1, updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, id],
        )?;
        
        crate::events::emit(app, crate::events::AppEvent::TaskUpdated { id });
    }

    Ok(())
}
