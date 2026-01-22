use crate::{Project, ProjectStatus};

use chrono::Utc;
use uuid::Uuid;

#[test]
fn test_project_new() {
    let user_id = Uuid::new_v4();
    let project = Project::new("My Project".to_string(), "MYPROJ".to_string(), user_id);

    assert_eq!(project.title, "My Project");
    assert_eq!(project.key, "MYPROJ");
    assert_eq!(project.status, ProjectStatus::Active);
    assert_eq!(project.version, 1);
    assert_eq!(project.created_by, user_id);
    assert_eq!(project.updated_by, user_id);
    assert!(!project.is_deleted());
    assert!(!project.is_archived());
}

#[test]
fn test_project_is_deleted() {
    let user_id = Uuid::new_v4();
    let mut project = Project::new("Test".to_string(), "TEST".to_string(), user_id);

    assert!(!project.is_deleted());

    project.deleted_at = Some(Utc::now());
    assert!(project.is_deleted());
}

#[test]
fn test_project_is_archived() {
    let user_id = Uuid::new_v4();
    let mut project = Project::new("Test".to_string(), "TEST".to_string(), user_id);

    assert!(!project.is_archived());

    project.status = ProjectStatus::Archived;
    assert!(project.is_archived());
}
