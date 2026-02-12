use pm_proto::{GetLlmContextRequest, WebSocketMessage, web_socket_message::Payload};
use pm_ws::{
    CircuitBreaker, CircuitBreakerConfig, ConnectionLimits, ConnectionRegistry, HandlerContext,
    dispatch,
};

use std::sync::Arc;

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

struct TestFixture {
    pool: SqlitePool,
    circuit_breaker: Arc<CircuitBreaker>,
    user_id: Uuid,
}

impl TestFixture {
    async fn new() -> Self {
        let pool = SqlitePool::connect(":memory:")
            .await
            .expect("Failed to create test database");

        sqlx::migrate!("../pm-db/migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        Self {
            pool,
            circuit_breaker: Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default())),
            user_id: Uuid::new_v4(),
        }
    }

    fn ctx(&self, msg_id: &str) -> HandlerContext {
        let registry = ConnectionRegistry::new(ConnectionLimits::default());
        HandlerContext::new(
            msg_id.to_string(),
            self.user_id,
            self.pool.clone(),
            self.circuit_breaker.clone(),
            "test-connection".to_string(),
            registry,
            pm_config::ValidationConfig::default(),
        )
    }
}

#[tokio::test]
async fn given_get_llm_context_when_dispatched_then_returns_seeded_entries() {
    let fixture = TestFixture::new().await;

    let msg = WebSocketMessage {
        message_id: "msg-001".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::GetLlmContextRequest(GetLlmContextRequest {
            category: None,
            context_type: None,
            min_priority: None,
        })),
    };

    let response = dispatch(msg, fixture.ctx("msg-001")).await;

    match response.payload {
        Some(Payload::LlmContextList(list)) => {
            assert_eq!(list.entries.len(), 28);
        }
        _ => panic!("Expected LlmContextList response"),
    }
}
