use crate::api::error::Result as ApiResult;

use pm_core::ExportData;
use pm_core::{
    CommentDto, DependencyDto, ProjectDto, SprintDto, SwimLaneDto, TimeEntryDto, WorkItemDto,
};
use pm_db::{
    CommentRepository, DependencyRepository, ProjectRepository, SprintRepository,
    SwimLaneRepository, TimeEntryRepository, WorkItemRepository,
};
use pm_ws::AppState;

use axum::{Json, extract::State};
use chrono::Utc;

// ============================================================================
// Export Handler
// ============================================================================

/// Export all data (GET /api/v1/sync/export)
pub async fn sync_export(State(state): State<AppState>) -> ApiResult<Json<ExportData>> {
    let pool = &state.pool;

    let projects = ProjectRepository::new(pool.clone()).find_all().await?;
    let sprints = SprintRepository::new(pool.clone()).find_all().await?;
    let swim_lanes = SwimLaneRepository::new(pool.clone()).find_all().await?;
    let work_items = WorkItemRepository::find_all(pool).await?;
    let comments = CommentRepository::new(pool.clone()).find_all().await?;
    let dependencies = DependencyRepository::new(pool.clone()).find_all().await?;
    let time_entries = TimeEntryRepository::new(pool.clone()).find_all().await?;

    let data = ExportData {
        schema_version: 1,
        exported_at: Utc::now().to_rfc3339(),
        exported_by: "pm-server".to_string(),
        projects: projects.into_iter().map(ProjectDto::from).collect(),
        sprints: sprints.into_iter().map(SprintDto::from).collect(),
        swim_lanes: swim_lanes.into_iter().map(SwimLaneDto::from).collect(),
        work_items: work_items
            .into_iter()
            .map(|w| WorkItemDto::from_work_item(w, "UNKNOWN"))
            .collect(),
        comments: comments.into_iter().map(CommentDto::from).collect(),
        dependencies: dependencies.into_iter().map(DependencyDto::from).collect(),
        time_entries: time_entries.into_iter().map(TimeEntryDto::from).collect(),
    };

    Ok(Json(data))
}
