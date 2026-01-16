use crate::{HandlerContext, Result as WsErrorResult, WsError};

use pm_core::Permission;
use pm_db::ProjectMemberRepository;

use std::panic::Location;

use error_location::ErrorLocation;
use uuid::Uuid;

/// Check if user has required permission on a project.
///
/// Returns Ok(()) if authorized, Err(WsError::Unauthorized) otherwise.
pub async fn check_permission(
    ctx: &HandlerContext,
    project_id: Uuid,
    required: Permission,
) -> WsErrorResult<()> {
    let repo = ProjectMemberRepository::new(ctx.pool.clone());
    let member = repo
        .find_by_user_and_project(ctx.user_id, project_id)
        .await
        .map_err(|e| WsError::Internal {
            message: format!("Failed to check permission: {e}"),
            location: ErrorLocation::from(Location::caller()),
        })?;

    match member {
        None => Err(WsError::Unauthorized {
            message: "Not a member of this project".to_string(),
            location: ErrorLocation::from(Location::caller()),
        }),
        Some(m) if !m.has_permission(required) => Err(WsError::Unauthorized {
            message: format!(
                "Insufficient permission. Required: {required:?}, have: {}",
                m.role
            ),
            location: ErrorLocation::from(Location::caller()),
        }),
        Some(_) => Ok(()),
    }
}
