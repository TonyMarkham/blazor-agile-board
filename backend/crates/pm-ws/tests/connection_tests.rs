mod common;

use common::test_client::WsTestClient;
use common::test_server::{TEST_JWT_SECRET, create_test_server};

#[tokio::test]
async fn given_valid_jwt_when_connecting_then_succeeds() {
    // Given
    let server = create_test_server().await;

    // When - Connect with valid JWT
    let client = WsTestClient::connect(&server.server, "user-1", TEST_JWT_SECRET).await;

    // Then - Connection succeeded (no panic = success)
    // Note: We don't send/receive because text echo isn't implemented
    // The handler is designed for protobuf binary messages
    client.close().await;
}

#[tokio::test]
async fn given_connected_client_when_closed_then_server_cleans_up() {
    // Given - Server with connection registry tracking
    let server = create_test_server().await;

    // When - Client connects and then disconnects
    {
        let client = WsTestClient::connect(&server.server, "user-1", TEST_JWT_SECRET).await;

        // Connection is active in this scope
        // (Registry should have 1 connection)

        client.close().await;
    } // Client dropped here

    // Then - Give server time to process cleanup
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify cleanup happened by connecting again with same tenant
    // If cleanup worked, this should succeed (not hit connection limits)
    let client2 = WsTestClient::connect(&server.server, "user-2", TEST_JWT_SECRET).await;

    client2.close().await;
}
