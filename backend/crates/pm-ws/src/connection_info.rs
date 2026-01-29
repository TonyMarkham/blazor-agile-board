use crate::{ClientSubscriptions, ConnectionId};

use axum::extract::ws::Message;
use chrono::DateTime;
use tokio::sync::mpsc;

/// Information about an active connection
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub connection_id: ConnectionId,
    pub user_id: String,
    pub connected_at: DateTime<chrono::Utc>,
    pub sender: mpsc::Sender<Message>,
    pub subscriptions: ClientSubscriptions,
}
