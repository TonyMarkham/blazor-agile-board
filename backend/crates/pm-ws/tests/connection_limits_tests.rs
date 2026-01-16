mod common;

use common::{
    jwt_helper::create_test_token,
    test_client::WsTestClient,
    test_server::{TEST_JWT_SECRET, TestServerConfig, create_test_server_with_config},
};

use tokio::time::{Duration, sleep};

#[tokio::test]
async fn given_tenant_at_limit_when_new_connection_then_rejected_503() {
    // Given - Server with limit of 2 connections per tenant
    let config = TestServerConfig::with_strict_limits();
    let test_server = create_test_server_with_config(config);
    let tenant_id = "tenant-limit-test";

    // Create 2 connections (at limit)
    let _client1 =
        WsTestClient::connect(&test_server.server, tenant_id, "user-1", TEST_JWT_SECRET).await;
    let _client2 =
        WsTestClient::connect(&test_server.server, tenant_id, "user-2", TEST_JWT_SECRET).await;

    // When - Try to create 3rd connection for same tenant
    let response = test_server
        .server
        .get_websocket("/ws")
        .add_header(
            "Authorization",
            format!(
                "Bearer {}",
                create_test_token(tenant_id, "user-3", TEST_JWT_SECRET)
            ),
        )
        .await;

    // Then - Rejected with 503
    response.assert_status_service_unavailable();
}

#[tokio::test]
async fn given_server_at_total_limit_when_new_connection_then_rejected_503() {
    // Given - Server with total limit of 5 connections
    let config = TestServerConfig::with_strict_limits();
    let test_server = create_test_server_with_config(config);

    // Create 5 connections across different tenants (at total limit)
    let _client1 =
        WsTestClient::connect(&test_server.server, "tenant-1", "user-1", TEST_JWT_SECRET).await;
    let _client2 =
        WsTestClient::connect(&test_server.server, "tenant-2", "user-1", TEST_JWT_SECRET).await;
    let _client3 =
        WsTestClient::connect(&test_server.server, "tenant-3", "user-1", TEST_JWT_SECRET).await;
    let _client4 =
        WsTestClient::connect(&test_server.server, "tenant-4", "user-1", TEST_JWT_SECRET).await;
    let _client5 =
        WsTestClient::connect(&test_server.server, "tenant-5", "user-1", TEST_JWT_SECRET).await;

    // When - Try to create 6th connection
    let response = test_server
        .server
        .get_websocket("/ws")
        .add_header(
            "Authorization",
            format!(
                "Bearer {}",
                create_test_token("tenant-6", "user-1", TEST_JWT_SECRET)
            ),
        )
        .await;

    // Then - Rejected with 503
    response.assert_status_service_unavailable();
}

#[tokio::test]
async fn given_tenant_at_limit_when_other_tenant_connects_then_succeeds() {
    // Given - Tenant-1 at its limit (2 connections)
    let config = TestServerConfig::with_strict_limits();
    let test_server = create_test_server_with_config(config);

    let _client1 =
        WsTestClient::connect(&test_server.server, "tenant-1", "user-1", TEST_JWT_SECRET).await;
    let _client2 =
        WsTestClient::connect(&test_server.server, "tenant-1", "user-2", TEST_JWT_SECRET).await;

    // When - Tenant-2 tries to connect
    let client3 =
        WsTestClient::connect(&test_server.server, "tenant-2", "user-1", TEST_JWT_SECRET).await;

    // Then - Succeeds (limits are per-tenant, not global)
    client3.close().await;
}

#[tokio::test]
async fn given_tenant_at_limit_when_one_disconnects_then_new_can_connect() {
    // Given - Tenant at limit
    let config = TestServerConfig::with_strict_limits();
    let test_server = create_test_server_with_config(config);
    let tenant_id = "tenant-disconnect-test";

    let client1 =
        WsTestClient::connect(&test_server.server, tenant_id, "user-1", TEST_JWT_SECRET).await;
    let _client2 =
        WsTestClient::connect(&test_server.server, tenant_id, "user-2", TEST_JWT_SECRET).await;

    // When - One client disconnects
    client1.close().await;

    // Give server time to process disconnect
    sleep(Duration::from_millis(100)).await;

    // Then - New connection succeeds (slot freed)
    let client3 =
        WsTestClient::connect(&test_server.server, tenant_id, "user-3", TEST_JWT_SECRET).await;
    client3.close().await;
}
