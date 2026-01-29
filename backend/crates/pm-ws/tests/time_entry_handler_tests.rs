//! Integration tests for time entry handlers.
//!
//! Tests verify:
//! - Timer start/stop operations
//! - Atomic timer switching (one running timer per user)
//! - Owner-only edit/delete permissions
//! - Timestamp validation (no future, max 24 hours)
//! - Pagination support

use pm_proto::{
    CreateTimeEntryRequest, GetRunningTimerRequest, GetTimeEntriesRequest, StartTimerRequest,
    StopTimerRequest, WebSocketMessage, web_socket_message::Payload,
};
use pm_ws::{
    CircuitBreaker, CircuitBreakerConfig, ConnectionLimits, ConnectionRegistry, HandlerContext,
    dispatch,
};

use std::sync::Arc;

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

// =============================================================================
// Test Fixtures
// =============================================================================

struct TestFixture {
    pool: SqlitePool,
    circuit_breaker: Arc<CircuitBreaker>,
    user_id: Uuid,
    other_user_id: Uuid,
    project_id: Uuid,
    work_item_id: Uuid,
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

        let user_id = Uuid::new_v4();
        let other_user_id = Uuid::new_v4();
        let project_id = Uuid::new_v4();
        let work_item_id = Uuid::new_v4();

        // Create test users
        sqlx::query(
            r#"
                INSERT INTO users (id, email, name, created_at)
                VALUES (?, 'test@example.com', 'Test User', ?)
                "#,
        )
        .bind(user_id.to_string())
        .bind(Utc::now().timestamp())
        .execute(&pool)
        .await
        .expect("Failed to create test user");

        sqlx::query(
            r#"
                INSERT INTO users (id, email, name, created_at)
                VALUES (?, 'other@example.com', 'Other User', ?)
                "#,
        )
        .bind(other_user_id.to_string())
        .bind(Utc::now().timestamp())
        .execute(&pool)
        .await
        .expect("Failed to create other user");

        // Create test project
        sqlx::query(
            r#"
                INSERT INTO pm_projects (id, title, key, status, version, created_at, updated_at, created_by, updated_by)
                VALUES (?, 'Test Project', 'TEST', 'active', 1, ?, ?, ?, ?)
                "#
        )
            .bind(project_id.to_string())
            .bind(Utc::now().timestamp())
            .bind(Utc::now().timestamp())
            .bind(user_id.to_string())
            .bind(user_id.to_string())
            .execute(&pool)
            .await
            .expect("Failed to create test project");

        // Add user as project member with editor role
        sqlx::query(
            r#"
                INSERT INTO pm_project_members (id, project_id, user_id, role, created_at)
                VALUES (?, ?, ?, 'editor', ?)
                "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(project_id.to_string())
        .bind(user_id.to_string())
        .bind(Utc::now().timestamp())
        .execute(&pool)
        .await
        .expect("Failed to add project member");

        // Create test work item (task)
        sqlx::query(
            r#"
                INSERT INTO pm_work_items (id, item_type, parent_id, project_id, position, title, status, priority, version, created_at, updated_at, created_by, updated_by)
                VALUES (?, 'task', NULL, ?, 1, 'Test Task', 'todo', 'medium', 1, ?, ?, ?, ?)
                "#
        )
            .bind(work_item_id.to_string())
            .bind(project_id.to_string())
            .bind(Utc::now().timestamp())
            .bind(Utc::now().timestamp())
            .bind(user_id.to_string())
            .bind(user_id.to_string())
            .execute(&pool)
            .await
            .expect("Failed to create test work item");

        Self {
            pool,
            circuit_breaker: Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default())),
            user_id,
            other_user_id,
            project_id,
            work_item_id,
        }
    }

    fn create_context(&self, message_id: &str) -> HandlerContext {
        let registry = ConnectionRegistry::new(ConnectionLimits::default());
        HandlerContext::new(
            message_id.to_string(),
            self.user_id,
            self.pool.clone(),
            self.circuit_breaker.clone(),
            "test-connection".to_string(),
            registry,
        )
    }

    fn create_context_as(&self, message_id: &str, user_id: Uuid) -> HandlerContext {
        let registry = ConnectionRegistry::new(ConnectionLimits::default());
        HandlerContext::new(
            message_id.to_string(),
            user_id,
            self.pool.clone(),
            self.circuit_breaker.clone(),
            "test-connection".to_string(),
            registry,
        )
    }
}

