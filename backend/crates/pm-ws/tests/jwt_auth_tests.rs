mod common;

use common::{
    create_test_server, TEST_JWT_SECRET, create_expired_token,
    create_token_wrong_secret, create_token_empty_tenant,
};

#[tokio::test]
async fn given_expired_token_when_connecting_then_returns_401() {
    // Given
    let server = create_test_server();
    let expired_token = create_expired_token("tenant-1", "user-1", TEST_JWT_SECRET);

    // When
    let response = server.server
        .get_websocket("/ws")
        .add_header("Authorization", format!("Bearer {}", expired_token))
        .await;

    // Then
    response.assert_status_unauthorized();
}

#[tokio::test]
async fn given_token_with_wrong_signature_when_connecting_then_returns_401() {
    // Given
    let server = create_test_server();
    let invalid_token = create_token_wrong_secret("tenant-1", "user-1");

    // When
    let response = server.server
        .get_websocket("/ws")
        .add_header("Authorization", format!("Bearer {}", invalid_token))
        .await;

    // Then
    response.assert_status_unauthorized();
}

#[tokio::test]
async fn given_missing_authorization_header_when_connecting_then_returns_401() {
    // Given
    let server = create_test_server();

    // When - No Authorization header
    let response = server.server
        .get_websocket("/ws")
        .await;

    // Then
    response.assert_status_unauthorized();
}

#[tokio::test]
async fn given_token_with_empty_tenant_id_when_connecting_then_returns_401() {
    // Given
    let server = create_test_server();
    let invalid_token = create_token_empty_tenant("user-1", TEST_JWT_SECRET);

    // When
    let response = server.server
        .get_websocket("/ws")
        .add_header("Authorization", format!("Bearer {}", invalid_token))
        .await;

    // Then
    response.assert_status_unauthorized();
}