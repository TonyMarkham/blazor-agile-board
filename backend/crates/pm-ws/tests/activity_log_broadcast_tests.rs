use pm_core::ActivityLog;
use pm_proto::{WebSocketMessage, web_socket_message::Payload};
use pm_ws::{ConnectionLimits, ConnectionRegistry, build_activity_log_created_event};

use axum::extract::ws::Message;
use prost::Message as ProstMessage;
use tokio::sync::mpsc;
use uuid::Uuid;

#[tokio::test]
async fn broadcast_only_reaches_subscribed_clients() {
    let registry = ConnectionRegistry::new(ConnectionLimits::default());

    let (tx_a, mut rx_a) = mpsc::channel::<Message>(8);
    let (tx_b, mut rx_b) = mpsc::channel::<Message>(8);

    let conn_a = registry.register("user-a".to_string(), tx_a).await.unwrap();
    let _conn_b = registry.register("user-b".to_string(), tx_b).await.unwrap();

    registry
        .subscribe(&conn_a.to_string(), &["p1".into()], &[])
        .await
        .unwrap();

    let activity = ActivityLog::created("work_item", Uuid::new_v4(), Uuid::new_v4());
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.clone().into());

    let delivered = registry
        .broadcast_activity_log_created("p1", Some("wi-1"), None, message)
        .await
        .unwrap();
    assert_eq!(delivered, 1);

    let received = rx_a
        .recv()
        .await
        .expect("expected message for subscribed client");
    if let Message::Binary(payload) = received {
        let decoded = WebSocketMessage::decode(&payload[..]).unwrap();
        assert!(matches!(
            decoded.payload,
            Some(Payload::ActivityLogCreated(_))
        ));
    } else {
        panic!("Expected binary message");
    }

    assert!(rx_b.try_recv().is_err());
}

#[tokio::test]
async fn broadcast_reaches_sprint_subscribers() {
    let registry = ConnectionRegistry::new(ConnectionLimits::default());

    let (tx_a, mut rx_a) = mpsc::channel::<Message>(8);
    let conn_a = registry.register("user-a".to_string(), tx_a).await.unwrap();

    registry
        .subscribe(&conn_a.to_string(), &[], &["s1".into()])
        .await
        .unwrap();

    let activity = ActivityLog::created("sprint", Uuid::new_v4(), Uuid::new_v4());
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.clone().into());

    let delivered = registry
        .broadcast_activity_log_created("p1", None, Some("s1"), message)
        .await
        .unwrap();
    assert_eq!(delivered, 1);

    let received = rx_a
        .recv()
        .await
        .expect("expected message for sprint subscriber");
    if let Message::Binary(payload) = received {
        let decoded = WebSocketMessage::decode(&payload[..]).unwrap();
        assert!(matches!(
            decoded.payload,
            Some(Payload::ActivityLogCreated(_))
        ));
    } else {
        panic!("Expected binary message");
    }
}