// =============================================================================
// StartTimer Tests
// =============================================================================

#[tokio::test]
async fn given_valid_request_when_start_timer_then_creates_running_entry() {
    // Given
    let fixture = TestFixture::new().await;
    let ctx = fixture.create_context("msg-001");

    let msg = WebSocketMessage {
        message_id: "msg-001".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::StartTimerRequest(StartTimerRequest {
            work_item_id: fixture.work_item_id.to_string(),
            description: Some("Working on tests".to_string()),
        })),
    };

    // When
    let response = dispatch(msg, ctx).await;

    // Then
    match response.payload {
        Some(Payload::TimerStarted(started)) => {
            assert!(started.time_entry.is_some());
            let entry = started.time_entry.unwrap();
            assert_eq!(entry.work_item_id, fixture.work_item_id.to_string());
            assert_eq!(entry.user_id, fixture.user_id.to_string());
            assert!(entry.ended_at.is_none()); // Running
            assert_eq!(entry.description, Some("Working on tests".to_string()));
            assert!(started.stopped_entry.is_none()); // No previous timer
        }
        Some(Payload::Error(err)) => {
            panic!(
                "Got error response: code={}, message={}",
                err.code, err.message
            );
        }
        _ => panic!("Expected TimerStarted response"),
    }
}

#[tokio::test]
async fn given_running_timer_when_start_new_timer_then_auto_stops_previous() {
    // Given
    let fixture = TestFixture::new().await;

    // Start first timer
    let ctx1 = fixture.create_context("msg-001");
    let msg1 = WebSocketMessage {
        message_id: "msg-001".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::StartTimerRequest(StartTimerRequest {
            work_item_id: fixture.work_item_id.to_string(),
            description: None,
        })),
    };
    let first_response = dispatch(msg1, ctx1).await;
    let first_entry_id = match &first_response.payload {
        Some(Payload::TimerStarted(s)) => s.time_entry.as_ref().unwrap().id.clone(),
        _ => panic!("Expected TimerStarted"),
    };

    // Small delay
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // When - Start second timer (different work item)
    let other_work_item_id = Uuid::new_v4();
    sqlx::query(
        r#"
            INSERT INTO pm_work_items (id, item_type, parent_id, project_id, position, title, status, priority, version, created_at, updated_at, created_by, updated_by)
            VALUES (?, 'task', NULL, ?, 2, 'Other Task', 'todo', 'medium', 1, ?, ?, ?, ?)
            "#
    )
        .bind(other_work_item_id.to_string())
        .bind(fixture.project_id.to_string())
        .bind(Utc::now().timestamp())
        .bind(Utc::now().timestamp())
        .bind(fixture.user_id.to_string())
        .bind(fixture.user_id.to_string())
        .execute(&fixture.pool)
        .await
        .expect("Failed to create second work item");

    let ctx2 = fixture.create_context("msg-002");
    let msg2 = WebSocketMessage {
        message_id: "msg-002".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::StartTimerRequest(StartTimerRequest {
            work_item_id: other_work_item_id.to_string(),
            description: None,
        })),
    };
    let response = dispatch(msg2, ctx2).await;

    // Then
    match response.payload {
        Some(Payload::TimerStarted(started)) => {
            // New timer started
            assert!(started.time_entry.is_some());
            let new_entry = started.time_entry.unwrap();
            assert_eq!(new_entry.work_item_id, other_work_item_id.to_string());
            assert!(new_entry.ended_at.is_none()); // Running

            // Previous timer stopped
            assert!(started.stopped_entry.is_some());
            let stopped = started.stopped_entry.unwrap();
            assert_eq!(stopped.id, first_entry_id);
            assert!(stopped.ended_at.is_some()); // Now stopped
            assert!(stopped.duration_seconds.is_some());
            assert!(stopped.duration_seconds.unwrap() >= 0);
        }
        Some(Payload::Error(err)) => {
            panic!(
                "Got error response: code={}, message={}",
                err.code, err.message
            );
        }
        _ => panic!("Expected TimerStarted response"),
    }
}

