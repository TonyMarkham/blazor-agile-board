//! Integration tests for comment handlers.
//!
//! Tests verify:
//! - Comment CRUD operations
//! - Author-only edit/delete permissions
//! - Comment attachment to work items

use pm_proto::{
    CreateCommentRequest, UpdateCommentRequest, DeleteCommentRequest, GetCommentsRequest,
    WebSocketMessage, web_socket_message::Payload,
};
use pm_ws::{CircuitBreaker, CircuitBreakerConfig, HandlerContext, dispatch};
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

// =========================================================================
// Test Fixtures
// =========================================================================

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

        // Create test users in users table (required for activity log FK)
        sqlx::query(
            r#"
              INSERT INTO users (id, email, name, created_at)
              VALUES (?, 'test@example.com', 'Test User', ?)
              "#
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
              "#
        )
            .bind(other_user_id.to_string())
            .bind(Utc::now().timestamp())
            .execute(&pool)
            .await
            .expect("Failed to create other user");

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
              "#
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
        HandlerContext::new(
            message_id.to_string(),
            self.user_id,
            self.pool.clone(),
            self.circuit_breaker.clone(),
            "test-connection".to_string(),
        )
    }

    fn create_context_as(&self, message_id: &str, user_id: Uuid) -> HandlerContext {
        HandlerContext::new(
            message_id.to_string(),
            user_id,
            self.pool.clone(),
            self.circuit_breaker.clone(),
            "test-connection".to_string(),
        )
    }
}

// =========================================================================
// Create Comment Tests
// =========================================================================

#[tokio::test]
async fn given_valid_request_when_create_comment_then_succeeds() {
    // Given
    let fixture = TestFixture::new().await;
    let ctx = fixture.create_context("msg-001");

    let msg = WebSocketMessage {
        message_id: "msg-001".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateCommentRequest(CreateCommentRequest {
            work_item_id: fixture.work_item_id.to_string(),
            content: "This is a test comment".to_string(),
        })),
    };

    // When
    let response = dispatch(msg, ctx).await;

    // Then
    match response.payload {
        Some(Payload::CommentCreated(created)) => {
            let comment = created.comment.unwrap();
            assert_eq!(comment.content, "This is a test comment");
            assert_eq!(comment.work_item_id, fixture.work_item_id.to_string());
            assert_eq!(comment.created_by, fixture.user_id.to_string());
        }
        Some(Payload::Error(err)) => {
            panic!("Got error response: code={}, message={}", err.code, err.message);
        }
        _ => panic!("Expected CommentCreated response"),
    }
}

#[tokio::test]
async fn given_empty_content_when_create_comment_then_validation_error() {
    // Given
    let fixture = TestFixture::new().await;
    let ctx = fixture.create_context("msg-002");

    let msg = WebSocketMessage {
        message_id: "msg-002".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateCommentRequest(CreateCommentRequest {
            work_item_id: fixture.work_item_id.to_string(),
            content: "".to_string(),
        })),
    };

    // When
    let response = dispatch(msg, ctx).await;

    // Then
    match response.payload {
        Some(Payload::Error(err)) => {
            assert_eq!(err.code, "INVALID_MESSAGE");
        }
        _ => panic!("Expected INVALID_MESSAGE"),
    }
}

#[tokio::test]
async fn given_nonexistent_work_item_when_create_comment_then_not_found() {
    // Given
    let fixture = TestFixture::new().await;
    let ctx = fixture.create_context("msg-003");

    let msg = WebSocketMessage {
        message_id: "msg-003".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateCommentRequest(CreateCommentRequest {
            work_item_id: Uuid::new_v4().to_string(),
            content: "Test comment".to_string(),
        })),
    };

    // When
    let response = dispatch(msg, ctx).await;

    // Then
    match response.payload {
        Some(Payload::Error(err)) => {
            assert_eq!(err.code, "NOT_FOUND");
        }
        _ => panic!("Expected NOT_FOUND error"),
    }
}

// =========================================================================
// Author-Only Edit Tests
// =========================================================================

#[tokio::test]
async fn given_author_when_update_comment_then_succeeds() {
    // Given - Create a comment
    let fixture = TestFixture::new().await;
    let ctx = fixture.create_context("msg-create");

    let create_msg = WebSocketMessage {
        message_id: "msg-create".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateCommentRequest(CreateCommentRequest {
            work_item_id: fixture.work_item_id.to_string(),
            content: "Original content".to_string(),
        })),
    };
    let create_response = dispatch(create_msg, ctx).await;
    let comment_id = match create_response.payload {
        Some(Payload::CommentCreated(c)) => c.comment.unwrap().id,
        _ => panic!("Failed to create comment"),
    };

    // When - Author updates their comment
    let ctx = fixture.create_context("msg-update");
    let update_msg = WebSocketMessage {
        message_id: "msg-update".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::UpdateCommentRequest(UpdateCommentRequest {
            comment_id: comment_id.clone(),
            content: "Updated content".to_string(),
        })),
    };
    let response = dispatch(update_msg, ctx).await;

    // Then
    match response.payload {
        Some(Payload::CommentUpdated(updated)) => {
            let comment = updated.comment.unwrap();
            assert_eq!(comment.content, "Updated content");
        }
        Some(Payload::Error(err)) => {
            panic!("Got error response: code={}, message={}", err.code, err.message);
        }
        _ => panic!("Expected CommentUpdated response"),
    }
}

