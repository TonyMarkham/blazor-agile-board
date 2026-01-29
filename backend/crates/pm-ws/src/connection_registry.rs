use crate::{
    ClientSubscriptions, ConnectionId, ConnectionInfo, ConnectionLimits, Result as WsErrorResult,
    WsError,
};

use std::collections::HashMap;
use std::panic::Location;
use std::sync::Arc;

use axum::extract::ws::Message;
use error_location::ErrorLocation;
use log::{info, warn};
use tokio::sync::{RwLock, mpsc};

/// Registry for tracking active WebSocket connections                                                                                                                           
pub struct ConnectionRegistry {
    inner: Arc<RwLock<RegistryInner>>,
    limits: ConnectionLimits,
}

struct RegistryInner {
    /// All active connections by connection_id                                                                                                                                  
    connections: HashMap<ConnectionId, ConnectionInfo>,
}

impl ConnectionRegistry {
    pub fn new(limits: ConnectionLimits) -> Self {
        Self {
            inner: Arc::new(RwLock::new(RegistryInner {
                connections: HashMap::new(),
            })),
            limits,
        }
    }

    /// Register a new connection, returns ConnectionId if successful
    pub async fn register(
        &self,
        user_id: String,
        sender: mpsc::Sender<Message>,
    ) -> WsErrorResult<ConnectionId> {
        let mut inner = self.inner.write().await;

        // Check total connection limit
        if inner.connections.len() >= self.limits.max_total {
            warn!(
                "Total connection limit reached: {}/{}",
                inner.connections.len(),
                self.limits.max_total
            );
            return Err(WsError::ConnectionLimitExceeded {
                current: inner.connections.len(),
                max: self.limits.max_total,
                location: ErrorLocation::from(Location::caller()),
            });
        }

        // Create new connection
        let connection_id = ConnectionId::new();
        let info = ConnectionInfo {
            connection_id,
            user_id,
            connected_at: chrono::Utc::now(),
            sender,
            subscriptions: ClientSubscriptions::new(),
        };

        inner.connections.insert(connection_id, info);
        info!(
            "Registered connection {connection_id} ({} total)",
            inner.connections.len()
        );

        Ok(connection_id)
    }

    /// Unregister a connection                                                                                                                                                  
    pub async fn unregister(&self, connection_id: ConnectionId) {
        let mut inner = self.inner.write().await;

        if inner.connections.remove(&connection_id).is_some() {
            info!(
                "Unregistered connection {connection_id} ({} total remaining)",
                inner.connections.len()
            );
        }
    }

    /// Get information about a specific connection                                                                                                                              
    pub async fn get(&self, connection_id: ConnectionId) -> Option<ConnectionInfo> {
        let inner = self.inner.read().await;
        inner.connections.get(&connection_id).cloned()
    }

    /// Get total connection count                                                                                                                                               
    pub async fn total_count(&self) -> usize {
        let inner = self.inner.read().await;
        inner.connections.len()
    }
}

impl Clone for ConnectionRegistry {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            limits: self.limits.clone(),
        }
    }
}
