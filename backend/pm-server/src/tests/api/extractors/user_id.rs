use crate::UserId;

use pm_auth::RateLimiterFactory;
use pm_config::ApiConfig;
use pm_ws::{
    AppState, CircuitBreaker, CircuitBreakerConfig, ConnectionConfig, ConnectionRegistry, Metrics,
    ShutdownCoordinator,
};

use std::sync::Arc;

use axum::{body::Body, extract::FromRequestParts, http::Request};
use sqlx::SqlitePool;

async fn create_test_state() -> AppState {
    let pool = SqlitePool::connect(":memory:")
        .await
        .expect("Failed to create test pool");

    sqlx::migrate!("../crates/pm-db/migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let shutdown = ShutdownCoordinator::new();

    AppState {
        pool,
        circuit_breaker: Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default())),
        jwt_validator: None,
        desktop_user_id: "desktop-user".to_string(),
        rate_limiter_factory: RateLimiterFactory::new(pm_auth::RateLimitConfig {
            max_requests: 100,
            window_secs: 60,
        }),
        registry: ConnectionRegistry::new(pm_ws::ConnectionLimits { max_total: 10000 }),
        metrics: Metrics::new(),
        shutdown,
        config: ConnectionConfig::default(),
        api_config: ApiConfig::default(),
    }
}

#[tokio::test]
async fn test_extractor_with_valid_header() {
    let state = create_test_state().await;
    let request = Request::builder()
        .header("X-User-Id", "12345678-1234-1234-1234-123456789abc")
        .body(Body::empty())
        .unwrap();

    let (mut parts, _body) = request.into_parts();
    let result = UserId::from_request_parts(&mut parts, &state).await;

    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().0.to_string(),
        "12345678-1234-1234-1234-123456789abc"
    );
}

#[tokio::test]
async fn test_extractor_falls_back_to_llm_user_when_missing() {
    let state = create_test_state().await;
    let request = Request::builder().body(Body::empty()).unwrap();

    let (mut parts, _body) = request.into_parts();
    let result = UserId::from_request_parts(&mut parts, &state).await;

    assert!(result.is_ok());
    let user_id = result.unwrap().0;
    assert_eq!(user_id, state.api_config.llm_user_uuid());
}

#[tokio::test]
async fn test_extractor_falls_back_when_header_invalid_uuid() {
    let state = create_test_state().await;
    let request = Request::builder()
        .header("X-User-Id", "not-a-valid-uuid")
        .body(Body::empty())
        .unwrap();

    let (mut parts, _body) = request.into_parts();
    let result = UserId::from_request_parts(&mut parts, &state).await;

    assert!(result.is_ok());
    let user_id = result.unwrap().0;
    assert_eq!(user_id, state.api_config.llm_user_uuid());
}

#[tokio::test]
async fn test_extractor_preserves_custom_llm_user_id() {
    let pool = SqlitePool::connect(":memory:")
        .await
        .expect("Failed to create test pool");

    sqlx::migrate!("../crates/pm-db/migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let shutdown = ShutdownCoordinator::new();

    let custom_llm_id = "99999999-9999-9999-9999-999999999999";
    let custom_config = ApiConfig {
        enabled: true,
        llm_user_id: custom_llm_id.to_string(),
        llm_user_name: "Custom LLM".to_string(),
    };

    let state = AppState {
        pool,
        circuit_breaker: Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default())),
        jwt_validator: None,
        desktop_user_id: "desktop-user".to_string(),
        rate_limiter_factory: RateLimiterFactory::new(pm_auth::RateLimitConfig {
            max_requests: 100,
            window_secs: 60,
        }),
        registry: ConnectionRegistry::new(pm_ws::ConnectionLimits { max_total: 10000 }),
        metrics: Metrics::new(),
        shutdown,
        config: ConnectionConfig::default(),
        api_config: custom_config,
    };

    let request = Request::builder().body(Body::empty()).unwrap();

    let (mut parts, _body) = request.into_parts();
    let result = UserId::from_request_parts(&mut parts, &state).await;

    assert!(result.is_ok());
    let user_id = result.unwrap().0;
    assert_eq!(user_id.to_string(), custom_llm_id);
}
