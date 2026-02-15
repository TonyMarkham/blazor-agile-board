use crate::parse_display_key;

#[test]
fn test_parse_display_key_valid() {
    assert_eq!(parse_display_key("PONE-126").unwrap(), ("PONE", 126));
    assert_eq!(parse_display_key("ABC-1").unwrap(), ("ABC", 1));
    assert_eq!(
        parse_display_key("MYPROJECT-999").unwrap(),
        ("MYPROJECT", 999)
    );
    assert_eq!(parse_display_key("X-1").unwrap(), ("X", 1));
    assert_eq!(
        parse_display_key("ABCDEFGHIJ-1234567890").unwrap(),
        ("ABCDEFGHIJ", 1234567890)
    );
}

#[test]
fn test_parse_display_key_invalid_format() {
    // No hyphen
    assert!(parse_display_key("PONE126").is_err());

    // Multiple hyphens
    assert!(parse_display_key("PONE-126-1").is_err());

    // Empty string
    assert!(parse_display_key("").is_err());
}

#[test]
fn test_parse_display_key_invalid_project_key() {
    // Empty project key
    assert!(parse_display_key("-126").is_err());

    // Too long (>10 chars)
    assert!(parse_display_key("ABCDEFGHIJK-126").is_err());

    // Lowercase letters
    assert!(parse_display_key("pone-126").is_err());

    // Mixed case
    assert!(parse_display_key("Pone-126").is_err());

    // Numbers in key
    assert!(parse_display_key("PONE1-126").is_err());

    // Special characters
    assert!(parse_display_key("PONE_-126").is_err());
}

#[test]
fn test_parse_display_key_invalid_item_number() {
    // Zero
    assert!(parse_display_key("PONE-0").is_err());

    // Negative
    assert!(parse_display_key("PONE--1").is_err());

    // Non-numeric
    assert!(parse_display_key("PONE-abc").is_err());

    // Empty
    assert!(parse_display_key("PONE-").is_err());

    // Decimal
    assert!(parse_display_key("PONE-12.5").is_err());
}
