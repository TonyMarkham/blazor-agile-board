//! Project REST API handlers
//!
//! Read-only handlers for listing and retrieving projects.

use crate::{ApiError, ApiResult, ProjectDto, ProjectListResponse, ProjectResponse};

use pm_db::ProjectRepository;
use pm_ws::AppState;

use std::panic::Location;

use axum::{
    Json,
    extract::{Path, State},
};
use error_location::ErrorLocation;
use uuid::Uuid;

// =============================================================================
// Handlers
// =============================================================================

/// GET /api/v1/projects
///
/// List all projects
pub async fn list_projects(State(state): State<AppState>) -> ApiResult<Json<ProjectListResponse>> {
    let repo = ProjectRepository::new(state.pool.clone());
    let projects = repo.find_all().await?;

    Ok(Json(ProjectListResponse {
        projects: projects.into_iter().map(ProjectDto::from).collect(),
    }))
}

/// GET /api/v1/projects/:id
///
/// Get a single project by ID
pub async fn get_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ProjectResponse>> {
    let project_id = Uuid::parse_str(&id)?;

    let repo = ProjectRepository::new(state.pool.clone());
    let project = repo
        .find_by_id(project_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: format!("Project {} not found", id),
            location: ErrorLocation::from(Location::caller()),
        })?;

    Ok(Json(ProjectResponse {
        project: project.into(),
    }))
}
