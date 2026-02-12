//! Integration tests for the message dispatcher
//!
//! These tests verify end-to-end message handling through the dispatcher.
//! Uses RAII patterns for resource cleanup and proper test fixtures.

use pm_proto::{
    CreateWorkItemRequest, GetWorkItemsRequest, WebSocketMessage,
    WorkItemType as ProtoWorkItemType, web_socket_message::Payload,
};
use pm_ws::{
    CircuitBreaker, CircuitBreakerConfig, CircuitState, ConnectionLimits, ConnectionRegistry,
    HandlerContext, RequestContext, dispatch,
};

use std::sync::Arc;

use sqlx::SqlitePool;
use uuid::Uuid;

// =========================================================================
// Test Fixtures - RAII wrappers for test resources
// =========================================================================

/// RAII wrapper for test database pool.
/// Ensures proper cleanup of in-memory database on drop.
struct TestDatabase {
    pool: SqlitePool,
}

impl TestDatabase {
    async fn new() -> Self {
        let pool = SqlitePool::connect(":memory:")
            .await
            .expect("Failed to create test database");

        // Run migrations
        sqlx::migrate!("../pm-db/migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        Self { pool }
    }

    fn pool(&self) -> SqlitePool {
        self.pool.clone()
    }
}

impl Drop for TestDatabase {
    fn drop(&mut self) {
        // Pool is automatically closed when all references are dropped.
        // In-memory SQLite databases are destroyed when all connections close.
    }
}

/// RAII wrapper for test circuit breaker with configurable thresholds.
struct TestCircuitBreaker {
    inner: Arc<CircuitBreaker>,
}

impl TestCircuitBreaker {
    fn new() -> Self {
        Self {
            inner: Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default())),
        }
    }

    fn with_config(config: CircuitBreakerConfig) -> Self {
        Self {
            inner: Arc::new(CircuitBreaker::new(config)),
        }
    }

    fn arc(&self) -> Arc<CircuitBreaker> {
        self.inner.clone()
    }

    fn state(&self) -> CircuitState {
        self.inner.state()
    }

    fn record_success(&self) {
        self.inner.record_success();
    }

    fn record_failure(&self) {
        self.inner.record_failure();
    }
}

/// Combined test fixture for dispatcher tests.
/// Owns all test resources and ensures cleanup via RAII.
struct DispatcherTestFixture {
    db: TestDatabase,
    circuit_breaker: TestCircuitBreaker,
    user_id: Uuid,
}

impl DispatcherTestFixture {
    async fn new() -> Self {
        Self {
            db: TestDatabase::new().await,
            circuit_breaker: TestCircuitBreaker::new(),
            user_id: Uuid::new_v4(),
        }
    }

    async fn with_circuit_breaker_config(config: CircuitBreakerConfig) -> Self {
        Self {
            db: TestDatabase::new().await,
            circuit_breaker: TestCircuitBreaker::with_config(config),
            user_id: Uuid::new_v4(),
        }
    }

    fn create_context(&self, message_id: &str) -> HandlerContext {
        let registry = ConnectionRegistry::new(ConnectionLimits::default());
        HandlerContext::new(
            message_id.to_string(),
            self.user_id,
            self.db.pool(),
            self.circuit_breaker.arc(),
            "test-connection".to_string(),
            registry,
            pm_config::ValidationConfig::default(),
        )
    }

    fn circuit_breaker(&self) -> &TestCircuitBreaker {
        &self.circuit_breaker
    }
}

// =========================================================================
// Dispatcher Tests - Ping/Pong
// =========================================================================

#[tokio::test]
async fn given_ping_message_when_dispatched_then_returns_pong() {
    // Given
    let fixture = DispatcherTestFixture::new().await;
    let ctx = fixture.create_context("msg-001");
    let msg = WebSocketMessage {
        message_id: "msg-001".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        payload: Some(Payload::Ping(pm_proto::Ping { timestamp: 12345 })),
    };

    // When
    let response = dispatch(msg, ctx).await;

    // Then
    assert_eq!(response.message_id, "msg-001");
    match response.payload {
        Some(Payload::Pong(pong)) => {
            assert_eq!(pong.timestamp, 12345);
        }
        _ => panic!("Expected Pong response"),
    }
}

// =========================================================================
// Dispatcher Tests - Invalid Messages
// =========================================================================

#[tokio::test]
async fn given_empty_payload_when_dispatched_then_returns_invalid_message_error() {
    // Given
    let fixture = DispatcherTestFixture::new().await;
    let ctx = fixture.create_context("msg-002");
    let msg = WebSocketMessage {
        message_id: "msg-002".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        payload: None,
    };

    // When
    let response = dispatch(msg, ctx).await;

    // Then
    assert_eq!(response.message_id, "msg-002");
    match response.payload {
        Some(Payload::Error(err)) => {
            assert_eq!(err.code, "INVALID_MESSAGE");
        }
        _ => panic!("Expected Error response"),
    }
}

// =========================================================================
// Dispatcher Tests - Work Items
// =========================================================================

