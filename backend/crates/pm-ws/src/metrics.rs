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
    pub fn connection_established(&self, tenant_id: &str) {
        counter!(format!("{}.connections.established", self.prefix)).increment(1);
        counter!(format!(
            "{}.connections.established.{}",
            self.prefix, tenant_id
        ))
        .increment(1);

        // Update active connection gauge
        gauge!(format!("{}.connections.active", self.prefix)).increment(1.0);
    }

    /// Record connection closed
    pub fn connection_closed(&self, tenant_id: &str, reason: &str) {
        counter!(format!("{}.connections.closed", self.prefix)).increment(1);
        counter!(format!("{}.connections.closed.{}", self.prefix, reason)).increment(1);
        counter!(format!(
            "{}.connections.closed.{}.{}",
            self.prefix, tenant_id, reason
        ))
        .increment(1);

        // Update active connection gauge
        gauge!(format!("{}.connections.active", self.prefix)).decrement(1.0);
    }

    /// Record message received from client
    pub fn message_received(&self, tenant_id: &str, message_type: &str) {
        counter!(format!("{}.messages.received", self.prefix)).increment(1);
        counter!(format!(
            "{}.messages.received.{}",
            self.prefix, message_type
        ))
        .increment(1);
        counter!(format!(
            "{}.messages.received.{}.{}",
            self.prefix, tenant_id, message_type
        ))
        .increment(1);
    }

    /// Record message sent to client
    pub fn message_sent(&self, tenant_id: &str, message_type: &str) {
        counter!(format!("{}.messages.sent", self.prefix)).increment(1);
        counter!(format!("{}.messages.sent.{}", self.prefix, message_type)).increment(1);
        counter!(format!(
            "{}.messages.sent.{}.{}",
            self.prefix, tenant_id, message_type
        ))
        .increment(1);
    }

    /// Record broadcast message published
    pub fn broadcast_published(
        &self,
        tenant_id: &str,
        _message_type: &str,
        subscriber_count: usize,
    ) {
        counter!(format!("{}.broadcast.published", self.prefix)).increment(1);
        counter!(format!("{}.broadcast.published.{}", self.prefix, tenant_id)).increment(1);
        gauge!(format!(
            "{}.broadcast.subscribers.{}",
            self.prefix, tenant_id
        ))
        .set(subscriber_count as f64);
    }

    /// Record error occurrence
    pub fn error_occurred(&self, tenant_id: &str, error_type: &str) {
        counter!(format!("{}.errors.total", self.prefix)).increment(1);
        counter!(format!("{}.errors.{}", self.prefix, error_type)).increment(1);
        counter!(format!(
            "{}.errors.{}.{}",
            self.prefix, tenant_id, error_type
        ))
        .increment(1);
    }

    /// Record message processing latency
    pub fn message_latency(&self, duration: std::time::Duration) {
        histogram!(format!("{}.messages.latency_ms", self.prefix))
            .record(duration.as_millis() as f64);
    }

    /// Record subscription change
    pub fn subscription_changed(&self, tenant_id: &str, action: &str) {
        counter!(format!("{}.subscriptions.{}", self.prefix, action)).increment(1);
        counter!(format!(
            "{}.subscriptions.{}.{}",
            self.prefix, tenant_id, action
        ))
        .increment(1);
    }

    /// Update connection count for tenant
    pub fn update_tenant_connection_count(&self, tenant_id: &str, count: usize) {
        gauge!(format!(
            "{}.connections.per_tenant.{}",
            self.prefix, tenant_id
        ))
        .set(count as f64);
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}
