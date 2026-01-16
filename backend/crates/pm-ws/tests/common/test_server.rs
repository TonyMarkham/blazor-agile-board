use pm_auth::{JwtValidator, RateLimitConfig, RateLimiterFactory};
use pm_ws::{
    AppState, BroadcastConfig, ConnectionConfig, ConnectionLimits, ConnectionRegistry, Metrics,
    ShutdownCoordinator, TenantBroadcaster,
};

use axum::{routing::get, Router};
use axum_test::TestServer;

/// Default JWT secret for all tests (HS256 requires at least 32 bytes)
pub const TEST_JWT_SECRET: &[u8] = b"test-secret-key-for-integration-tests-min-32-bytes-long";

/// Configuration for test server instances
#[derive(Debug, Clone)]
pub struct TestServerConfig {
    pub jwt_secret: Vec<u8>,
    pub max_connections_total: usize,
    pub max_connections_per_tenant: usize,
    pub rate_limit_max_requests: u32,
    pub rate_limit_window_secs: u64,
}

impl Default for TestServerConfig {
    fn default() -> Self {
        Self {
            jwt_secret: TEST_JWT_SECRET.to_vec(),
            max_connections_total: 100,
            max_connections_per_tenant: 10,
            rate_limit_max_requests: 100,
            rate_limit_window_secs: 60,
        }
    }
}

impl TestServerConfig {
    /// Create config with strict connection limits (for limit tests)
    pub fn with_strict_limits() -> Self {
        Self {
            max_connections_total: 5,
            max_connections_per_tenant: 2,
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

    TestServerWithState {
        server,
        app_state,
    }
}

/// Build the Axum Router with AppState
fn create_app(config: TestServerConfig) -> (Router, AppState) {
    // Create JWT validator (HS256)
    let jwt_validator = JwtValidator::with_hs256(&config.jwt_secret);

    // Create rate limiter factory
    let rate_limit_config = RateLimitConfig {
        max_requests: config.rate_limit_max_requests,
        window_secs: config.rate_limit_window_secs,
    };
    let rate_limiter_factory = RateLimiterFactory::new(rate_limit_config);

    // Create connection registry with limits
    let limits = ConnectionLimits {
        max_per_tenant: config.max_connections_per_tenant,
        max_total: config.max_connections_total,
    };
    let registry = ConnectionRegistry::new(limits);

    // Create tenant broadcaster with default config
    let broadcast_config = BroadcastConfig::default();
    let broadcaster = TenantBroadcaster::new(broadcast_config);

    // Create metrics tracker
    let metrics = Metrics::default();

    // Create shutdown coordinator
    let shutdown = ShutdownCoordinator::new();

    // Create connection config
    let connection_config = ConnectionConfig::default();

    // Build AppState
    let app_state = AppState {
        jwt_validator: std::sync::Arc::new(jwt_validator),
        rate_limiter_factory,
        broadcaster,
        registry,
        metrics,
        shutdown,
        config: connection_config,
    };

    let router = Router::new()
        .route("/ws", get(pm_ws::app_state::handler))
        .with_state(app_state.clone());

    (router, app_state)
}