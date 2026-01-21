//! Health monitoring with circuit breaker pattern.

use crate::server::{ServerError, ServerResult};

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

/// Current health status of the server process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    /// Server is healthy and responding
    Healthy { latency_ms: u64, version: String },
    /// Server is starting up
    Starting,
    /// Server is not responding
    Unhealthy {
        consecutive_failures: u32,
        last_error: String,
    },
    /// Server process has crashed
    Crashed { exit_code: Option<i32> },
    /// Server is shutting down gracefully
    ShuttingDown,
    /// Server is stopped
    Stopped,
}

/// Response from the /ready endpoint.
/// **Note**: Matches actual backend response from pm-server/src/health.rs
#[derive(Debug, Clone, serde::Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    #[serde(default)]
    pub database: Option<DatabaseHealth>,
    #[serde(default)]
    pub circuit_breaker: Option<CircuitBreakerHealth>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DatabaseHealth {
    pub status: String,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct CircuitBreakerHealth {
    pub state: String,
}

/// Health checker with circuit breaker behavior.
///
/// After multiple consecutive failures, the checker enters a
/// "failed" state and stops retrying until explicitly reset.
pub struct HealthChecker {
    client: reqwest::Client,
    port: u16,
    status: Arc<RwLock<HealthStatus>>,
    consecutive_failures: AtomicU32,
    last_check_ms: AtomicU64,
    failure_threshold: u32,
}

impl HealthChecker {
    /// Create a new health checker for the given port.
    ///
    /// # Arguments
    /// * `port` - The port where the server is listening
    /// * `failure_threshold` - Number of consecutive failures before marking as failed
    pub fn new(port: u16, failure_threshold: u32) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .pool_max_idle_per_host(1)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            port,
            status: Arc::new(RwLock::new(HealthStatus::Starting)),
            consecutive_failures: AtomicU32::new(0),
            last_check_ms: AtomicU64::new(0),
            failure_threshold,
        }
    }

    /// Perform a single health check against the server.
    ///
    /// Calls the /ready endpoint and records the result.
    /// Updates internal status based on response.
    pub async fn check(&self) -> HealthStatus {
        let start = Instant::now();
        let url = format!("http://127.0.0.1:{}/ready", self.port);

        let result = self.client.get(&url).send().await;
        let latency_ms = start.elapsed().as_millis() as u64;

        self.last_check_ms.store(latency_ms, Ordering::Relaxed);

        let new_status = match result {
            Ok(resp) if resp.status().is_success() => {
                self.consecutive_failures.store(0, Ordering::Relaxed);

                match resp.json::<HealthResponse>().await {
                    Ok(health) => HealthStatus::Healthy {
                        latency_ms,
                        version: health.version,
                    },
                    Err(e) => HealthStatus::Unhealthy {
                        consecutive_failures: 1,
                        last_error: format!("Invalid response: {}", e),
                    },
                }
            }
            Ok(resp) => {
                let failures = self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
                HealthStatus::Unhealthy {
                    consecutive_failures: failures,
                    last_error: format!("HTTP {}", resp.status()),
                }
            }
            Err(e) => {
                let failures = self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
                HealthStatus::Unhealthy {
                    consecutive_failures: failures,
                    last_error: e.to_string(),
                }
            }
        };

        // Update cached status
        *self.status.write().await = new_status.clone();

        new_status
    }

    /// Wait for server to become healthy with timeout.
    ///
    /// Polls the health endpoint at 100ms intervals until
    /// the server reports healthy or timeout is reached.
    pub async fn wait_ready(&self, timeout: Duration) -> ServerResult<()> {
        let start = Instant::now();
        let poll_interval = Duration::from_millis(100);

        while start.elapsed() < timeout {
            match self.check().await {
                HealthStatus::Healthy { .. } => return Ok(()),
                HealthStatus::Unhealthy {
                    consecutive_failures,
                    ..
                } if consecutive_failures >= self.failure_threshold => {
                    return Err(ServerError::HealthCheckFailed {
                        message: "Too many consecutive failures".into(),
                        location: error_location::ErrorLocation::from(
                            std::panic::Location::caller(),
                        ),
                    });
                }
                _ => {}
            }
            tokio::time::sleep(poll_interval).await;
        }

        Err(ServerError::StartupTimeout {
            timeout_secs: timeout.as_secs(),
            location: error_location::ErrorLocation::from(std::panic::Location::caller()),
        })
    }

    /// Get current cached status.
    pub async fn status(&self) -> HealthStatus {
        self.status.read().await.clone()
    }

    /// Set status directly (for crash/shutdown notifications).
    pub async fn set_status(&self, status: HealthStatus) {
        *self.status.write().await = status;
    }

    /// Check if server should be considered failed.
    pub fn is_failed(&self) -> bool {
        self.consecutive_failures.load(Ordering::Relaxed) >= self.failure_threshold
    }

    /// Get last check latency.
    pub fn last_latency_ms(&self) -> u64 {
        self.last_check_ms.load(Ordering::Relaxed)
    }
}