#[tokio::test]
async fn given_non_author_when_update_comment_then_unauthorized() {
    // Given - Create a comment as user_id
    let fixture = TestFixture::new().await;
    let ctx = fixture.create_context("msg-create");

    let create_msg = WebSocketMessage {
        message_id: "msg-create".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateCommentRequest(CreateCommentRequest {
            work_item_id: fixture.work_item_id.to_string(),
            content: "Original content".to_string(),
        })),
    };
    let create_response = dispatch(create_msg, ctx).await;
    let comment_id = match create_response.payload {
        Some(Payload::CommentCreated(c)) => c.comment.unwrap().id,
        _ => panic!("Failed to create comment"),
    };

    // When - Different user tries to update
    let ctx = fixture.create_context_as("msg-update", fixture.other_user_id);
    let update_msg = WebSocketMessage {
        message_id: "msg-update".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::UpdateCommentRequest(UpdateCommentRequest {
            comment_id,
            content: "Hacked content".to_string(),
        })),
    };
    let response = dispatch(update_msg, ctx).await;

    // Then
    match response.payload {
        Some(Payload::Error(err)) => {
            assert_eq!(err.code, "UNAUTHORIZED");
            assert!(err.message.contains("another user"));
        }
        _ => panic!("Expected UNAUTHORIZED error"),
    }
}

// =========================================================================
// Author-Only Delete Tests
// =========================================================================

#[tokio::test]
async fn given_author_when_delete_comment_then_succeeds() {
    // Given - Create a comment
    let fixture = TestFixture::new().await;
    let ctx = fixture.create_context("msg-create");

    let create_msg = WebSocketMessage {
        message_id: "msg-create".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateCommentRequest(CreateCommentRequest {
            work_item_id: fixture.work_item_id.to_string(),
            content: "To be deleted".to_string(),
        })),
    };
    let create_response = dispatch(create_msg, ctx).await;
    let comment_id = match create_response.payload {
        Some(Payload::CommentCreated(c)) => c.comment.unwrap().id,
        _ => panic!("Failed to create comment"),
    };

    // When - Author deletes their comment
    let ctx = fixture.create_context("msg-delete");
    let delete_msg = WebSocketMessage {
        message_id: "msg-delete".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::DeleteCommentRequest(DeleteCommentRequest {
            comment_id: comment_id.clone(),
        })),
    };
    let response = dispatch(delete_msg, ctx).await;

    // Then
    match response.payload {
        Some(Payload::CommentDeleted(deleted)) => {
            assert_eq!(deleted.comment_id, comment_id);
        }
        Some(Payload::Error(err)) => {
            panic!("Got error response: code={}, message={}", err.code, err.message);
        }
        _ => panic!("Expected CommentDeleted response"),
    }
}

#[tokio::test]
async fn given_non_author_when_delete_comment_then_unauthorized() {
    // Given - Create a comment as user_id
    let fixture = TestFixture::new().await;
    let ctx = fixture.create_context("msg-create");

    let create_msg = WebSocketMessage {
        message_id: "msg-create".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::CreateCommentRequest(CreateCommentRequest {
            work_item_id: fixture.work_item_id.to_string(),
            content: "Protected content".to_string(),
        })),
    };
    let create_response = dispatch(create_msg, ctx).await;
    let comment_id = match create_response.payload {
        Some(Payload::CommentCreated(c)) => c.comment.unwrap().id,
        _ => panic!("Failed to create comment"),
    };

    // When - Different user tries to delete
    let ctx = fixture.create_context_as("msg-delete", fixture.other_user_id);
    let delete_msg = WebSocketMessage {
        message_id: "msg-delete".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::DeleteCommentRequest(DeleteCommentRequest {
            comment_id,
        })),
    };
    let response = dispatch(delete_msg, ctx).await;

    // Then
    match response.payload {
        Some(Payload::Error(err)) => {
            assert_eq!(err.code, "UNAUTHORIZED");
        }
        _ => panic!("Expected UNAUTHORIZED error"),
    }
}

// =========================================================================
// Get Comments Tests
// =========================================================================

#[tokio::test]
async fn given_work_item_with_comments_when_get_comments_then_returns_all() {
    // Given - Create two comments
    let fixture = TestFixture::new().await;

    for i in 1..=2 {
        let ctx = fixture.create_context(&format!("msg-create-{}", i));
        let create_msg = WebSocketMessage {
            message_id: format!("msg-create-{}", i),
            timestamp: Utc::now().timestamp(),
            payload: Some(Payload::CreateCommentRequest(CreateCommentRequest {
                work_item_id: fixture.work_item_id.to_string(),
                content: format!("Comment {}", i),
            })),
        };
        dispatch(create_msg, ctx).await;
    }

    // When - Get comments
    let ctx = fixture.create_context("msg-get");
    let get_msg = WebSocketMessage {
        message_id: "msg-get".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::GetCommentsRequest(GetCommentsRequest {
            work_item_id: fixture.work_item_id.to_string(),
        })),
    };
    let response = dispatch(get_msg, ctx).await;

    // Then
    match response.payload {
        Some(Payload::CommentsList(list)) => {
            assert_eq!(list.comments.len(), 2);
        }
        _ => panic!("Expected CommentsList response"),
    }
}