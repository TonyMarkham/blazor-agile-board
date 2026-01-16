use sqlx::SqlitePool;
use uuid::Uuid;

/// Context passed to all handlers containing request metadata and resources.
#[derive(Debug, Clone)]
pub struct HandlerContext {
    /// Unique message ID for request/response correlation
    pub message_id: String,
    /// Tenant ID extracted from JWT
    pub tenant_id: String,
    /// User ID extracted from JWT
    pub user_id: Uuid,
    /// Database connection pool for this tenant
    pub pool: SqlitePool,
}

impl HandlerContext {
    pub fn new(message_id: String, tenant_id: String, user_id: Uuid, pool: SqlitePool) -> Self {
        Self {
            message_id,
            tenant_id,
            user_id,
            pool,
        }
    }
}
