use crate::Metrics;

use std::time::Instant;

/// Helper for timing operations
pub struct MetricsTimer {
    start: Instant,
    metrics: Metrics,
}

impl MetricsTimer {
    pub fn new(metrics: Metrics) -> Self {
        Self {
            start: Instant::now(),
            metrics,
        }
    }

    /// Record elapsed time when dropped
    pub fn finish(self) {
        let duration = self.start.elapsed();
        self.metrics.message_latency(duration);
    }
}