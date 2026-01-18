#![allow(dead_code)]

use pm_auth::{JwtValidator, RateLimitConfig, RateLimiterFactory};
use pm_ws::{
    AppState, ConnectionConfig, ConnectionLimits, ConnectionRegistry, Metrics, ShutdownCoordinator,
};

use std::sync::Arc;

use axum::{Router, routing::get};
use axum_test::TestServer;

/// Default JWT secret for all tests (HS256 requires at least 32 bytes)
pub const TEST_JWT_SECRET: &[u8] = b"test-secret-key-for-integration-tests-min-32-bytes-long";

/// Default desktop user ID for tests
pub const TEST_DESKTOP_USER_ID: &str = "test-user";

/// Configuration for test server instances
#[derive(Debug, Clone)]
pub struct TestServerConfig {
    pub jwt_secret: Option<Vec<u8>>,
    pub desktop_user_id: String,
    pub max_connections_total: usize,
    pub rate_limit_max_requests: u32,
    pub rate_limit_window_secs: u64,
}

impl Default for TestServerConfig {
    fn default() -> Self {
        Self {
            jwt_secret: Some(TEST_JWT_SECRET.to_vec()),
            desktop_user_id: TEST_DESKTOP_USER_ID.to_string(),
            max_connections_total: 100,
            rate_limit_max_requests: 100,
            rate_limit_window_secs: 60,
        }
    }
}

impl TestServerConfig {
    /// Create config for desktop mode (no JWT authentication)
    pub fn with_desktop_mode() -> Self {
        Self {
            jwt_secret: None,
            desktop_user_id: "local-user".to_string(),
            ..Default::default()
        }
    }

    /// Create config with custom desktop user ID
    pub fn with_desktop_user_id(user_id: impl Into<String>) -> Self {
        Self {
            jwt_secret: None,
            desktop_user_id: user_id.into(),
            ..Default::default()
        }
    }

    /// Create config with strict connection limits (for limit tests)
    pub fn with_strict_limits() -> Self {
        Self {
            max_connections_total: 5,
            ..Default::default()
        }
    }

    /// Create config with strict rate limits (for rate limit tests)
    pub fn with_strict_rate_limits() -> Self {
        Self {
            rate_limit_max_requests: 5,
            rate_limit_window_secs: 1,
            ..Default::default()
        }
    }
}

/// Test server with access to AppState for testing
pub struct TestServerWithState {
    pub server: TestServer,
    pub app_state: AppState,
}

/// Create a TestServer with default configuration
pub fn create_test_server() -> TestServerWithState {
    create_test_server_with_config(TestServerConfig::default())
}

/// Create a TestServer with custom configuration
pub fn create_test_server_with_config(config: TestServerConfig) -> TestServerWithState {
    let (app, app_state) = create_app(config);
    let server = TestServer::builder()
        .http_transport()
        .build(app)
        .expect("Failed to create test server");

    TestServerWithState { server, app_state }
}

/// Build the Axum Router with AppState
fn create_app(config: TestServerConfig) -> (Router, AppState) {
    // Create JWT validator (optional based on config)
    let jwt_validator: Option<Arc<JwtValidator>> = config
        .jwt_secret
        .map(|secret| Arc::new(JwtValidator::with_hs256(&secret)));

    // Create rate limiter factory
    let rate_limit_config = RateLimitConfig {
        max_requests: config.rate_limit_max_requests,
        window_secs: config.rate_limit_window_secs,
    };
    let rate_limiter_factory = RateLimiterFactory::new(rate_limit_config);

    // Create connection registry with limits
    let limits = ConnectionLimits {
        max_total: config.max_connections_total,
    };
    let registry = ConnectionRegistry::new(limits);

    // Create metrics tracker
    let metrics = Metrics::default();

    // Create shutdown coordinator
    let shutdown = ShutdownCoordinator::new();

    // Create connection config
    let connection_config = ConnectionConfig::default();

    // Build AppState
    let app_state = AppState {
        jwt_validator,
        desktop_user_id: config.desktop_user_id,
        rate_limiter_factory,
        registry,
        metrics,
        shutdown,
        config: connection_config,
    };

    let router = Router::new()
        .route("/ws", get(pm_ws::handler))
        .with_state(app_state.clone());

    (router, app_state)
}
