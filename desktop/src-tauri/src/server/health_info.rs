use crate::server::HealthStatus;

use serde::Serialize;

/// Health information for frontend display.
#[derive(Debug, Clone, Serialize)]
pub struct HealthInfo {
    pub status: String,
    pub latency_ms: Option<u64>,
    pub version: Option<String>,
}

impl From<&HealthStatus> for HealthInfo {
    fn from(status: &HealthStatus) -> Self {
        match status {
            HealthStatus::Healthy {
                latency_ms,
                version,
            } => HealthInfo {
                status: "healthy".into(),
                latency_ms: Some(*latency_ms),
                version: Some(version.clone()),
            },
            HealthStatus::Starting => HealthInfo {
                status: "starting".into(),
                latency_ms: None,
                version: None,
            },
            HealthStatus::Unhealthy { last_error, .. } => HealthInfo {
                status: format!("unhealthy: {}", last_error),
                latency_ms: None,
                version: None,
            },
            HealthStatus::Crashed { exit_code } => HealthInfo {
                status: format!("crashed (code: {:?})", exit_code),
                latency_ms: None,
                version: None,
            },
            HealthStatus::ShuttingDown => HealthInfo {
                status: "shutting_down".into(),
                latency_ms: None,
                version: None,
            },
            HealthStatus::Stopped => HealthInfo {
                status: "stopped".into(),
                latency_ms: None,
                version: None,
            },
        }
    }
}
