use crate::Config;
use crate::tests::{EnvGuard, setup_config_dir};

use googletest::assert_that;
use googletest::prelude::{anything, err, ok};
use serial_test::serial;

// =========================================================================
// Validation Tests - Circuit Breaker
// =========================================================================

#[test]
#[serial]
fn given_failure_threshold_zero_when_validate_then_error() {
    // Given
    let _temp = setup_config_dir();
    let _threshold = EnvGuard::set("PM_CB_FAILURE_THRESHOLD", "0");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
}

#[test]
#[serial]
fn given_failure_threshold_over_max_when_validate_then_error() {
    // Given
    let _temp = setup_config_dir();
    let _threshold = EnvGuard::set("PM_CB_FAILURE_THRESHOLD", "101");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
}

#[test]
#[serial]
fn given_open_duration_zero_when_validate_then_error() {
    // Given
    let _temp = setup_config_dir();
    let _duration = EnvGuard::set("PM_CB_OPEN_DURATION_SECS", "0");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
}

#[test]
#[serial]
fn given_open_duration_over_max_when_validate_then_error() {
    // Given
    let _temp = setup_config_dir();
    let _duration = EnvGuard::set("PM_CB_OPEN_DURATION_SECS", "301");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
}

#[test]
#[serial]
fn given_valid_circuit_breaker_config_when_validate_then_ok() {
    // Given
    let _temp = setup_config_dir();
    let _threshold = EnvGuard::set("PM_CB_FAILURE_THRESHOLD", "10");
    let _duration = EnvGuard::set("PM_CB_OPEN_DURATION_SECS", "60");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, ok(anything()));
}
