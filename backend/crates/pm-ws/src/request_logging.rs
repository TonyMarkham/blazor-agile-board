use crate::RequestContext;

use log::{debug, error, info, warn};

/// Structured logging with request context
pub struct RequestLogger<'a> {
    ctx: &'a RequestContext,
}

impl<'a> RequestLogger<'a> {
    pub fn new(ctx: &'a RequestContext) -> Self {
        Self { ctx }
    }

    pub fn info(&self, message: &str) {
        info!("{} {}", self.ctx.log_prefix(), message);
    }

    pub fn debug(&self, message: &str) {
        debug!("{} {}", self.ctx.log_prefix(), message);
    }

    pub fn warn(&self, message: &str) {
        warn!("{} {}", self.ctx.log_prefix(), message);
    }

    pub fn error(&self, message: &str) {
        error!("{} {}", self.ctx.log_prefix(), message);
    }

    pub fn info_with_duration(&self, message: &str) {
        info!(
            "{} {} ({}ms)",
            self.ctx.log_prefix(),
            message,
            self.ctx.elapsed_ms()
        );
    }

    pub fn error_with_duration(&self, message: &str) {
        error!(
            "{} {} ({}ms)",
            self.ctx.log_prefix(),
            message,
            self.ctx.elapsed_ms()
        );
    }
}

/// Log a handler entry point
#[macro_export]
macro_rules! log_handler_entry {
    ($ctx:expr, $handler:expr) => {
        log::debug!("{} -> {} handler", $ctx.log_prefix(), $handler);
    };
}

/// Log a handler exit with duration
#[macro_export]
macro_rules! log_handler_exit {
    ($ctx:expr, $handler:expr, $result:expr) => {
        match &$result {
            Ok(_) => log::info!(
                "{} <- {} OK ({}ms)",
                $ctx.log_prefix(),
                $handler,
                $ctx.elapsed_ms()
            ),
            Err(e) => log::warn!(
                "{} <- {} ERR: {} ({}ms)",
                $ctx.log_prefix(),
                $handler,
                e,
                $ctx.elapsed_ms()
            ),
        }
    };
}
