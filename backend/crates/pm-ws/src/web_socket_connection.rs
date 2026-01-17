use crate::{
    ClientSubscriptions, ConnectionConfig, ConnectionId, Metrics, Result as WsErrorResult,
    ShutdownGuard, WsError,
};

use pm_auth::ConnectionRateLimiter;

use std::panic::Location;

use axum::extract::ws::{Message, WebSocket};
use error_location::ErrorLocation;
use futures::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use tokio::sync::mpsc;

pub const MAX_VIOLATIONS: u32 = 5;

/// Manages a single WebSocket connection
pub struct WebSocketConnection {
    connection_id: ConnectionId,
    config: ConnectionConfig,
    metrics: Metrics,
    rate_limiter: ConnectionRateLimiter,
    #[allow(dead_code)]
    subscriptions: ClientSubscriptions,
    rate_limit_violations: u32,
}

impl WebSocketConnection {
    pub fn new(
        connection_id: ConnectionId,
        config: ConnectionConfig,
        metrics: Metrics,
        rate_limiter: ConnectionRateLimiter,
    ) -> Self {
        Self {
            connection_id,
            config,
            metrics,
            rate_limiter,
            subscriptions: ClientSubscriptions::new(),
            rate_limit_violations: 0,
        }
    }

    /// Handle the WebSocket connection lifecycle
    pub async fn handle(
        mut self,
        socket: WebSocket,
        mut shutdown_guard: ShutdownGuard,
    ) -> WsErrorResult<()> {
        info!("WebSocket connection {} established", self.connection_id);

        // Split socket into sender and receiver
        let (mut ws_sender, mut ws_receiver) = socket.split();

        // Create bounded channel for outgoing messages (backpressure handling)
        let (tx, mut rx) = mpsc::channel::<Message>(self.config.send_buffer_size);

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
                                error!(
                                    "Error handling message from connection {}: {}",
                                    self.connection_id,
                                    e
                                );
                                self.metrics.error_occurred("message_handling");
                                break Err(e);
                            }
                        }
                        Some(Err(e)) => {
                            error!(
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
                            info!("Connection {} closed by client", self.connection_id);
                            break Ok(());
                        }
                    }
                }

                // Handle graceful shutdown
                _ = shutdown_guard.wait() => {
                    info!("Shutting down connection {} gracefully", self.connection_id);
                    break Ok(());
                }
            }
        };

        // Cleanup
        drop(tx); // Close channel to terminate send task
        let _ = send_task.await;

        self.metrics
            .connection_closed(if result.is_ok() { "normal" } else { "error" });

        info!("WebSocket connection {} closed", self.connection_id);

        result
    }

    /// Handle a message from the client
    async fn handle_client_message(
        &mut self,
        msg: Message,
        tx: &mpsc::Sender<Message>,
    ) -> WsErrorResult<()> {
        // Check rate limit
        if let Err(e) = self.rate_limiter.check() {
            self.rate_limit_violations += 1;

            if self.rate_limit_violations >= MAX_VIOLATIONS {
                // Too many violations - close connection to prevent DoS
                warn!(
                    "Connection {} exceeded rate limit {} times, closing connection",
                    self.connection_id, self.rate_limit_violations
                );

                let close_frame = axum::extract::ws::CloseFrame {
                    code: axum::extract::ws::close_code::POLICY,
                    reason: format!(
                        "Rate limit exceeded {} times. Connection closed.",
                        MAX_VIOLATIONS
                    )
                    .into(),
                };

                let _ = tx.send(Message::Close(Some(close_frame))).await;
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

                return Err(WsError::Internal {
                    message: format!("Rate limit: {}", e),
                    location: ErrorLocation::from(Location::caller()),
                });
            } else {
                // Send warning but keep connection open
                warn!(
                    "Rate limit exceeded for connection {} (violation {}/{})",
                    self.connection_id, self.rate_limit_violations, MAX_VIOLATIONS
                );

                let warning = format!(
                    "Rate limit exceeded. Slow down. ({}/{} warnings)",
                    self.rate_limit_violations, MAX_VIOLATIONS
                );
                let _ = tx.send(Message::Text(warning.into())).await;

                // Drop this message but continue processing
                return Ok(());
            }
        } else {
            // Successful message - reset violation counter
            self.rate_limit_violations = 0;
        }

        match msg {
            Message::Binary(data) => self.handle_binary_message(data, tx).await,
            Message::Text(text) => {
                debug!("Received text message: {}", text);
                // Could support JSON for compatibility, but we prefer binary protobuf
                Ok(())
            }
            Message::Ping(data) => {
                tx.send(Message::Pong(data))
                    .await
                    .map_err(|_| WsError::SendBufferFull {
                        location: ErrorLocation::from(Location::caller()),
                    })?;
                Ok(())
            }
            Message::Pong(_) => {
                // Heartbeat response received
                Ok(())
            }
            Message::Close(_) => {
                info!(
                    "Received close frame from connection {}",
                    self.connection_id
                );
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
        debug!(
            "Received binary message ({} bytes) from connection {}",
            data.len(),
            self.connection_id
        );

        self.metrics.message_received("unknown");

        // Example: Subscribe/Unsubscribe handling
        // This will be expanded when protobuf definitions are complete

        Ok(())
    }
}
