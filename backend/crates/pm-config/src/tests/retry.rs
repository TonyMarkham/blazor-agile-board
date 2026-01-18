use crate::Config;
use crate::tests::{EnvGuard, setup_config_dir};

use googletest::assert_that;
use googletest::prelude::{anything, err, ok};
use serial_test::serial;

// =========================================================================
// Validation Tests - Retry
// =========================================================================

#[test]
#[serial]
fn given_max_attempts_zero_when_validate_then_error() {
    // Given
    let (_temp, _guard) = setup_config_dir();
    let _attempts = EnvGuard::set("PM_RETRY_MAX_ATTEMPTS", "0");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
}

#[test]
#[serial]
fn given_max_attempts_over_max_when_validate_then_error() {
    // Given
    let (_temp, _guard) = setup_config_dir();
    let _attempts = EnvGuard::set("PM_RETRY_MAX_ATTEMPTS", "11");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
}

#[test]
#[serial]
fn given_initial_delay_below_min_when_validate_then_error() {
    // Given
    let (_temp, _guard) = setup_config_dir();
    let _delay = EnvGuard::set("PM_RETRY_INITIAL_DELAY_MS", "5");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
}

#[test]
#[serial]
fn given_backoff_multiplier_below_min_when_validate_then_error() {
    // Given
    let (_temp, _guard) = setup_config_dir();
    let _multiplier = EnvGuard::set("PM_RETRY_BACKOFF_MULTIPLIER", "0.5");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
}

#[test]
#[serial]
fn given_backoff_multiplier_over_max_when_validate_then_error() {
    // Given
    let (_temp, _guard) = setup_config_dir();
    let _multiplier = EnvGuard::set("PM_RETRY_BACKOFF_MULTIPLIER", "11.0");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
}

#[test]
#[serial]
fn given_valid_retry_config_when_validate_then_ok() {
    // Given
    let (_temp, _guard) = setup_config_dir();
    let _attempts = EnvGuard::set("PM_RETRY_MAX_ATTEMPTS", "5");
    let _delay = EnvGuard::set("PM_RETRY_INITIAL_DELAY_MS", "200");
    let _multiplier = EnvGuard::set("PM_RETRY_BACKOFF_MULTIPLIER", "3.0");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, ok(anything()));
}
