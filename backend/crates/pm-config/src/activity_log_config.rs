use serde::Deserialize;

// Activity log retention configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ActivityLogConfig {
    /// Number of days to retain activity logs (default: 90)
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,

    /// Cleanup interval in hours (default: 24)
    #[serde(default = "default_cleanup_interval_hours")]
    pub cleanup_interval_hours: u32,
}

fn default_retention_days() -> u32 {
    90
}

fn default_cleanup_interval_hours() -> u32 {
    24
}

impl Default for ActivityLogConfig {
    fn default() -> Self {
        Self {
            retention_days: default_retention_days(),
            cleanup_interval_hours: default_cleanup_interval_hours(),
        }
    }
}
