use std::sync::atomic::{AtomicU64, Ordering};

use uuid::Uuid;

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Request context for correlation and tracing
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Unique correlation ID for this request chain
    pub correlation_id: String,
    /// Sequence number within this server instance
    pub request_seq: u64,
    /// User making the request
    pub user_id: Uuid,
    /// Connection ID for WebSocket tracking
    pub connection_id: String,
    /// Start time for latency tracking
    pub started_at: std::time::Instant,
}

impl RequestContext {
    pub fn new(user_id: Uuid, connection_id: String, message_id: &str) -> Self {
        let request_seq = REQUEST_COUNTER.fetch_add(1, Ordering::SeqCst);

        // Use message_id as correlation_id if provided, otherwise generate
        let correlation_id = if message_id.is_empty() {
            format!("req-{}-{}", request_seq, Uuid::new_v4().as_simple())
        } else {
            message_id.to_string()
        };

        Self {
            correlation_id,
            request_seq,
            user_id,
            connection_id,
            started_at: std::time::Instant::now(),
        }
    }

    /// Get elapsed time since request started
    pub fn elapsed_ms(&self) -> u64 {
        self.started_at.elapsed().as_millis() as u64
    }

    /// Create a log prefix for structured logging
    pub fn log_prefix(&self) -> String {
        format!(
            "[req={} user={} conn={}]",
            &self.correlation_id[..8.min(self.correlation_id.len())],
            &self.user_id.to_string()[..8],
            &self.connection_id[..8.min(self.connection_id.len())]
        )
    }
}
