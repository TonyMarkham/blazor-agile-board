use crate::error::{Result as ServerErrorResult, ServerError};

use std::path::PathBuf;
use std::time::SystemTime;

use fern::Dispatch;
use fern::colors::{Color, ColoredLevelConfig};
use log::info;

/// Initialize logger with fern
///
/// # Arguments
/// * `log_level` - Log level filter
/// * `log_file` - Optional path to log file. None = stdout, Some = file output
/// * `colored` - Enable colored output (ignored when logging to file)
#[track_caller]
pub fn initialize(
    log_level: pm_config::LogLevel,
    log_file: Option<PathBuf>,
    colored: bool,
) -> ServerErrorResult<()> {
    let level_filter = log_level.0;

    let base_dispatch = Dispatch::new().level(level_filter);

    let dispatch = if let Some(ref log_path) = log_file {
        // File output (no colors, plain format)
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .map_err(|e| ServerError::EnvVar {
                message: format!("Failed to open log file {}: {}", log_path.display(), e),
            })?;

        Dispatch::new()
            .format(|out, message, record| {
                out.finish(format_args!(
                    "[{date} - {level}] {message} [{file}:{line}]",
                    date = humantime::format_rfc3339(SystemTime::now()),
                    level = record.level(),
                    message = message,
                    file = record.file().unwrap_or("unknown"),
                    line = record.line().unwrap_or(0),
                ))
            })
            .chain(file)
    } else if colored {
        // Colored output for TTY
        let colors = ColoredLevelConfig::new()
            .trace(Color::Magenta)
            .debug(Color::Blue)
            .info(Color::Green)
            .warn(Color::Yellow)
            .error(Color::Red);

        Dispatch::new()
            .format(move |out, message, record| {
                out.finish(format_args!(
                    "[{date} - {level}] {message} [{file}:{line}]",
                    date = humantime::format_rfc3339(SystemTime::now()),
                    level = colors.color(record.level()),
                    message = message,
                    file = record.file().unwrap_or("unknown"),
                    line = record.line().unwrap_or(0),
                ))
            })
            .chain(std::io::stdout())
    } else {
        // Plain output for non-TTY (systemd, docker logs)
        Dispatch::new()
            .format(|out, message, record| {
                out.finish(format_args!(
                    "[{date} - {level}] {message} [{file}:{line}]",
                    date = humantime::format_rfc3339(SystemTime::now()),
                    level = record.level(),
                    message = message,
                    file = record.file().unwrap_or("unknown"),
                    line = record.line().unwrap_or(0),
                ))
            })
            .chain(std::io::stdout())
    };

    base_dispatch
        .chain(dispatch)
        .apply()
        .map_err(|e| ServerError::EnvVar {
            message: format!("Failed to initialize logger: {e}"),
        })?;

    if let Some(ref path) = log_file {
        info!(
            "Logger initialized: level={:?}, file={}",
            level_filter,
            path.display()
        );
    } else {
        info!("Logger initialized: level={:?}, stdout", level_filter);
    }

    // Bridge tracing to log
    tracing_log::LogTracer::init().ok();

    Ok(())
}
