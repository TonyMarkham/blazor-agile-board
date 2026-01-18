use std::time::Duration;

use tokio::time::sleep;

// Conservative retry defaults for transient failures:
// - Try up to 3 times total (1 initial + 2 retries)
// - Start with 100ms delay, double each time (exponential backoff)
// - Cap maximum delay at 5 seconds to avoid excessive waiting
// - Use jitter (±50%) to prevent thundering herd problem
const DEFAULT_MAX_ATTEMPTS: u32 = 3;
const DEFAULT_INITIAL_DELAY_MS: u64 = 100;
const DEFAULT_MAX_DELAY_SECS: u64 = 5;
const DEFAULT_BACKOFF_MULTIPLIER: f64 = 2.0;
const DEFAULT_JITTER_ENABLED: bool = true;

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial delay before first retry
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
    /// Add jitter to prevent thundering herd
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: DEFAULT_MAX_ATTEMPTS,
            initial_delay: Duration::from_millis(DEFAULT_INITIAL_DELAY_MS),
            max_delay: Duration::from_secs(DEFAULT_MAX_DELAY_SECS),
            backoff_multiplier: DEFAULT_BACKOFF_MULTIPLIER,
            jitter: DEFAULT_JITTER_ENABLED,
        }
    }
}

/// Execute an async operation with retry logic
pub async fn with_retry<F, Fut, T, E>(
    config: &RetryConfig,
    operation_name: &str,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Display + IsRetryable,
{
    let mut attempts = 0;
    let mut delay = config.initial_delay;

    loop {
        attempts += 1;

        match operation().await {
            Ok(result) => {
                if attempts > 1 {
                    log::info!("{} succeeded after {} attempts", operation_name, attempts);
                }
                return Ok(result);
            }
            Err(e) => {
                if !e.is_retryable() || attempts >= config.max_attempts {
                    log::warn!(
                        "{} failed after {} attempts: {}",
                        operation_name,
                        attempts,
                        e
                    );
                    return Err(e);
                }

                // Calculate delay with optional jitter
                let actual_delay = if config.jitter {
                    let jitter_factor = 0.5 + rand::random::<f64>(); // 0.5 to 1.5
                    Duration::from_secs_f64(delay.as_secs_f64() * jitter_factor)
                } else {
                    delay
                };

                log::debug!(
                    "{} attempt {} failed: {}. Retrying in {:?}",
                    operation_name,
                    attempts,
                    e,
                    actual_delay
                );

                sleep(actual_delay).await;

                // Exponential backoff
                delay = Duration::from_secs_f64(
                    (delay.as_secs_f64() * config.backoff_multiplier)
                        .min(config.max_delay.as_secs_f64()),
                );
            }
        }
    }
}

/// Trait for errors that can indicate retryability
pub trait IsRetryable {
    fn is_retryable(&self) -> bool;
}

impl IsRetryable for crate::WsError {
    fn is_retryable(&self) -> bool {
        self.is_retryable()
    }
}
