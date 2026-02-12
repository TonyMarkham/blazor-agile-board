use crate::Config;
use crate::tests::{EnvGuard, setup_config_dir};

use crate::validation_config::{
    MAX_CONFIGURABLE_COMMENT_LENGTH, MAX_CONFIGURABLE_SPRINT_NAME_LENGTH, MAX_DESCRIPTION_LENGTH,
    MAX_ERROR_MESSAGE_LENGTH, MAX_STORY_POINTS, MAX_TITLE_LENGTH, MIN_CONFIGURABLE_COMMENT_LENGTH,
    MIN_CONFIGURABLE_SPRINT_NAME_LENGTH, MIN_ERROR_MESSAGE_LENGTH, MIN_STORY_POINTS,
    MIN_TITLE_LENGTH,
};
use googletest::assert_that;
use googletest::prelude::{anything, err, ok};
use serial_test::serial;

const BELOW_MIN_TITLE: usize = MIN_TITLE_LENGTH - 1;
const ABOVE_MAX_TITLE: usize = MAX_TITLE_LENGTH + 1;
const ABOVE_MAX_DESCRIPTION: usize = MAX_DESCRIPTION_LENGTH + 1;
const BELOW_MIN_STORY_POINTS: i32 = MIN_STORY_POINTS - 1;
const ABOVE_MAX_STORY_POINTS: i32 = MAX_STORY_POINTS + 1;
const BELOW_MIN_ERROR_MSG: usize = MIN_ERROR_MESSAGE_LENGTH - 1;
const BELOW_MIN_COMMENT: usize = MIN_CONFIGURABLE_COMMENT_LENGTH - 1;
const ABOVE_MAX_COMMENT: usize = MAX_CONFIGURABLE_COMMENT_LENGTH + 1;
const BELOW_MIN_SPRINT_NAME: usize = MIN_CONFIGURABLE_SPRINT_NAME_LENGTH - 1;
const ABOVE_MAX_SPRINT_NAME: usize = MAX_CONFIGURABLE_SPRINT_NAME_LENGTH + 1;
const VALID_TITLE_LENGTH: usize = (MIN_TITLE_LENGTH + MAX_TITLE_LENGTH) / 2;
const VALID_DESCRIPTION_LENGTH: usize = MAX_DESCRIPTION_LENGTH / 2;
const VALID_STORY_POINTS: i32 = (MIN_STORY_POINTS + MAX_STORY_POINTS) / 2;
const VALID_ERROR_MSG_LENGTH: usize = (MIN_ERROR_MESSAGE_LENGTH + MAX_ERROR_MESSAGE_LENGTH) / 2;
const VALID_COMMENT_LENGTH: usize =
    (MIN_CONFIGURABLE_COMMENT_LENGTH + MAX_CONFIGURABLE_COMMENT_LENGTH) / 2;
const VALID_SPRINT_NAME_LENGTH: usize =
    (MIN_CONFIGURABLE_SPRINT_NAME_LENGTH + MAX_CONFIGURABLE_SPRINT_NAME_LENGTH) / 2;

// =========================================================================
// Validation Tests - Validation Config
// =========================================================================

