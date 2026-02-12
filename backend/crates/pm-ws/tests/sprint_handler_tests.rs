//! Integration tests for sprint handlers.
//!
//! Tests verify:
//! - Sprint CRUD operations
//! - Optimistic locking (version conflicts)
//! - Status transitions (state machine)
//! - Authorization (permission checks)

use pm_proto::{
    CreateSprintRequest, GetSprintsRequest, SprintStatus as ProtoSprintStatus, UpdateSprintRequest,
    WebSocketMessage, web_socket_message::Payload,
};
use pm_ws::{
    CircuitBreaker, CircuitBreakerConfig, ConnectionLimits, ConnectionRegistry, HandlerContext,
    dispatch,
};

use std::sync::Arc;

use chrono::{Duration, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

// =========================================================================
// Test Fixtures
// =========================================================================

struct TestFixture {
    pool: SqlitePool,
    circuit_breaker: Arc<CircuitBreaker>,
    user_id: Uuid,
    project_id: Uuid,
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
        let project_id = Uuid::new_v4();

        // Create test user in users table (required for activity log FK)
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

        // Create test project in pm_projects table
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

        Self {
            pool,
            circuit_breaker: Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default())),
            user_id,
            project_id,
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
            pm_config::ValidationConfig::default(),
        )
    }
}

// =========================================================================
// Create Sprint Tests
// =========================================================================

#[tokio::test]
async fn given_valid_request_when_create_sprint_then_succeeds() {
    // Given
    let fixture = TestFixture::new().await;
    let ctx = fixture.create_context("msg-001");

    let start = Utc::now() + Duration::days(1);
    let end = start + Duration::days(14);

    let msg = WebSocketMessage {
        message_id: "msg-001".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateSprintRequest(CreateSprintRequest {
            project_id: fixture.project_id.to_string(),
            name: "Sprint 1".to_string(),
            goal: Some("Complete MVP".to_string()),
            start_date: start.timestamp(),
            end_date: end.timestamp(),
        })),
    };

    // When
    let response = dispatch(msg, ctx).await;

    // Then
    assert_eq!(response.message_id, "msg-001");
    match &response.payload {
        Some(Payload::SprintCreated(created)) => {
            let sprint = created.sprint.as_ref().unwrap();
            assert_eq!(sprint.name, "Sprint 1");
            assert_eq!(sprint.goal.as_deref(), Some("Complete MVP"));
            assert_eq!(sprint.status, ProtoSprintStatus::Planned as i32);
            assert_eq!(sprint.version, 1);
        }
        Some(Payload::Error(err)) => {
            panic!(
                "Got error response: code={}, message={}",
                err.code, err.message
            );
        }
        _ => panic!(
            "Expected SprintCreated response, got: {:?}",
            response.payload
        ),
    }
}

#[tokio::test]
async fn given_invalid_dates_when_create_sprint_then_validation_error() {
    // Given - end date before start date
    let fixture = TestFixture::new().await;
    let ctx = fixture.create_context("msg-002");

    let start = Utc::now() + Duration::days(14);
    let end = Utc::now() + Duration::days(1);

    let msg = WebSocketMessage {
        message_id: "msg-002".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateSprintRequest(CreateSprintRequest {
            project_id: fixture.project_id.to_string(),
            name: "Sprint 1".to_string(),
            goal: None,
            start_date: start.timestamp(),
            end_date: end.timestamp(),
        })),
    };

    // When
    let response = dispatch(msg, ctx).await;

    // Then
    match response.payload {
        Some(Payload::Error(err)) => {
            assert_eq!(err.code, "INVALID_MESSAGE");
            assert!(err.message.contains("start_date"));
        }
        _ => panic!("Expected Error response"),
    }
}

#[tokio::test]
async fn given_empty_name_when_create_sprint_then_validation_error() {
    // Given
    let fixture = TestFixture::new().await;
    let ctx = fixture.create_context("msg-003");

    let msg = WebSocketMessage {
        message_id: "msg-003".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateSprintRequest(CreateSprintRequest {
            project_id: fixture.project_id.to_string(),
            name: "".to_string(),
            goal: None,
            start_date: Utc::now().timestamp(),
            end_date: (Utc::now() + Duration::days(14)).timestamp(),
        })),
    };

    // When
    let response = dispatch(msg, ctx).await;

    // Then
    match response.payload {
        Some(Payload::Error(err)) => {
            assert_eq!(err.code, "INVALID_MESSAGE");
        }
        _ => panic!("Expected Error response"),
    }
}