#[tokio::test]
async fn given_nonexistent_work_item_when_start_timer_then_validation_error() {
    // Given
    let fixture = TestFixture::new().await;
    let ctx = fixture.create_context("msg-003");

    let msg = WebSocketMessage {
        message_id: "msg-003".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::StartTimerRequest(StartTimerRequest {
            work_item_id: Uuid::new_v4().to_string(), // Non-existent
            description: None,
        })),
    };

    // When
    let response = dispatch(msg, ctx).await;

    // Then
    match response.payload {
        Some(Payload::Error(err)) => {
            assert!(err.message.contains("not found") || err.message.contains("Work item"));
        }
        _ => panic!("Expected Error response"),
    }
}

// =============================================================================
// StopTimer Tests
// =============================================================================

#[tokio::test]
async fn given_running_timer_when_stop_timer_then_calculates_duration() {
    // Given
    let fixture = TestFixture::new().await;

    // Start a timer
    let ctx1 = fixture.create_context("msg-001");
    let start_msg = WebSocketMessage {
        message_id: "msg-001".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::StartTimerRequest(StartTimerRequest {
            work_item_id: fixture.work_item_id.to_string(),
            description: None,
        })),
    };
    let start_response = dispatch(start_msg, ctx1).await;
    let entry_id = match &start_response.payload {
        Some(Payload::TimerStarted(s)) => s.time_entry.as_ref().unwrap().id.clone(),
        _ => panic!("Expected TimerStarted"),
    };

    // Small delay to ensure measurable duration
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // When - Stop the timer
    let ctx2 = fixture.create_context("msg-002");
    let stop_msg = WebSocketMessage {
        message_id: "msg-002".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::StopTimerRequest(StopTimerRequest {
            time_entry_id: entry_id.clone(),
        })),
    };
    let response = dispatch(stop_msg, ctx2).await;

    // Then
    match response.payload {
        Some(Payload::TimerStopped(stopped)) => {
            let entry = stopped.time_entry.unwrap();
            assert_eq!(entry.id, entry_id);
            assert!(entry.ended_at.is_some());
            assert!(entry.duration_seconds.is_some());
            assert!(entry.duration_seconds.unwrap() >= 0);
        }
        Some(Payload::Error(err)) => {
            panic!(
                "Got error response: code={}, message={}",
                err.code, err.message
            );
        }
        _ => panic!("Expected TimerStopped response"),
    }
}

#[tokio::test]
async fn given_other_users_timer_when_stop_timer_then_permission_error() {
    // Given
    let fixture = TestFixture::new().await;

    // Owner starts timer
    let ctx1 = fixture.create_context("msg-001");
    let start_msg = WebSocketMessage {
        message_id: "msg-001".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::StartTimerRequest(StartTimerRequest {
            work_item_id: fixture.work_item_id.to_string(),
            description: None,
        })),
    };
    let start_response = dispatch(start_msg, ctx1).await;
    let entry_id = match &start_response.payload {
        Some(Payload::TimerStarted(s)) => s.time_entry.as_ref().unwrap().id.clone(),
        _ => panic!("Expected TimerStarted"),
    };

    // When - Other user tries to stop
    let ctx2 = fixture.create_context_as("msg-002", fixture.other_user_id);
    let stop_msg = WebSocketMessage {
        message_id: "msg-002".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::StopTimerRequest(StopTimerRequest {
            time_entry_id: entry_id,
        })),
    };
    let response = dispatch(stop_msg, ctx2).await;

    // Then
    match response.payload {
        Some(Payload::Error(err)) => {
            assert!(err.message.contains("another user") || err.message.contains("permission"));
        }
        _ => panic!("Expected Error response"),
    }
}

