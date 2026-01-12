use crate::{
    BroadcastMessage, ClientSubscriptions, ConnectionId, Metrics, Result, ShutdownGuard,
    SubscriptionFilter, TenantBroadcaster, WsError,
};

use pm_auth::{ConnectionRateLimiter, TenantContext};
use pm_core::ErrorLocation;

use std::panic::Location;

use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;

/// Configuration for WebSocket connections                                                                                                                                      
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// Send buffer size (bounded to handle backpressure)                                                                                                                        
    pub send_buffer_size: usize,
    /// Heartbeat interval in seconds                                                                                                                                            
    pub heartbeat_interval_secs: u64,
    /// Heartbeat timeout in seconds                                                                                                                                             
    pub heartbeat_timeout_secs: u64,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            send_buffer_size: 100,
            heartbeat_interval_secs: 30,
            heartbeat_timeout_secs: 60,
        }
    }
} 