use crate::{Result as WsErrorResult, WsError};

use pm_core::WorkItemType;
use pm_db::WorkItemRepository;

use std::panic::Location;

use error_location::ErrorLocation;
use sqlx::SqlitePool;
use uuid::Uuid;

/// Valid parent-child relationships:
/// - Epic: no parent (parent_id should be NULL)
/// - Story: parent must be Epic
/// - Task: parent must be Story
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
