#![allow(dead_code)]

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};

/// JWT claims matching production format (from pm-auth crate)
/// JWT claims matching production format (from pm-auth crate)
#[derive(Debug, Serialize, Deserialize)]
pub struct TestJwtClaims {
    pub sub: String, // user_id
    pub exp: u64,    // Expiration timestamp
    pub iat: u64,    // Issued at timestamp
}

/// Create a valid JWT token for testing
pub fn create_test_token(user_id: &str, jwt_secret: &[u8]) -> String {
    create_test_token_with_expiry(user_id, jwt_secret, Duration::from_secs(3600))
}

/// Create JWT token with custom expiration duration
pub fn create_test_token_with_expiry(
    user_id: &str,
    jwt_secret: &[u8],
    expires_in: Duration,
) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    let claims = TestJwtClaims {
        sub: user_id.to_string(),
        exp: now + expires_in.as_secs(),
        iat: now,
    };

    encode(
        &Header::default(), // HS256 by default
        &claims,
        &EncodingKey::from_secret(jwt_secret),
    )
    .expect("Failed to encode JWT")
}

/// Create an expired JWT token (for auth rejection tests)
pub fn create_expired_token(user_id: &str, jwt_secret: &[u8]) -> String {
    let past = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
        - 3600; // Expired 1 hour ago

    let claims = TestJwtClaims {
        sub: user_id.to_string(),
        exp: past,
        iat: past - 3600,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret),
    )
    .expect("Failed to encode JWT")
}

/// Create JWT token with malformed structure (for robustness testing)
pub fn create_malformed_token() -> String {
    "not.a.valid.jwt.token".to_string()
}
