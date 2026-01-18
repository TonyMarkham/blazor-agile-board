use crate::Config;
use crate::tests::{EnvGuard, setup_config_dir};

use googletest::assert_that;
use googletest::prelude::{anything, contains_substring, err, ok};
use serial_test::serial;

// =========================================================================
// Validation Tests - Auth
// =========================================================================

#[test]
#[serial]
fn given_auth_enabled_but_no_jwt_config_when_validate_then_error() {
    // Given
    let (_temp, _guard) = setup_config_dir();
    let _enabled = EnvGuard::set("PM_AUTH_ENABLED", "true");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
    let err_msg = format!("{}", result.unwrap_err());
    assert_that!(err_msg, contains_substring("jwt_secret"));
}

#[test]
#[serial]
fn given_jwt_secret_too_short_when_validate_then_error_mentions_32_chars() {
    // Given
    let (_temp, _guard) = setup_config_dir();
    let _enabled = EnvGuard::set("PM_AUTH_ENABLED", "true");
    let _secret = EnvGuard::set("PM_AUTH_JWT_SECRET", "tooshort");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
    let err_msg = format!("{}", result.unwrap_err());
    assert_that!(err_msg, contains_substring("32 characters"));
}

#[test]
#[serial]
fn given_jwt_secret_exactly_32_chars_when_validate_then_ok() {
    // Given
    let (_temp, _guard) = setup_config_dir();
    let _enabled = EnvGuard::set("PM_AUTH_ENABLED", "true");
    let _secret = EnvGuard::set("PM_AUTH_JWT_SECRET", "12345678901234567890123456789012"); // 32 chars

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, ok(anything()));
}

#[test]
#[serial]
fn given_jwt_secret_over_32_chars_when_validate_then_ok() {
    // Given
    let (_temp, _guard) = setup_config_dir();
    let _enabled = EnvGuard::set("PM_AUTH_ENABLED", "true");
    let _secret = EnvGuard::set(
        "PM_AUTH_JWT_SECRET",
        "this-is-a-very-long-secret-key-for-testing-purposes",
    );

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, ok(anything()));
}

#[test]
#[serial]
fn given_absolute_jwt_key_path_when_validate_then_error_mentions_relative() {
    // Given
    let (temp, _guard) = setup_config_dir();
    std::fs::write(
        temp.path().join("config.toml"),
        r#"
              [auth]
              enabled = true
              jwt_public_key_path = "/etc/passwd"
          "#,
    )
    .unwrap();

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
    let err_msg = format!("{}", result.unwrap_err());
    assert_that!(err_msg, contains_substring("relative"));
}

#[test]
#[serial]
fn given_path_traversal_in_jwt_key_path_when_validate_then_error() {
    // Given
    let (temp, _guard) = setup_config_dir();
    std::fs::write(
        temp.path().join("config.toml"),
        r#"
              [auth]
              enabled = true
              jwt_public_key_path = "../../../etc/passwd"
          "#,
    )
    .unwrap();

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
    let err_msg = format!("{}", result.unwrap_err());
    assert_that!(err_msg, contains_substring(".."));
}

#[test]
#[serial]
fn given_nonexistent_jwt_key_file_when_validate_then_error_mentions_path() {
    // Given
    let (temp, _guard) = setup_config_dir();
    std::fs::write(
        temp.path().join("config.toml"),
        r#"
              [auth]
              enabled = true
              jwt_public_key_path = "nonexistent.pem"
          "#,
    )
    .unwrap();

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
    let err_msg = format!("{}", result.unwrap_err());
    assert_that!(err_msg, contains_substring("does not exist"));
    assert_that!(err_msg, contains_substring("nonexistent.pem"));
}
