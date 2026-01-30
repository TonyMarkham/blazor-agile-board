use crate::{
    CircuitBreaker, ClientSubscriptions, ConnectionConfig, ConnectionId, ConnectionRegistry,
    HandlerContext, Metrics, Result as WsErrorResult, ShutdownGuard, WsError, dispatch,
};

use pm_auth::ConnectionRateLimiter;
use pm_proto::WebSocketMessage;

use std::panic::Location;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use error_location::ErrorLocation;
use futures::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use prost::Message as ProstMessage;
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use uuid::Uuid;

pub const MAX_VIOLATIONS: u32 = 5;

/// Manages a single WebSocket connection
pub struct WebSocketConnection {
    connection_id: ConnectionId,
    #[allow(dead_code)]
    config: ConnectionConfig,
    metrics: Metrics,
    rate_limiter: ConnectionRateLimiter,
    pool: SqlitePool,
    circuit_breaker: Arc<CircuitBreaker>,
    user_id: Uuid,
    #[allow(dead_code)]
    subscriptions: ClientSubscriptions,
    rate_limit_violations: u32,
    registry: ConnectionRegistry,
    outgoing_rx: mpsc::Receiver<Message>,
    outgoing_tx: mpsc::Sender<Message>,
}

pub struct WebSocketConnectionParams {
    pub connection_id: ConnectionId,
    pub config: ConnectionConfig,
    pub metrics: Metrics,
    pub rate_limiter: ConnectionRateLimiter,
    pub pool: SqlitePool,
    pub circuit_breaker: Arc<CircuitBreaker>,
    pub user_id: Uuid,
    pub registry: ConnectionRegistry,
    pub outgoing_rx: mpsc::Receiver<Message>,
    pub outgoing_tx: mpsc::Sender<Message>,
}

impl WebSocketConnection {
    pub fn new(params: WebSocketConnectionParams) -> Self {
        Self {
            connection_id: params.connection_id,
            config: params.config,
            metrics: params.metrics,
            rate_limiter: params.rate_limiter,
            pool: params.pool,
            circuit_breaker: params.circuit_breaker,
            user_id: params.user_id,
            subscriptions: ClientSubscriptions::new(),
            rate_limit_violations: 0,
            registry: params.registry,
            outgoing_rx: params.outgoing_rx,
            outgoing_tx: params.outgoing_tx,
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
        let mut rx = std::mem::replace(&mut self.outgoing_rx, mpsc::channel::<Message>(1).1);
        let outgoing_tx = self.outgoing_tx.clone();
        let mut connection = self;

        // Spawn send task (forward server messages to client)
        let send_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if ws_sender.send(msg).await.is_err() {
                    break;
                }
            }
        });

        let result = loop {
            tokio::select! {
                msg = ws_receiver.next() => {
                    match msg {
                        Some(Ok(msg)) => {
                            match msg {
                                Message::Close(_) => {
                                    info!("Connection {} closed by client", connection.connection_id);
                                    break Ok(());
                                }
                                _ => {
                                    if let Err(e) = connection.handle_client_message(msg, &outgoing_tx).await {
                                        error!(
                                            "Error handling message from connection {}: {}",
                                            connection.connection_id,
                                            e
                                        );
                                        connection.metrics.error_occurred("message_handling");
                                        break Err(e);
                                    }
                                }
                            }
                        }
                        Some(Err(e)) => {
                            error!(
                                "WebSocket error on connection {}: {}",
                                connection.connection_id,
                                e
                            );
                            break Err(WsError::ConnectionClosed {
                                reason: format!("WebSocket error: {}", e),
                                location: ErrorLocation::from(Location::caller()),
                            });
                        }
                        None => {
                            info!("Connection {} closed by client", connection.connection_id);
                            break Ok(());
                        }
                    }
                }

                // Handle graceful shutdown
                _ = shutdown_guard.wait() => {
                    info!("Shutting down connection {} gracefully", connection.connection_id);
                    break Ok(());
                }
            }
        };

        // Unregister early to drop registry-held sender clone
        connection
            .registry
            .unregister(connection.connection_id)
            .await;

        // Cleanup
        drop(outgoing_tx); // Drop the local clone
        let (dummy_tx, _dummy_rx) = mpsc::channel::<Message>(1);
        let _old_tx = std::mem::replace(&mut connection.outgoing_tx, dummy_tx);
        drop(_old_tx); // Drop the connection-held sender
        let _ = send_task.await;

        connection
            .metrics
            .connection_closed(if result.is_ok() { "normal" } else { "error" });

        info!("WebSocket connection {} closed", connection.connection_id);

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
        tx: &mpsc::Sender<Message>,
    ) -> WsErrorResult<()> {
        debug!(
            "Received binary message ({} bytes) from connection {}",
            data.len(),
            self.connection_id
        );

        // Decode protobuf message
        let msg = WebSocketMessage::decode(&data[..]).map_err(|e| WsError::Internal {
            message: format!("Failed to decode protobuf message: {}", e),
            location: ErrorLocation::from(Location::caller()),
        })?;

        self.metrics.message_received("binary");

        // Create handler context
        let ctx = HandlerContext::new(
            msg.message_id.clone(),
            self.user_id,
            self.pool.clone(),
            self.circuit_breaker.clone(),
            self.connection_id.to_string(),
            self.registry.clone(),
        );

        // Dispatch to appropriate handler
        let response = dispatch(msg, ctx).await;

        // Encode response
        let response_bytes = response.encode_to_vec();

        // Send response back to client
        tx.send(Message::Binary(response_bytes.into()))
            .await
            .map_err(|_| WsError::SendBufferFull {
                location: ErrorLocation::from(Location::caller()),
            })?;

        Ok(())
    }
}
