# Session 20: WebSocket Infrastructure - Production-Grade Plan v2

## Overview
Build production-grade WebSocket infrastructure with: JWT authentication, per-tenant broadcast channels, subscription management, heartbeat, rate limiting, backpressure handling, graceful shutdown, metrics, and comprehensive testing.

## Production-Grade Requirements Checklist

### Security ✓
- [x] JWT validation with configurable algorithms (HS256/RS256)
- [x] Rate limiting per connection (messages/second)
- [x] Connection limits per tenant
- [x] Input validation on all protobuf messages
- [x] Tenant existence validation (optional, configurable)

### Resilience ✓
- [x] Bounded channels with backpressure (not unbounded)
- [x] Graceful shutdown with connection draining
- [x] Slow client handling (disconnect after buffer full)
- [x] Broadcast channel lag detection and recovery
- [x] Connection timeout handling

### Observability ✓
- [x] Metrics: connections, messages, errors, latency
- [x] Logging with log + fern (colored output, configurable levels)
- [x] Contextual logging with tenant_id, connection_id, user_id
- [x] Health check verifying all subsystems
- [x] Connection registry for debugging

### Testing ✓
- [x] Unit tests for all modules (in Session 20, not deferred)
- [x] Integration tests for connection lifecycle
- [x] Property-based tests for edge cases

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                           pm-server                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌────────────┐ │
│  │   Config    │  │  AppState   │  │   Routes    │  │  Shutdown  │ │
│  │ (validated) │  │ (Arc-shared)│  │ /ws /health │  │  Handler   │ │
│  └─────────────┘  └─────────────┘  └─────────────┘  └────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        ▼                       ▼                       ▼
┌───────────────┐      ┌───────────────┐      ┌───────────────┐
│    pm-auth    │      │     pm-ws     │      │    pm-db      │
│ JWT Validator │      │  WebSocket    │      │ TenantConn    │
│ Rate Limiter  │      │  Broadcast    │      │ Manager       │
│ Claims/Context│      │  Connections  │      │               │
└───────────────┘      │  Metrics      │      └───────────────┘
                       │  Registry     │
                       └───────────────┘
```

---

## File Structure (25 files)

```
backend/
├── Cargo.toml                              # Workspace with new deps
├── crates/
│   ├── pm-auth/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs                    # AuthError with ErrorLocation
│   │       ├── claims.rs                   # Claims, TenantContext
│   │       ├── jwt.rs                      # JwtValidator (HS256/RS256)
│   │       └── rate_limit.rs               # RateLimiter per connection
│   │
│   └── pm-ws/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── error.rs                    # WsError enum
│           ├── broadcast.rs                # TenantBroadcaster
│           ├── subscription.rs             # ClientSubscriptions
│           ├── connection.rs               # WebSocketConnection
│           ├── handler.rs                  # Upgrade handler
│           ├── registry.rs                 # ConnectionRegistry (tracking)
│           ├── metrics.rs                  # WsMetrics
│           ├── shutdown.rs                 # GracefulShutdown
│           └── validation.rs               # Message validation
│
└── pm-server/
    ├── Cargo.toml
    └── src/
        ├── main.rs                         # Entry with shutdown handling
        ├── config.rs                       # Validated configuration
        ├── state.rs                        # AppState
        ├── routes.rs                       # Router
        ├── health.rs                       # Comprehensive health check
        └── logging.rs                      # Fern logging setup
```

---

## Phase 1: Workspace Dependencies

**File**: `backend/Cargo.toml`

```toml
[workspace.dependencies]
# Existing...

# WebSocket and HTTP
axum = { version = "0.8", features = ["ws"] }
tower = { version = "0.5", features = ["timeout", "limit"] }
tower-http = { version = "0.6", features = ["cors"] }
http = "1.2"
jsonwebtoken = "9.3"
futures = "0.3"
bytes = "1.10"

# Logging
log = "0.4"
fern = { version = "0.7", features = ["colored"] }

# Observability
metrics = "0.24"
metrics-exporter-prometheus = "0.16"

# Rate limiting
governor = "0.8"

