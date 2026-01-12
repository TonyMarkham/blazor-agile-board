use crate::{
    BroadcastMessage, ClientSubscriptions, ConnectionId, Metrics, Result as WsErrorResult, ShutdownGuard,
    ConnectionConfig, TenantBroadcaster, WsError,
};

use pm_auth::{ConnectionRateLimiter, TenantContext};
use pm_core::ErrorLocation;

use std::panic::Location;

use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;

/// Manages a single WebSocket connection
pub struct WebSocketConnection {
    connection_id: ConnectionId,
    tenant_context: TenantContext,
    config: ConnectionConfig,
    metrics: Metrics,
    rate_limiter: ConnectionRateLimiter,
    broadcaster: TenantBroadcaster,
    subscriptions: ClientSubscriptions,
}

impl WebSocketConnection {
    pub fn new(
        connection_id: ConnectionId,
        tenant_context: TenantContext,
        config: ConnectionConfig,
        metrics: Metrics,
        rate_limiter: ConnectionRateLimiter,
        broadcaster: TenantBroadcaster,
    ) -> Self {
        Self {
            connection_id,
            tenant_context,
            config,
            metrics,
            rate_limiter,
            broadcaster,
            subscriptions: ClientSubscriptions::new(),
        }
    }

    /// Handle the WebSocket connection lifecycle
    pub async fn handle(
        mut self,
        socket: WebSocket,
        mut shutdown_guard: ShutdownGuard,
    ) -> WsErrorResult<()> {
        log::info!(
              "WebSocket connection {} established for tenant {} (user {})",
              self.connection_id,
              self.tenant_context.tenant_id,
              self.tenant_context.user_id
          );

        self.metrics.connection_established(&self.tenant_context.tenant_id);

        // Split socket into sender and receiver
        let (mut ws_sender, mut ws_receiver) = socket.split();

        // Create bounded channel for outgoing messages (backpressure handling)
        let (tx, mut rx) = mpsc::channel::<Message>(self.config.send_buffer_size);

        // Subscribe to tenant broadcasts
        let mut broadcast_rx = self.broadcaster.subscribe(&self.tenant_context.tenant_id).await;

        // Spawn send task
        let send_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if ws_sender.send(msg).await.is_err() {
                    break;
                }
            }
        });

        let result = loop {
            tokio::select! {
                  // Handle incoming messages from client
                  msg = ws_receiver.next() => {
                      match msg {
                          Some(Ok(msg)) => {
                              if let Err(e) = self.handle_client_message(msg, &tx).await {
                                  log::error!(
                                      "Error handling message from connection {}: {}",
                                      self.connection_id,
                                      e
                                  );
                                  self.metrics.error_occurred(
                                      &self.tenant_context.tenant_id,
                                      "message_handling"
                                  );
                                  break Err(e);
                              }
                          }
                          Some(Err(e)) => {
                              log::error!(
                                  "WebSocket error on connection {}: {}",
                                  self.connection_id,
                                  e
                              );
                              break Err(WsError::ConnectionClosed {
                                  reason: format!("WebSocket error: {}", e),
                                  location: ErrorLocation::from(Location::caller()),
                              });
                          }
                          None => {
                              log::info!("Connection {} closed by client", self.connection_id);
                              break Ok(());
                          }
                      }
                  }

                  // Handle broadcast messages from server
                  broadcast_msg = broadcast_rx.recv() => {
                      match broadcast_msg {
                          Ok(msg) => {
                              if let Err(e) = self.handle_broadcast_message(msg, &tx).await {
                                  log::error!(
                                      "Error handling broadcast for connection {}: {}",
                                      self.connection_id,
                                      e
                                  );
                                  // Don't break on broadcast errors, just log
                              }
                          }
                          Err(tokio::sync::broadcast::error::RecvError::Lagged(missed)) => {
                              log::warn!(
                                  "Connection {} lagged, missed {} messages",
                                  self.connection_id,
                                  missed
                              );
                              self.metrics.error_occurred(
                                  &self.tenant_context.tenant_id,
                                  "broadcast_lagged"
                              );
                          }
                          Err(_) => {
                              log::info!("Broadcast channel closed for connection {}", self.connection_id);
                              break Ok(());
                          }
                      }
                  }

                  // Handle graceful shutdown
                  _ = shutdown_guard.wait() => {
                      log::info!("Shutting down connection {} gracefully", self.connection_id);
                      break Ok(());
                  }
              }
        };

        // Cleanup
        self.broadcaster.unsubscribe(&self.tenant_context.tenant_id).await;
        drop(tx); // Close channel to terminate send task
        let _ = send_task.await;

        self.metrics.connection_closed(
            &self.tenant_context.tenant_id,
            if result.is_ok() { "normal" } else { "error" }
        );

        log::info!(
              "WebSocket connection {} closed for tenant {}",
              self.connection_id,
              self.tenant_context.tenant_id
          );

        result
    }

    /// Handle a message from the client
    async fn handle_client_message(
        &mut self,
        msg: Message,
        tx: &mpsc::Sender<Message>,
    ) -> WsErrorResult<()> {
        // Check rate limit
        self.rate_limiter.check().map_err(|e| {
            log::warn!(
                  "Rate limit exceeded for connection {} (tenant {})",
                  self.connection_id,
                  self.tenant_context.tenant_id
              );
            WsError::Internal {
                message: format!("Rate limit: {}", e),
                location: ErrorLocation::from(Location::caller()),
            }
        })?;

        match msg {
            Message::Binary(data) => {
                self.handle_binary_message(data, tx).await
            }
            Message::Text(text) => {
                log::debug!("Received text message: {}", text);
                // Could support JSON for compatibility, but we prefer binary protobuf
                Ok(())
            }
            Message::Ping(data) => {
                tx.send(Message::Pong(data)).await.map_err(|_| WsError::SendBufferFull {
                    location: ErrorLocation::from(Location::caller()),
                })?;
                Ok(())
            }
            Message::Pong(_) => {
                // Heartbeat response received
                Ok(())
            }
            Message::Close(_) => {
                log::info!("Received close frame from connection {}", self.connection_id);
                Ok(())
            }
        }
    }

    /// Handle a binary protobuf message from client
    async fn handle_binary_message(
        &mut self,
        data: bytes::Bytes,
        _tx: &mpsc::Sender<Message>,
    ) -> WsErrorResult<()> {
        // Decode protobuf message
        // TODO: Once protobuf client messages are defined, decode here
        // For now, just log
        log::debug!(
              "Received binary message ({} bytes) from connection {}",
              data.len(),
              self.connection_id
          );

        self.metrics.message_received(&self.tenant_context.tenant_id, "unknown");

        // Example: Subscribe/Unsubscribe handling
        // This will be expanded when protobuf definitions are complete

        Ok(())
    }

    /// Handle a broadcast message from server (filter and forward to client)
    async fn handle_broadcast_message(
        &self,
        msg: BroadcastMessage,
        tx: &mpsc::Sender<Message>,
    ) -> WsErrorResult<()> {
        // TODO: Parse message and check subscriptions
        // For now, forward all messages (will be filtered in later sessions)

        tx.send(Message::Binary(msg.payload))
            .await
            .map_err(|_| WsError::SendBufferFull {
                location: ErrorLocation::from(Location::caller()),
            })?;

        self.metrics.message_sent(&self.tenant_context.tenant_id, &msg.message_type);

        Ok(())
    }
}