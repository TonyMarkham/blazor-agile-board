//! Integration tests for dependency handlers.
//!
//! Tests verify:
//! - Dependency creation with validation
//! - Self-reference rejection
//! - Duplicate rejection
//! - Circular dependency detection (direct A→B→A and indirect A→B→C→A)
//! - RelatesTo allows bidirectional
//! - Cross-project enforcement
//! - Limit enforcement (50 max per item)
//! - Delete and query operations

use pm_proto::{
    CreateDependencyRequest, DeleteDependencyRequest, DependencyType as ProtoDependencyType,
    GetDependenciesRequest, WebSocketMessage, web_socket_message::Payload,
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
    project_id: Uuid,
    task_a_id: Uuid,
    task_b_id: Uuid,
    task_c_id: Uuid,
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
        let task_a_id = Uuid::new_v4();
        let task_b_id = Uuid::new_v4();
        let task_c_id = Uuid::new_v4();

        // Create test user
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

        // Create three test work items (tasks A, B, C)
        for (task_id, title) in [
            (task_a_id, "Task A"),
            (task_b_id, "Task B"),
            (task_c_id, "Task C"),
        ] {
            sqlx::query(
                r#"
                    INSERT INTO pm_work_items (id, item_type, parent_id, project_id, position, title, status, priority, version, created_at, updated_at, created_by, updated_by)
                    VALUES (?, 'task', NULL, ?, 1, ?, 'todo', 'medium', 1, ?, ?, ?, ?)
                    "#
            )
                .bind(task_id.to_string())
                .bind(project_id.to_string())
                .bind(title)
                .bind(Utc::now().timestamp())
                .bind(Utc::now().timestamp())
                .bind(user_id.to_string())
                .bind(user_id.to_string())
                .execute(&pool)
                .await
                .expect("Failed to create test work item");
        }

        Self {
            pool,
            circuit_breaker: Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default())),
            user_id,
            project_id,
            task_a_id,
            task_b_id,
            task_c_id,
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
}

// =============================================================================
// CreateDependency Tests
// =============================================================================

#[tokio::test]
async fn given_valid_request_when_create_dependency_then_succeeds() {
    // Given
    let fixture = TestFixture::new().await;
    let ctx = fixture.create_context("msg-001");

    let msg = WebSocketMessage {
        message_id: "msg-001".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateDependencyRequest(CreateDependencyRequest {
            blocking_item_id: fixture.task_a_id.to_string(),
            blocked_item_id: fixture.task_b_id.to_string(),
            dependency_type: ProtoDependencyType::Blocks as i32,
        })),
    };

    // When
    let response = dispatch(msg, ctx).await;

    // Then
    match response.payload {
        Some(Payload::DependencyCreated(created)) => {
            let dep = created.dependency.unwrap();
            assert_eq!(dep.blocking_item_id, fixture.task_a_id.to_string());
            assert_eq!(dep.blocked_item_id, fixture.task_b_id.to_string());
            assert_eq!(dep.dependency_type, ProtoDependencyType::Blocks as i32);
        }
        Some(Payload::Error(err)) => {
            panic!(
                "Got error response: code={}, message={}",
                err.code, err.message
            );
        }
        _ => panic!("Expected DependencyCreated response"),
    }
}

#[tokio::test]
async fn given_self_reference_when_create_dependency_then_validation_error() {
    // Given
    let fixture = TestFixture::new().await;
    let ctx = fixture.create_context("msg-002");

    let msg = WebSocketMessage {
        message_id: "msg-002".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateDependencyRequest(CreateDependencyRequest {
            blocking_item_id: fixture.task_a_id.to_string(),
            blocked_item_id: fixture.task_a_id.to_string(), // Same item!
            dependency_type: ProtoDependencyType::Blocks as i32,
        })),
    };

    // When
    let response = dispatch(msg, ctx).await;

    // Then
    match response.payload {
        Some(Payload::Error(err)) => {
            assert!(err.message.contains("cannot block itself") || err.message.contains("self"));
        }
        _ => panic!("Expected Error response"),
    }
}

