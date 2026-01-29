mod common;

use common::{
    jwt_helper::create_test_token,
    test_client::WsTestClient,
    test_server::{TEST_JWT_SECRET, TestServerConfig, create_test_server_with_config},
};

use tokio::time::{Duration, sleep};

#[tokio::test]
async fn given_server_at_total_limit_when_new_connection_then_rejected_503() {
    // Given - Server with total limit of 5 connections
    let config = TestServerConfig::with_strict_limits();
    let test_server = create_test_server_with_config(config).await;

    // Create 5 connections (at total limit)
    let _client1 = WsTestClient::connect(&test_server.server, "user-1", TEST_JWT_SECRET).await;
    let _client2 = WsTestClient::connect(&test_server.server, "user-2", TEST_JWT_SECRET).await;
    let _client3 = WsTestClient::connect(&test_server.server, "user-3", TEST_JWT_SECRET).await;
    let _client4 = WsTestClient::connect(&test_server.server, "user-4", TEST_JWT_SECRET).await;
    let _client5 = WsTestClient::connect(&test_server.server, "user-5", TEST_JWT_SECRET).await;

    // When - Try to create 6th connection
    let response = test_server
        .server
        .get_websocket("/ws")
        .add_header(
            "Authorization",
            format!("Bearer {}", create_test_token("user-6", TEST_JWT_SECRET)),
        )
        .await;

    // Then - Rejected with 503
    response.assert_status_service_unavailable();
}

#[tokio::test]
async fn given_server_at_limit_when_one_disconnects_then_new_can_connect() {
    // Given - Server at limit
    let config = TestServerConfig::with_strict_limits();
    let test_server = create_test_server_with_config(config).await;

    let client1 = WsTestClient::connect(&test_server.server, "user-1", TEST_JWT_SECRET).await;
    let _client2 = WsTestClient::connect(&test_server.server, "user-2", TEST_JWT_SECRET).await;
    let _client3 = WsTestClient::connect(&test_server.server, "user-3", TEST_JWT_SECRET).await;
    let _client4 = WsTestClient::connect(&test_server.server, "user-4", TEST_JWT_SECRET).await;
    let _client5 = WsTestClient::connect(&test_server.server, "user-5", TEST_JWT_SECRET).await;

    // When - One client disconnects
    client1.close().await;

    // Wait until registry reflects the disconnect
    let mut disconnected = false;
    for _ in 0..40 {
        if test_server.app_state.registry.total_count().await == 4 {
            disconnected = true;
            break;
        }
        sleep(Duration::from_millis(25)).await;
    }
    assert!(disconnected, "registry did not drop to 4 after close");

    // Then - New connection succeeds (slot freed)
    let client6 = WsTestClient::connect(&test_server.server, "user-6", TEST_JWT_SECRET).await;
    client6.close().await;
}
