mod common;

use common::{TEST_JWT_SECRET, create_expired_token, create_test_server};

#[tokio::test]
async fn given_expired_token_when_connecting_then_returns_401() {
    // Given
    let server = create_test_server().await;
    let expired_token = create_expired_token("user-1", TEST_JWT_SECRET);

    // When
    let response = server
        .server
        .get_websocket("/ws")
        .add_header("Authorization", format!("Bearer {}", expired_token))
        .await;

    // Then
    response.assert_status_unauthorized();
}

#[tokio::test]
async fn given_token_with_wrong_signature_when_connecting_then_returns_401() {
    // Given
    let server = create_test_server().await;
    // Create token with wrong secret manually
    let wrong_secret = b"wrong-secret-this-will-fail-validation-min-32-bytes";
    let invalid_token = common::jwt_helper::create_test_token("user-1", wrong_secret);

    // When
    let response = server
        .server
        .get_websocket("/ws")
        .add_header("Authorization", format!("Bearer {}", invalid_token))
        .await;

    // Then
    response.assert_status_unauthorized();
}

#[tokio::test]
async fn given_missing_authorization_header_when_connecting_then_returns_401() {
    // Given
    let server = create_test_server().await;

    // When - No Authorization header
    let response = server.server.get_websocket("/ws").await;

    // Then
    response.assert_status_unauthorized();
}
