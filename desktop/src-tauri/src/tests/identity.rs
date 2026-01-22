//! Unit tests for identity module.
//!
//! These tests can access crate internals via `use crate::`.

use crate::identity::{error::IdentityError, user_identity::UserIdentity};

use std::path::PathBuf;

// =============================================================================
// IdentityError Tests
// =============================================================================

#[test]
fn given_valid_data_when_serialize_roundtrip_then_preserves_all_fields() {
    let original = UserIdentity {
        id: uuid::Uuid::new_v4(),
        name: Some("Test User".into()),
        email: Some("test@example.com".into()),
        created_at: "2024-01-01T00:00:00Z".into(),
        schema_version: 1,
    };

    let json = serde_json::to_string(&original).unwrap();
    let restored: UserIdentity = serde_json::from_str(&json).unwrap();

    assert_eq!(original.id, restored.id);
    assert_eq!(original.name, restored.name);
    assert_eq!(original.email, restored.email);
    assert_eq!(original.created_at, restored.created_at);
    assert_eq!(original.schema_version, restored.schema_version);
}

#[test]
fn given_missing_optional_fields_when_deserialize_then_defaults_to_none() {
    let json = r#"{"id":"550e8400-e29b-41d4-a716-446655440000","created_at":"2024-01-01T00:00:00Z","schema_version":1}"#;
    let user: UserIdentity = serde_json::from_str(json).unwrap();

    assert!(user.name.is_none());
    assert!(user.email.is_none());
}

#[test]
fn given_all_fields_when_serialize_then_produces_valid_json() {
    let user = UserIdentity {
        id: uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
        name: Some("Alice".into()),
        email: Some("alice@example.com".into()),
        created_at: "2024-01-01T00:00:00Z".into(),
        schema_version: 1,
    };

    let json = serde_json::to_string_pretty(&user).unwrap();

    assert!(json.contains("550e8400-e29b-41d4-a716-446655440000"));
    assert!(json.contains("Alice"));
    assert!(json.contains("alice@example.com"));
    assert!(json.contains("schema_version"));
}

#[test]
fn given_file_read_error_when_is_transient_then_returns_true() {
    let err = IdentityError::file_read(
        PathBuf::from("/test"),
        std::io::Error::new(std::io::ErrorKind::Other, "test"),
    );
    assert!(err.is_transient());
}

#[test]
fn given_corrupted_error_when_is_transient_then_returns_false() {
    let err = IdentityError::corrupted(PathBuf::from("/test"), "bad json");
    assert!(!err.is_transient());
}

#[test]
fn given_any_error_when_recovery_hint_then_returns_non_empty_string() {
    let errors = vec![
        IdentityError::app_data_dir("test"),
        IdentityError::corrupted(PathBuf::from("/test"), "bad"),
        IdentityError::file_read(
            PathBuf::from("/test"),
            std::io::Error::new(std::io::ErrorKind::Other, "test"),
        ),
    ];

    for err in errors {
        let hint = err.recovery_hint();
        assert!(
            !hint.is_empty(),
            "recovery_hint should not be empty for {err:?}"
        );
    }
}

#[test]
fn given_serialization_error_when_from_serde_json_then_converts() {
    let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
    let identity_err: IdentityError = json_err.into();

    match identity_err {
        IdentityError::Serialization { .. } => {
            // Correct variant
        }
        _ => panic!("Expected Serialization variant"),
    }
}

#[test]
fn given_file_write_error_when_is_transient_then_returns_true() {
    let err = IdentityError::file_write(
        PathBuf::from("/test"),
        std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied"),
    );
    assert!(err.is_transient());
}

#[test]
fn given_atomic_rename_error_when_is_transient_then_returns_true() {
    let err = IdentityError::atomic_rename(
        PathBuf::from("/from"),
        PathBuf::from("/to"),
        std::io::Error::new(std::io::ErrorKind::Other, "test"),
    );
    assert!(err.is_transient());
}

#[test]
fn given_app_data_dir_error_when_is_transient_then_returns_false() {
    let err = IdentityError::app_data_dir("no directory");
    assert!(!err.is_transient());
}
