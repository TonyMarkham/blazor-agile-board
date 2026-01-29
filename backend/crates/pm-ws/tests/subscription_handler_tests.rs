use pm_proto::{Subscribe, Unsubscribe};
use pm_ws::{
    CircuitBreaker, CircuitBreakerConfig, ConnectionLimits, ConnectionRegistry, HandlerContext,
    handle_subscribe, handle_unsubscribe,
};

use std::sync::Arc;

use axum::extract::ws::Message;
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use uuid::Uuid;

fn test_context(registry: ConnectionRegistry, connection_id: String) -> HandlerContext {
    let pool = SqlitePool::connect_lazy(":memory:").unwrap();
    let circuit_breaker = Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default()));
    HandlerContext::new(
        "msg-1".to_string(),
        Uuid::new_v4(),
        pool,
        circuit_breaker,
        connection_id,
        registry,
    )
}

#[tokio::test]
async fn subscribe_adds_project_and_sprint_ids() {
    let registry = ConnectionRegistry::new(ConnectionLimits::default());
    let (tx, _rx) = mpsc::channel::<Message>(8);
    let connection_id = registry.register("user-1".to_string(), tx).await.unwrap();

    let ctx = test_context(registry.clone(), connection_id.to_string());

    handle_subscribe(
        Subscribe {
            project_ids: vec!["p1".into()],
            sprint_ids: vec!["s1".into()],
        },
        ctx,
    )
    .await
    .unwrap();

    let info = registry.get(connection_id).await.unwrap();
    assert!(info.subscriptions.is_subscribed_to_project("p1"));
    assert!(info.subscriptions.is_subscribed_to_sprint("s1"));
}

#[tokio::test]
async fn unsubscribe_removes_project_and_sprint_ids() {
    let registry = ConnectionRegistry::new(ConnectionLimits::default());
    let (tx, _rx) = mpsc::channel::<Message>(8);
    let connection_id = registry.register("user-1".to_string(), tx).await.unwrap();

    registry
        .subscribe(&connection_id.to_string(), &["p1".into()], &["s1".into()])
        .await
        .unwrap();

    let ctx = test_context(registry.clone(), connection_id.to_string());

    handle_unsubscribe(
        Unsubscribe {
            project_ids: vec!["p1".into()],
            sprint_ids: vec!["s1".into()],
        },
        ctx,
    )
    .await
    .unwrap();

    let info = registry.get(connection_id).await.unwrap();
    assert!(!info.subscriptions.is_subscribed_to_project("p1"));
    assert!(!info.subscriptions.is_subscribed_to_sprint("s1"));
}

#[tokio::test]
async fn subscribe_missing_connection_returns_not_found() {
    let registry = ConnectionRegistry::new(ConnectionLimits::default());
    let missing_id = "00000000-0000-0000-0000-000000000000".to_string();
    let ctx = test_context(registry, missing_id);

    let err = handle_subscribe(
        Subscribe {
            project_ids: vec!["p1".into()],
            sprint_ids: vec![],
        },
        ctx,
    )
    .await
    .unwrap_err();

    assert!(matches!(err, pm_ws::WsError::NotFound { .. }));
}
