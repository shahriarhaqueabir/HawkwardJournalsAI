use std::fs::OpenOptions;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn init(data_dir: &std::path::Path) {
    let log_path = data_dir.join("hawkward-debug.log");
    
    // Create or append to the log file
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .expect("Failed to open log file");

    // We write to both stdout and the file
    let stdout = std::io::stdout.with_max_level(tracing::Level::INFO);
    let file_writer = file.with_max_level(tracing::Level::DEBUG);

    let format_layer = tracing_subscriber::fmt::layer()
        .with_writer(stdout.and(file_writer))
        .with_target(false)
        .with_thread_ids(true)
        .with_line_number(true);

    tracing_subscriber::registry()
        .with(format_layer)
        .init();

    tracing::info!("Logger initialized. Logging to: {:?}", log_path);
}
