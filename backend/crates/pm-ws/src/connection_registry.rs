use crate::{
    ClientSubscriptions, ConnectionId, ConnectionInfo, ConnectionLimits, Result as WsErrorResult,
    SubscriptionFilter, WsError,
};

use std::collections::HashMap;
use std::panic::Location;
use std::sync::Arc;

use axum::extract::ws::Message;
use error_location::ErrorLocation;
use log::{debug, info, warn};
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

    /// True if total connections reached max_total
    pub async fn is_at_total_limit(&self) -> bool {
        let inner = self.inner.read().await;
        inner.connections.len() >= self.limits.max_total
    }

    /// Subscribe a connection to project/sprint updates
    pub async fn subscribe(
        &self,
        connection_id: &str,
        projects: &[String],
        sprints: &[String],
    ) -> WsErrorResult<()> {
        let mut inner = self.inner.write().await;
        let connection_id = ConnectionId::parse(connection_id)?;
        let info = inner
            .connections
            .get_mut(&connection_id)
            .ok_or_else(|| WsError::NotFound {
                message: format!("Connection {} not found", connection_id),
                location: ErrorLocation::from(Location::caller()),
            })?;

        for project_id in projects {
            info.subscriptions.subscribe_project(project_id.clone());
        }
        for sprint_id in sprints {
            info.subscriptions.subscribe_sprint(sprint_id.clone());
        }

        Ok(())
    }

    /// Unsubscribe a connection from project/sprint updates
    pub async fn unsubscribe(
        &self,
        connection_id: &str,
        projects: &[String],
        sprints: &[String],
    ) -> WsErrorResult<()> {
        let mut inner = self.inner.write().await;
        let connection_id = ConnectionId::parse(connection_id)?;
        let info = inner
            .connections
            .get_mut(&connection_id)
            .ok_or_else(|| WsError::NotFound {
                message: format!("Connection {} not found", connection_id),
                location: ErrorLocation::from(Location::caller()),
            })?;

        for project_id in projects {
            info.subscriptions.unsubscribe_project(project_id);
        }
        for sprint_id in sprints {
            info.subscriptions.unsubscribe_sprint(sprint_id);
        }

        Ok(())
    }

    /// Broadcast ActivityLogCreated event to matching subscribers
    pub async fn broadcast_activity_log_created(
        &self,
        project_id: &str,
        work_item_id: Option<&str>,
        sprint_id: Option<&str>,
        message: Message,
    ) -> WsErrorResult<usize> {
        // Clone senders first to avoid holding lock across .await
        let inner = self.inner.read().await;
        let connections: Vec<(ClientSubscriptions, mpsc::Sender<Message>)> = inner
            .connections
            .values()
            .map(|info| (info.subscriptions.clone(), info.sender.clone()))
            .collect();
        drop(inner);

        let mut delivered = 0;
        for (subscriptions, sender) in connections {
            let should_receive = if let Some(work_item_id) = work_item_id {
                SubscriptionFilter::should_receive_work_item_event(
                    &subscriptions,
                    project_id,
                    work_item_id,
                )
            } else if let Some(sprint_id) = sprint_id {
                SubscriptionFilter::should_receive_sprint_event(
                    &subscriptions,
                    project_id,
                    sprint_id,
                )
            } else {
                subscriptions.is_subscribed_to_project(project_id)
            };

            if should_receive {
                if sender.send(message.clone()).await.is_ok() {
                    delivered += 1;
                } else {
                    debug!("Broadcast send failed; skipping connection");
                }
            }
        }

        Ok(delivered)
    }

    /// Broadcast any message to all clients subscribed to a project
    pub async fn broadcast_to_project(
        &self,
        project_id: &str,
        message: Message,
    ) -> WsErrorResult<usize> {
        let inner = self.inner.read().await;
        let connections: Vec<(ClientSubscriptions, mpsc::Sender<Message>)> = inner
            .connections
            .values()
            .map(|info| (info.subscriptions.clone(), info.sender.clone()))
            .collect();
        drop(inner);

        let mut delivered = 0;
        for (subscriptions, sender) in connections {
            if subscriptions.is_subscribed_to_project(project_id)
                && sender.send(message.clone()).await.is_ok()
            {
                delivered += 1;
            }
        }

        Ok(delivered)
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
