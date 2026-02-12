use crate::MessageValidator;

use pm_config::ValidationConfig;

#[test]
fn given_valid_subscription_when_validated_then_succeeds() {
    let result = MessageValidator::validate_subscribe("project-123", "project");
    assert!(result.is_ok());
}

#[test]
fn given_empty_project_id_when_validated_then_fails() {
    let result = MessageValidator::validate_subscribe("", "project");
    assert!(result.is_err());
}

#[test]
fn given_invalid_resource_type_when_validated_then_fails() {
    let result = MessageValidator::validate_subscribe("project-123", "invalid");
    assert!(result.is_err());
}

#[test]
fn given_valid_uuid_when_validated_then_succeeds() {
    let result = MessageValidator::validate_uuid("550e8400-e29b-41d4-a716-446655440000", "id");
    assert!(result.is_ok());
}

#[test]
fn given_invalid_uuid_when_validated_then_fails() {
    let result = MessageValidator::validate_uuid("not-a-uuid", "id");
    assert!(result.is_err());
}

#[test]
fn given_valid_string_when_validated_then_succeeds() {
    let result = MessageValidator::validate_string("hello", "field", 1, 10);
    assert!(result.is_ok());
}

#[test]
fn given_too_short_string_when_validated_then_fails() {
    let result = MessageValidator::validate_string("", "field", 1, 10);
    assert!(result.is_err());
}

#[test]
fn given_too_long_string_when_validated_then_fails() {
    let result = MessageValidator::validate_string("hello world", "field", 1, 5);
    assert!(result.is_err());
}

#[test]
fn given_valid_work_item_when_validated_then_succeeds() {
    let result = MessageValidator::validate_work_item_create(
        "Test Task",
        Some("Description"),
        "task",
        &ValidationConfig::default(),
    );
    assert!(result.is_ok());
}

#[test]
fn given_invalid_item_type_when_validated_then_fails() {
    let result = MessageValidator::validate_work_item_create(
        "Test",
        None,
        "invalid",
        &ValidationConfig::default(),
    );
    assert!(result.is_err());
}

#[test]
fn given_valid_comment_when_validated_then_succeeds() {
    let result =
        MessageValidator::validate_comment_create("Good work!", &ValidationConfig::default());
    assert!(result.is_ok());
}

#[test]
fn given_empty_comment_when_validated_then_fails() {
    let result = MessageValidator::validate_comment_create("", &ValidationConfig::default());
    assert!(result.is_err());
}

#[test]
fn given_valid_sprint_dates_when_validated_then_succeeds() {
    let now = chrono::Utc::now().timestamp();
    let start = now + 86400; // tomorrow
    let end = now + (14 * 86400); // two weeks later

    let result = MessageValidator::validate_sprint_create(
        "Sprint 1",
        start,
        end,
        &ValidationConfig::default(),
    );
    assert!(result.is_ok());
}

#[test]
fn given_end_before_start_when_validated_then_fails() {
    let now = chrono::Utc::now().timestamp();
    let start = now + (14 * 86400);
    let end = now + 86400;

    let result = MessageValidator::validate_sprint_create(
        "Sprint 1",
        start,
        end,
        &ValidationConfig::default(),
    );
    assert!(result.is_err());
}

#[test]
fn given_valid_pagination_when_validated_then_succeeds() {
    let result = MessageValidator::validate_pagination(50, 0);
    assert!(result.is_ok());
}

#[test]
fn given_zero_limit_when_validated_then_fails() {
    let result = MessageValidator::validate_pagination(0, 0);
    assert!(result.is_err());
}

#[test]
fn given_excessive_limit_when_validated_then_fails() {
    let result = MessageValidator::validate_pagination(2000, 0);
    assert!(result.is_err());
}

#[test]
fn given_emoji_title_within_char_limit_when_validated_then_succeeds() {
    let mut config = ValidationConfig::default();
    config.max_title_length = 10;
    let title: String = std::iter::repeat('🔥').take(10).collect();
    let result = MessageValidator::validate_work_item_create(&title, None, "task", &config);
    assert!(result.is_ok());
}

#[test]
fn given_emoji_title_over_char_limit_when_validated_then_fails() {
    let mut config = ValidationConfig::default();
    config.max_title_length = 10;
    let title: String = std::iter::repeat('🔥').take(11).collect();
    let result = MessageValidator::validate_work_item_create(&title, None, "task", &config);
    assert!(result.is_err());
}

#[test]
fn given_cjk_description_within_char_limit_when_validated_then_succeeds() {
    // CJK characters are 3 bytes each in UTF-8
    let mut config = ValidationConfig::default();
    config.max_description_length = 10;
    let desc: String = std::iter::repeat('日').take(10).collect(); // 10 chars, 30 bytes
    assert_eq!(desc.chars().count(), 10);
    assert_eq!(desc.len(), 30);
    let result = MessageValidator::validate_work_item_create("title", Some(&desc), "task", &config);
    assert!(result.is_ok());
}

#[test]
fn given_cjk_description_over_char_limit_when_validated_then_fails() {
    let mut config = ValidationConfig::default();
    config.max_description_length = 10;
    let desc: String = std::iter::repeat('日').take(11).collect(); // 11 chars, 33 bytes
    let result = MessageValidator::validate_work_item_create("title", Some(&desc), "task", &config);
    assert!(result.is_err());
}
