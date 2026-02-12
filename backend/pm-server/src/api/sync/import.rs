use crate::api::error::{ApiError, Result as ApiResult};

use pm_core::{Comment, Dependency, Project, ProjectMember, Sprint, TimeEntry, WorkItem};
use pm_core::{ExportData, ImportResult};
use pm_db::{
    CommentRepository, DependencyRepository, ProjectMemberRepository, ProjectRepository,
    SprintRepository, TimeEntryRepository, WorkItemRepository,
};
use pm_ws::AppState;

use axum::{Json, extract::State};
use chrono::Utc;
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;

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

        let member_repo = ProjectMemberRepository::new(pool.clone());

        match repo.find_by_id(project.id).await? {
            None => {
                repo.create(&project).await?;
                // Add creator as admin member (mirrors normal project creation)
                let member = ProjectMember {
                    id: Uuid::new_v4(),
                    project_id: project.id,
                    user_id: project.created_by,
                    role: "admin".to_string(),
                    created_at: Utc::now(),
                };
                member_repo.create(&member).await?;
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

    // Convert all work item DTOs first, then topologically sort so parents
    // are inserted before children (parent_id FK references pm_work_items).
    let mut work_items: Vec<WorkItem> = data
        .work_items
        .into_iter()
        .map(|dto| {
            dto.try_into()
                .map_err(|e: pm_core::CoreError| ApiError::Internal {
                    message: format!("Failed to convert work item DTO: {}", e),
                    location: error_location::ErrorLocation::from(std::panic::Location::caller()),
                })
        })
        .collect::<ApiResult<Vec<_>>>()?;

    topological_sort_work_items(&mut work_items);

    for work_item in &work_items {
        match WorkItemRepository::find_by_id(pool, work_item.id).await? {
            None => {
                WorkItemRepository::create(pool, work_item).await?;
                result.work_items.created += 1;
            }
            Some(existing) if work_item.updated_at > existing.updated_at => {
                WorkItemRepository::update(pool, work_item).await?;
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

/// Topologically sort work items so parents appear before children.
/// Items with no parent_id (or parent outside the set) come first.
fn topological_sort_work_items(items: &mut Vec<WorkItem>) {
    let id_set: HashSet<uuid::Uuid> = items.iter().map(|w| w.id).collect();

    // Build adjacency: parent_id -> list of indices that depend on it
    let mut children_of: HashMap<uuid::Uuid, Vec<usize>> = HashMap::new();
    let mut in_degree: Vec<usize> = vec![0; items.len()];

    for (i, item) in items.iter().enumerate() {
        if let Some(pid) = item.parent_id
            && id_set.contains(&pid)
        {
            children_of.entry(pid).or_default().push(i);
            in_degree[i] = 1;
        }
    }

    // BFS from roots (in_degree == 0)
    let mut queue: VecDeque<usize> = VecDeque::new();
    for (i, &deg) in in_degree.iter().enumerate() {
        if deg == 0 {
            queue.push_back(i);
        }
    }

    let mut order: Vec<usize> = Vec::with_capacity(items.len());
    while let Some(idx) = queue.pop_front() {
        order.push(idx);
        if let Some(deps) = children_of.get(&items[idx].id) {
            for &child_idx in deps {
                in_degree[child_idx] -= 1;
                if in_degree[child_idx] == 0 {
                    queue.push_back(child_idx);
                }
            }
        }
    }

    // Append any remaining items (shouldn't happen unless cycles exist)
    if order.len() < items.len() {
        for i in 0..items.len() {
            if !order.contains(&i) {
                order.push(i);
            }
        }
    }

    // Reorder in-place by building a new vec from the order
    let sorted: Vec<WorkItem> = order.into_iter().map(|i| items[i].clone()).collect();
    *items = sorted;
}
