use crate::{sanitize_string, validate_priority, validate_status};

use proptest::prelude::*;

// =========================================================================
// Property-Based Tests - Sanitization
// =========================================================================

proptest! {
    #[test]
    fn given_valid_status_when_validated_then_succeeds(status in prop_oneof![
        Just("backlog".to_string()),
        Just("todo".to_string()),
        Just("in_progress".to_string()),
        Just("review".to_string()),
        Just("done".to_string()),
        Just("blocked".to_string()),
    ]) {
        prop_assert!(validate_status(&status).is_ok());
    }

    #[test]
    fn given_random_status_when_validated_then_fails(status in "[a-z]{6,20}") {
        if !["backlog", "todo", "in_progress", "review", "done", "blocked"].contains(&status.as_str()) {
            prop_assert!(validate_status(&status).is_err());
        }
    }

    #[test]
    fn given_valid_priority_when_validated_then_succeeds(priority in prop_oneof![
        Just("low".to_string()),
        Just("medium".to_string()),
        Just("high".to_string()),
        Just("critical".to_string()),
    ]) {
        prop_assert!(validate_priority(&priority).is_ok());
    }

    #[test]
    fn given_random_priority_when_validated_then_fails(priority in "[a-z]{5,15}") {
        if !["low", "medium", "high", "critical"].contains(&priority.as_str()) {
            prop_assert!(validate_priority(&priority).is_err());
        }
    }

    #[test]
    fn given_empty_string_when_sanitized_then_empty(input in Just("".to_string())) {
        let sanitized = sanitize_string(&input);
        prop_assert!(sanitized.is_empty());
    }

    #[test]
    fn given_whitespace_only_when_sanitized_then_empty(input in r"\s{1,10}") {
        let sanitized = sanitize_string(&input);
        prop_assert!(sanitized.is_empty());
    }

    #[test]
    fn given_alphanumeric_when_sanitized_then_preserved(input in "[a-zA-Z0-9]{1,50}") {
        let sanitized = sanitize_string(&input);
        prop_assert_eq!(input, sanitized);
    }
}

// =========================================================================
// Unit Tests - Sanitization
// =========================================================================

#[test]
fn given_normal_text_when_sanitized_then_preserved() {
    // Given
    let input = "Hello World 123";

    // When
    let sanitized = sanitize_string(input);

    // Then
    assert_eq!(input, sanitized);
}

#[test]
fn given_whitespace_padded_text_when_sanitized_then_trimmed() {
    // Given
    let input = "  hello  ";

    // When
    let sanitized = sanitize_string(input);

    // Then
    assert_eq!("hello", sanitized);
}

// =========================================================================
// Unit Tests - Status Validation
// =========================================================================

#[test]
fn given_valid_statuses_when_validated_then_all_succeed() {
    assert!(validate_status("backlog").is_ok());
    assert!(validate_status("todo").is_ok());
    assert!(validate_status("in_progress").is_ok());
    assert!(validate_status("review").is_ok());
    assert!(validate_status("done").is_ok());
    assert!(validate_status("blocked").is_ok());
}

#[test]
fn given_invalid_status_when_validated_then_fails() {
    assert!(validate_status("invalid").is_err());
}

#[test]
fn given_empty_status_when_validated_then_fails() {
    assert!(validate_status("").is_err());
}

#[test]
fn given_uppercase_status_when_validated_then_fails() {
    // Status validation is case-sensitive
    assert!(validate_status("TODO").is_err());
}

#[test]
fn given_code_with_special_chars_when_sanitized_then_preserved() {
    // Given - Rust code with characters that were previously mangled
    let input = r#"fn foo(x: &str) -> bool { x < "bar" && x > "baz" }"#;

    // When
    let sanitized = sanitize_string(input);

    // Then - special characters pass through unchanged
    assert_eq!(input, sanitized);
}

// =========================================================================
// Unit Tests - Priority Validation
// =========================================================================

#[test]
fn given_valid_priorities_when_validated_then_all_succeed() {
    assert!(validate_priority("low").is_ok());
    assert!(validate_priority("medium").is_ok());
    assert!(validate_priority("high").is_ok());
    assert!(validate_priority("critical").is_ok());
}

#[test]
fn given_invalid_priority_when_validated_then_fails() {
    assert!(validate_priority("invalid").is_err());
}

#[test]
fn given_empty_priority_when_validated_then_fails() {
    assert!(validate_priority("").is_err());
}

#[test]
fn given_uppercase_priority_when_validated_then_fails() {
    // Priority validation is case-sensitive
    assert!(validate_priority("HIGH").is_err());
}
