use crate::{CircuitBreaker, ConnectionRegistry, RequestContext, RetryConfig};

use std::sync::Arc;

use sqlx::SqlitePool;
use uuid::Uuid;

/// Context passed to all handlers containing request metadata and resources.
#[derive(Clone)]
pub struct HandlerContext {
    /// Unique message ID for request/response correlation
    pub message_id: String,
    /// User ID extracted from JWT
    pub user_id: Uuid,
    /// Database connection pool
    pub pool: SqlitePool,
    /// Circuit breaker for database operations
    pub circuit_breaker: Arc<CircuitBreaker>,
    /// Request context for tracing
    pub request_ctx: RequestContext,
    /// Retry configuration
    pub retry_config: RetryConfig,
    /// Connection registry for broadcasts
    pub registry: ConnectionRegistry,
}

impl HandlerContext {
    pub fn new(
        message_id: String,
        user_id: Uuid,
        pool: SqlitePool,
        circuit_breaker: Arc<CircuitBreaker>,
        connection_id: String,
        registry: ConnectionRegistry,
    ) -> Self {
        let request_ctx = RequestContext::new(user_id, connection_id, &message_id);

        Self {
            message_id,
            user_id,
            pool,
            circuit_breaker,
            request_ctx,
            retry_config: RetryConfig::default(),
            registry,
        }
    }

    /// Check circuit breaker before database operation
    pub fn check_circuit(&self) -> crate::Result<()> {
        self.circuit_breaker.allow_request()?;
        Ok(())
    }

    /// Record successful database operation
    pub fn record_db_success(&self) {
        self.circuit_breaker.record_success();
    }

    /// Record failed database operation
    pub fn record_db_failure(&self) {
        self.circuit_breaker.record_failure();
    }

    /// Get log prefix for structured logging
    pub fn log_prefix(&self) -> String {
        self.request_ctx.log_prefix()
    }
}

impl std::fmt::Debug for HandlerContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandlerContext")
            .field("message_id", &self.message_id)
            .field("user_id", &self.user_id)
            .field("correlation_id", &self.request_ctx.correlation_id)
            .finish()
    }
}