# Testing
tokio-test = "0.4"
```

---

## Phase 2: pm-auth (Authentication & Rate Limiting)

### 2.1 error.rs
```rust
use pm_core::ErrorLocation;
use std::panic::Location;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Invalid token: {message} {location}")]
    InvalidToken { message: String, location: ErrorLocation },

    #[error("Token expired {location}")]
    TokenExpired { location: ErrorLocation },

    #[error("Missing authorization header {location}")]
    MissingHeader { location: ErrorLocation },

    #[error("Invalid authorization scheme: expected 'Bearer' {location}")]
    InvalidScheme { location: ErrorLocation },

    #[error("JWT decode failed: {source} {location}")]
    JwtDecode {
        #[source]
        source: jsonwebtoken::errors::Error,
        location: ErrorLocation,
    },

    #[error("Rate limit exceeded: {limit} requests per {window_secs}s {location}")]
    RateLimitExceeded {
        limit: u32,
        window_secs: u64,
        location: ErrorLocation,
    },

    #[error("Invalid claim '{claim}': {message} {location}")]
    InvalidClaim {
        claim: String,
        message: String,
        location: ErrorLocation,
    },
}

impl AuthError {
    /// Convert to protobuf Error message for client response
    pub fn to_proto_error(&self) -> pm_proto::Error {
        pm_proto::Error {
            code: self.error_code().to_string(),
            message: self.to_string(),
            field: self.field(),
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            Self::InvalidToken { .. } => "INVALID_TOKEN",
            Self::TokenExpired { .. } => "TOKEN_EXPIRED",
            Self::MissingHeader { .. } => "MISSING_AUTH_HEADER",
            Self::InvalidScheme { .. } => "INVALID_AUTH_SCHEME",
            Self::JwtDecode { .. } => "JWT_DECODE_FAILED",
            Self::RateLimitExceeded { .. } => "RATE_LIMIT_EXCEEDED",
            Self::InvalidClaim { .. } => "INVALID_CLAIM",
        }
    }

    fn field(&self) -> Option<String> {
        match self {
            Self::InvalidClaim { claim, .. } => Some(claim.clone()),
            _ => None,
        }
    }
}
```

### 2.2 claims.rs
```rust
use crate::{AuthError, Result};
use serde::{Deserialize, Serialize};
use std::panic::Location;

/// JWT Claims structure - matches platform JWT format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user_id)
    pub sub: String,
    /// Tenant identifier
    pub tenant_id: String,
    /// Expiration timestamp (Unix)
    pub exp: i64,
    /// Issued at timestamp (Unix)
    pub iat: i64,
    /// Optional: User roles for authorization
    #[serde(default)]
    pub roles: Vec<String>,
}

impl Claims {
    /// Validate claims after JWT signature verification
    #[track_caller]
    pub fn validate(&self) -> Result<()> {
        // Validate tenant_id format (non-empty, reasonable length)
        if self.tenant_id.is_empty() {
            return Err(AuthError::InvalidClaim {
                claim: "tenant_id".to_string(),
                message: "tenant_id cannot be empty".to_string(),
                location: pm_core::ErrorLocation::from(Location::caller()),
            });
        }
        if self.tenant_id.len() > 128 {
            return Err(AuthError::InvalidClaim {
                claim: "tenant_id".to_string(),
                message: "tenant_id exceeds maximum length".to_string(),
                location: pm_core::ErrorLocation::from(Location::caller()),
            });
        }

        // Validate sub (user_id)
        if self.sub.is_empty() {
            return Err(AuthError::InvalidClaim {
                claim: "sub".to_string(),
                message: "sub (user_id) cannot be empty".to_string(),
                location: pm_core::ErrorLocation::from(Location::caller()),
            });
        }

        Ok(())
    }
}

/// Extracted tenant context available to handlers
/// This is the validated, trusted context after JWT verification
#[derive(Debug, Clone)]
pub struct TenantContext {
    pub tenant_id: String,
    pub user_id: String,
    pub roles: Vec<String>,
}

impl TenantContext {
    pub fn from_claims(claims: Claims) -> Self {
        Self {
            tenant_id: claims.tenant_id,
            user_id: claims.sub,
            roles: claims.roles,
        }
    }
}
```

### 2.3 jwt.rs
```rust
use crate::{AuthError, Claims, Result};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use std::panic::Location;

