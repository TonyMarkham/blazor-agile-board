mod common;

use common::{
    test_client::WsTestClient,
    test_server::{TEST_JWT_SECRET, create_test_server},
};

use pm_ws::BroadcastMessage;

use bytes::Bytes;
use tokio::time::{Duration, timeout};

#[tokio::test]
async fn given_two_clients_same_tenant_when_broadcast_then_both_receive() {
    // Given - Two clients connected to same tenant
    let test_server = create_test_server();
    let tenant_id = "tenant-broadcast-1";

    let mut client1 =
        WsTestClient::connect(&test_server.server, tenant_id, "user-1", TEST_JWT_SECRET).await;

    let mut client2 =
        WsTestClient::connect(&test_server.server, tenant_id, "user-2", TEST_JWT_SECRET).await;

    // When - Broadcast a message to the tenant
    let payload = Bytes::from("test broadcast message");
    let message = BroadcastMessage::new(payload.clone(), "test_message".to_string());

    let sent_count = test_server
        .app_state
        .broadcaster
        .broadcast(tenant_id, message)
        .await
        .expect("Broadcast should succeed");

    // Then - Both clients receive the message
    assert_eq!(sent_count, 2, "Should broadcast to 2 receivers");

    let received1 = client1.receive_binary().await;
    let received2 = client2.receive_binary().await;

    assert_eq!(received1, payload);
    assert_eq!(received2, payload);

    client1.close().await;
    client2.close().await;
}

#[tokio::test]
async fn given_tenant_with_multiple_users_when_broadcast_then_all_users_receive() {
    // Given - 5 clients for same tenant
    let test_server = create_test_server();
    let tenant_id = "tenant-multi-user";

    let mut clients = vec![];
    for i in 1..=5 {
        let client = WsTestClient::connect(
            &test_server.server,
            tenant_id,
            &format!("user-{}", i),
            TEST_JWT_SECRET,
        )
        .await;
        clients.push(client);
    }

    // When - Broadcast message
    let payload = Bytes::from("multi-user broadcast");
    let message = BroadcastMessage::new(payload.clone(), "multi_user_test".to_string());

    let sent_count = test_server
        .app_state
        .broadcaster
        .broadcast(tenant_id, message)
        .await
        .expect("Broadcast should succeed");

    // Then - All 5 clients receive
    assert_eq!(sent_count, 5);

    for client in &mut clients {
        let received = client.receive_binary().await;
        assert_eq!(received, payload);
    }

    for client in clients {
        client.close().await;
    }
}

#[tokio::test]
async fn given_fast_broadcasts_when_channel_full_then_handles_backpressure() {
    // Given - Client connected with limited channel capacity
    let test_server = create_test_server();
    let tenant_id = "tenant-backpressure";

    let mut client =
        WsTestClient::connect(&test_server.server, tenant_id, "user-1", TEST_JWT_SECRET).await;

    // When - Send many broadcasts rapidly (more than channel capacity)
    let broadcast_count = 200; // Default channel capacity is 128
    for i in 0..broadcast_count {
        let payload = Bytes::from(format!("message-{}", i));
        let message = BroadcastMessage::new(payload, "backpressure_test".to_string());

        let _ = test_server
            .app_state
            .broadcaster
            .broadcast(tenant_id, message)
            .await;
    }

    // Then - Client handles backpressure (either receives messages or gets Lagged error)
    // The WebSocketConnection logs RecvError::Lagged but continues running
    // We just verify the connection is still alive

    // Send one more message after the flood
    let final_payload = Bytes::from("final message");
    let final_message = BroadcastMessage::new(final_payload.clone(), "final".to_string());
    test_server
        .app_state
        .broadcaster
        .broadcast(tenant_id, final_message)
        .await
        .expect("Final broadcast should succeed");

    // Client should still be connected and receive the final message
    // Drain messages until we get the final one (with timeout to avoid infinite loop)
    let mut received_final = false;
    for _ in 0..250 {
        // More than we sent, to ensure we get to the final message
        let result = timeout(Duration::from_millis(100), client.receive_binary()).await;
        if let Ok(received) = result {
            if received == final_payload {
                received_final = true;
                break;
            }
        } else {
            // Timeout - no more messages
            break;
        }
    }

    assert!(
        received_final,
        "Client should still receive messages after backpressure"
    );

    client.close().await;
}

#[tokio::test]
async fn given_broadcast_when_client_disconnects_mid_receive_then_continues() {
    // Given - Two clients connected
    let test_server = create_test_server();
    let tenant_id = "tenant-disconnect";

    let mut client1 =
        WsTestClient::connect(&test_server.server, tenant_id, "user-1", TEST_JWT_SECRET).await;

    let mut client2 =
        WsTestClient::connect(&test_server.server, tenant_id, "user-2", TEST_JWT_SECRET).await;

    // When - Client 1 disconnects
    client1.close().await;

    // Give server time to process disconnect
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Broadcast message after client1 disconnected
    let payload = Bytes::from("after disconnect");
    let message = BroadcastMessage::new(payload.clone(), "disconnect_test".to_string());

    let sent_count = test_server
        .app_state
        .broadcaster
        .broadcast(tenant_id, message)
        .await
        .expect("Broadcast should succeed");

    // Then - Only client2 receives (client1 already disconnected)
    assert_eq!(
        sent_count, 1,
        "Should only broadcast to 1 remaining receiver"
    );

    let received = client2.receive_binary().await;
    assert_eq!(received, payload);

    client2.close().await;
}

#[tokio::test]
async fn given_multiple_tenants_when_broadcast_to_one_then_others_unaffected() {
    // Given - Clients from 3 different tenants
    let test_server = create_test_server();

    let mut tenant1_client =
        WsTestClient::connect(&test_server.server, "tenant-1", "user-1", TEST_JWT_SECRET).await;

    let mut tenant2_client =
        WsTestClient::connect(&test_server.server, "tenant-2", "user-1", TEST_JWT_SECRET).await;

    let mut tenant3_client =
        WsTestClient::connect(&test_server.server, "tenant-3", "user-1", TEST_JWT_SECRET).await;

    // When - Broadcast only to tenant-2
    let payload = Bytes::from("tenant-2 only message");
    let message = BroadcastMessage::new(payload.clone(), "isolation_test".to_string());

    let sent_count = test_server
        .app_state
        .broadcaster
        .broadcast("tenant-2", message)
        .await
        .expect("Broadcast should succeed");

    // Then - Only tenant-2 receives
    assert_eq!(sent_count, 1, "Should only broadcast to tenant-2");

    let received = tenant2_client.receive_binary().await;
    assert_eq!(received, payload);

    // Verify tenant-1 and tenant-3 did NOT receive (use timeout)
    use tokio::time::{Duration, timeout};

    let timeout_duration = Duration::from_millis(200);

    let tenant1_result = timeout(timeout_duration, tenant1_client.receive_binary()).await;
    assert!(
        tenant1_result.is_err(),
        "Tenant-1 should NOT receive message"
    );

    let tenant3_result = timeout(timeout_duration, tenant3_client.receive_binary()).await;
    assert!(
        tenant3_result.is_err(),
        "Tenant-3 should NOT receive message"
    );

    tenant1_client.close().await;
    tenant2_client.close().await;
    tenant3_client.close().await;
}
