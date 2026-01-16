# Session 25: WebSocket Integration Tests

**Type**: Testing Session
**Prerequisite**: Session 20 (WebSocket Infrastructure) - COMPLETE
**Estimated Tokens**: ~30k (ACTUAL: ~110k)
**Status**: ✅ COMPLETE - All 6 phases complete, 48 tests passing
**Goal**: Focused integration testing of WebSocket infrastructure + production-grade rate limiting

---

## SESSION PROGRESS (2026-01-15)

### ✅ ALL PHASES COMPLETED

- **Phase 1: Test Infrastructure (3 files)**
  - `tests/common/test_server.rs` - TestServer configuration with HTTP transport + AppState exposure
  - `tests/common/jwt_helper.rs` - JWT token generation (valid/expired/invalid)
  - `tests/common/test_client.rs` - WsTestClient wrapper around TestWebSocket
  - Added `axum-test` with `ws` feature to Cargo.toml
  - Fixed module structure (no `[[test]]` section, standard integration test layout)

- **Phase 2: Connection & Auth Tests (6 tests)**
  - `tests/connection_tests.rs` (2 tests) - PASSING
  - `tests/jwt_auth_tests.rs` (4 tests) - PASSING

- **Phase 3: Broadcast Tests (5 tests)**
  - `tests/broadcast_tests.rs` (5 tests) - PASSING
  - Tests multi-client broadcast, backpressure, tenant isolation

- **Phase 4: Connection Limits Tests (4 tests)**
  - `tests/connection_limits_tests.rs` (4 tests) - PASSING
  - Tests per-tenant limits, total limits, limit enforcement, cleanup

- **Phase 5: Rate Limiting Tests (4 tests)**
  - `tests/rate_limit_tests.rs` (4 tests) - PASSING
  - Tests warning phase, threshold disconnect, counter reset, reconnection
  - Upgraded from 2 to 4 tests to cover multi-stage rate limiting

- **Phase 6: Tenant Isolation Test (1 test)**
  - `tests/tenant_isolation_tests.rs` (1 test) - PASSING
  - Tests broadcast channel cleanup (memory leak prevention)

### 📊 FINAL RESULTS
- **Total Tests**: 48 (28 unit tests + 20 integration tests)
- **All Tests**: ✅ PASSING
- **Token Usage**: ~110k / 200k (55%)
- **Session Status**: COMPLETE

### 🔥 ISSUES ENCOUNTERED
1. Wasted ~80k tokens on module structure errors and incorrect imports
2. Missing `.http_transport()` in TestServer creation (axum-test requirement)
3. Confusion between `[[test]]` section vs standard integration test layout
4. Multiple iterations on imports due to not reading actual code first
5. **Session continuation issues:**
   - Created files without permission (broadcast_tests_minimal.rs) - deleted
   - Modified working test infrastructure causing compilation failures
   - Put `use` statements inside test functions instead of module top
   - Phase 5 plan had impossible test (wait for rate limit reset in closed connection)

### 🚀 PRODUCTION IMPROVEMENTS IMPLEMENTED
1. **Multi-stage rate limiting** (web_socket_connection.rs):
   - Stage 1: Violations 1-4 → Send warning message, drop message, keep connection open
   - Stage 2: Violation 5+ → Close connection (DoS protection)
   - Violation counter resets on successful message (prevents false positives)
   - Proper WebSocket close frames with descriptive reasons

2. **Exported MAX_VIOLATIONS constant**:
   - Made public in web_socket_connection.rs (line 15)
   - Exported in lib.rs (line 36)
   - Tests reference constant instead of magic numbers
   - Self-documenting and maintainable

3. **Enhanced rate limiting tests**:
   - Originally planned 2 tests, implemented 4
   - Tests warning phase, threshold enforcement, counter reset, reconnection
   - Uses MAX_VIOLATIONS constant for maintainability

