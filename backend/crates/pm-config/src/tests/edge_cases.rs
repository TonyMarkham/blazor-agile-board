use crate::Config;
use crate::tests::{EnvGuard, setup_config_dir};

use googletest::assert_that;
use googletest::prelude::{anything, contains_substring, err};
use serial_test::serial;

// =========================================================================
// Edge Cases
// =========================================================================

#[test]
#[serial]
fn given_malformed_toml_when_load_then_error_mentions_file() {
    // Given
    let temp = setup_config_dir();
    std::env::set_current_dir(temp.path()).unwrap();
    std::fs::write(
        temp.path().join(".pm/config.toml"),
        "this is not valid toml {{{{",
    )
    .unwrap();

    // When
    let result = Config::load();

    // Then
    assert_that!(result, err(anything()));
    let err_msg = format!("{}", result.unwrap_err());
    assert_that!(err_msg, contains_substring("config.toml"));
}

#[test]
#[serial]
fn given_database_path_with_traversal_when_validate_then_error() {
    // Given
    let _temp = setup_config_dir();
    let _path = EnvGuard::set("PM_DATABASE_PATH", "../../../etc/passwd");

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
fn given_absolute_database_path_when_validate_then_error() {
    // Given
    let _temp = setup_config_dir();
    let _path = EnvGuard::set("PM_DATABASE_PATH", "/tmp/data.db");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
}
