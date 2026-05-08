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
    JournalAnalysisQueued {
        entry_id: String,
    },
    JournalAnalysisProcessing {
        entry_id: String,
    },
    JournalAnalysisCompleted {
        entry_id: String,
        result: crate::ai::AnalysisResult,
    },
    JournalAnalysisError {
        entry_id: String,
        error: String,
    },
    AiModelMissing {
        model: String,
    },
    AiToken {
        conversation_id: String,
        token: String,
    },
    AiResponseComplete {
        conversation_id: String,
    },
}

pub fn emit(app: &AppHandle, event: AppEvent) {
    app.emit("app_event", event).ok();
}
