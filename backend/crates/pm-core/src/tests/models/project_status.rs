use crate::ProjectStatus;

use std::str::FromStr;

#[test]
fn test_project_status_as_str() {
    assert_eq!(ProjectStatus::Active.as_str(), "active");
    assert_eq!(ProjectStatus::Archived.as_str(), "archived");
}

#[test]
fn test_project_status_from_str() {
    assert_eq!(
        ProjectStatus::from_str("active").unwrap(),
        ProjectStatus::Active
    );
    assert_eq!(
        ProjectStatus::from_str("archived").unwrap(),
        ProjectStatus::Archived
    );
    assert!(ProjectStatus::from_str("invalid").is_err());
}

#[test]
fn test_project_status_default() {
    assert_eq!(ProjectStatus::default(), ProjectStatus::Active);
}