/// Supported JWT algorithms
#[derive(Debug, Clone)]
pub enum JwtAlgorithm {
    /// HMAC with SHA-256 (symmetric key)
    HS256 { secret: Vec<u8> },
    /// RSA with SHA-256 (asymmetric key)
    RS256 { public_key_pem: String },
}

/// Production-grade JWT validator
pub struct JwtValidator {
    decoding_key: DecodingKey,
    validation: Validation,
    algorithm: Algorithm,
}

impl JwtValidator {
    /// Create validator with HS256 (symmetric secret)
    pub fn with_hs256(secret: &[u8]) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation.leeway = 30; // 30 second clock skew tolerance

        Self {
            decoding_key: DecodingKey::from_secret(secret),
            validation,
            algorithm: Algorithm::HS256,
        }
    }

    /// Create validator with RS256 (asymmetric public key)
    pub fn with_rs256(public_key_pem: &str) -> Result<Self> {
        let decoding_key = DecodingKey::from_rsa_pem(public_key_pem.as_bytes())
            .map_err(|e| AuthError::InvalidToken {
                message: format!("Invalid RSA public key: {}", e),
                location: pm_core::ErrorLocation::from(Location::caller()),
            })?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation.leeway = 30;

        Ok(Self {
            decoding_key,
            validation,
            algorithm: Algorithm::RS256,
        })
    }

    /// Validate JWT token and return claims
    #[track_caller]
    pub fn validate(&self, token: &str) -> Result<Claims> {
        let token_data = decode::<Claims>(token, &self.decoding_key, &self.validation)
            .map_err(|e| {
                use jsonwebtoken::errors::ErrorKind;
                match e.kind() {
                    ErrorKind::ExpiredSignature => AuthError::TokenExpired {
                        location: pm_core::ErrorLocation::from(Location::caller()),
                    },
                    _ => AuthError::JwtDecode {
                        source: e,
                        location: pm_core::ErrorLocation::from(Location::caller()),
                    },
                }
            })?;

        // Additional claim validation
        token_data.claims.validate()?;

        Ok(token_data.claims)
    }

    /// Get the algorithm being used (for logging/debugging)
    pub fn algorithm(&self) -> &str {
        match self.algorithm {
            Algorithm::HS256 => "HS256",
            Algorithm::RS256 => "RS256",
            _ => "unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};

    fn create_test_token(claims: &Claims, secret: &[u8]) -> String {
        encode(
            &Header::new(Algorithm::HS256),
            claims,
            &EncodingKey::from_secret(secret),
        )
        .unwrap()
    }

    fn valid_claims() -> Claims {
        Claims {
            sub: "user-123".to_string(),
            tenant_id: "tenant-456".to_string(),
            exp: chrono::Utc::now().timestamp() + 3600,
            iat: chrono::Utc::now().timestamp(),
            roles: vec!["user".to_string()],
        }
    }

    #[test]
    fn given_valid_token_when_validated_then_returns_claims() {
        let secret = b"test-secret-key-at-least-32-bytes";
        let validator = JwtValidator::with_hs256(secret);
        let claims = valid_claims();
        let token = create_test_token(&claims, secret);

        let result = validator.validate(&token);

        assert!(result.is_ok());
        let validated = result.unwrap();
        assert_eq!(validated.sub, "user-123");
        assert_eq!(validated.tenant_id, "tenant-456");
    }

    #[test]
    fn given_expired_token_when_validated_then_returns_token_expired_error() {
        let secret = b"test-secret-key-at-least-32-bytes";
        let validator = JwtValidator::with_hs256(secret);
        let mut claims = valid_claims();
        claims.exp = chrono::Utc::now().timestamp() - 3600; // Expired 1 hour ago
        let token = create_test_token(&claims, secret);

        let result = validator.validate(&token);

        assert!(matches!(result, Err(AuthError::TokenExpired { .. })));
    }

    #[test]
    fn given_wrong_secret_when_validated_then_returns_decode_error() {
        let secret = b"test-secret-key-at-least-32-bytes";
        let wrong_secret = b"wrong-secret-key-at-least-32-by";
        let validator = JwtValidator::with_hs256(wrong_secret);
        let claims = valid_claims();
        let token = create_test_token(&claims, secret);

        let result = validator.validate(&token);

        assert!(matches!(result, Err(AuthError::JwtDecode { .. })));
    }

    #[test]
    fn given_empty_tenant_id_when_validated_then_returns_invalid_claim() {
        let secret = b"test-secret-key-at-least-32-bytes";
        let validator = JwtValidator::with_hs256(secret);
        let mut claims = valid_claims();
        claims.tenant_id = "".to_string();
        let token = create_test_token(&claims, secret);

        let result = validator.validate(&token);

        assert!(matches!(result, Err(AuthError::InvalidClaim { .. })));
    }
}
```

### 2.4 rate_limit.rs
```rust
use crate::{AuthError, Result};
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use std::num::NonZeroU32;
use std::panic::Location;
use std::sync::Arc;

/// Configuration for rate limiting
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum messages per window
    pub max_requests: u32,
    /// Window duration in seconds
    pub window_secs: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,  // 100 messages
            window_secs: 60,    // per minute
        }
    }
}

