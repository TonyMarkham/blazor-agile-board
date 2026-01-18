use crate::AuthConfig;

use googletest::assert_that;
use googletest::prelude::{eq, gt, starts_with};

// =========================================================================
// Desktop User ID Tests
// =========================================================================

#[test]
fn given_default_auth_config_when_get_desktop_user_id_then_returns_default() {
    // Given
    let config = AuthConfig::default();

    // When
    let user_id = config.get_desktop_user_id();

    // Then
    assert_that!(user_id.as_str(), eq(crate::DEFAULT_DESKTOP_USER_ID));
}

#[test]
fn given_custom_desktop_user_id_when_get_then_returns_custom() {
    // Given
    let mut config = AuthConfig::default();
    config.desktop_user_id = Some("my-custom-user".to_string());

    // When
    let user_id = config.get_desktop_user_id();

    // Then
    assert_that!(user_id.as_str(), eq("my-custom-user"));
}

#[test]
fn given_empty_desktop_user_id_when_get_then_generates_session_uuid() {
    // Given
    let mut config = AuthConfig::default();
    config.desktop_user_id = Some("".to_string());

    // When
    let user_id = config.get_desktop_user_id();

    // Then
    assert_that!(user_id, starts_with("session-"));
    assert_that!(user_id.len(), gt(20)); // UUID is longer than just "session-"
}

#[test]
fn given_none_desktop_user_id_when_get_then_generates_session_uuid() {
    // Given
    let mut config = AuthConfig::default();
    config.desktop_user_id = None;

    // When
    let user_id = config.get_desktop_user_id();

    // Then
    assert_that!(user_id, starts_with("session-"));
}