#[tokio::test]
async fn given_duplicate_when_create_dependency_then_validation_error() {
    // Given
    let fixture = TestFixture::new().await;

    // Create first dependency
    let ctx1 = fixture.create_context("msg-001");
    let msg1 = WebSocketMessage {
        message_id: "msg-001".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateDependencyRequest(CreateDependencyRequest {
            blocking_item_id: fixture.task_a_id.to_string(),
            blocked_item_id: fixture.task_b_id.to_string(),
            dependency_type: ProtoDependencyType::Blocks as i32,
        })),
    };
    dispatch(msg1, ctx1).await;

    // When - Try to create duplicate
    let ctx2 = fixture.create_context("msg-002");
    let msg2 = WebSocketMessage {
        message_id: "msg-002".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateDependencyRequest(CreateDependencyRequest {
            blocking_item_id: fixture.task_a_id.to_string(),
            blocked_item_id: fixture.task_b_id.to_string(),
            dependency_type: ProtoDependencyType::Blocks as i32,
        })),
    };
    let response = dispatch(msg2, ctx2).await;

    // Then
    match response.payload {
        Some(Payload::Error(err)) => {
            assert!(err.message.contains("already exists") || err.message.contains("duplicate"));
        }
        _ => panic!("Expected Error response"),
    }
}

#[tokio::test]
async fn given_direct_cycle_when_create_dependency_then_circular_error() {
    // Given: A blocks B
    let fixture = TestFixture::new().await;

    let ctx1 = fixture.create_context("msg-001");
    let msg1 = WebSocketMessage {
        message_id: "msg-001".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateDependencyRequest(CreateDependencyRequest {
            blocking_item_id: fixture.task_a_id.to_string(),
            blocked_item_id: fixture.task_b_id.to_string(),
            dependency_type: ProtoDependencyType::Blocks as i32,
        })),
    };
    dispatch(msg1, ctx1).await;

    // When - Try B blocks A (would create cycle)
    let ctx2 = fixture.create_context("msg-002");
    let msg2 = WebSocketMessage {
        message_id: "msg-002".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateDependencyRequest(CreateDependencyRequest {
            blocking_item_id: fixture.task_b_id.to_string(),
            blocked_item_id: fixture.task_a_id.to_string(),
            dependency_type: ProtoDependencyType::Blocks as i32,
        })),
    };
    let response = dispatch(msg2, ctx2).await;

    // Then
    match response.payload {
        Some(Payload::Error(err)) => {
            assert!(err.message.contains("Circular dependency") || err.message.contains("cycle"));
        }
        _ => panic!("Expected Error response"),
    }
}

#[tokio::test]
async fn given_indirect_cycle_when_create_dependency_then_circular_error_with_path() {
    // Given: A blocks B, B blocks C
    let fixture = TestFixture::new().await;

    // A blocks B
    let ctx1 = fixture.create_context("msg-001");
    let msg1 = WebSocketMessage {
        message_id: "msg-001".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateDependencyRequest(CreateDependencyRequest {
            blocking_item_id: fixture.task_a_id.to_string(),
            blocked_item_id: fixture.task_b_id.to_string(),
            dependency_type: ProtoDependencyType::Blocks as i32,
        })),
    };
    dispatch(msg1, ctx1).await;

    // B blocks C
    let ctx2 = fixture.create_context("msg-002");
    let msg2 = WebSocketMessage {
        message_id: "msg-002".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateDependencyRequest(CreateDependencyRequest {
            blocking_item_id: fixture.task_b_id.to_string(),
            blocked_item_id: fixture.task_c_id.to_string(),
            dependency_type: ProtoDependencyType::Blocks as i32,
        })),
    };
    dispatch(msg2, ctx2).await;

    // When - Try C blocks A (would create A→B→C→A cycle)
    let ctx3 = fixture.create_context("msg-003");
    let msg3 = WebSocketMessage {
        message_id: "msg-003".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateDependencyRequest(CreateDependencyRequest {
            blocking_item_id: fixture.task_c_id.to_string(),
            blocked_item_id: fixture.task_a_id.to_string(),
            dependency_type: ProtoDependencyType::Blocks as i32,
        })),
    };
    let response = dispatch(msg3, ctx3).await;

    // Then
    match response.payload {
        Some(Payload::Error(err)) => {
            assert!(err.message.contains("Circular dependency") || err.message.contains("cycle"));
            // Error should include path information
            assert!(err.message.contains("→") || err.message.contains("path"));
        }
        _ => panic!("Expected Error response with cycle path"),
    }
}