// =========================================================================
// Update Sprint Tests
// =========================================================================

#[tokio::test]
async fn given_correct_version_when_update_sprint_then_succeeds() {
    // Given - Create a sprint first
    let fixture = TestFixture::new().await;
    let ctx = fixture.create_context("msg-create");

    let create_msg = WebSocketMessage {
        message_id: "msg-create".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateSprintRequest(CreateSprintRequest {
            project_id: fixture.project_id.to_string(),
            name: "Sprint 1".to_string(),
            goal: None,
            start_date: Utc::now().timestamp(),
            end_date: (Utc::now() + Duration::days(14)).timestamp(),
        })),
    };
    let create_response = dispatch(create_msg, ctx).await;
    let sprint_id = match create_response.payload {
        Some(Payload::SprintCreated(c)) => c.sprint.unwrap().id,
        _ => panic!("Failed to create sprint"),
    };

    // When - Update with correct version
    let ctx = fixture.create_context("msg-update");
    let update_msg = WebSocketMessage {
        message_id: "msg-update".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::UpdateSprintRequest(UpdateSprintRequest {
            sprint_id: sprint_id.clone(),
            expected_version: 1,
            name: Some("Sprint 1 Updated".to_string()),
            goal: None,
            start_date: None,
            end_date: None,
            status: None,
        })),
    };
    let response = dispatch(update_msg, ctx).await;

    // Then
    match response.payload {
        Some(Payload::SprintUpdated(updated)) => {
            let sprint = updated.sprint.unwrap();
            assert_eq!(sprint.name, "Sprint 1 Updated");
            assert_eq!(sprint.version, 2);
        }
        _ => panic!("Expected SprintUpdated response"),
    }
}

#[tokio::test]
async fn given_wrong_version_when_update_sprint_then_conflict_error() {
    // Given - Create a sprint first
    let fixture = TestFixture::new().await;
    let ctx = fixture.create_context("msg-create");

    let create_msg = WebSocketMessage {
        message_id: "msg-create".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateSprintRequest(CreateSprintRequest {
            project_id: fixture.project_id.to_string(),
            name: "Sprint 1".to_string(),
            goal: None,
            start_date: Utc::now().timestamp(),
            end_date: (Utc::now() + Duration::days(14)).timestamp(),
        })),
    };
    let create_response = dispatch(create_msg, ctx).await;
    let sprint_id = match create_response.payload {
        Some(Payload::SprintCreated(c)) => c.sprint.unwrap().id,
        _ => panic!("Failed to create sprint"),
    };

    // When - Update with wrong version
    let ctx = fixture.create_context("msg-update");
    let update_msg = WebSocketMessage {
        message_id: "msg-update".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::UpdateSprintRequest(UpdateSprintRequest {
            sprint_id,
            expected_version: 99, // Wrong version
            name: Some("Sprint 1 Updated".to_string()),
            goal: None,
            start_date: None,
            end_date: None,
            status: None,
        })),
    };
    let response = dispatch(update_msg, ctx).await;

    // Then
    match response.payload {
        Some(Payload::Error(err)) => {
            assert_eq!(err.code, "CONFLICT");
        }
        _ => panic!("Expected CONFLICT error"),
    }
}

// =========================================================================
// Status Transition Tests
// =========================================================================