#[tokio::test]
async fn given_get_work_items_request_when_dispatched_then_returns_response() {
    // Given
    let fixture = DispatcherTestFixture::new().await;
    let project_id = Uuid::new_v4();
    let ctx = fixture.create_context("msg-003");
    let msg = WebSocketMessage {
        message_id: "msg-003".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        payload: Some(Payload::GetWorkItemsRequest(GetWorkItemsRequest {
            project_id: project_id.to_string(),
            since_timestamp: None,
        })),
    };

    // When
    let response = dispatch(msg, ctx).await;

    // Then - Message ID preserved in response
    assert_eq!(response.message_id, "msg-003");
}

#[tokio::test]
async fn given_empty_title_when_creating_work_item_then_returns_validation_error() {
    // Given
    let fixture = DispatcherTestFixture::new().await;
    let ctx = fixture.create_context("msg-004");
    let msg = WebSocketMessage {
        message_id: "msg-004".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        payload: Some(Payload::CreateWorkItemRequest(CreateWorkItemRequest {
            title: "".to_string(),
            description: None,
            item_type: ProtoWorkItemType::Task as i32,
            project_id: Uuid::new_v4().to_string(),
            parent_id: None,
            status: None,
            priority: None,
        })),
    };

    // When
    let response = dispatch(msg, ctx).await;

    // Then
    assert_eq!(response.message_id, "msg-004");
    match response.payload {
        Some(Payload::Error(err)) => {
            assert_eq!(err.code, "INVALID_MESSAGE");
            assert!(err.message.contains("title"));
        }
        _ => panic!("Expected ValidationError response"),
    }
}

// =========================================================================
// Circuit Breaker Tests
// =========================================================================

#[tokio::test]
async fn given_new_circuit_breaker_when_checked_then_state_is_closed() {
    // Given
    let cb = TestCircuitBreaker::new();

    // When/Then
    assert_eq!(cb.state(), CircuitState::Closed);
}

#[tokio::test]
async fn given_circuit_breaker_when_success_recorded_then_remains_closed() {
    // Given
    let cb = TestCircuitBreaker::new();

    // When
    cb.record_success();

    // Then
    assert_eq!(cb.state(), CircuitState::Closed);
}

#[tokio::test]
async fn given_circuit_breaker_when_failures_exceed_threshold_then_opens() {
    // Given - Circuit breaker with low threshold for testing
    let config = CircuitBreakerConfig {
        failure_threshold: 2,
        ..Default::default()
    };
    let cb = TestCircuitBreaker::with_config(config);

    // When - Record failures exceeding threshold
    cb.record_failure();
    cb.record_failure();

    // Then
    assert_eq!(cb.state(), CircuitState::Open);
}

#[tokio::test]
async fn given_open_circuit_breaker_when_request_attempted_then_rejected() {
    // Given - Fixture with low failure threshold
    let config = CircuitBreakerConfig {
        failure_threshold: 1,
        ..Default::default()
    };
    let fixture = DispatcherTestFixture::with_circuit_breaker_config(config).await;

    // Force circuit breaker open
    fixture.circuit_breaker().record_failure();
    assert_eq!(fixture.circuit_breaker().state(), CircuitState::Open);

    // When - Attempt request through dispatcher
    let ctx = fixture.create_context("msg-rejected");
    let msg = WebSocketMessage {
        message_id: "msg-rejected".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        payload: Some(Payload::GetWorkItemsRequest(GetWorkItemsRequest {
            project_id: Uuid::new_v4().to_string(),
            since_timestamp: None,
        })),
    };
    let response = dispatch(msg, ctx).await;

    // Then - Should get service unavailable error
    match response.payload {
        Some(Payload::Error(err)) => {
            assert_eq!(err.code, "SERVICE_UNAVAILABLE");
        }
        _ => panic!("Expected SERVICE_UNAVAILABLE error"),
    }
}

// =========================================================================
// Request Context Tests
// =========================================================================

#[tokio::test]
async fn given_message_id_when_creating_context_then_uses_as_correlation_id() {
    // Given
    let user_id = Uuid::new_v4();

    // When
    let ctx = RequestContext::new(user_id, "conn-123".to_string(), "msg-abc");

    // Then
    assert_eq!(ctx.correlation_id, "msg-abc");
    assert_eq!(ctx.user_id, user_id);
    assert!(ctx.log_prefix().contains("msg-abc"));
}

#[tokio::test]
async fn given_empty_message_id_when_creating_context_then_generates_correlation_id() {
    // Given
    let user_id = Uuid::new_v4();

    // When
    let ctx = RequestContext::new(user_id, "conn-123".to_string(), "");

    // Then
    assert!(ctx.correlation_id.starts_with("req-"));
}

#[tokio::test]
async fn given_request_context_when_elapsed_called_then_returns_positive_duration() {
    // Given
    let user_id = Uuid::new_v4();
    let ctx = RequestContext::new(user_id, "conn-123".to_string(), "msg-001");

    // When - Small delay to ensure elapsed time > 0
    tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
    let elapsed = ctx.elapsed_ms();

    // Then
    assert!(elapsed >= 1, "Expected elapsed >= 1ms, got {}", elapsed);
}