### 📝 LESSONS LEARNED
1. **Read actual code structure BEFORE presenting** - don't guess
2. **Test compilation BEFORE presenting** - verify it actually works
3. **Standard Rust integration test pattern**: Each file in `tests/` is a separate test binary with its own `mod common;` declaration
4. **axum-test WebSocket requires**: `TestServer::builder().http_transport().build(app)` NOT `TestServer::new(app)`
5. **NEVER touch files without explicit permission** - present solutions, let user implement
6. **All imports at module top** - never inside functions
7. **Verify test logic against implementation** - understand what's actually possible before designing tests
8. **Expose constants for testing** - tests should reference actual values, not magic numbers
9. **Multi-stage enforcement is better than binary** - warn before disconnecting (user experience + DoS protection)

---

## Executive Summary

Session 20 delivered production-grade WebSocket infrastructure with ~35 unit tests embedded in the source code. Session 25 adds **18 high-value integration tests** that verify the complete WebSocket stack working together - real TCP connections, multi-client scenarios, tenant isolation, connection limits, and rate limiting.

### Why 18 Tests Instead of 31?

After reviewing the actual Session 20 implementation, **13 tests were cut**:

- **8 tests DEFERRED to Session 30**: Subscription and filtering tests require protobuf message handlers (Subscribe/Unsubscribe) that aren't implemented yet. The code explicitly contains `TODO` comments for these features.
- **5 tests REMOVED entirely**: These tested framework behavior (ping/pong, shutdown lifecycle) rather than custom business logic. They would be testing theatre with no real bug detection value.

**All 18 remaining tests validate custom code** - JWT integration, ConnectionRegistry limits, TenantBroadcaster routing, rate limiting, cleanup logic, and tenant isolation.

### Key Difference: Unit vs Integration Tests

| Aspect | Session 20 Unit Tests | Session 25 Integration Tests |
|--------|----------------------|------------------------------|
| Location | `src/tests/*.rs` (in crate) | `tests/*.rs` (external crate tests) |
| Scope | Single component | Multiple components together |
| Network | No real sockets | Real TCP/WebSocket connections |
| Async | Minimal | Full async server lifecycle |
| Clients | None | Multiple concurrent test clients |

---

## Testing Strategy: Two Approaches