#[tokio::test]
async fn given_planned_sprint_when_start_then_becomes_active() {
    // Given - Create a planned sprint
    let fixture = TestFixture::new().await;
    let ctx = fixture.create_context("msg-create");

    let create_msg = WebSocketMessage {
        message_id: "msg-create".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateSprintRequest(CreateSprintRequest {
            project_id: fixture.project_id.to_string(),
            name: "Sprint 1".to_string(),
            goal: None,
            start_date: Utc::now().timestamp(),
            end_date: (Utc::now() + Duration::days(14)).timestamp(),
        })),
    };
    let create_response = dispatch(create_msg, ctx).await;
    let sprint_id = match create_response.payload {
        Some(Payload::SprintCreated(c)) => c.sprint.unwrap().id,
        _ => panic!("Failed to create sprint"),
    };

    // When - Transition to Active
    let ctx = fixture.create_context("msg-start");
    let start_msg = WebSocketMessage {
        message_id: "msg-start".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::UpdateSprintRequest(UpdateSprintRequest {
            sprint_id: sprint_id.clone(),
            expected_version: 1,
            name: None,
            goal: None,
            start_date: None,
            end_date: None,
            status: Some(ProtoSprintStatus::Active as i32),
        })),
    };
    let response = dispatch(start_msg, ctx).await;

    // Then
    match response.payload {
        Some(Payload::SprintUpdated(updated)) => {
            let sprint = updated.sprint.unwrap();
            assert_eq!(sprint.status, ProtoSprintStatus::Active as i32);
        }
        _ => panic!("Expected SprintUpdated response"),
    }
}

#[tokio::test]
async fn given_planned_sprint_when_complete_then_invalid_transition() {
    // Given - Create a planned sprint
    let fixture = TestFixture::new().await;
    let ctx = fixture.create_context("msg-create");

    let create_msg = WebSocketMessage {
        message_id: "msg-create".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateSprintRequest(CreateSprintRequest {
            project_id: fixture.project_id.to_string(),
            name: "Sprint 1".to_string(),
            goal: None,
            start_date: Utc::now().timestamp(),
            end_date: (Utc::now() + Duration::days(14)).timestamp(),
        })),
    };
    let create_response = dispatch(create_msg, ctx).await;
    let sprint_id = match create_response.payload {
        Some(Payload::SprintCreated(c)) => c.sprint.unwrap().id,
        _ => panic!("Failed to create sprint"),
    };

    // When - Try invalid transition (Planned -> Completed)
    let ctx = fixture.create_context("msg-complete");
    let complete_msg = WebSocketMessage {
        message_id: "msg-complete".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::UpdateSprintRequest(UpdateSprintRequest {
            sprint_id,
            expected_version: 1,
            name: None,
            goal: None,
            start_date: None,
            end_date: None,
            status: Some(ProtoSprintStatus::Completed as i32),
        })),
    };
    let response = dispatch(complete_msg, ctx).await;

    // Then
    match response.payload {
        Some(Payload::Error(err)) => {
            assert_eq!(err.code, "VALIDATION_ERROR");
            assert!(err.message.contains("Invalid status transition"));
        }
        _ => panic!("Expected VALIDATION_ERROR"),
    }
}

// =========================================================================
// Get Sprints Tests
// =========================================================================

#[tokio::test]
async fn given_project_with_sprints_when_get_sprints_then_returns_all() {
    // Given - Create two sprints
    let fixture = TestFixture::new().await;

    for i in 1..=2 {
        let ctx = fixture.create_context(&format!("msg-create-{}", i));
        let create_msg = WebSocketMessage {
            message_id: format!("msg-create-{}", i),
            timestamp: Utc::now().timestamp(),
            payload: Some(Payload::CreateSprintRequest(CreateSprintRequest {
                project_id: fixture.project_id.to_string(),
                name: format!("Sprint {}", i),
                goal: None,
                start_date: (Utc::now() + Duration::days(i as i64 * 14)).timestamp(),
                end_date: (Utc::now() + Duration::days((i as i64 + 1) * 14)).timestamp(),
            })),
        };
        dispatch(create_msg, ctx).await;
    }

    // When - Get sprints
    let ctx = fixture.create_context("msg-get");
    let get_msg = WebSocketMessage {
        message_id: "msg-get".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::GetSprintsRequest(GetSprintsRequest {
            project_id: fixture.project_id.to_string(),
        })),
    };
    let response = dispatch(get_msg, ctx).await;

    // Then
    match response.payload {
        Some(Payload::SprintsList(list)) => {
            assert_eq!(list.sprints.len(), 2);
        }
        _ => panic!("Expected SprintsList response"),
    }
}
