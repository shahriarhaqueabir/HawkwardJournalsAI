use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AppEvent {
    LogEvent {
        timestamp: String,
        level: String,
        source: String,
        message: String,
    },
    StartupComplete,
    
    // Journal Events
    JournalAnalysisQueued { entry_id: String },
    JournalAnalysisProcessing { entry_id: String },
    JournalAnalysisCompleted {
        entry_id: String,
        result: crate::ai::AnalysisResult,
    },
    JournalAnalysisError {
        entry_id: String,
        error: String,
    },
    JournalSaved { entry_id: String },

    // Task Events
    TaskCreated { id: String, title: String },
    TaskUpdated { id: String },
    TaskCompleted { id: String },
    TaskDeleted { id: String },

    // Weekly Review
    WeeklyReviewGenerated { date: String },

    // AI & System
    AiModelMissing { model: String },
    SystemStatus { message: String },
    DatabaseError {
        operation: String,
        error: String,
    },
}

pub fn emit(app: &AppHandle, event: AppEvent) {
    app.emit("app_event", event).ok();
}
