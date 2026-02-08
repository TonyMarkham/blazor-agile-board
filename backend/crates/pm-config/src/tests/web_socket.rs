use crate::Config;
use crate::tests::{EnvGuard, setup_config_dir};

use googletest::assert_that;
use googletest::prelude::{anything, err};
use serial_test::serial;

// =========================================================================
// Validation Tests - WebSocket
// =========================================================================

#[test]
#[serial]
fn given_timeout_less_than_interval_when_validate_then_error() {
    // Given
    let _temp = setup_config_dir();
    let _interval = EnvGuard::set("PM_WS_HEARTBEAT_INTERVAL_SECS", "60");
    let _timeout = EnvGuard::set("PM_WS_HEARTBEAT_TIMEOUT_SECS", "30");

    // When
    let config = Config::load().unwrap();
    let result = config.validate();

    // Then
    assert_that!(result, err(anything()));
}
