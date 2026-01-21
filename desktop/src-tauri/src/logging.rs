//! Logging setup with file rotation.

use std::path::Path;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    EnvFilter, fmt, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt,
};

/// Setup logging with console and rotating file output.
///
/// # Log Layers
/// - Console: Human-readable, colored output
/// - File: JSON format, daily rotation, 7-day retention
pub fn setup_logging(data_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let logs_dir = data_dir.join("logs");
    std::fs::create_dir_all(&logs_dir)?;

    // Console layer - human readable for development
    let console_layer = fmt::layer()
        .with_target(true)
        .with_level(true)
        .with_ansi(true);

    // File layer - JSON for easier parsing
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .max_log_files(7) // Keep 7 days of logs
        .filename_prefix("pm-desktop")
        .filename_suffix("log")
        .build(&logs_dir)?;

    let file_layer = fmt::layer()
        .with_target(true)
        .with_level(true)
        .with_ansi(false)
        .with_writer(file_appender);

    // Combine layers with environment filter
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,pm_server=debug"));

    tracing_subscriber::registry()
        .with(filter)
        .with(console_layer)
        .with(file_layer)
        .init();

    Ok(())
}

/// Get path to current log file (for diagnostics export).
pub fn current_log_path(data_dir: &Path) -> std::path::PathBuf {
    let logs_dir = data_dir.join("logs");
    let today = chrono::Local::now().format("%Y-%m-%d");
    logs_dir.join(format!("pm-desktop.{}.log", today))
}
