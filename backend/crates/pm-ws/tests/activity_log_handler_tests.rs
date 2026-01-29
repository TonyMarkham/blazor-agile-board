use pm_proto::{GetActivityLogRequest, WebSocketMessage, web_socket_message::Payload};
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
    #[allow(dead_code)]
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
        let project_id = Uuid::new_v4();
        let work_item_id = Uuid::new_v4();

        // Seed user
        sqlx::query("INSERT INTO users (id, email, name, created_at) VALUES (?, 'test@example.com', 'Test User', ?)")
            .bind(user_id.to_string())
            .bind(Utc::now().timestamp())
            .execute(&pool)
            .await
            .expect("Failed to seed user");

        // Seed project
        sqlx::query(
            "INSERT INTO pm_projects (id, title, key, status, version, created_at, updated_at, created_by, updated_by)
             VALUES (?, 'Test Project', 'TEST', 'active', 1, ?, ?, ?, ?)"
        )
            .bind(project_id.to_string())
            .bind(Utc::now().timestamp())
            .bind(Utc::now().timestamp())
            .bind(user_id.to_string())
            .bind(user_id.to_string())
            .execute(&pool)
            .await
            .expect("Failed to seed project");

        // Seed membership
        sqlx::query(
            "INSERT INTO pm_project_members (id, project_id, user_id, role, created_at)
             VALUES (?, ?, ?, 'viewer', ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(project_id.to_string())
        .bind(user_id.to_string())
        .bind(Utc::now().timestamp())
        .execute(&pool)
        .await
        .expect("Failed to seed project member");

        // Seed work item
        sqlx::query(
            "INSERT INTO pm_work_items (id, item_type, parent_id, project_id, position, title, status, priority, version, created_at, updated_at, created_by, updated_by)
             VALUES (?, 'task', NULL, ?, 1, 'Test Task', 'todo', 'medium', 1, ?, ?, ?, ?)"
        )
            .bind(work_item_id.to_string())
            .bind(project_id.to_string())
            .bind(Utc::now().timestamp())
            .bind(Utc::now().timestamp())
            .bind(user_id.to_string())
            .bind(user_id.to_string())
            .execute(&pool)
            .await
            .expect("Failed to seed work item");

        Self {
            pool,
            circuit_breaker: Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default())),
            user_id,
            project_id,
            work_item_id,
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
        )
    }
}

#[tokio::test]
async fn given_get_activity_log_when_dispatched_then_returns_list() {
    let fixture = TestFixture::new().await;

    let msg = WebSocketMessage {
        message_id: "msg-001".to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::GetActivityLogRequest(GetActivityLogRequest {
            entity_type: "work_item".to_string(),
            entity_id: fixture.work_item_id.to_string(),
            limit: 10,
            offset: 0,
        })),
    };

    let response = dispatch(msg, fixture.ctx("msg-001")).await;

    match response.payload {
        Some(Payload::ActivityLogList(list)) => {
            assert_eq!(list.total_count, 0);
            assert!(list.entries.is_empty());
        }
        _ => panic!("Expected ActivityLogList response"),
    }
}