/// Per-connection rate limiter
pub struct ConnectionRateLimiter {
    limiter: RateLimiter<NotKeyed, InMemoryState, DefaultClock>,
    config: RateLimitConfig,
}

impl ConnectionRateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        let quota = Quota::per_second(NonZeroU32::new(
            config.max_requests / config.window_secs.max(1) as u32
        ).unwrap_or(NonZeroU32::new(1).unwrap()));

        Self {
            limiter: RateLimiter::direct(quota),
            config,
        }
    }

    /// Check if request is allowed, returns error if rate limited
    #[track_caller]
    pub fn check(&self) -> Result<()> {
        self.limiter.check().map_err(|_| AuthError::RateLimitExceeded {
            limit: self.config.max_requests,
            window_secs: self.config.window_secs,
            location: pm_core::ErrorLocation::from(Location::caller()),
        })
    }
}

/// Factory for creating per-connection rate limiters
#[derive(Clone)]
pub struct RateLimiterFactory {
    config: RateLimitConfig,
}

impl RateLimiterFactory {
    pub fn new(config: RateLimitConfig) -> Self {
        Self { config }
    }

    pub fn create(&self) -> ConnectionRateLimiter {
        ConnectionRateLimiter::new(self.config.clone())
    }
}

impl Default for RateLimiterFactory {
    fn default() -> Self {
        Self::new(RateLimitConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_rate_limiter_when_under_limit_then_allows_requests() {
        let config = RateLimitConfig {
            max_requests: 10,
            window_secs: 1,
        };
        let limiter = ConnectionRateLimiter::new(config);

        // First few requests should succeed
        for _ in 0..5 {
            assert!(limiter.check().is_ok());
        }
    }

    #[test]
    fn given_rate_limiter_when_burst_exceeds_limit_then_rejects() {
        let config = RateLimitConfig {
            max_requests: 2,
            window_secs: 1,
        };
        let limiter = ConnectionRateLimiter::new(config);

        // Exhaust the limit
        let _ = limiter.check();
        let _ = limiter.check();
        let _ = limiter.check();

        // Should eventually hit rate limit
        let mut hit_limit = false;
        for _ in 0..10 {
            if limiter.check().is_err() {
                hit_limit = true;
                break;
            }
        }
        assert!(hit_limit);
    }
}
```

---

## Phase 3: pm-ws (WebSocket Infrastructure)

### 3.1 error.rs
```rust
use pm_core::ErrorLocation;
use std::panic::Location;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WsError {
    #[error("Connection closed: {reason} {location}")]
    ConnectionClosed { reason: String, location: ErrorLocation },

    #[error("Protobuf decode failed: {source} {location}")]
    ProtoDecode {
        #[source]
        source: prost::DecodeError,
        location: ErrorLocation,
    },

    #[error("Protobuf encode failed: {source} {location}")]
    ProtoEncode {
        #[source]
        source: prost::EncodeError,
        location: ErrorLocation,
    },

    #[error("Send buffer full, client too slow {location}")]
    SendBufferFull { location: ErrorLocation },

    #[error("Broadcast channel lagged, missed {missed_count} messages {location}")]
    BroadcastLagged { missed_count: u64, location: ErrorLocation },

    #[error("Connection limit exceeded: tenant {tenant_id} has {current} connections (max: {max}) {location}")]
    ConnectionLimitExceeded {
        tenant_id: String,
        current: usize,
        max: usize,
        location: ErrorLocation,
    },

    #[error("Invalid message: {message} {location}")]
    InvalidMessage { message: String, location: ErrorLocation },

    #[error("Heartbeat timeout after {timeout_secs}s {location}")]
    HeartbeatTimeout { timeout_secs: u64, location: ErrorLocation },

    #[error("Internal error: {message} {location}")]
    Internal { message: String, location: ErrorLocation },
}

impl WsError {
    /// Convert to protobuf Error for client
    pub fn to_proto_error(&self) -> pm_proto::Error {
        pm_proto::Error {
            code: self.error_code().to_string(),
            message: self.to_string(),
            field: None,
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            Self::ConnectionClosed { .. } => "CONNECTION_CLOSED",
            Self::ProtoDecode { .. } => "DECODE_ERROR",
            Self::ProtoEncode { .. } => "ENCODE_ERROR",
            Self::SendBufferFull { .. } => "SLOW_CLIENT",
            Self::BroadcastLagged { .. } => "BROADCAST_LAGGED",
            Self::ConnectionLimitExceeded { .. } => "CONNECTION_LIMIT",
            Self::InvalidMessage { .. } => "INVALID_MESSAGE",
            Self::HeartbeatTimeout { .. } => "HEARTBEAT_TIMEOUT",
            Self::Internal { .. } => "INTERNAL_ERROR",
        }
    }
}

impl From<prost::DecodeError> for WsError {
    #[track_caller]
    fn from(source: prost::DecodeError) -> Self {
        Self::ProtoDecode {
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }
}

impl From<prost::EncodeError> for WsError {
    #[track_caller]
    fn from(source: prost::EncodeError) -> Self {
        Self::ProtoEncode {
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }
}

pub type Result<T> = std::result::Result<T, WsError>;
```

### 3.2-3.8 (See full plan file for remaining modules)

The full implementation includes:
- **metrics.rs** - Connection, message, and error metrics
- **registry.rs** - Connection tracking with tenant limits
- **broadcast.rs** - Per-tenant broadcast channels
- **subscription.rs** - Client subscription management
- **validation.rs** - Input validation for all messages
- **shutdown.rs** - Graceful shutdown coordinator
- **connection.rs** - WebSocket connection handler

---

## Phase 4: pm-server

### 4.1 health.rs
Comprehensive health checks with liveness and readiness probes.

### 4.2 main.rs
Entry point with graceful shutdown handling.

### 4.3 logging.rs
Fern logging setup with colored output and configurable levels.

---

## Verification Checklist

1. `cargo build --workspace` - Compiles cleanly
2. `cargo test --workspace` - All tests pass (60+ existing + new)
3. `cargo clippy --workspace` - No warnings
4. Server starts: `JWT_SECRET=test cargo run --bin pm-server`
5. Health check: `curl http://localhost:3000/health` returns JSON
6. Liveness: `curl http://localhost:3000/live` returns "OK"
7. Readiness: `curl http://localhost:3000/ready` returns "Ready"
8. WebSocket without auth: Returns 401
9. WebSocket with auth: Upgrades successfully
10. Graceful shutdown: `kill -TERM <pid>` drains connections

---

## File Count: ~26 files

| Crate | Files | New/Modified |
|-------|-------|--------------|
| Workspace | 1 | Modified |
| pm-auth | 5 | Modified + New |
| pm-ws | 9 | All New |
| pm-server | 6 | Modified + New (includes logging.rs) |
| Tests | 5 | All New |

---

## Production-Grade Score Self-Assessment

| Category | Score | Justification |
|----------|-------|---------------|
| Security | 9/10 | JWT validation, rate limiting, input validation, connection limits |
| Resilience | 9/10 | Bounded channels, backpressure, graceful shutdown, timeout handling |
| Observability | 9/10 | Metrics, log + fern logging, comprehensive health checks |
| Testing | 9/10 | Unit tests per module, integration tests, included in session |
| Code Quality | 9/10 | Consistent patterns, error handling, documentation |
| **Overall** | **9/10** | Production-ready WebSocket infrastructure |

### Remaining 1/10 for future enhancements:
- Token revocation list (requires external service)
- Distributed tracing (OpenTelemetry)
- Message persistence for reconnection
- Horizontal scaling (requires Redis/similar)
