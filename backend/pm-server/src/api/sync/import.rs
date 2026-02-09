use crate::api::error::{ApiError, Result as ApiResult};

use pm_core::{Comment, Dependency, Project, Sprint, TimeEntry, WorkItem};
use pm_core::{ExportData, ImportResult};
use pm_db::{
    CommentRepository, DependencyRepository, ProjectRepository, SprintRepository,
    TimeEntryRepository, WorkItemRepository,
};
use pm_ws::AppState;

use axum::{Json, extract::State};

// ============================================================================
// Import Handler
// ============================================================================

/// Import data (POST /api/v1/sync/import)
pub async fn sync_import(
    State(state): State<AppState>,
    Json(data): Json<ExportData>,
) -> ApiResult<Json<ImportResult>> {
    if data.schema_version != 1 {
        return Err(ApiError::Validation {
            message: format!("Unsupported schema version: {}", data.schema_version),
            field: Some("schema_version".into()),
            location: error_location::ErrorLocation::from(std::panic::Location::caller()),
        });
    }

    let pool = &state.pool;
    let mut result = ImportResult::default();

    for dto in data.projects {
        let project: Project =
            dto.try_into()
                .map_err(|e: pm_core::CoreError| ApiError::Internal {
                    message: format!("Failed to convert project DTO: {}", e),
                    location: error_location::ErrorLocation::from(std::panic::Location::caller()),
                })?;
        let repo = ProjectRepository::new(pool.clone());

        match repo.find_by_id(project.id).await? {
            None => {
                repo.create(&project).await?;
                result.projects.created += 1;
            }
            Some(existing) if project.updated_at > existing.updated_at => {
                repo.update(&project).await?;
                result.projects.updated += 1;
            }
            Some(_) => {
                result.projects.skipped += 1;
            }
        }
    }

    for dto in data.sprints {
        let sprint: Sprint =
            dto.try_into()
                .map_err(|e: pm_core::CoreError| ApiError::Internal {
                    message: format!("Failed to convert sprint DTO: {}", e),
                    location: error_location::ErrorLocation::from(std::panic::Location::caller()),
                })?;
        let repo = SprintRepository::new(pool.clone());

        match repo.find_by_id(sprint.id).await? {
            None => {
                repo.create(&sprint).await?;
                result.sprints.created += 1;
            }
            Some(existing) if sprint.updated_at > existing.updated_at => {
                repo.update(&sprint).await?;
                result.sprints.updated += 1;
            }
            Some(_) => {
                result.sprints.skipped += 1;
            }
        }
    }

    result.swim_lanes.skipped = data.swim_lanes.len();

    for dto in data.work_items {
        let work_item: WorkItem =
            dto.try_into()
                .map_err(|e: pm_core::CoreError| ApiError::Internal {
                    message: format!("Failed to convert work item DTO: {}", e),
                    location: error_location::ErrorLocation::from(std::panic::Location::caller()),
                })?;

        match WorkItemRepository::find_by_id(pool, work_item.id).await? {
            None => {
                WorkItemRepository::create(pool, &work_item).await?;
                result.work_items.created += 1;
            }
            Some(existing) if work_item.updated_at > existing.updated_at => {
                WorkItemRepository::update(pool, &work_item).await?;
                result.work_items.updated += 1;
            }
            Some(_) => {
                result.work_items.skipped += 1;
            }
        }
    }

    for dto in data.comments {
        let comment: Comment =
            dto.try_into()
                .map_err(|e: pm_core::CoreError| ApiError::Internal {
                    message: format!("Failed to convert comment DTO: {}", e),
                    location: error_location::ErrorLocation::from(std::panic::Location::caller()),
                })?;
        let repo = CommentRepository::new(pool.clone());

        match repo.find_by_id(comment.id).await? {
            None => {
                repo.create(&comment).await?;
                result.comments.created += 1;
            }
            Some(existing) if comment.updated_at > existing.updated_at => {
                repo.update(&comment).await?;
                result.comments.updated += 1;
            }
            Some(_) => {
                result.comments.skipped += 1;
            }
        }
    }

    for dto in data.dependencies {
        let dependency: Dependency =
            dto.try_into()
                .map_err(|e: pm_core::CoreError| ApiError::Internal {
                    message: format!("Failed to convert dependency DTO: {}", e),
                    location: error_location::ErrorLocation::from(std::panic::Location::caller()),
                })?;
        let repo = DependencyRepository::new(pool.clone());

        match repo.find_by_id(dependency.id).await? {
            None => {
                repo.create(&dependency).await?;
                result.dependencies.created += 1;
            }
            Some(_) => {
                // Dependencies are immutable - skip if exists
                result.dependencies.skipped += 1;
            }
        }
    }

    for dto in data.time_entries {
        let time_entry: TimeEntry =
            dto.try_into()
                .map_err(|e: pm_core::CoreError| ApiError::Internal {
                    message: format!("Failed to convert time entry DTO: {}", e),
                    location: error_location::ErrorLocation::from(std::panic::Location::caller()),
                })?;
        let repo = TimeEntryRepository::new(pool.clone());

        match repo.find_by_id(time_entry.id).await? {
            None => {
                repo.create(&time_entry).await?;
                result.time_entries.created += 1;
            }
            Some(existing) if time_entry.updated_at > existing.updated_at => {
                repo.update(&time_entry).await?;
                result.time_entries.updated += 1;
            }
            Some(_) => {
                result.time_entries.skipped += 1;
            }
        }
    }

    Ok(Json(result))
}
