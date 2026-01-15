use crate::{Result as WsErrorResult, BroadcastConfig, BroadcastMessage};

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{broadcast, RwLock};

/// Manages broadcast channels for all tenants
pub struct TenantBroadcaster {
    inner: Arc<RwLock<BroadcasterInner>>,
    config: BroadcastConfig,
}

struct BroadcasterInner {
    channels: HashMap<String, TenantChannel>,
}

/// Per-tenant broadcast channel
pub(crate) struct TenantChannel {
    sender: broadcast::Sender<BroadcastMessage>,
    subscriber_count: usize,
}

impl TenantBroadcaster {
    pub fn new(config: BroadcastConfig) -> Self {
        Self {
            inner: Arc::new(RwLock::new(BroadcasterInner {
                channels: HashMap::new(),
            })),
            config,
        }
    }

    /// Subscribe to a tenant's broadcast channel
    pub async fn subscribe(&self, tenant_id: &str) -> broadcast::Receiver<BroadcastMessage> {
        let mut inner = self.inner.write().await;

        let channel = inner
            .channels
            .entry(tenant_id.to_string())
            .or_insert_with(|| {
                let (sender, _) = broadcast::channel(self.config.channel_capacity);
                log::info!("Created broadcast channel for tenant {}", tenant_id);
                TenantChannel {
                    sender,
                    subscriber_count: 0,
                }
            });

        channel.subscriber_count += 1;
        let receiver = channel.sender.subscribe();

        log::debug!(
              "Client subscribed to tenant {} broadcast ({} total subscribers)",
              tenant_id,
              channel.subscriber_count
          );

        receiver
    }

    /// Unsubscribe from a tenant's broadcast channel
    pub async fn unsubscribe(&self, tenant_id: &str) {
        let mut inner = self.inner.write().await;

        if let Some(channel) = inner.channels.get_mut(tenant_id) {
            channel.subscriber_count = channel.subscriber_count.saturating_sub(1);

            log::debug!(
                  "Client unsubscribed from tenant {} broadcast ({} remaining subscribers)",
                  tenant_id,
                  channel.subscriber_count
              );

            // Clean up empty channels
            if channel.subscriber_count == 0 {
                inner.channels.remove(tenant_id);
                log::info!("Removed empty broadcast channel for tenant {}", tenant_id);
            }
        }
    }

    /// Broadcast a message to all subscribers of a tenant
    pub async fn broadcast(
        &self,
        tenant_id: &str,
        message: BroadcastMessage,
    ) -> WsErrorResult<usize> {
        let inner = self.inner.read().await;

        if let Some(channel) = inner.channels.get(tenant_id) {
            let _subscriber_count = channel.subscriber_count;

            match channel.sender.send(message) {
                Ok(receiver_count) => {
                    log::debug!(
                          "Broadcast message to tenant {} ({} receivers)",
                          tenant_id,
                          receiver_count
                      );
                    Ok(receiver_count)
                }
                Err(_) => {
                    // No active receivers - this is OK, channel exists but no one listening
                    log::debug!("Broadcast to tenant {} had no active receivers", tenant_id);
                    Ok(0)
                }
            }
        } else {
            // No channel for this tenant - no subscribers yet
            log::debug!("No broadcast channel exists for tenant {}", tenant_id);
            Ok(0)
        }
    }

    /// Get subscriber count for a tenant
    pub async fn subscriber_count(&self, tenant_id: &str) -> usize {
        let inner = self.inner.read().await;
        inner
            .channels
            .get(tenant_id)
            .map(|c| c.subscriber_count)
            .unwrap_or(0)
    }

    /// Get all active tenant channels
    pub async fn active_tenants(&self) -> Vec<String> {
        let inner = self.inner.read().await;
        inner.channels.keys().cloned().collect()
    }

    /// Get total number of channels
    pub async fn channel_count(&self) -> usize {
        let inner = self.inner.read().await;
        inner.channels.len()
    }
}

impl Clone for TenantBroadcaster {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            config: self.config.clone(),
        }
    }
}