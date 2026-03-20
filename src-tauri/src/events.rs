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
    // -- AI Chat Events
    AiToken {
        token: String,
        done: bool,
        source: crate::events::AiTokenSource,
    },
    AiToolPending {
        call_id: String,
        name: String,
        args: serde_json::Value,
        description: String,
    },
    AiToolResult {
        call_id: String,
        name: String,
        result: serde_json::Value,
        confirmed: bool,
    },
    AiConfirmTimeout {
        call_id: String,
        tool_name: String,
    },
    AiStatus(String),
    // --
    AiModelMissing { model: String },
    SystemStatus { message: String },
    DatabaseError {
        operation: String,
        error: String,
    },
}

#[allow(dead_code)]
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum AiTokenSource {
    Chat,
    Analysis,
}


pub fn emit(app: &AppHandle, event: AppEvent) {
    app.emit("app_event", event).ok();
}
