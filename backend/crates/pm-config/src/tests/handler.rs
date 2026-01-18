use crate::Config;
use crate::tests::{EnvGuard, setup_config_dir};

use googletest::assert_that;
use googletest::prelude::{anything, err, ok};
use serial_test::serial;

// =========================================================================
// Validation Tests - Handler
// =========================================================================

#[test]
#[serial]
fn given_timeout_zero_when_validate_then_error() {
    // Given
    let (_temp, _guard) = setup_config_dir();
    let _timeout = EnvGuard::set("PM_HANDLER_TIMEOUT_SECS", "0");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
}

#[test]
#[serial]
fn given_timeout_over_max_when_validate_then_error() {
    // Given
    let (_temp, _guard) = setup_config_dir();
    let _timeout = EnvGuard::set("PM_HANDLER_TIMEOUT_SECS", "301");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
}

#[test]
#[serial]
fn given_valid_timeout_when_validate_then_ok() {
    // Given
    let (_temp, _guard) = setup_config_dir();
    let _timeout = EnvGuard::set("PM_HANDLER_TIMEOUT_SECS", "60");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, ok(anything()));
}
