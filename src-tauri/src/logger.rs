use crate::events::{AppEvent, emit};
use tauri::AppHandle;
use std::sync::OnceLock;
use tracing::{Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::{prelude::*, EnvFilter};
use chrono::Utc;

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

pub fn init() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,hawkward_journal_ai=debug"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true);

    let tauri_layer = TauriEventLayer;

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(tauri_layer)
        .init();
}

pub fn set_handle(handle: AppHandle) {
    let _ = APP_HANDLE.set(handle);
}

struct TauriEventLayer;

impl<S> tracing_subscriber::Layer<S> for TauriEventLayer
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        if let Some(handle) = APP_HANDLE.get() {
            let mut message = String::new();
            let mut visitor = MessageVisitor { message: &mut message };
            event.record(&mut visitor);

            let metadata = event.metadata();
            let level = metadata.level().to_string();
            let source = metadata.target().to_string();
            let timestamp = Utc::now().to_rfc3339();

            // We avoid infinite recursion by not logging our own log emission
            if !source.contains("hawkward_journal_ai::events") {
                emit(handle, AppEvent::LogEvent {
                    timestamp,
                    level,
                    source,
                    message,
                });
            }
        }
    }
}

struct MessageVisitor<'a> {
    message: &'a mut String,
}

impl<'a> tracing::field::Visit for MessageVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            use std::fmt::Write;
            let _ = write!(self.message, "{:?}", value);
        }
    }
}
