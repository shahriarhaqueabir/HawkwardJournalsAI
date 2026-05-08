use tauri::AppHandle;
use tauri::Manager;
use chrono::Utc;
use crate::AppState;
use crate::db::tasks::{Task, get_task};
use tauri_plugin_notification::NotificationExt;

pub fn spawn_reminder_worker(handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            
            let state = handle.state::<AppState>();
            let conn = state.conn.lock().await;
            
            // Find tasks with pending reminders
            let now = Utc::now().to_rfc3339();
            let mut stmt = match conn.prepare(
                "SELECT id FROM tasks WHERE reminder_at <= ?1 AND reminder_fired = 0 AND is_deleted = 0 AND status != 'done'"
            ) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let ids: Vec<String> = stmt.query_map([now], |row| row.get(0))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect();

            for id in ids {
                if let Ok(Some(task)) = get_task(&conn, &id) {
                    // Send notification
                    handle.notification()
                        .builder()
                        .title("Task Reminder")
                        .body(&task.title)
                        .show()
                        .unwrap_or_default();

                    // Mark as fired
                    let _ = conn.execute(
                        "UPDATE tasks SET reminder_fired = 1 WHERE id = ?1",
                        [id]
                    );
                }
            }
        }
    });
}
