use std::sync::RwLock;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};

// Conservative defaults for database circuit breaker:
// - Open after 5 failures in 60 seconds (failure_threshold/failure_window)
// - Stay open for 30 seconds before testing recovery (open_duration)
// - Require 3 consecutive successes to fully close (half_open_success_threshold)
const DEFAULT_FAILURE_THRESHOLD: u32 = 5;
const DEFAULT_OPEN_DURATION_SECS: u64 = 30;
const DEFAULT_HALF_OPEN_SUCCESS_THRESHOLD: u32 = 3;
const DEFAULT_FAILURE_WINDOW_SECS: u64 = 60;

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    /// Normal operation - requests flow through
    Closed,
    /// Too many failures - requests rejected immediately
    Open,
    /// Testing if service recovered - limited requests allowed
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening circuit
    pub failure_threshold: u32,
    /// Duration to keep circuit open before testing
    pub open_duration: Duration,
    /// Number of successful requests in half-open to close circuit
    pub half_open_success_threshold: u32,
    /// Window for counting failures
    pub failure_window: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: DEFAULT_FAILURE_THRESHOLD,
            open_duration: Duration::from_secs(DEFAULT_OPEN_DURATION_SECS),
            half_open_success_threshold: DEFAULT_HALF_OPEN_SUCCESS_THRESHOLD,
            failure_window: Duration::from_secs(DEFAULT_FAILURE_WINDOW_SECS),
        }
    }
}

/// Thread-safe circuit breaker
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: RwLock<CircuitState>,
    failure_count: AtomicU32,
    success_count: AtomicU32,
    last_failure_time: AtomicU64,
    opened_at: AtomicU64,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: RwLock::new(CircuitState::Closed),
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            last_failure_time: AtomicU64::new(0),
            opened_at: AtomicU64::new(0),
        }
    }

    /// Check if request should be allowed
    pub fn allow_request(&self) -> Result<(), CircuitBreakerError> {
        let state = *self.state.read().unwrap();

        match state {
            CircuitState::Closed => Ok(()),
            CircuitState::Open => {
                // Check if we should transition to half-open
                let opened_at = self.opened_at.load(Ordering::SeqCst);
                let now = Instant::now().elapsed().as_secs();

                if now - opened_at >= self.config.open_duration.as_secs() {
                    // Transition to half-open
                    let mut state_guard = self.state.write().unwrap();
                    if *state_guard == CircuitState::Open {
                        *state_guard = CircuitState::HalfOpen;
                        self.success_count.store(0, Ordering::SeqCst);
                        log::info!("Circuit breaker transitioning to HalfOpen");
                    }
                    Ok(())
                } else {
                    Err(CircuitBreakerError::CircuitOpen {
                        retry_after_secs: self.config.open_duration.as_secs() - (now - opened_at),
                    })
                }
            }
            CircuitState::HalfOpen => Ok(()), // Allow limited requests in half-open
        }
    }

    /// Record a successful request
    pub fn record_success(&self) {
        let state = *self.state.read().unwrap();

        match state {
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::SeqCst);
            }
            CircuitState::HalfOpen => {
                let successes = self.success_count.fetch_add(1, Ordering::SeqCst) + 1;
                if successes >= self.config.half_open_success_threshold {
                    // Close the circuit
                    let mut state_guard = self.state.write().unwrap();
                    *state_guard = CircuitState::Closed;
                    self.failure_count.store(0, Ordering::SeqCst);
                    log::info!("Circuit breaker closed after {} successes", successes);
                }
            }
            CircuitState::Open => {} // Shouldn't happen, but ignore
        }
    }

    /// Record a failed request
    pub fn record_failure(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let last_failure = self.last_failure_time.load(Ordering::SeqCst);

        // Reset count if outside failure window
        if now - last_failure > self.config.failure_window.as_secs() {
            self.failure_count.store(0, Ordering::SeqCst);
        }

        self.last_failure_time.store(now, Ordering::SeqCst);
        let failures = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;

        let state = *self.state.read().unwrap();

        match state {
            CircuitState::Closed => {
                if failures >= self.config.failure_threshold {
                    // Open the circuit
                    let mut state_guard = self.state.write().unwrap();
                    *state_guard = CircuitState::Open;
                    self.opened_at
                        .store(Instant::now().elapsed().as_secs(), Ordering::SeqCst);
                    log::warn!("Circuit breaker OPEN after {} failures", failures);
                }
            }
            CircuitState::HalfOpen => {
                // Single failure in half-open reopens circuit
                let mut state_guard = self.state.write().unwrap();
                *state_guard = CircuitState::Open;
                self.opened_at
                    .store(Instant::now().elapsed().as_secs(), Ordering::SeqCst);
                log::warn!("Circuit breaker reopened due to failure in HalfOpen state");
            }
            CircuitState::Open => {} // Already open
        }
    }

    /// Get current state
    pub fn state(&self) -> CircuitState {
        *self.state.read().unwrap()
    }
}

#[derive(Debug, Clone)]
pub enum CircuitBreakerError {
    CircuitOpen { retry_after_secs: u64 },
}

impl std::fmt::Display for CircuitBreakerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CircuitOpen { retry_after_secs } => {
                write!(
                    f,
                    "Circuit breaker open. Retry after {} seconds",
                    retry_after_secs
                )
            }
        }
    }
}

impl std::error::Error for CircuitBreakerError {}
