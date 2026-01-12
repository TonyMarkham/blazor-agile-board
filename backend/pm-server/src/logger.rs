use crate::error::{ServerError, Result};

use std::time::SystemTime;

use fern::colors::{Color, ColoredLevelConfig};
use fern::Dispatch;
use humantime;
use log::LevelFilter;

/// Initialize logger with fern (stdout only, colored optional)
#[track_caller]
pub fn initialize(log_level: &str, colored: bool) -> Result<()> {
    let level_filter = parse_log_level(log_level)?;

    let base_dispatch = Dispatch::new().level(level_filter);

    let dispatch = if colored {
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
            message: format!("Failed to initialize logger: {}", e),
        })?;

    log::info!("Logger initialized with level: {:?}", level_filter);

    Ok(())
}

/// Parse log level string to LevelFilter
fn parse_log_level(level: &str) -> Result<LevelFilter> {
    match level.to_lowercase().as_str() {
        "trace" => Ok(LevelFilter::Trace),
        "debug" => Ok(LevelFilter::Debug),
        "info" => Ok(LevelFilter::Info),
        "warn" => Ok(LevelFilter::Warn),
        "error" => Ok(LevelFilter::Error),
        "off" => Ok(LevelFilter::Off),
        _ => Err(ServerError::EnvVar {
            message: format!("Invalid log level: {}", level),
        }),
    }
}