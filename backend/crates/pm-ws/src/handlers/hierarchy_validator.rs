//! Project repository for CRUD operations on projects.
//!
//! ## Work Item Number Counter
//!
//! The `next_work_item_number` field is an atomic counter for assigning
//! sequential numbers to work items within a project. Numbers are assigned
//! when work items are created via `get_and_increment_work_item_number()`.
//!
//! **IMPORTANT: Counter gaps are EXPECTED and CORRECT behavior.**
//!
//! Gaps occur when:
//! - Transaction rolls back after incrementing counter (e.g., validation fails)
//! - Work item is soft-deleted (item_number is preserved, gap in active items)
//!
//! Example timeline:
//! 1. Create work item → assigned #5, counter becomes 6
//! 2. Transaction fails (e.g., circular reference detected)
//! 3. Counter is still at 6 (gap at #5)
//! 4. Next work item → assigned #6 (gap at #5 remains)
//!
//! This is INTENTIONAL. Work item numbers are unique identifiers, not a
//! sequential count. Users see "TEST-1, TEST-2, TEST-6" and this is correct.

use crate::{Result as WsErrorResult, WsError};

use pm_core::WorkItemType;
use pm_db::WorkItemRepository;

use std::panic::Location;

use error_location::ErrorLocation;
use sqlx::SqlitePool;
use uuid::Uuid;

/// Valid parent-child relationships when a parent IS specified:
/// - Epic: no parent (parent_id should be NULL)
/// - Story: parent must be Epic
/// - Task: parent must be Story
///
/// **Important:** Stories and Tasks CAN be orphans (parent_id = NULL).
/// This function is only called when parent_id is provided.
/// Orphan items belong directly to the project via project_id.
///
/// **Feature B (Session 90):** Orphan stories/tasks are explicitly supported.
/// Work items do NOT require a parent in the hierarchy - parent is optional.
///
/// Returns Ok(()) if hierarchy is valid.
pub async fn validate_hierarchy(
    pool: &SqlitePool,
    child_type: WorkItemType,
    parent_id: Uuid,
) -> WsErrorResult<()> {
    let parent = WorkItemRepository::find_by_id(pool, parent_id)
        .await
        .map_err(|e| WsError::Internal {
            message: format!("Failed to fetch parent: {e}"),
            location: ErrorLocation::from(Location::caller()),
        })?
        .ok_or_else(|| WsError::ValidationError {
            message: "Parent work item not found".to_string(),
            field: Some("parent_id".to_string()),
            location: ErrorLocation::from(Location::caller()),
        })?;

    let valid = matches!(
        (parent.item_type.clone(), child_type.clone()),
        (WorkItemType::Epic, WorkItemType::Story) | (WorkItemType::Story, WorkItemType::Task)
    );

    if !valid {
        return Err(WsError::ValidationError {
            message: format!(
                "Invalid hierarchy: {child_type:?} cannot be a child of {:?}",
                parent.item_type
            ),
            field: Some("parent_id".to_string()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    Ok(())
}
