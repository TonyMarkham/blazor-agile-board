use crate::{WsError, Result as WsErrorResult, ConnectionLimits, ConnectionId, ConnectionInfo};

use std::collections::HashMap;
use std::panic::Location;
use std::sync::Arc;

use error_location::ErrorLocation;
use tokio::sync::RwLock;

/// Registry for tracking active WebSocket connections                                                                                                                           
pub struct ConnectionRegistry {
    inner: Arc<RwLock<RegistryInner>>,
    limits: ConnectionLimits,
}

struct RegistryInner {
    /// All active connections by connection_id                                                                                                                                  
    connections: HashMap<ConnectionId, ConnectionInfo>,
    /// Connections grouped by tenant_id for quick tenant lookups                                                                                                                
    by_tenant: HashMap<String, Vec<ConnectionId>>,
}

impl ConnectionRegistry {
    pub fn new(limits: ConnectionLimits) -> Self {
        Self {
            inner: Arc::new(RwLock::new(RegistryInner {
                connections: HashMap::new(),
                by_tenant: HashMap::new(),
            })),
            limits,
        }
    }

    /// Register a new connection, returns ConnectionId if successful
    pub async fn register(
        &self,
        tenant_id: String,
        user_id: String,
    ) -> WsErrorResult<ConnectionId> {
        let mut inner = self.inner.write().await;

        // Check total connection limit                                                                                                                                          
        if inner.connections.len() >= self.limits.max_total {
            log::warn!(                                                                                                                                                          
                  "Total connection limit reached: {}/{}",                                                                                                                         
                  inner.connections.len(),                                                                                                                                         
                  self.limits.max_total                                                                                                                                            
              );
            return Err(WsError::ConnectionLimitExceeded {
                tenant_id: tenant_id.clone(),
                current: inner.connections.len(),
                max: self.limits.max_total,
                location: ErrorLocation::from(Location::caller()),
            });
        }

        // Check per-tenant connection limit                                                                                                                                     
        let tenant_connections = inner.by_tenant.get(&tenant_id).map(|v| v.len()).unwrap_or(0);
        if tenant_connections >= self.limits.max_per_tenant {
            log::warn!(                                                                                                                                                          
                  "Tenant {} connection limit reached: {}/{}",                                                                                                                     
                  tenant_id,                                                                                                                                                       
                  tenant_connections,                                                                                                                                              
                  self.limits.max_per_tenant                                                                                                                                       
              );
            return Err(WsError::ConnectionLimitExceeded {
                tenant_id: tenant_id.clone(),
                current: tenant_connections,
                max: self.limits.max_per_tenant,
                location: ErrorLocation::from(Location::caller()),
            });
        }

        // Create new connection                                                                                                                                                 
        let connection_id = ConnectionId::new();
        let info = ConnectionInfo {
            connection_id,
            tenant_id: tenant_id.clone(),
            user_id,
            connected_at: chrono::Utc::now(),
        };

        inner.connections.insert(connection_id, info);
        inner
            .by_tenant
            .entry(tenant_id.clone())
            .or_insert_with(Vec::new)
            .push(connection_id);

        log::info!(                                                                                                                                                              
              "Registered connection {} for tenant {} ({} total, {} for tenant)",                                                                                                  
              connection_id,                                                                                                                                                       
              tenant_id,                                                                                                                                                           
              inner.connections.len(),                                                                                                                                             
              tenant_connections + 1                                                                                                                                               
          );

        Ok(connection_id)
    }

    /// Unregister a connection                                                                                                                                                  
    pub async fn unregister(&self, connection_id: ConnectionId) {
        let mut inner = self.inner.write().await;

        if let Some(info) = inner.connections.remove(&connection_id) {
            // Remove from tenant list                                                                                                                                           
            if let Some(tenant_connections) = inner.by_tenant.get_mut(&info.tenant_id) {
                tenant_connections.retain(|&id| id != connection_id);
                if tenant_connections.is_empty() {
                    inner.by_tenant.remove(&info.tenant_id);
                }
            }

            log::info!(                                                                                                                                                          
                  "Unregistered connection {} for tenant {} ({} total remaining)",                                                                                                 
                  connection_id,                                                                                                                                                   
                  info.tenant_id,                                                                                                                                                  
                  inner.connections.len()                                                                                                                                          
              );
        }
    }

    /// Get information about a specific connection                                                                                                                              
    pub async fn get(&self, connection_id: ConnectionId) -> Option<ConnectionInfo> {
        let inner = self.inner.read().await;
        inner.connections.get(&connection_id).cloned()
    }

    /// Get all connections for a tenant                                                                                                                                         
    pub async fn get_tenant_connections(&self, tenant_id: &str) -> Vec<ConnectionInfo> {
        let inner = self.inner.read().await;
        inner
            .by_tenant
            .get(tenant_id)
            .map(|connection_ids| {
                connection_ids
                    .iter()
                    .filter_map(|id| inner.connections.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get total connection count                                                                                                                                               
    pub async fn total_count(&self) -> usize {
        let inner = self.inner.read().await;
        inner.connections.len()
    }

    /// Get connection count for a specific tenant                                                                                                                               
    pub async fn tenant_count(&self, tenant_id: &str) -> usize {
        let inner = self.inner.read().await;
        inner
            .by_tenant
            .get(tenant_id)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Get all active tenant IDs                                                                                                                                                
    pub async fn active_tenants(&self) -> Vec<String> {
        let inner = self.inner.read().await;
        inner.by_tenant.keys().cloned().collect()
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