#[tokio::test]
async fn given_already_stopped_timer_when_stop_timer_then_error() {
    // Given
    let fixture = TestFixture::new().await;

    // Start and stop a timer
    let ctx1 = fixture.create_context("msg-001");
    let start_msg = WebSocketMessage {
        message_id: "msg-001".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::StartTimerRequest(StartTimerRequest {
            work_item_id: fixture.work_item_id.to_string(),
            description: None,
        })),
    };
    let start_response = dispatch(start_msg, ctx1).await;
    let entry_id = match &start_response.payload {
        Some(Payload::TimerStarted(s)) => s.time_entry.as_ref().unwrap().id.clone(),
        _ => panic!("Expected TimerStarted"),
    };

    let ctx2 = fixture.create_context("msg-002");
    let stop_msg = WebSocketMessage {
        message_id: "msg-002".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::StopTimerRequest(StopTimerRequest {
            time_entry_id: entry_id.clone(),
        })),
    };
    dispatch(stop_msg, ctx2).await;

    // When - Try to stop again
    let ctx3 = fixture.create_context("msg-003");
    let stop_msg2 = WebSocketMessage {
        message_id: "msg-003".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::StopTimerRequest(StopTimerRequest {
            time_entry_id: entry_id,
        })),
    };
    let response = dispatch(stop_msg2, ctx3).await;

    // Then
    match response.payload {
        Some(Payload::Error(err)) => {
            assert!(err.message.contains("not running") || err.message.contains("already stopped"));
        }
        _ => panic!("Expected Error response"),
    }
}

// =============================================================================
// CreateTimeEntry Tests
// =============================================================================

#[tokio::test]
async fn given_valid_timestamps_when_create_manual_entry_then_succeeds() {
    // Given
    let fixture = TestFixture::new().await;
    let ctx = fixture.create_context("msg-004");

    let now = Utc::now().timestamp();
    let msg = WebSocketMessage {
        message_id: "msg-004".to_string(),
        timestamp: now,
        payload: Some(Payload::CreateTimeEntryRequest(CreateTimeEntryRequest {
            work_item_id: fixture.work_item_id.to_string(),
            started_at: now - 3600, // 1 hour ago
            ended_at: now - 1800,   // 30 minutes ago
            description: Some("Manual entry".to_string()),
        })),
    };

    // When
    let response = dispatch(msg, ctx).await;

    // Then
    match response.payload {
        Some(Payload::TimeEntryCreated(created)) => {
            let entry = created.time_entry.unwrap();
            assert_eq!(entry.duration_seconds, Some(1800)); // 30 minutes
            assert_eq!(entry.description, Some("Manual entry".to_string()));
        }
        Some(Payload::Error(err)) => {
            panic!(
                "Got error response: code={}, message={}",
                err.code, err.message
            );
        }
        _ => panic!("Expected TimeEntryCreated response"),
    }
}

#[tokio::test]
async fn given_future_timestamps_when_create_manual_entry_then_validation_error() {
    // Given
    let fixture = TestFixture::new().await;
    let ctx = fixture.create_context("msg-005");

    let future = Utc::now().timestamp() + 3600; // 1 hour in future
    let msg = WebSocketMessage {
        message_id: "msg-005".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateTimeEntryRequest(CreateTimeEntryRequest {
            work_item_id: fixture.work_item_id.to_string(),
            started_at: future,
            ended_at: future + 1800,
            description: None,
        })),
    };

    // When
    let response = dispatch(msg, ctx).await;

    // Then
    match response.payload {
        Some(Payload::Error(err)) => {
            assert!(err.message.contains("future"));
        }
        _ => panic!("Expected Error response"),
    }
}

