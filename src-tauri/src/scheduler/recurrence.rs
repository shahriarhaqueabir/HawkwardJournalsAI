use chrono::{Datelike, Duration, Local, NaiveDate, Weekday};
use crate::error::AppError;
use tauri::AppHandle;
use rusqlite::Connection;
use uuid::Uuid;

/// Calculates the next occurrence date based on a reference date and a recurrence string.
pub fn calculate_next_occurrence(current_due: NaiveDate, recurrence: &str) -> Result<NaiveDate, AppError> {
    let next = match recurrence.to_lowercase().as_str() {
        "daily" => current_due + Duration::days(1),
        "weekly" => current_due + Duration::weeks(1),
        "weekdays" => {
            let mut next_day = current_due + Duration::days(1);
            while next_day.weekday() == Weekday::Sat || next_day.weekday() == Weekday::Sun {
                next_day = next_day + Duration::days(1);
            }
            next_day
        }
        "monthly" => {
            // Simple monthly logic: try adding a month, if out of range, use last day of next month
            let year = if current_due.month() == 12 { current_due.year() + 1 } else { current_due.year() };
            let month = if current_due.month() == 12 { 1 } else { current_due.month() + 1 };
            let day = current_due.day();
            
            NaiveDate::from_ymd_opt(year, month, day)
                .unwrap_or_else(|| {
                    // Fallback to last day of next month if day (e.g., 31) doesn't exist
                    let next_month_start = if month == 12 {
                        NaiveDate::from_ymd_opt(year + 1, 1, 1)
                    } else {
                        NaiveDate::from_ymd_opt(year, month + 1, 1)
                    }.unwrap();
                    next_month_start - Duration::days(1)
                })
        }
        r if r.starts_with("every ") => {
            parse_complex_recurrence(current_due, r)?
        }
        _ => return Err(AppError::AiError(format!("Unsupported recurrence format: {}", recurrence))),
    };

    Ok(next)
}

fn parse_complex_recurrence(current_due: NaiveDate, r: &str) -> Result<NaiveDate, AppError> {
    let parts: Vec<&str> = r.split_whitespace().collect();
    if parts.len() < 3 {
        return Err(AppError::AiError(format!("Invalid complex recurrence: {}", r)));
    }

    let n: i64 = parts[1].parse().map_err(|_| AppError::AiError(format!("Invalid number in recurrence: {}", parts[1])))?;
    let unit = parts[2].to_lowercase();

    match unit.as_str() {
        "day" | "days" => Ok(current_due + Duration::days(n)),
        "week" | "weeks" => Ok(current_due + Duration::weeks(n)),
        "month" | "months" => {
             // Basic month addition for N months
             let mut year = current_due.year();
             let mut month = current_due.month() as i32 + n as i32;
             while month > 12 {
                 year += 1;
                 month -= 12;
             }
             Ok(NaiveDate::from_ymd_opt(year, month as u32, current_due.day())
                 .unwrap_or_else(|| {
                    let next_m = if month == 12 { 1 } else { month + 1 };
                    let next_y = if month == 12 { year + 1 } else { year };
                     NaiveDate::from_ymd_opt(next_y, next_m as u32, 1).unwrap() - Duration::days(1)
                 }))
        }
        _ => Err(AppError::AiError(format!("Unsupported unit in recurrence: {}", unit))),
    }
}

/// Interval-based recurrence worker (D-120).
/// Scans on an interval schedule to trigger rolling-chain recurrences.
pub fn poll_recurrences(app: &AppHandle, conn: &Connection) -> Result<(), AppError> {
    let today = Local::now().date_naive();
    let today_str = today.to_string();
    let now = Local::now().to_rfc3339();

    // 1. Find all active generator tasks that have reached their scheduled interval.
    // Explicitly exclude 'cancelled' tasks and completed ones from regenerating.
    let mut stmt = conn.prepare(
        "SELECT id, recurrence, next_occurrence FROM tasks 
         WHERE is_deleted = 0 
           AND recurrence IS NOT NULL AND recurrence != ''
           AND next_occurrence IS NOT NULL AND next_occurrence <= ?1
           AND status NOT IN ('cancelled')"
    )?;

    let rows = stmt.query_map([&today_str], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    let mut generators = Vec::new();
    for row in rows {
        if let Ok(data) = row {
            generators.push(data);
        }
    }

    if generators.is_empty() {
        return Ok(()); // Nothing to do
    }

    for (id, recurrence, next_occurrence_str) in generators {
        if let Ok(next_occurrence_date) = NaiveDate::parse_from_str(&next_occurrence_str, "%Y-%m-%d") {
            // Predict the subsequent date (e.g. the 2nd recurrence after the one we are creating right now)
            if let Ok(next_next_date) = calculate_next_occurrence(next_occurrence_date, &recurrence) {
                if let Ok(Some(task)) = crate::db::tasks::get_task(conn, &id) {
                    
                    // 2. Rolling Chain Pivot:
                    // Remove the recurrence properties from the CURRENT task (it becomes static)
                    conn.execute(
                        "UPDATE tasks SET recurrence = NULL, next_occurrence = NULL, updated_at = ?1 WHERE id = ?2",
                        rusqlite::params![now, id],
                    )?;

                    crate::events::emit(app, crate::events::AppEvent::TaskUpdated { id: id.clone() });

                    // 3. Spawning: Create the NEW interval task to carry the recurrence torch
                    let mut cloned_task = task.clone();
                    cloned_task.id = Uuid::new_v4().to_string();
                    cloned_task.status = "todo".into();
                    cloned_task.due_date = Some(next_occurrence_str);
                    cloned_task.recurrence = Some(recurrence);
                    cloned_task.next_occurrence = Some(next_next_date.to_string());
                    cloned_task.created_at = now.clone();
                    cloned_task.updated_at = now.clone();
                    cloned_task.completed_at = None;
                    
                    if crate::db::tasks::create_task(conn, &cloned_task).is_ok() {
                        crate::events::emit(app, crate::events::AppEvent::TaskCreated { 
                            id: cloned_task.id.clone(), 
                            title: format!("(Recur) {}", cloned_task.title) 
                        });
                    }
                }
            }
        }
    }

    Ok(())
}
