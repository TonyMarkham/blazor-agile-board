use crate::common::jwt_helper::create_test_token;

use axum_test::{TestServer, TestWebSocket, WsMessage};
use bytes::Bytes;

/// WebSocket test client wrapper
pub struct WsTestClient {
    ws: TestWebSocket,
    pub tenant_id: String,
    pub user_id: String,
}

impl WsTestClient {
    /// Connect to WebSocket endpoint with JWT authentication
    pub async fn connect(
        server: &TestServer,
        tenant_id: &str,
        user_id: &str,
        jwt_secret: &[u8],
    ) -> Self {
        let token = create_test_token(tenant_id, user_id, jwt_secret);

        let ws = server
            .get_websocket("/ws")
            .add_header("Authorization", format!("Bearer {}", token))
            .await
            .into_websocket()
            .await;

        Self {
            ws,
            tenant_id: tenant_id.to_string(),
            user_id: user_id.to_string(),
        }
    }

    /// Send binary message (for protobuf messages)
    pub async fn send_binary(&mut self, data: impl Into<Bytes>) {
        let bytes = data.into();
        self.ws.send_message(WsMessage::Binary(bytes)).await;
    }

    /// Receive binary message
    pub async fn receive_binary(&mut self) -> Bytes {
        self.ws.receive_bytes().await
    }

    /// Send text message (for debugging/simple tests)
    pub async fn send_text(&mut self, text: impl std::fmt::Display) {
        self.ws.send_text(text).await;
    }

    /// Receive text message
    pub async fn receive_text(&mut self) -> String {
        self.ws.receive_text().await
    }

    /// Close the WebSocket connection
    pub async fn close(self) {
        self.ws.close().await;
    }

    /// Get mutable reference to underlying TestWebSocket for advanced usage
    pub fn ws_mut(&mut self) -> &mut TestWebSocket {
        &mut self.ws
    }
}

/// Create multiple clients for the same tenant (helper for broadcast tests)
pub async fn create_clients_for_tenant(
    server: &TestServer,
    tenant_id: &str,
    user_id_prefix: &str,
    count: usize,
) -> Vec<WsTestClient> {
    let mut clients = Vec::with_capacity(count);
    for i in 0..count {
        let user_id = format!("{}-{}", user_id_prefix, i + 1);
        let client = WsTestClient::connect(server, tenant_id, &user_id, super::test_server::TEST_JWT_SECRET).await;
        clients.push(client);
    }
    clients
}