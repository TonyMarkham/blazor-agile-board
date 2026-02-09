use crate::Config;
use crate::tests::{EnvGuard, setup_config_dir};

use googletest::assert_that;
use googletest::prelude::{anything, eq, ok};
use serial_test::serial;

// =========================================================================
// Happy Path Tests
// =========================================================================

#[test]
#[serial]
fn given_no_config_file_when_load_then_ok_with_defaults() {
    // Given
    let _temp = setup_config_dir();

    // When
    let result = Config::load();

    // Then
    assert_that!(result, ok(anything()));
    let config = result.unwrap();
    assert_that!(config.server.port, eq(crate::DEFAULT_PORT));
    assert_that!(
        config.server.max_connections,
        eq(crate::DEFAULT_MAX_CONNECTIONS)
    );
    assert_that!(config.auth.enabled, eq(false));
}

#[test]
#[serial]
fn given_no_config_file_when_load_and_validate_then_ok() {
    // Given
    let _temp = setup_config_dir();

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, ok(anything()));
}

#[test]
#[serial]
fn given_valid_toml_file_when_load_then_ok_and_uses_toml_values() {
    // Given
    let temp = setup_config_dir();
    std::env::set_current_dir(temp.path()).unwrap();
    std::fs::write(
        temp.path().join(".pm/config.toml"),
        r#"
              [server]
              port = 9000
              max_connections = 5000

              [auth]
              enabled = false
          "#,
    )
    .unwrap();

    // When
    let result = Config::load();

    // Then
    assert_that!(result, ok(anything()));
    let config = result.unwrap();
    assert_that!(config.server.port, eq(9000));
    assert_that!(config.server.max_connections, eq(5000));
}

#[test]
#[serial]
fn given_env_var_and_toml_when_load_then_env_var_overrides_toml() {
    // Given
    let temp = setup_config_dir();
    std::env::set_current_dir(temp.path()).unwrap();
    std::fs::write(temp.path().join(".pm/config.toml"), "[server]\nport = 9000").unwrap();
    let _port_guard = EnvGuard::set("PM_SERVER_PORT", "8888");

    // When
    let config = Config::load().unwrap();

    // Then
    assert_that!(config.server.port, eq(8888));
}

#[test]
#[serial]
fn given_multiple_env_overrides_when_load_then_all_apply() {
    // Given
    let _temp = setup_config_dir();
    let _port = EnvGuard::set("PM_SERVER_PORT", "7777");
    let _host = EnvGuard::set("PM_SERVER_HOST", "0.0.0.0");
    let _max = EnvGuard::set("PM_SERVER_MAX_CONNECTIONS", "2000");
    let _colored = EnvGuard::set("PM_LOG_COLORED", "false");

    // When
    let config = Config::load().unwrap();

    // Then
    assert_that!(config.server.port, eq(7777));
    assert_that!(config.server.host.as_str(), eq("0.0.0.0"));
    assert_that!(config.server.max_connections, eq(2000));
    assert_that!(config.logging.colored, eq(false));
}