#[tokio::test]
async fn given_relates_to_when_create_bidirectional_then_succeeds() {
    // Given: A relates_to B
    let fixture = TestFixture::new().await;

    let ctx1 = fixture.create_context("msg-001");
    let msg1 = WebSocketMessage {
        message_id: "msg-001".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateDependencyRequest(CreateDependencyRequest {
            blocking_item_id: fixture.task_a_id.to_string(),
            blocked_item_id: fixture.task_b_id.to_string(),
            dependency_type: ProtoDependencyType::RelatesTo as i32,
        })),
    };
    dispatch(msg1, ctx1).await;

    // When - B relates_to A (bidirectional is OK for RelatesTo)
    let ctx2 = fixture.create_context("msg-002");
    let msg2 = WebSocketMessage {
        message_id: "msg-002".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateDependencyRequest(CreateDependencyRequest {
            blocking_item_id: fixture.task_b_id.to_string(),
            blocked_item_id: fixture.task_a_id.to_string(),
            dependency_type: ProtoDependencyType::RelatesTo as i32,
        })),
    };
    let response = dispatch(msg2, ctx2).await;

    // Then - Should succeed (RelatesTo doesn't create cycles)
    match response.payload {
        Some(Payload::DependencyCreated(created)) => {
            let dep = created.dependency.unwrap();
            assert_eq!(dep.dependency_type, ProtoDependencyType::RelatesTo as i32);
        }
        Some(Payload::Error(err)) => {
            panic!(
                "Got error response: code={}, message={}",
                err.code, err.message
            );
        }
        _ => panic!("Expected DependencyCreated response"),
    }
}

#[tokio::test]
async fn given_cross_project_when_create_dependency_then_validation_error() {
    // Given
    let fixture = TestFixture::new().await;

    // Create a second project with a task
    let project2_id = Uuid::new_v4();
    let task_other_id = Uuid::new_v4();

    sqlx::query(
        r#"
            INSERT INTO pm_projects (id, title, key, status, version, created_at, updated_at, created_by, updated_by)
            VALUES (?, 'Other Project', 'OTHER', 'active', 1, ?, ?, ?, ?)
            "#
    )
        .bind(project2_id.to_string())
        .bind(Utc::now().timestamp())
        .bind(Utc::now().timestamp())
        .bind(fixture.user_id.to_string())
        .bind(fixture.user_id.to_string())
        .execute(&fixture.pool)
        .await
        .expect("Failed to create second project");

    sqlx::query(
        r#"
            INSERT INTO pm_work_items (id, item_type, parent_id, project_id, position, title, status, priority, version, created_at, updated_at, created_by, updated_by)
            VALUES (?, 'task', NULL, ?, 1, 'Task in Other Project', 'todo', 'medium', 1, ?, ?, ?, ?)
            "#
    )
        .bind(task_other_id.to_string())
        .bind(project2_id.to_string())
        .bind(Utc::now().timestamp())
        .bind(Utc::now().timestamp())
        .bind(fixture.user_id.to_string())
        .bind(fixture.user_id.to_string())
        .execute(&fixture.pool)
        .await
        .expect("Failed to create task in other project");

    // When - Try to create cross-project dependency
    let ctx = fixture.create_context("msg-001");
    let msg = WebSocketMessage {
        message_id: "msg-001".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateDependencyRequest(CreateDependencyRequest {
            blocking_item_id: fixture.task_a_id.to_string(),
            blocked_item_id: task_other_id.to_string(), // Different project!
            dependency_type: ProtoDependencyType::Blocks as i32,
        })),
    };
    let response = dispatch(msg, ctx).await;

    // Then
    match response.payload {
        Some(Payload::Error(err)) => {
            assert!(err.message.contains("same project") || err.message.contains("cross-project"));
        }
        _ => panic!("Expected Error response"),
    }
}