#[tokio::test]
async fn given_excessive_duration_when_create_manual_entry_then_validation_error() {
    // Given
    let fixture = TestFixture::new().await;
    let ctx = fixture.create_context("msg-006");

    let now = Utc::now().timestamp();
    let msg = WebSocketMessage {
        message_id: "msg-006".to_string(),
        timestamp: now,
        payload: Some(Payload::CreateTimeEntryRequest(CreateTimeEntryRequest {
            work_item_id: fixture.work_item_id.to_string(),
            started_at: now - 100000, // More than 24 hours
            ended_at: now,
            description: None,
        })),
    };

    // When
    let response = dispatch(msg, ctx).await;

    // Then
    match response.payload {
        Some(Payload::Error(err)) => {
            assert!(
                err.message.contains("24 hours")
                    || err.message.contains("Duration")
                    || err.message.contains("exceeds")
            );
        }
        _ => panic!("Expected Error response"),
    }
}

// =============================================================================
// GetTimeEntries Tests
// =============================================================================

#[tokio::test]
async fn given_multiple_entries_when_get_time_entries_then_pagination_works() {
    // Given
    let fixture = TestFixture::new().await;

    // Create 5 entries
    let now = Utc::now().timestamp();
    for i in 0..5 {
        let ctx = fixture.create_context(&format!("msg-{:03}", i));
        let msg = WebSocketMessage {
            message_id: format!("msg-{:03}", i),
            timestamp: now,
            payload: Some(Payload::CreateTimeEntryRequest(CreateTimeEntryRequest {
                work_item_id: fixture.work_item_id.to_string(),
                started_at: now - (i + 1) * 7200,
                ended_at: now - i * 7200,
                description: Some(format!("Entry {}", i)),
            })),
        };
        dispatch(msg, ctx).await;
    }

    // When - Get first page (limit 2)
    let ctx = fixture.create_context("msg-get");
    let get_msg = WebSocketMessage {
        message_id: "msg-get".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::GetTimeEntriesRequest(GetTimeEntriesRequest {
            work_item_id: fixture.work_item_id.to_string(),
            limit: Some(2),
            offset: Some(0),
        })),
    };
    let response = dispatch(get_msg, ctx).await;

    // Then
    match response.payload {
        Some(Payload::TimeEntriesList(list)) => {
            assert_eq!(list.time_entries.len(), 2);
            assert_eq!(list.total_count, 5);
        }
        Some(Payload::Error(err)) => {
            panic!(
                "Got error response: code={}, message={}",
                err.code, err.message
            );
        }
        _ => panic!("Expected TimeEntriesList response"),
    }
}

// =============================================================================
// GetRunningTimer Tests
// =============================================================================

#[tokio::test]
async fn given_running_timer_when_get_running_timer_then_returns_current_user_timer_only() {
    // Given
    let fixture = TestFixture::new().await;

    // User starts a timer
    let ctx1 = fixture.create_context("msg-001");
    let start_msg = WebSocketMessage {
        message_id: "msg-001".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::StartTimerRequest(StartTimerRequest {
            work_item_id: fixture.work_item_id.to_string(),
            description: None,
        })),
    };
    dispatch(start_msg, ctx1).await;

    // When - Other user checks for running timer
    let ctx2 = fixture.create_context_as("msg-002", fixture.other_user_id);
    let get_msg = WebSocketMessage {
        message_id: "msg-002".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::GetRunningTimerRequest(GetRunningTimerRequest {})),
    };
    let response = dispatch(get_msg, ctx2).await;

    // Then - Other user should have no running timer
    match response.payload {
        Some(Payload::RunningTimerResponse(r)) => {
            assert!(r.time_entry.is_none());
        }
        Some(Payload::Error(err)) => {
            panic!(
                "Got error response: code={}, message={}",
                err.code, err.message
            );
        }
        _ => panic!("Expected RunningTimerResponse"),
    }

    // And - Original user should have their timer
    let ctx1b = fixture.create_context("msg-003");
    let get_msg2 = WebSocketMessage {
        message_id: "msg-003".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::GetRunningTimerRequest(GetRunningTimerRequest {})),
    };
    let response1 = dispatch(get_msg2, ctx1b).await;

    match response1.payload {
        Some(Payload::RunningTimerResponse(r)) => {
            assert!(r.time_entry.is_some());
        }
        Some(Payload::Error(err)) => {
            panic!(
                "Got error response: code={}, message={}",
                err.code, err.message
            );
        }
        _ => panic!("Expected RunningTimerResponse"),
    }
}
