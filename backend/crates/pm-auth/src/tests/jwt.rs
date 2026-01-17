use crate::{AuthError, Claims, JwtValidator};

use jsonwebtoken::Algorithm;
use jsonwebtoken::{EncodingKey, Header, encode};

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
