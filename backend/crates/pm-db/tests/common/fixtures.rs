#![allow(dead_code)]

use chrono::Utc;
use pm_core::{
    ActivityLog, Comment, Dependency, DependencyType, Project, ProjectStatus, Sprint, SprintStatus,
    SwimLane, TimeEntry, WorkItem, WorkItemType,
};
use uuid::Uuid;

/// Creates a test Project
pub fn create_test_project(user_id: Uuid) -> Project {
    let project_id = Uuid::new_v4();
    let now = Utc::now();
    Project {
        id: project_id,
        title: "Test Project".to_string(),
        description: Some("Test project description".to_string()),
        key: "TESTPROJ".to_string(),
        status: ProjectStatus::Active,
        version: 1,
        created_at: now,
        updated_at: now,
        created_by: user_id,
        updated_by: user_id,
        deleted_at: None,
        next_work_item_number: 1,
    }
}

/// Creates a test WorkItem with sensible defaults
pub fn create_test_work_item(project_id: Uuid, user_id: Uuid, item_number: i32) -> WorkItem {
    WorkItem {
        id: Uuid::new_v4(),
        item_type: WorkItemType::Story,
        parent_id: None,
        project_id,
        position: 0,
        title: "Test Work Item".to_string(),
        description: Some("Test description".to_string()),
        status: "backlog".to_string(),
        priority: "medium".to_string(),
        assignee_id: None,
        story_points: None,
        sprint_id: None,
        item_number,
        version: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        created_by: user_id,
        updated_by: user_id,
        deleted_at: None,
    }
}

/// Creates a test Sprint with sensible defaults
pub fn create_test_sprint(project_id: Uuid, user_id: Uuid) -> Sprint {
    let now = Utc::now();
    Sprint {
        id: Uuid::new_v4(),
        project_id,
        name: "Test Sprint".to_string(),
        goal: Some("Test sprint goal".to_string()),
        start_date: now,
        end_date: now + chrono::Duration::days(14), // 2-week sprint
        status: SprintStatus::Planned,
        version: 1,
        created_at: now,
        updated_at: now,
        created_by: user_id,
        updated_by: user_id,
        deleted_at: None,
    }
}

/// Creates a test Comment with sensible defaults
pub fn create_test_comment(work_item_id: Uuid, user_id: Uuid) -> Comment {
    let now = Utc::now();
    Comment {
        id: Uuid::new_v4(),
        work_item_id,
        content: "Test comment content".to_string(),
        created_at: now,
        updated_at: now,
        created_by: user_id,
        updated_by: user_id,
        deleted_at: None,
    }
}

/// Creates a test TimeEntry with sensible defaults (completed timer)
pub fn create_test_time_entry(work_item_id: Uuid, user_id: Uuid) -> TimeEntry {
    let now = Utc::now();
    let started = now - chrono::Duration::hours(2);
    TimeEntry {
        id: Uuid::new_v4(),
        work_item_id,
        user_id,
        started_at: started,
        ended_at: Some(now),          // Completed timer
        duration_seconds: Some(7200), // 2 hours
        description: Some("Test time entry".to_string()),
        created_at: now,
        updated_at: now,
        deleted_at: None,
    }
}

/// Creates a running TimeEntry (ended_at is None)
pub fn create_running_time_entry(work_item_id: Uuid, user_id: Uuid) -> TimeEntry {
    let now = Utc::now();
    TimeEntry {
        id: Uuid::new_v4(),
        work_item_id,
        user_id,
        started_at: now,
        ended_at: None, // Running timer!
        duration_seconds: None,
        description: Some("Running timer".to_string()),
        created_at: now,
        updated_at: now,
        deleted_at: None,
    }
}

/// Creates a test Dependency with sensible defaults
pub fn create_test_dependency(
    blocking_item_id: Uuid,
    blocked_item_id: Uuid,
    user_id: Uuid,
) -> Dependency {
    let now = Utc::now();
    Dependency {
        id: Uuid::new_v4(),
        blocking_item_id,
        blocked_item_id,
        dependency_type: DependencyType::Blocks,
        created_at: now,
        created_by: user_id,
        deleted_at: None,
    }
}

/// Creates a test ActivityLog with sensible defaults
pub fn create_test_activity_log(entity_type: &str, entity_id: Uuid, user_id: Uuid) -> ActivityLog {
    create_test_activity_log_at(entity_type, entity_id, user_id, 0)
}

/// Creates a field change activity log
pub fn create_field_change_log(
    entity_type: &str,
    entity_id: Uuid,
    field_name: &str,
    old_value: &str,
    new_value: &str,
    user_id: Uuid,
) -> ActivityLog {
    create_field_change_log_at(
        entity_type,
        entity_id,
        field_name,
        old_value,
        new_value,
        user_id,
        0,
    )
}

/// Creates a test ActivityLog with custom timestamp offset
pub fn create_test_activity_log_at(
    entity_type: &str,
    entity_id: Uuid,
    user_id: Uuid,
    seconds_offset: i64,
) -> ActivityLog {
    let timestamp = Utc::now() + chrono::Duration::seconds(seconds_offset);
    ActivityLog {
        id: Uuid::new_v4(),
        entity_type: entity_type.to_string(),
        entity_id,
        action: "created".to_string(),
        field_name: None,
        old_value: None,
        new_value: None,
        user_id,
        timestamp,
        comment: Some("Test activity log".to_string()),
    }
}

/// Creates a field change activity log with custom timestamp offset
pub fn create_field_change_log_at(
    entity_type: &str,
    entity_id: Uuid,
    field_name: &str,
    old_value: &str,
    new_value: &str,
    user_id: Uuid,
    seconds_offset: i64,
) -> ActivityLog {
    let timestamp = Utc::now() + chrono::Duration::seconds(seconds_offset);
    ActivityLog {
        id: Uuid::new_v4(),
        entity_type: entity_type.to_string(),
        entity_id,
        action: "updated".to_string(),
        field_name: Some(field_name.to_string()),
        old_value: Some(old_value.to_string()),
        new_value: Some(new_value.to_string()),
        user_id,
        timestamp,
        comment: None,
    }
}

/// Creates a test SwimLane with sensible defaults
pub fn create_test_swim_lane(project_id: Uuid) -> SwimLane {
    let now = Utc::now();
    SwimLane {
        id: Uuid::new_v4(),
        project_id,
        name: "Test Lane".to_string(),
        status_value: "in-progress".to_string(),
        position: 0,
        is_default: false,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    }
}

/// Creates a test SwimLane with a specific status value
pub fn create_test_swim_lane_with_status(project_id: Uuid, status_value: &str) -> SwimLane {
    let now = Utc::now();
    SwimLane {
        id: Uuid::new_v4(),
        project_id,
        name: format!("Lane {}", status_value),
        status_value: status_value.to_string(),
        position: 0,
        is_default: false,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    }
}

/// Creates a default swim lane (cannot be deleted)
pub fn create_default_swim_lane(project_id: Uuid) -> SwimLane {
    let now = Utc::now();
    SwimLane {
        id: Uuid::new_v4(),
        project_id,
        name: "Default Lane".to_string(),
        status_value: "backlog".to_string(),
        position: 0,
        is_default: true,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    }
}