#[tokio::test]
async fn given_50_dependencies_when_create_51st_then_limit_error() {
    // Given - Create 50 blocking tasks (at the limit)
    let fixture = TestFixture::new().await;

    for i in 0..50 {
        let blocker_id = Uuid::new_v4();

        // Create work item
        sqlx::query(
            r#"
                INSERT INTO pm_work_items (id, item_type, parent_id, project_id, position, title, status, priority, version, created_at, updated_at, created_by, updated_by)
                VALUES (?, 'task', NULL, ?, 1, ?, 'todo', 'medium', 1, ?, ?, ?, ?)
                "#
        )
            .bind(blocker_id.to_string())
            .bind(fixture.project_id.to_string())
            .bind(format!("Blocker {}", i))
            .bind(Utc::now().timestamp())
            .bind(Utc::now().timestamp())
            .bind(fixture.user_id.to_string())
            .bind(fixture.user_id.to_string())
            .execute(&fixture.pool)
            .await
            .expect("Failed to create blocker work item");

        // Create dependency
        let ctx = fixture.create_context(&format!("msg-{:03}", i));
        let msg = WebSocketMessage {
            message_id: format!("msg-{:03}", i),
            timestamp: Utc::now().timestamp(),
            payload: Some(Payload::CreateDependencyRequest(CreateDependencyRequest {
                blocking_item_id: blocker_id.to_string(),
                blocked_item_id: fixture.task_a_id.to_string(),
                dependency_type: ProtoDependencyType::Blocks as i32,
            })),
        };
        dispatch(msg, ctx).await;
    }

    // When - Try to create 51st
    let extra_blocker_id = Uuid::new_v4();

    sqlx::query(
        r#"
            INSERT INTO pm_work_items (id, item_type, parent_id, project_id, position, title, status, priority, version, created_at, updated_at, created_by, updated_by)
            VALUES (?, 'task', NULL, ?, 1, 'Extra Blocker', 'todo', 'medium', 1, ?, ?, ?, ?)
            "#
    )
        .bind(extra_blocker_id.to_string())
        .bind(fixture.project_id.to_string())
        .bind(Utc::now().timestamp())
        .bind(Utc::now().timestamp())
        .bind(fixture.user_id.to_string())
        .bind(fixture.user_id.to_string())
        .execute(&fixture.pool)
        .await
        .expect("Failed to create extra blocker");

    let ctx = fixture.create_context("msg-051");
    let msg = WebSocketMessage {
        message_id: "msg-051".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateDependencyRequest(CreateDependencyRequest {
            blocking_item_id: extra_blocker_id.to_string(),
            blocked_item_id: fixture.task_a_id.to_string(),
            dependency_type: ProtoDependencyType::Blocks as i32,
        })),
    };
    let response = dispatch(msg, ctx).await;

    // Then
    match response.payload {
        Some(Payload::Error(err)) => {
            assert!(
                err.message.contains("50")
                    || err.message.contains("max")
                    || err.message.contains("limit")
            );
        }
        _ => panic!("Expected Error response with limit message"),
    }
}

// =============================================================================
// DeleteDependency Tests
// =============================================================================

