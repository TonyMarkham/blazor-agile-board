mod common;

use common::{
    test_client::WsTestClient,
    test_server::{TEST_JWT_SECRET, TestServerConfig, create_test_server_with_config},
};

use pm_ws::{BroadcastMessage, MAX_VIOLATIONS};

use bytes::Bytes;
use tokio::time::{Duration, timeout};

#[tokio::test]
async fn given_client_when_slightly_exceeding_rate_limit_then_receives_warnings_and_stays_connected()
 {
    // Given - Server with strict rate limit (5 req/sec, 5 token bucket)
    let config = TestServerConfig::with_strict_rate_limits();
    let test_server = create_test_server_with_config(config);
    let tenant_id = "tenant-warning";

    let mut client =
        WsTestClient::connect(&test_server.server, tenant_id, "user-1", TEST_JWT_SECRET).await;

    // When - Send enough messages to trigger warnings but stay under threshold
    // First 5 use token bucket, next (MAX_VIOLATIONS - 2) trigger warnings
    let message_count = 5 + (MAX_VIOLATIONS - 2);
    for i in 0..message_count {
        let payload = Bytes::from(format!("message-{}", i));
        client.send_binary(payload).await;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Give server time to process
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Then - Connection should still be open (only 3 violations, threshold is 5)
    let broadcast_payload = Bytes::from("test broadcast");
    let message = BroadcastMessage::new(broadcast_payload.clone(), "warning_test".to_string());

    let receiver_count = test_server
        .app_state
        .broadcaster
        .broadcast(tenant_id, message)
        .await
        .expect("Broadcast should succeed");

    assert_eq!(
        receiver_count, 1,
        "Client should still be connected after warnings"
    );

    // Should be able to receive broadcast
    let received = timeout(Duration::from_millis(500), client.receive_binary()).await;
    assert!(
        received.is_ok(),
        "Client should receive broadcast after warnings"
    );

    client.close().await;
}

#[tokio::test]
async fn given_client_when_severely_exceeding_rate_limit_then_disconnected_after_threshold() {
    // Given - Server with strict rate limit
    let config = TestServerConfig::with_strict_rate_limits();
    let test_server = create_test_server_with_config(config);
    let tenant_id = "tenant-disconnect";

    let mut client =
        WsTestClient::connect(&test_server.server, tenant_id, "user-1", TEST_JWT_SECRET).await;

    // When - Send enough messages to exceed threshold
    // First 5 use token bucket, next MAX_VIOLATIONS trigger disconnect
    let message_count = 5 + MAX_VIOLATIONS;
    for i in 0..message_count {
        let payload = Bytes::from(format!("message-{}", i));
        client.send_binary(payload).await;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Give server time to process and close connection
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Then - Connection should be closed (5 violations hit threshold)
    let broadcast_payload = Bytes::from("test broadcast");
    let message = BroadcastMessage::new(broadcast_payload, "threshold_test".to_string());

    let receiver_count = test_server
        .app_state
        .broadcaster
        .broadcast(tenant_id, message)
        .await
        .expect("Broadcast should succeed");

    assert_eq!(
        receiver_count, 0,
        "Client should be disconnected after threshold"
    );
}

#[tokio::test]
async fn given_client_when_rate_limited_then_slows_down_then_violation_counter_resets() {
    // Given - Server with strict rate limit
    let config = TestServerConfig::with_strict_rate_limits();
    let test_server = create_test_server_with_config(config);
    let tenant_id = "tenant-reset";

    let mut client =
        WsTestClient::connect(&test_server.server, tenant_id, "user-1", TEST_JWT_SECRET).await;

    // When - Send messages to trigger some violations (but not all)
    let initial_burst = 5 + 2; // Use 5 tokens + 2 violations
    for i in 0..initial_burst {
        let payload = Bytes::from(format!("burst-{}", i));
        client.send_binary(payload).await;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Wait for token bucket to refill (500ms = 2-3 tokens refilled)
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Send message successfully (should reset violation counter)
    let success_payload = Bytes::from("success after wait");
    client.send_binary(success_payload).await;

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Then - Can send messages again without hitting threshold
    // After reset, we can trigger (MAX_VIOLATIONS - 1) violations and still be connected
    // After 500ms wait, we have ~2 tokens, so send 2 + (MAX_VIOLATIONS - 1)
    let after_reset_count = 2 + (MAX_VIOLATIONS - 1);
    for i in 0..after_reset_count {
        let payload = Bytes::from(format!("after-reset-{}", i));
        client.send_binary(payload).await;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Connection should still be open (only 3 new violations after reset)
    let broadcast_payload = Bytes::from("test broadcast");
    let message = BroadcastMessage::new(broadcast_payload.clone(), "reset_test".to_string());

    let receiver_count = test_server
        .app_state
        .broadcaster
        .broadcast(tenant_id, message)
        .await
        .expect("Broadcast should succeed");

    assert_eq!(
        receiver_count, 1,
        "Client should still be connected (counter was reset)"
    );

    client.close().await;
}

#[tokio::test]
async fn given_rate_limited_connection_when_reconnecting_then_fresh_limiter_works() {
    // Given - Server with strict rate limits
    let config = TestServerConfig::with_strict_rate_limits();
    let test_server = create_test_server_with_config(config);
    let tenant_id = "tenant-reconnect";

    // First connection - hit threshold and get disconnected
    {
        let mut client1 =
            WsTestClient::connect(&test_server.server, tenant_id, "user-1", TEST_JWT_SECRET).await;

        // Send enough messages to exceed threshold
        // First 5 use token bucket, next MAX_VIOLATIONS trigger disconnect
        let message_count = 5 + MAX_VIOLATIONS;
        for i in 0..message_count {
            let payload = Bytes::from(format!("burst-{}", i));
            client1.send_binary(payload).await;
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        tokio::time::sleep(Duration::from_millis(200)).await;

        // Verify first connection is closed
        let check_payload = Bytes::from("check disconnection");
        let check_message = BroadcastMessage::new(check_payload, "check".to_string());
        let count = test_server
            .app_state
            .broadcaster
            .broadcast(tenant_id, check_message)
            .await
            .expect("Broadcast should succeed");

        assert_eq!(count, 0, "First connection should be disconnected");
    }

    // When - Reconnect with new connection (fresh rate limiter, violations = 0)
    let mut client2 =
        WsTestClient::connect(&test_server.server, tenant_id, "user-2", TEST_JWT_SECRET).await;

    // Then - New connection works with fresh token bucket
    let broadcast_payload = Bytes::from("broadcast after reconnect");
    let message = BroadcastMessage::new(broadcast_payload.clone(), "reconnect_test".to_string());

    let receiver_count = test_server
        .app_state
        .broadcaster
        .broadcast(tenant_id, message)
        .await
        .expect("Broadcast should succeed");

    assert_eq!(receiver_count, 1, "New connection should be active");

    let received = timeout(Duration::from_millis(500), client2.receive_binary())
        .await
        .expect("Should receive broadcast");

    assert_eq!(received, broadcast_payload);

    client2.close().await;
}
