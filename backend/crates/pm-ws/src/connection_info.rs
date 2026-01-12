use crate::ConnectionId;

use chrono::DateTime;

/// Information about an active connection
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub connection_id: ConnectionId,
    pub tenant_id: String,
    pub user_id: String,
    pub connected_at: DateTime<chrono::Utc>,
}