#[tokio::test]
async fn given_existing_dependency_when_delete_then_succeeds() {
    // Given
    let fixture = TestFixture::new().await;

    // Create dependency
    let ctx1 = fixture.create_context("msg-001");
    let create_msg = WebSocketMessage {
        message_id: "msg-001".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateDependencyRequest(CreateDependencyRequest {
            blocking_item_id: fixture.task_a_id.to_string(),
            blocked_item_id: fixture.task_b_id.to_string(),
            dependency_type: ProtoDependencyType::Blocks as i32,
        })),
    };
    let create_response = dispatch(create_msg, ctx1).await;

    let dep_id = match &create_response.payload {
        Some(Payload::DependencyCreated(c)) => c.dependency.as_ref().unwrap().id.clone(),
        _ => panic!("Expected DependencyCreated"),
    };

    // When
    let ctx2 = fixture.create_context("msg-002");
    let delete_msg = WebSocketMessage {
        message_id: "msg-002".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::DeleteDependencyRequest(DeleteDependencyRequest {
            dependency_id: dep_id.clone(),
        })),
    };
    let response = dispatch(delete_msg, ctx2).await;

    // Then
    match response.payload {
        Some(Payload::DependencyDeleted(deleted)) => {
            assert_eq!(deleted.dependency_id, dep_id);
            assert_eq!(deleted.blocking_item_id, fixture.task_a_id.to_string());
            assert_eq!(deleted.blocked_item_id, fixture.task_b_id.to_string());
        }
        Some(Payload::Error(err)) => {
            panic!(
                "Got error response: code={}, message={}",
                err.code, err.message
            );
        }
        _ => panic!("Expected DependencyDeleted response"),
    }
}

// =============================================================================
// GetDependencies Tests
// =============================================================================

#[tokio::test]
async fn given_dependencies_when_get_then_returns_both_directions() {
    // Given
    let fixture = TestFixture::new().await;

    // A blocks B
    let ctx1 = fixture.create_context("msg-001");
    let msg1 = WebSocketMessage {
        message_id: "msg-001".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateDependencyRequest(CreateDependencyRequest {
            blocking_item_id: fixture.task_a_id.to_string(),
            blocked_item_id: fixture.task_b_id.to_string(),
            dependency_type: ProtoDependencyType::Blocks as i32,
        })),
    };
    dispatch(msg1, ctx1).await;

    // B blocks C
    let ctx2 = fixture.create_context("msg-002");
    let msg2 = WebSocketMessage {
        message_id: "msg-002".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateDependencyRequest(CreateDependencyRequest {
            blocking_item_id: fixture.task_b_id.to_string(),
            blocked_item_id: fixture.task_c_id.to_string(),
            dependency_type: ProtoDependencyType::Blocks as i32,
        })),
    };
    dispatch(msg2, ctx2).await;

    // When - Get dependencies for B
    let ctx3 = fixture.create_context("msg-003");
    let get_msg = WebSocketMessage {
        message_id: "msg-003".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::GetDependenciesRequest(GetDependenciesRequest {
            work_item_id: fixture.task_b_id.to_string(),
        })),
    };
    let response = dispatch(get_msg, ctx3).await;

    // Then
    match response.payload {
        Some(Payload::DependenciesList(list)) => {
            // B is blocked by A
            assert_eq!(list.blocking.len(), 1);
            assert_eq!(
                list.blocking[0].blocking_item_id,
                fixture.task_a_id.to_string()
            );

            // B blocks C
            assert_eq!(list.blocked.len(), 1);
            assert_eq!(
                list.blocked[0].blocked_item_id,
                fixture.task_c_id.to_string()
            );
        }
        Some(Payload::Error(err)) => {
            panic!(
                "Got error response: code={}, message={}",
                err.code, err.message
            );
        }
        _ => panic!("Expected DependenciesList response"),
    }
}
