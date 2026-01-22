//! Unit tests for connection handling.

use crate::WsError;
use crate::handlers::connection::extract_user_id;

use std::collections::HashMap;
use std::env;

use pm_auth::Claims;
use serial_test::serial;
use uuid::Uuid;

const AUTH_ENABLED_KEY: &str = "PM_AUTH_ENABLED";

/// RAII guard for environment variables - automatically restores on drop
struct EnvGuard {
    key: &'static str,
    original: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        unsafe {
            let original = env::var(key).ok();
            env::set_var(key, value);
            Self { key, original }
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            match &self.original {
                Some(val) => env::set_var(self.key, val),
                None => env::remove_var(self.key),
            }
        }
    }
}

// =============================================================================
// Auth Enabled Tests (Web Mode - Security Critical)
// =============================================================================

#[test]
#[serial]
fn given_auth_enabled_and_user_id_param_when_extract_then_unauthorized() {
    let _guard = EnvGuard::set(AUTH_ENABLED_KEY, "true");

    let mut params = HashMap::new();
    params.insert("user_id".into(), Uuid::new_v4().to_string());

    let result = extract_user_id(&params, None);

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), WsError::Unauthorized { .. }));
}

#[test]
#[serial]
fn given_auth_enabled_and_valid_jwt_when_extract_then_returns_uuid() {
    let _guard = EnvGuard::set(AUTH_ENABLED_KEY, "true");

    let user_id = Uuid::new_v4();
    let claims = Claims {
        sub: user_id.to_string(),
        exp: 9999999999,
        iat: 1234567890,
        roles: vec![],
    };

    let params = HashMap::new();
    let result = extract_user_id(&params, Some(&claims));

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), user_id);
}

#[test]
#[serial]
fn given_auth_enabled_and_no_jwt_when_extract_then_unauthorized() {
    let _guard = EnvGuard::set(AUTH_ENABLED_KEY, "true");

    let params = HashMap::new();
    let result = extract_user_id(&params, None);

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), WsError::Unauthorized { .. }));
}

#[test]
#[serial]
fn given_auth_enabled_and_invalid_uuid_in_jwt_when_extract_then_unauthorized() {
    let _guard = EnvGuard::set(AUTH_ENABLED_KEY, "true");

    let claims = Claims {
        sub: "not-a-valid-uuid".into(),
        exp: 9999999999,
        iat: 1234567890,
        roles: vec![],
    };

    let params = HashMap::new();
    let result = extract_user_id(&params, Some(&claims));

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), WsError::Unauthorized { .. }));
}

// =============================================================================
// Auth Disabled Tests (Desktop Mode)
// =============================================================================

#[test]
#[serial]
fn given_auth_disabled_and_valid_user_id_when_extract_then_returns_uuid() {
    let _guard = EnvGuard::set(AUTH_ENABLED_KEY, "false");

    let user_id = Uuid::new_v4();
    let mut params = HashMap::new();
    params.insert("user_id".into(), user_id.to_string());

    let result = extract_user_id(&params, None);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), user_id);
}

#[test]
#[serial]
fn given_auth_disabled_and_no_user_id_when_extract_then_generates_new_uuid() {
    let _guard = EnvGuard::set(AUTH_ENABLED_KEY, "false");

    let params = HashMap::new();
    let result = extract_user_id(&params, None);

    assert!(result.is_ok());
    assert_ne!(result.unwrap(), Uuid::nil());
}

#[test]
#[serial]
fn given_auth_disabled_and_invalid_uuid_when_extract_then_invalid_message_error() {
    let _guard = EnvGuard::set(AUTH_ENABLED_KEY, "false");

    let mut params = HashMap::new();
    params.insert("user_id".into(), "not-a-valid-uuid".into());

    let result = extract_user_id(&params, None);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        WsError::InvalidMessage { .. }
    ));
}

// =============================================================================
// Default Behavior (No Env Var)
// =============================================================================

#[test]
#[serial]
fn given_no_env_var_when_extract_then_defaults_to_auth_disabled() {
    unsafe {
        let _guard = EnvGuard::set(AUTH_ENABLED_KEY, "");
        env::remove_var(AUTH_ENABLED_KEY);

        let user_id = Uuid::new_v4();
        let mut params = HashMap::new();
        params.insert("user_id".into(), user_id.to_string());

        let result = extract_user_id(&params, None);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), user_id);
    }
}
