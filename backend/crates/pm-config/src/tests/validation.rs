use crate::Config;
use crate::tests::{EnvGuard, setup_config_dir};

use googletest::assert_that;
use googletest::prelude::{anything, err, ok};
use serial_test::serial;

// =========================================================================
// Validation Tests - Validation Config
// =========================================================================

#[test]
#[serial]
fn given_max_title_length_zero_when_validate_then_error() {
    // Given
    let (_temp, _guard) = setup_config_dir();
    let _length = EnvGuard::set("PM_VALIDATION_MAX_TITLE_LENGTH", "0");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
}

#[test]
#[serial]
fn given_max_title_length_over_max_when_validate_then_error() {
    // Given
    let (_temp, _guard) = setup_config_dir();
    let _length = EnvGuard::set("PM_VALIDATION_MAX_TITLE_LENGTH", "501");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
}

#[test]
#[serial]
fn given_max_story_points_negative_when_validate_then_error() {
    // Given
    let (_temp, _guard) = setup_config_dir();
    let _points = EnvGuard::set("PM_VALIDATION_MAX_STORY_POINTS", "-1");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
}

#[test]
#[serial]
fn given_max_story_points_over_max_when_validate_then_error() {
    // Given
    let (_temp, _guard) = setup_config_dir();
    let _points = EnvGuard::set("PM_VALIDATION_MAX_STORY_POINTS", "1001");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
}

#[test]
#[serial]
fn given_max_error_message_length_below_min_when_validate_then_error() {
    // Given
    let (_temp, _guard) = setup_config_dir();
    let _length = EnvGuard::set("PM_VALIDATION_MAX_ERROR_MESSAGE_LENGTH", "49");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
}

#[test]
#[serial]
fn given_valid_validation_config_when_validate_then_ok() {
    // Given
    let (_temp, _guard) = setup_config_dir();
    let _title = EnvGuard::set("PM_VALIDATION_MAX_TITLE_LENGTH", "300");
    let _desc = EnvGuard::set("PM_VALIDATION_MAX_DESCRIPTION_LENGTH", "50000");
    let _points = EnvGuard::set("PM_VALIDATION_MAX_STORY_POINTS", "200");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, ok(anything()));
}
