mod common;

use common::{
    test_client::WsTestClient,
    test_server::{TEST_JWT_SECRET, TestServerConfig, create_test_server_with_config},
};

use tokio::time::{Duration, sleep};

#[tokio::test]
async fn given_tenant_channel_when_all_clients_disconnect_then_channel_cleaned_up() {
    // Given - Server with multiple clients for a tenant
    let config = TestServerConfig::default();
    let test_server = create_test_server_with_config(config);
    let tenant_id = "tenant-cleanup";

    // Connect 3 clients to same tenant
    let client1 =
        WsTestClient::connect(&test_server.server, tenant_id, "user-1", TEST_JWT_SECRET).await;
    let client2 =
        WsTestClient::connect(&test_server.server, tenant_id, "user-2", TEST_JWT_SECRET).await;
    let client3 =
        WsTestClient::connect(&test_server.server, tenant_id, "user-3", TEST_JWT_SECRET).await;

    // Verify channel exists and has 3 subscribers
    let active_tenants = test_server.app_state.broadcaster.active_tenants().await;
    assert!(
        active_tenants.contains(&tenant_id.to_string()),
        "Tenant channel should exist"
    );

    let subscriber_count = test_server
        .app_state
        .broadcaster
        .subscriber_count(tenant_id)
        .await;
    assert_eq!(subscriber_count, 3, "Should have 3 subscribers");

    // When - All clients disconnect
    client1.close().await;
    client2.close().await;
    client3.close().await;

    // Give server time to process disconnections and cleanup
    sleep(Duration::from_millis(200)).await;

    // Then - Channel should be cleaned up (no memory leak)
    let active_tenants_after = test_server.app_state.broadcaster.active_tenants().await;
    assert!(
        !active_tenants_after.contains(&tenant_id.to_string()),
        "Tenant channel should be removed when all clients disconnect"
    );

    let subscriber_count_after = test_server
        .app_state
        .broadcaster
        .subscriber_count(tenant_id)
        .await;
    assert_eq!(subscriber_count_after, 0, "Subscriber count should be 0");
}