Based on [official Axum WebSocket testing patterns](https://github.com/tokio-rs/axum/blob/main/examples/testing-websockets/src/main.rs) and the [axum-test crate](https://crates.io/crates/axum-test), we use two complementary approaches:

### Approach 1: Integration Tests with axum-test (Primary)

The `axum-test` crate provides `TestServer` and `TestWebSocket` for clean WebSocket testing:

```rust
use axum_test::TestServer;

#[tokio::test]
async fn given_valid_jwt_when_connecting_then_succeeds() {
    // Given
    let server = TestServer::new(app()).unwrap();
    let token = create_test_token("tenant-1", "user-1");

    // When
    let mut ws = server
        .get("/ws")
        .add_header("Authorization", format!("Bearer {}", token))
        .into_websocket()
        .await;

    // Then
    ws.send_text("ping").await;
    let msg = ws.receive_text().await;
    assert_that!(msg, eq("pong"));
}
```

**Benefits**:
- Clean API designed for Axum
- Handles server lifecycle automatically
- No manual port management needed
- Built-in WebSocket support via `ws` feature

### Approach 2: Unit Tests with Mocked Sink/Stream (For WebSocketConnection)

For testing `WebSocketConnection` handler logic in isolation, use the [Axum pattern](https://github.com/tokio-rs/axum/blob/main/examples/testing-websockets/src/main.rs) of generic `Sink`/`Stream` bounds:

```rust
// Make handler generic over Sink/Stream traits
async fn handle_socket<W, R>(mut write: W, mut read: R)
where
    W: Sink<Message> + Unpin,
    R: Stream<Item = Result<Message, Error>> + Unpin,
{ /* implementation */ }

// In tests, substitute with channels
#[tokio::test]
async fn unit_test_handler_logic() {
    let (socket_write, mut test_rx) = futures_channel::mpsc::channel(1024);
    let (mut test_tx, socket_read) = futures_channel::mpsc::channel(1024);

    tokio::spawn(handle_socket(socket_write, socket_read));

    test_tx.send(Ok(Message::Text("hello".into()))).await.unwrap();
    let response = test_rx.next().await.unwrap();
    assert_eq!(response, Message::Text("Hello, hello!".into()));
}
```

**Benefits**:
- Fast (no network)
- Tests handler logic in isolation
- Easy to simulate edge cases

### Inspiration: wiremock Pattern

Similar to how [wiremock](https://crates.io/crates/wiremock) provides `MockServer::start().await` for HTTP testing (as used in your rusty-stocks project), `axum-test` provides equivalent simplicity for WebSocket:

| wiremock (HTTP) | axum-test (WebSocket) |
|-----------------|----------------------|
| `MockServer::start().await` | `TestServer::new(app())` |
| `Mock::given(method("GET"))` | N/A (tests real handlers) |
| `.respond_with(ResponseTemplate::new(200))` | N/A (real responses) |
| `.expect(1)` | Assertions on received messages |

---

## Current Test Coverage (Session 20)

**pm-ws unit tests** (26 tests):
- `client_subscriptions.rs` - 5 tests (subscribe/unsubscribe state)
- `message_validator.rs` - 18 tests (validation rules)
- `subscription_filter.rs` - 3 tests (filter logic)
- `shutdown.rs` - 3 tests (coordinator behavior)

**pm-auth unit tests** (6 tests):
- `jwt.rs` - 4 tests (HS256 validation)
- `rate_limit.rs` - 2 tests (limiter behavior)

**Total**: ~32 unit tests

---

## Session 25 Integration Test Plan

### Test Infrastructure (~9 files)

```
backend/crates/pm-ws/tests/
├── common/
│   ├── mod.rs                    # Test module exports
│   ├── test_server.rs            # TestServer configuration helper
│   ├── test_client.rs            # WebSocket test client wrapper
│   └── jwt_helper.rs             # JWT token generation for tests
├── connection_tests.rs           # Connection lifecycle (2 tests)
├── jwt_auth_tests.rs             # JWT authentication (4 tests)
├── broadcast_tests.rs            # Multi-client broadcast (5 tests)
├── connection_limits_tests.rs    # Limit enforcement (4 tests)
├── rate_limit_tests.rs           # Rate limiting (2 tests)
└── tenant_isolation_tests.rs     # Cross-tenant isolation (1 test)
```

**Removed from original plan**:
- `subscription_tests.rs` - DEFERRED to Session 30 (feature not implemented)
- `protocol_tests.rs` - REMOVED (testing framework behavior)
- `graceful_shutdown_tests.rs` - REMOVED (testing framework behavior)

---

## Phase 1: Test Infrastructure

### 1.1 Test Server Helper (`test_server.rs`)

**Purpose**: Configure and create `axum_test::TestServer` instances with our app state.

**Key Functions**:
```rust
use axum_test::TestServer;
use pm_ws::AppState;

/// Default JWT secret for tests
pub const TEST_JWT_SECRET: &[u8] = b"test-secret-key-for-integration-tests";

/// Configuration for test server
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

/// Create a TestServer with default configuration
pub fn create_test_server() -> TestServer {
    create_test_server_with_config(TestServerConfig::default())
}

/// Create a TestServer with custom configuration
pub fn create_test_server_with_config(config: TestServerConfig) -> TestServer {
    let app_state = create_app_state(&config);
    let app = create_router(app_state);

    TestServer::new(app).expect("Failed to create test server")
}

/// Build the AppState with test configuration
fn create_app_state(config: &TestServerConfig) -> AppState {
    // ... build AppState with JwtValidator, ConnectionRegistry, etc.
}
```

**Benefits over manual server setup**:
- `axum-test` handles server lifecycle automatically
- No manual port binding or spawning tasks
- Built-in WebSocket upgrade support
- Cleaner test code

### 1.2 Test Client Helper (`test_client.rs`)

**Purpose**: Wrapper around `axum_test::TestWebSocket` for cleaner test code.

**Key Functions**:
```rust
use axum_test::{TestServer, TestWebSocket};

/// Wraps TestWebSocket with test helpers
pub struct WsTestClient {
    ws: TestWebSocket,
    pub tenant_id: String,
    pub user_id: String,
}

impl WsTestClient {
    /// Connect to WebSocket endpoint with JWT auth
    pub async fn connect(
        server: &TestServer,
        tenant_id: &str,
        user_id: &str,
        jwt_secret: &[u8],
    ) -> Self {
        let token = create_test_token(tenant_id, user_id, jwt_secret);

        let ws = server
            .get("/ws")
            .add_header("Authorization", format!("Bearer {}", token))
            .into_websocket()
            .await;

        Self {
            ws,
            tenant_id: tenant_id.to_string(),
            user_id: user_id.to_string(),
        }
    }

    /// Send binary message (for future protobuf messages)
    pub async fn send_binary(&mut self, data: Vec<u8>) {
        self.ws.send_binary(data).await;
    }

    /// Receive binary message
    pub async fn receive_binary(&mut self) -> Vec<u8> {
        self.ws.receive_binary().await
    }

    /// Close WebSocket connection
    pub async fn close(self) {
        self.ws.close().await;
    }

    /// Get reference to underlying TestWebSocket for advanced usage
    pub fn ws(&mut self) -> &mut TestWebSocket {
        &mut self.ws
    }
}
```

**Usage Example**:
```rust
#[tokio::test]
async fn given_client_when_connecting_then_succeeds() {
    // Given
    let server = create_test_server();

    // When
    let mut client = WsTestClient::connect(
        &server, "tenant-1", "user-1", TEST_JWT_SECRET
    ).await;

    // Then - connection succeeded (no panic)
    client.close().await;
}
```

**Note**: Protobuf message helpers (subscribe, ping, etc.) will be added in Session 30 when those message handlers are implemented.

### 1.3 JWT Helper (`jwt_helper.rs`)

**Purpose**: Generate valid/invalid JWT tokens for testing.

**Key Functions**:
```rust
pub struct JwtTestHelper {
    secret: Vec<u8>,
}

impl JwtTestHelper {
    pub fn new(secret: &[u8]) -> Self;

    /// Generate valid JWT for tenant/user
    pub fn create_token(&self, tenant_id: &str, user_id: &str) -> String;

    /// Generate token with custom expiration
    pub fn create_token_with_expiry(
        &self,
        tenant_id: &str,
        user_id: &str,
        expires_in: Duration,
    ) -> String;

    /// Generate expired token
    pub fn create_expired_token(&self, tenant_id: &str, user_id: &str) -> String;

    /// Generate token with empty tenant_id (invalid)
    pub fn create_invalid_tenant_token(&self, user_id: &str) -> String;

    /// Generate token signed with wrong secret
    pub fn create_token_wrong_secret(&self, tenant_id: &str, user_id: &str) -> String;
}
```

---

## Phase 2: Connection & Authentication Tests

### `connection_tests.rs` (2 tests)

| Test Name | Description |
|-----------|-------------|
| `given_valid_jwt_when_connecting_then_succeeds` | Smoke test - basic successful connection |
| `given_connected_client_when_closed_then_server_cleans_up` | Resource leak prevention - verify registry cleanup (app_state.rs:100) |

**Removed**:
- ❌ `given_connected_client_when_server_shuts_down_then_client_notified` - Testing framework shutdown behavior
- ❌ `given_client_when_connection_dropped_then_server_detects` - Testing tokio::select! framework behavior

### `jwt_auth_tests.rs` (4 tests)

| Test Name | Description |
|-----------|-------------|
| `given_expired_token_when_connecting_then_returns_401` | Security boundary - expired tokens rejected |
| `given_token_with_wrong_signature_when_connecting_then_returns_401` | Security boundary - invalid signatures rejected |
| `given_missing_authorization_header_when_connecting_then_returns_401` | Security boundary - auth required |
| `given_token_with_empty_tenant_id_when_connecting_then_returns_401` | Tenant isolation - tenant_id required |

---

## Phase 3: Broadcast Tests (Multi-Client)

### `broadcast_tests.rs` (5 tests)

| Test Name | Description |
|-----------|-------------|
| `given_two_clients_same_tenant_when_broadcast_then_both_receive` | Core feature - broadcast delivery (tests TenantBroadcaster) |
| `given_tenant_with_multiple_users_when_broadcast_then_all_users_receive` | Same-tenant multi-user works |
| `given_fast_broadcasts_when_channel_full_then_handles_backpressure` | Production resilience - tests RecvError::Lagged handling |
| `given_broadcast_when_client_disconnects_mid_receive_then_continues` | Error resilience - other clients unaffected |
| `given_multiple_tenants_when_broadcast_to_one_then_others_unaffected` | Tenant isolation - CRITICAL security boundary |

**Removed**:
- ❌ `given_subscribed_client_when_event_for_subscribed_project_then_receives` - DEFERRED to Session 30 (subscription filtering not implemented)
- ❌ `given_subscribed_client_when_event_for_other_project_then_does_not_receive` - DEFERRED to Session 30 (subscription filtering not implemented)

---

## Phase 4: Connection Limits Tests

### `connection_limits_tests.rs` (4 tests)

| Test Name | Description |
|-----------|-------------|
| `given_tenant_at_limit_when_new_connection_then_rejected_503` | Per-tenant limit enforcement |
| `given_server_at_total_limit_when_new_connection_then_rejected_503` | Total limit enforcement |
| `given_tenant_at_limit_when_other_tenant_connects_then_succeeds` | Limits are per-tenant, not global |
| `given_tenant_at_limit_when_one_disconnects_then_new_can_connect` | Limit released on disconnect |

---

## Phase 5: Rate Limiting Tests

### `rate_limit_tests.rs` (2 tests)

| Test Name | Description |
|-----------|-------------|
| `given_client_when_sending_rapidly_then_rate_limited_and_disconnects` | Rate limit enforcement causes connection close (tests ConnectionRateLimiter integration) |
| `given_rate_limited_connection_when_reconnecting_then_new_limiter_allows_requests` | Each connection gets fresh rate limiter (per-connection, not per-user) |

---

## Phase 6: Tenant Isolation Tests

### `tenant_isolation_tests.rs` (1 test)

| Test Name | Description |
|-----------|-------------|
| `given_tenant_channel_when_all_clients_disconnect_then_channel_cleaned_up` | Resource cleanup - tests TenantBroadcaster channel removal (tenant_broadcaster.rs:76-79) |

**Note**: The critical tenant isolation broadcast test is in `broadcast_tests.rs` (Phase 3).

---

## Test Summary

| Category | Test Count | What's Tested |
|----------|-----------|---------------|
| Connection Lifecycle | 2 | WebSocket upgrade, registry cleanup |
| JWT Authentication | 4 | Token validation, security boundaries |
| Broadcast (Multi-Client) | 5 | TenantBroadcaster routing, backpressure, multi-tenant isolation |
| Connection Limits | 4 | ConnectionRegistry limit enforcement |
| Rate Limiting | 2 | ConnectionRateLimiter integration |
| Tenant Isolation | 1 | Channel cleanup on disconnect |
| **Total** | **18** | All tests validate custom code, not framework behavior |

### What Was Cut from Original Plan?

**8 tests DEFERRED to Session 30** (subscription feature not implemented):
- ❌ All subscription management tests (4 tests)
- ❌ Subscription filtering tests (2 tests in broadcast)
- ❌ Subscription idempotency tests (2 tests)

**5 tests REMOVED entirely** (testing framework, not custom logic):
- ❌ Ping/pong protocol test (trivial framework behavior)
- ❌ Invalid protobuf test (validation not implemented yet)
- ❌ Graceful shutdown tests (2 tests - framework behavior, can't test with axum-test)
- ❌ Connection drop detection (testing tokio::select!, not custom logic)

### Why These 18 Tests Matter

Each remaining test validates **custom business logic**:
- **JWT integration** - Not just the library, but how it's wired into the Axum handler
- **ConnectionRegistry** - Complex multi-tenant limit enforcement with cleanup
- **TenantBroadcaster** - Custom HashMap routing with channel lifecycle management
- **Rate limiting** - Integration of rate limiter into message handling loop
- **Backpressure** - Custom handling of RecvError::Lagged
- **Security** - Multi-tenant data isolation (CRITICAL)

---

## Dependencies

Add to `pm-ws/Cargo.toml` under `[dev-dependencies]`:

```toml
[dev-dependencies]
# Testing framework
axum-test = { version = "16", features = ["ws"] }  # WebSocket testing support
googletest = "0.14.2"                               # Expressive assertions

# Async runtime for tests
tokio = { version = "1.49.0", features = ["full", "test-util"] }

# For Sink/Stream unit testing pattern
futures-channel = "0.3"
futures-util = "0.3"

# For real WebSocket client tests (alternative to axum-test)
tokio-tungstenite = "0.26"

# Utilities
tempfile = "3.24.0"
uuid = { version = "1.19.0", features = ["v4"] }
```

**Note**: `axum-test` requires Axum v0.8.7+. The project uses Axum 0.8.8, so this is compatible.

---

## Success Criteria

1. **All 18 integration tests pass**: `cargo test --workspace`
2. **Tests use Given/When/Then naming**: Consistent with Session 15 patterns
3. **Tests use googletest assertions**: `assert_that!`, `eq()`, `ok()`, `err()`
4. **Tests are independent**: No shared state between tests
5. **Tests are fast**: All tests complete in < 20 seconds total
6. **Multi-tenant isolation verified**: No cross-tenant data leakage
7. **Connection limits enforced**: Registry correctly rejects over-limit connections
8. **Rate limiting verified**: ConnectionRateLimiter integrated correctly

---

## Implementation Order

The developer should implement in this order:

1. **Test Infrastructure** (Phase 1) - Critical foundation
   - `test_server.rs` - TestServer configuration
   - `test_client.rs` - WebSocket test client wrapper
   - `jwt_helper.rs` - Token generation

2. **Connection & Auth** (Phase 2) - Core functionality
   - `connection_tests.rs` (2 tests)
   - `jwt_auth_tests.rs` (4 tests)

3. **Broadcast** (Phase 3) - Main features
   - `broadcast_tests.rs` (5 tests)

4. **Limits & Rate Limiting** (Phases 4-5)
   - `connection_limits_tests.rs` (4 tests)
   - `rate_limit_tests.rs` (2 tests)

5. **Cleanup** (Phase 6)
   - `tenant_isolation_tests.rs` (1 test)

---

## Notes for Developer

### Primary Approach: axum-test

Use `axum-test` for most tests - it provides the cleanest API:

```rust
use axum_test::TestServer;

#[tokio::test]
async fn test_websocket_connection() {
    // Server setup is simple
    let server = create_test_server();

    // WebSocket connection is clean
    let mut ws = server
        .get("/ws")
        .add_header("Authorization", format!("Bearer {}", token))
        .into_websocket()
        .await;

    // Send/receive is straightforward
    ws.send_binary(encoded_protobuf).await;
    let response = ws.receive_binary().await;
}
```

### Fallback: tokio-tungstenite

For tests that need lower-level control (e.g., testing HTTP upgrade rejection, raw frame handling), use `tokio-tungstenite` directly:

```rust
use tokio_tungstenite::connect_async;
use http::Request;

// Build request with custom headers
let request = Request::builder()
    .uri("ws://127.0.0.1:3000/ws")
    .header("Authorization", format!("Bearer {}", token))
    .header("Upgrade", "websocket")
    .header("Connection", "Upgrade")
    .header("Sec-WebSocket-Key", tungstenite::handshake::client::generate_key())
    .header("Sec-WebSocket-Version", "13")
    .body(())
    .unwrap();

// This returns Result, letting us test failure cases
let result = connect_async(request).await;
assert!(result.is_err()); // For testing auth rejection
```

### Test Isolation

Each test should:
- Create its own `TestServer` instance (axum-test handles lifecycle)
- Use unique tenant/user IDs (use `uuid::Uuid::new_v4()`)
- Not share state with other tests

```rust
#[tokio::test]
async fn test_example() {
    let server = create_test_server();  // Fresh server per test
    let tenant_id = uuid::Uuid::new_v4().to_string();  // Unique tenant
    let user_id = uuid::Uuid::new_v4().to_string();    // Unique user

    let mut client = WsTestClient::connect(
        &server, &tenant_id, &user_id, TEST_JWT_SECRET
    ).await;
    // ...
}
```

### Timeouts

`axum-test` handles timeouts internally, but for custom assertions:

```rust
use tokio::time::{timeout, Duration};

let result = timeout(Duration::from_secs(5), client.receive()).await;
assert!(result.is_ok(), "Timed out waiting for response");
```

### Logging in Tests

Enable logging for debugging failed tests:

```rust
// In tests/common/mod.rs
use std::sync::Once;

static INIT: Once = Once::new();

pub fn init_logging() {
    INIT.call_once(|| {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .is_test(true)
            .try_init();
    });
}

// In each test file
#[tokio::test]
async fn test_example() {
    init_logging();  // Safe to call multiple times
    // ...
}
```

### Multi-Client Test Pattern

For testing broadcast scenarios with multiple clients:

```rust
#[tokio::test]
async fn given_two_clients_when_broadcast_then_both_receive() {
    // Given
    let server = create_test_server();
    let tenant_id = uuid::Uuid::new_v4().to_string();

    let mut client1 = WsTestClient::connect(
        &server, &tenant_id, "user-1", TEST_JWT_SECRET
    ).await;
    let mut client2 = WsTestClient::connect(
        &server, &tenant_id, "user-2", TEST_JWT_SECRET
    ).await;

    // Both subscribe to same project
    client1.subscribe(vec!["project-1".to_string()], vec![]).await;
    client2.subscribe(vec!["project-1".to_string()], vec![]).await;

    // Consume subscription confirmations
    let _ = client1.receive().await;
    let _ = client2.receive().await;

    // When - trigger a broadcast (requires server-side method or another client action)
    // ...

    // Then - both clients should receive
    let msg1 = client1.receive().await;
    let msg2 = client2.receive().await;
    assert_eq!(msg1.message_id, msg2.message_id);
}
```

---

## Relationship to Session 30

Session 30 will add:
1. **Work item handlers** that process `CreateWorkItemRequest`, `UpdateWorkItemRequest`, etc.
2. **Subscribe/Unsubscribe message handlers** - The 8 deferred subscription tests will be added in Session 30
3. **Subscription filtering logic** - Currently all broadcasts are forwarded; Session 30 will add project/sprint filtering

Session 25's broadcast tests create the foundation for verifying that work item events are properly delivered to subscribed clients.

The test infrastructure created here (`TestServer`, `TestClient`) will be **reused** in Sessions 35, 45, 55 for testing work items, sprints, comments, and time entries via WebSocket.
