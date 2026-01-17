use metrics::{counter, gauge, histogram};

/// Metrics collector for WebSocket operations
#[derive(Clone)]
pub struct Metrics {
    prefix: &'static str,
}

impl Metrics {
    pub fn new() -> Self {
        Self { prefix: "pm_ws" }
    }

    /// Record new connection established
    pub fn connection_established(&self) {
        counter!(format!("{}.connections.established", self.prefix)).increment(1);
        gauge!(format!("{}.connections.active", self.prefix)).increment(1.0);
    }

    /// Record connection closed
    pub fn connection_closed(&self, reason: &str) {
        counter!(format!("{}.connections.closed", self.prefix)).increment(1);
        counter!(format!("{}.connections.closed.{}", self.prefix, reason)).increment(1);
        gauge!(format!("{}.connections.active", self.prefix)).decrement(1.0);
    }

    /// Record message received from client
    pub fn message_received(&self, message_type: &str) {
        counter!(format!("{}.messages.received", self.prefix)).increment(1);
        counter!(format!(
            "{}.messages.received.{}",
            self.prefix, message_type
        ))
        .increment(1);
    }

    /// Record message sent to client
    pub fn message_sent(&self, message_type: &str) {
        counter!(format!("{}.messages.sent", self.prefix)).increment(1);
        counter!(format!("{}.messages.sent.{}", self.prefix, message_type)).increment(1);
    }

    /// Record broadcast message published
    pub fn broadcast_published(&self, _message_type: &str, subscriber_count: usize) {
        counter!(format!("{}.broadcast.published", self.prefix)).increment(1);
        gauge!(format!("{}.broadcast.subscribers", self.prefix)).set(subscriber_count as f64);
    }

    /// Record error occurrence
    pub fn error_occurred(&self, error_type: &str) {
        counter!(format!("{}.errors.total", self.prefix)).increment(1);
        counter!(format!("{}.errors.{}", self.prefix, error_type)).increment(1);
    }

    /// Record message processing latency
    pub fn message_latency(&self, duration: std::time::Duration) {
        histogram!(format!("{}.messages.latency_ms", self.prefix))
            .record(duration.as_millis() as f64);
    }

    /// Record subscription change
    pub fn subscription_changed(&self, action: &str) {
        counter!(format!("{}.subscriptions.{}", self.prefix, action)).increment(1);
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}