#[test]
#[serial]
fn given_max_title_length_zero_when_validate_then_error() {
    // Given
    let _temp = setup_config_dir();
    let _length = EnvGuard::set(
        "PM_VALIDATION_MAX_TITLE_LENGTH",
        &BELOW_MIN_TITLE.to_string(),
    );

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
    let _temp = setup_config_dir();
    let _length = EnvGuard::set(
        "PM_VALIDATION_MAX_TITLE_LENGTH",
        &ABOVE_MAX_TITLE.to_string(),
    );

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
    let _temp = setup_config_dir();
    let _points = EnvGuard::set(
        "PM_VALIDATION_MAX_STORY_POINTS",
        &BELOW_MIN_STORY_POINTS.to_string(),
    );

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
    let _temp = setup_config_dir();
    let _points = EnvGuard::set(
        "PM_VALIDATION_MAX_STORY_POINTS",
        &ABOVE_MAX_STORY_POINTS.to_string(),
    );

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
    let _temp = setup_config_dir();
    let _length = EnvGuard::set(
        "PM_VALIDATION_MAX_ERROR_MESSAGE_LENGTH",
        &BELOW_MIN_ERROR_MSG.to_string(),
    );

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
    let _temp = setup_config_dir();
    let _title = EnvGuard::set(
        "PM_VALIDATION_MAX_TITLE_LENGTH",
        &VALID_TITLE_LENGTH.to_string(),
    );
    let _desc = EnvGuard::set(
        "PM_VALIDATION_MAX_DESCRIPTION_LENGTH",
        &VALID_DESCRIPTION_LENGTH.to_string(),
    );
    let _points = EnvGuard::set(
        "PM_VALIDATION_MAX_STORY_POINTS",
        &VALID_STORY_POINTS.to_string(),
    );
    let _error_msg = EnvGuard::set(
        "PM_VALIDATION_MAX_ERROR_MESSAGE_LENGTH",
        &VALID_ERROR_MSG_LENGTH.to_string(),
    );
    let _comment = EnvGuard::set(
        "PM_VALIDATION_MAX_COMMENT_LENGTH",
        &VALID_COMMENT_LENGTH.to_string(),
    );
    let _sprint_name = EnvGuard::set(
        "PM_VALIDATION_MAX_SPRINT_NAME_LENGTH",
        &VALID_SPRINT_NAME_LENGTH.to_string(),
    );

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, ok(anything()));
}

#[test]
#[serial]
fn given_max_comment_length_zero_when_validate_then_error() {
    // Given
    let _temp = setup_config_dir();
    let _length = EnvGuard::set(
        "PM_VALIDATION_MAX_COMMENT_LENGTH",
        &BELOW_MIN_COMMENT.to_string(),
    );

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
}

#[test]
#[serial]
fn given_max_comment_length_over_max_when_validate_then_error() {
    // Given
    let _temp = setup_config_dir();
    let _length = EnvGuard::set(
        "PM_VALIDATION_MAX_COMMENT_LENGTH",
        &ABOVE_MAX_COMMENT.to_string(),
    );

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
}

#[test]
#[serial]
fn given_max_sprint_name_length_zero_when_validate_then_error() {
    // Given
    let _temp = setup_config_dir();
    let _length = EnvGuard::set(
        "PM_VALIDATION_MAX_SPRINT_NAME_LENGTH",
        &BELOW_MIN_SPRINT_NAME.to_string(),
    );

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
}

#[test]
#[serial]
fn given_max_sprint_name_length_over_max_when_validate_then_error() {
    // Given
    let _temp = setup_config_dir();
    let _length = EnvGuard::set(
        "PM_VALIDATION_MAX_SPRINT_NAME_LENGTH",
        &ABOVE_MAX_SPRINT_NAME.to_string(),
    );

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
}

#[test]
#[serial]
fn given_max_description_length_over_max_when_validate_then_error() {
    // Given
    let _temp = setup_config_dir();
    let _length = EnvGuard::set(
        "PM_VALIDATION_MAX_DESCRIPTION_LENGTH",
        &ABOVE_MAX_DESCRIPTION.to_string(),
    );

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
}

#[test]
#[serial]
fn given_non_numeric_max_comment_length_when_load_then_error() {
    // Given
    let _temp = setup_config_dir();
    let _length = EnvGuard::set("PM_VALIDATION_MAX_COMMENT_LENGTH", "abc");

    // When
    let result = Config::load();

    // Then
    // Should fail at deserialization, not validation
    assert!(result.is_err());
}

#[test]
#[serial]
fn given_negative_max_comment_length_when_load_then_error() {
    // Given
    let _temp = setup_config_dir();
    let _length = EnvGuard::set("PM_VALIDATION_MAX_COMMENT_LENGTH", "-1");

    // When
    let result = Config::load();

    // Then
    // usize cannot be negative — should fail at deserialization
    assert!(result.is_err());
}

#[test]
#[serial]
fn given_empty_max_sprint_name_length_when_load_then_error() {
    // Given
    let _temp = setup_config_dir();
    let _length = EnvGuard::set("PM_VALIDATION_MAX_SPRINT_NAME_LENGTH", "");

    // When
    let result = Config::load();

    // Then
    assert!(result.is_err());
}
