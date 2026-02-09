use crate::{Config, ConfigError, PortFileInfo, tests::setup_config_dir};

use googletest::assert_that;
use googletest::prelude::{anything, contains_substring, eq, err, none, ok, pat};
use serial_test::serial;
use tempfile::TempDir;

const TEST_HOST: &str = "127.0.0.1";
const TEST_PORT: u16 = 8080;
const ALT_TEST_PORT: u16 = 9000;

// =========================================================================
// Write & Read Tests
// =========================================================================

#[test]
#[serial]
fn given_port_and_host_when_write_then_file_created_with_correct_fields() {
    let _temp = setup_config_dir();
    let config_dir = Config::config_dir_from_git(_temp.path()).unwrap();

    let path = PortFileInfo::write_in(&config_dir, TEST_PORT, TEST_HOST).unwrap();

    assert!(path.exists());
    let info = PortFileInfo::read_in(&config_dir).unwrap().unwrap();
    assert_that!(info.port, eq(TEST_PORT));
    assert_that!(info.host, eq(TEST_HOST));
    assert_that!(info.pid, eq(std::process::id()));
    assert!(!info.started_at.is_empty());
    assert!(!info.version.is_empty());
}

#[test]
#[serial]
fn given_no_port_file_when_read_then_returns_none() {
    let _temp = setup_config_dir();
    let config_dir = Config::config_dir_from_git(_temp.path()).unwrap();

    let result = PortFileInfo::read_in(&config_dir).unwrap();

    assert_that!(result, none());
}

// =========================================================================
// Read Live Tests (PID Liveness)
// =========================================================================

#[test]
#[serial]
fn given_port_file_with_current_pid_when_read_live_then_returns_some() {
    let _temp = setup_config_dir();
    let config_dir = Config::config_dir_from_git(_temp.path()).unwrap();
    PortFileInfo::write_in(&config_dir, ALT_TEST_PORT, TEST_HOST).unwrap();

    let result = PortFileInfo::read_live_in(&config_dir).unwrap();

    assert!(result.is_some());
    let info = result.unwrap();
    assert_that!(info.port, eq(ALT_TEST_PORT));
}

#[test]
#[serial]
fn given_port_file_with_dead_pid_when_read_live_then_returns_none_and_removes_file() {
    let temp = setup_config_dir();
    let config_dir = Config::config_dir_from_git(temp.path()).unwrap();
    PortFileInfo::write_in(&config_dir, ALT_TEST_PORT, TEST_HOST).unwrap();

    let stale_info = serde_json::json!({
        "pid": 999999,
        "port": ALT_TEST_PORT,
        "host": TEST_HOST,
        "started_at": "2026-01-01T00:00:00Z",
        "version": "0.1.0"
    });
    let path = config_dir.join("server.json");
    std::fs::write(&path, serde_json::to_string_pretty(&stale_info).unwrap()).unwrap();
    assert!(path.exists());

    let result = PortFileInfo::read_live_in(&config_dir).unwrap();

    assert_that!(result, none());
    assert!(!path.exists());
}

#[test]
#[serial]
fn given_no_port_file_when_read_live_then_returns_none() {
    let _temp = setup_config_dir();
    let config_dir = Config::config_dir_from_git(_temp.path()).unwrap();

    let result = PortFileInfo::read_live_in(&config_dir).unwrap();

    assert_that!(result, none());
}

// =========================================================================
// Remove Tests
// =========================================================================

#[test]
#[serial]
fn given_port_file_exists_when_remove_then_file_deleted() {
    let temp = setup_config_dir();
    let config_dir = Config::config_dir_from_git(temp.path()).unwrap();
    PortFileInfo::write_in(&config_dir, TEST_PORT, TEST_HOST).unwrap();
    let path = config_dir.join("server.json");
    assert!(path.exists());

    PortFileInfo::remove_in(&config_dir).unwrap();

    assert!(!path.exists());
}

#[test]
#[serial]
fn given_no_port_file_when_remove_then_succeeds() {
    let _temp = setup_config_dir();
    let config_dir = Config::config_dir_from_git(_temp.path()).unwrap();

    let result = PortFileInfo::remove_in(&config_dir);

    assert_that!(result, ok(anything()));
}

// =========================================================================
// Race Condition Protection Tests
// =========================================================================

#[test]
#[serial]
fn given_live_server_when_write_then_error() {
    let _temp = setup_config_dir();
    let config_dir = Config::config_dir_from_git(_temp.path()).unwrap();

    PortFileInfo::write_in(&config_dir, TEST_PORT, TEST_HOST).unwrap();

    let result = PortFileInfo::write_in(&config_dir, 9999, TEST_HOST);

    assert_that!(
        result,
        err(pat!(ConfigError::Generic {
            message: contains_substring("Another pm-server is already running"),
            ..
        }))
    );
}

#[test]
#[serial]
fn given_stale_server_when_write_then_overwrites() {
    let temp = setup_config_dir();
    let config_dir = Config::config_dir_from_git(temp.path()).unwrap();

    let stale_info = serde_json::json!({
        "pid": 999999,
        "port": ALT_TEST_PORT,
        "host": TEST_HOST,
        "started_at": "2026-01-01T00:00:00Z",
        "version": "0.1.0"
    });
    let path = config_dir.join("server.json");
    std::fs::write(&path, serde_json::to_string_pretty(&stale_info).unwrap()).unwrap();

    let result = PortFileInfo::write_in(&config_dir, TEST_PORT, TEST_HOST);

    assert_that!(result, ok(anything()));
    let info = PortFileInfo::read_in(&config_dir).unwrap().unwrap();
    assert_that!(info.port, eq(TEST_PORT));
    assert_that!(info.pid, eq(std::process::id()));
}

// =========================================================================
// Edge Case Tests
// =========================================================================

#[test]
#[serial]
fn given_directory_not_exist_when_write_then_creates_directory() {
    let temp = TempDir::new().unwrap();
    let output = std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .expect("git init failed");
    assert!(output.status.success());

    let config_dir = temp.path().join(".pm");

    let result = PortFileInfo::write_in(&config_dir, TEST_PORT, TEST_HOST);

    assert_that!(result, ok(anything()));
    assert!(config_dir.exists());
}

#[test]
#[serial]
fn given_malformed_json_when_read_then_error() {
    let temp = setup_config_dir();
    let config_dir = Config::config_dir_from_git(temp.path()).unwrap();
    let path = config_dir.join("server.json");
    std::fs::write(&path, "not json").unwrap();

    let result = PortFileInfo::read_in(&config_dir);

    assert_that!(
        result,
        err(pat!(ConfigError::Generic {
            message: contains_substring("Invalid port file"),
            ..
        }))
    );
}
