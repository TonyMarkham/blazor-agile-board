use crate::{
    PortFileInfo,
    tests::{EnvGuard, setup_config_dir},
};

use googletest::assert_that;
use googletest::prelude::{anything, eq, err, none, ok};
use serial_test::serial;

const TEST_HOST: &str = "127.0.0.1";
const TEST_PORT: u16 = 8080;
const ALT_TEST_PORT: u16 = 9000;

// =========================================================================
// Write & Read Tests
// =========================================================================

#[test]
#[serial]
fn given_port_and_host_when_write_then_file_created_with_correct_fields() {
    // Given
    let (_temp, _guard) = setup_config_dir();

    // When
    let path = PortFileInfo::write(TEST_PORT, TEST_HOST).unwrap();

    // Then
    assert!(path.exists());
    let info = PortFileInfo::read().unwrap().unwrap();
    assert_that!(info.port, eq(TEST_PORT));
    assert_that!(info.host, eq(TEST_HOST));
    assert_that!(info.pid, eq(std::process::id()));
    assert!(!info.started_at.is_empty());
    assert!(!info.version.is_empty());
}

#[test]
#[serial]
fn given_no_port_file_when_read_then_returns_none() {
    // Given
    let (_temp, _guard) = setup_config_dir();

    // When
    let result = PortFileInfo::read().unwrap();

    // Then
    assert_that!(result, none());
}

// =========================================================================
// Read Live Tests (PID Liveness)
// =========================================================================

#[test]
#[serial]
fn given_port_file_with_current_pid_when_read_live_then_returns_some() {
    // Given
    let (_temp, _guard) = setup_config_dir();
    PortFileInfo::write(ALT_TEST_PORT, TEST_HOST).unwrap();

    // When - current process PID is definitely alive
    let result = PortFileInfo::read_live().unwrap();

    // Then
    assert!(result.is_some());
    let info = result.unwrap();
    assert_that!(info.port, eq(ALT_TEST_PORT));
}

#[test]
#[serial]
fn given_port_file_with_dead_pid_when_read_live_then_returns_none_and_removes_file() {
    // Given - Write a port file, then overwrite it with a dead PID
    let (temp, _guard) = setup_config_dir();
    PortFileInfo::write(ALT_TEST_PORT, TEST_HOST).unwrap();

    // Overwrite with a PID that's (almost certainly) not running
    let stale_info = serde_json::json!({
        "pid": 999999,
        "port": ALT_TEST_PORT,
        "host": TEST_HOST,
        "started_at": "2026-01-01T00:00:00Z",
        "version": "0.1.0"
    });
    let path = temp.path().join("server.json");
    std::fs::write(&path, serde_json::to_string_pretty(&stale_info).unwrap()).unwrap();
    assert!(path.exists());

    // When
    let result = PortFileInfo::read_live().unwrap();

    // Then
    assert_that!(result, none());
    // Stale file should be removed
    assert!(!path.exists());
}

#[test]
#[serial]
fn given_no_port_file_when_read_live_then_returns_none() {
    // Given
    let (_temp, _guard) = setup_config_dir();

    // When
    let result = PortFileInfo::read_live().unwrap();

    // Then
    assert_that!(result, none());
}

// =========================================================================
// Remove Tests
// =========================================================================

#[test]
#[serial]
fn given_port_file_exists_when_remove_then_file_deleted() {
    // Given
    let (temp, _guard) = setup_config_dir();
    PortFileInfo::write(TEST_PORT, TEST_HOST).unwrap();
    let path = temp.path().join("server.json");
    assert!(path.exists());

    // When
    PortFileInfo::remove().unwrap();

    // Then
    assert!(!path.exists());
}

#[test]
#[serial]
fn given_no_port_file_when_remove_then_succeeds() {
    // Given
    let (_temp, _guard) = setup_config_dir();

    // When
    let result = PortFileInfo::remove();

    // Then
    assert_that!(result, ok(anything()));
}

// =========================================================================
// Race Condition Protection Tests
// =========================================================================

#[test]
#[serial]
fn given_live_server_when_write_then_error() {
    // Given - a port file exists with the current (live) PID
    let (_temp, _guard) = setup_config_dir();
    PortFileInfo::write(TEST_PORT, TEST_HOST).unwrap();

    // When - try to write another port file while server is "running"
    let result = PortFileInfo::write(9090, "0.0.0.0");

    // Then - should error because existing server is still running
    assert_that!(result, err(anything()));
}

#[test]
#[serial]
fn given_stale_server_when_write_then_overwrites() {
    // Given - a port file with a dead PID
    let (temp, _guard) = setup_config_dir();
    let stale_info = serde_json::json!({
        "pid": 999999,
        "port": TEST_PORT,
        "host": TEST_HOST,
        "started_at": "2026-01-01T00:00:00Z",
        "version": "0.1.0"
    });
    let path = temp.path().join("server.json");
    std::fs::write(&path, serde_json::to_string_pretty(&stale_info).unwrap()).unwrap();

    // When - write a new port file (stale should be auto-cleaned by read_live)
    let result = PortFileInfo::write(9090, "0.0.0.0");

    // Then - should succeed (stale file was cleaned before write)
    assert_that!(result, ok(anything()));
    let info = PortFileInfo::read().unwrap().unwrap();
    assert_that!(info.port, eq(9090));
    assert_that!(info.host, eq("0.0.0.0"));
}

// =========================================================================
// Edge Case Tests
// =========================================================================

#[test]
#[serial]
fn given_directory_not_exist_when_write_then_creates_directory() {
    // Given - PM_CONFIG_DIR points to a non-existent nested directory
    let temp = tempfile::TempDir::new().unwrap();
    let nested = temp.path().join("nested").join("config");
    let _guard = EnvGuard::set("PM_CONFIG_DIR", nested.to_str().unwrap());
    assert!(!nested.exists());

    // When
    let result = PortFileInfo::write(TEST_PORT, TEST_HOST);

    // Then
    assert_that!(result, ok(anything()));
    assert!(nested.exists());
    assert!(nested.join("server.json").exists());
}

#[test]
#[serial]
fn given_malformed_json_when_read_then_error() {
    // Given - a port file with invalid JSON
    let (temp, _guard) = setup_config_dir();
    let path = temp.path().join("server.json");
    std::fs::write(&path, "{ invalid json").unwrap();

    // When
    let result = PortFileInfo::read();

    // Then
    assert_that!(result, err(anything()));
}
