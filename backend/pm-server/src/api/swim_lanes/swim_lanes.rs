//! Swim Lane REST API handlers
//!
//! Swim lanes are fixed configuration created by migrations.
//! This module provides read-only access via GET endpoint.

use crate::{ApiResult, SwimLaneListResponse};

use pm_core::SwimLaneDto;
use pm_db::SwimLaneRepository;
use pm_ws::AppState;

use axum::{
    Json,
    extract::{Path, State},
};
use uuid::Uuid;

// =============================================================================
// Handlers
// =============================================================================

/// GET /api/v1/projects/:project_id/swim-lanes
///
/// List all swim lanes for a project (ordered by position)
pub async fn list_swim_lanes(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> ApiResult<Json<SwimLaneListResponse>> {
    let project_uuid = Uuid::parse_str(&project_id)?;

    let repo = SwimLaneRepository::new(state.pool.clone());
    let swim_lanes = repo.find_by_project(project_uuid).await?;

    Ok(Json(SwimLaneListResponse {
        swim_lanes: swim_lanes.into_iter().map(SwimLaneDto::from).collect(),
    }))
}
