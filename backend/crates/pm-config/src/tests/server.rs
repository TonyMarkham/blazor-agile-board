use crate::Config;
use crate::tests::{EnvGuard, setup_config_dir};

use googletest::assert_that;
use googletest::prelude::{anything, err, ok};
use serial_test::serial;

// =========================================================================
// Validation Tests - Server
// =========================================================================

#[test]
#[serial]
fn given_port_below_1024_when_validate_then_error() {
    // Given
    let _temp = setup_config_dir();
    let _port = EnvGuard::set("PM_SERVER_PORT", "80");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
}

#[test]
#[serial]
fn given_port_1024_when_validate_then_ok() {
    // Given
    let _temp = setup_config_dir();
    let _port = EnvGuard::set("PM_SERVER_PORT", "1024");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, ok(anything()));
}

#[test]
#[serial]
fn given_max_connections_zero_when_validate_then_error() {
    // Given
    let _temp = setup_config_dir();
    let _max = EnvGuard::set("PM_SERVER_MAX_CONNECTIONS", "0");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
}

#[test]
#[serial]
fn given_max_connections_over_limit_when_validate_then_error() {
    // Given
    let _temp = setup_config_dir();
    let _max = EnvGuard::set("PM_SERVER_MAX_CONNECTIONS", "200000");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
}

#[test]
#[serial]
fn given_port_zero_when_validate_then_ok() {
    // Given - port 0 means OS auto-assign
    let _temp = setup_config_dir();
    let _port = EnvGuard::set("PM_SERVER_PORT", "0");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, ok(anything()));
}

#[test]
#[serial]
fn given_port_500_nonzero_below_min_when_validate_then_error() {
    // Given - port 500 is below MIN_PORT (1024) and not the special 0
    let _temp = setup_config_dir();
    let _port = EnvGuard::set("PM_SERVER_PORT", "500");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
}
