use crate::error::{Result as ServerErrorResult, ServerError};

use std::time::SystemTime;

use fern::Dispatch;
use fern::colors::{Color, ColoredLevelConfig};
use log::info;

/// Initialize logger with fern (stdout only, colored optional)
#[track_caller]
pub fn initialize(log_level: pm_config::LogLevel, colored: bool) -> ServerErrorResult<()> {
    let level_filter = log_level.0; // Extract inner LevelFilter

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
            message: format!("Failed to initialize logger: {e}"),
        })?;

    info!("Logger initialized with level: {level_filter:?}");

    // Bridge tracing to log
    tracing_log::LogTracer::init().ok();

    Ok(())
}
