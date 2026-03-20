use chrono::{Datelike, Local, NaiveDate};
use rusqlite::Connection;
use tauri::AppHandle;
use crate::error::AppError;

/// Checks if the weekly review has already run for the current week.
pub fn has_review_run_this_week(conn: &Connection) -> bool {
    let mut stmt = match conn.prepare("SELECT value FROM app_settings WHERE key = 'weekly_review_last_run'") {
        Ok(s) => s,
        Err(_) => return false,
    };
    
    let last_run_str: String = match stmt.query_row([], |r| r.get(0)) {
        Ok(s) => s,
        Err(_) => return false,
    };

    if last_run_str.is_empty() {
        return false;
    }

    let last_run = match NaiveDate::parse_from_str(&last_run_str, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => return false,
    };

    let today = Local::now().date_naive();
    
    // Compare ISO week and year (D-109)
    last_run.iso_week() == today.iso_week() && last_run.year() == today.year()
}

/// Core function to generate the weekly report and update the last run date.
pub fn maybe_run_weekly_review(app: &AppHandle, conn: &Connection) -> Result<(), AppError> {
    let today = Local::now().date_naive();
    
    // Decision D-109: Only run on Mondays
    if today.weekday() != chrono::Weekday::Mon {
        return Ok(());
    }

    if has_review_run_this_week(conn) {
        return Ok(());
    }

    println!("[SCHEDULER] Running missed weekly review for week of {}", today);
    
    // --- WORKER START (PHASE 4) ---
    // TODO: Actual report generation logic will live here.
    // For now, we just mark it as run.
    
    let now_date = today.format("%Y-%m-%d").to_string();
    let now_ts = Local::now().to_rfc3339();
    
    conn.execute(
        "UPDATE app_settings SET value = ?1, updated_at = ?2 WHERE key = 'weekly_review_last_run'",
        rusqlite::params![now_date, now_ts],
    )?;

    // Emit event for frontend toast
    crate::events::emit(app, crate::events::AppEvent::SystemStatus { 
        message: "Weekly review generated successfully.".into() 
    });

    Ok(())
}
