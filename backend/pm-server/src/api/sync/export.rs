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

use axum::{
    Json,
    extract::{Query, State},
};
use chrono::Utc;
use error_location::ErrorLocation;
use serde::Deserialize;
use std::panic::Location;
use uuid::Uuid;

// ============================================================================
// Query Parameters
// ============================================================================

#[derive(Debug, Deserialize, Default)]
pub struct ExportQuery {
    /// Scope export to a single work item (UUID)
    pub work_item: Option<String>,
    /// Include N levels of descendants (0=just item, 1=children, 2=grandchildren)
    #[serde(default)]
    pub descendant_levels: u32,
    /// Include comments for matched work items
    #[serde(default)]
    pub comments: bool,
    /// Include sprint data for matched work items
    #[serde(default)]
    pub sprints: bool,
    /// Include dependency links for matched work items
    #[serde(default)]
    pub dependencies: bool,
    /// Include time entries for matched work items
    #[serde(default)]
    pub time_entries: bool,
}

// ============================================================================
// Export Handler
// ============================================================================

/// Export data (GET /api/v1/sync/export)
///
/// Without query params: exports the full database.
/// With `?work_item=<UUID>`: exports only the specified work item and opted-in related data.
pub async fn sync_export(
    State(state): State<AppState>,
    Query(query): Query<ExportQuery>,
) -> ApiResult<Json<ExportData>> {
    let pool = &state.pool;

    // Load projects for display_key resolution (project_id → key)
    let projects = ProjectRepository::new(pool.clone()).find_all().await?;
    let project_keys: std::collections::HashMap<Uuid, String> =
        projects.iter().map(|p| (p.id, p.key.clone())).collect();

    // Full export (no work_item filter)
    if query.work_item.is_none() {
        let sprints = SprintRepository::new(pool.clone()).find_all().await?;
        let swim_lanes = SwimLaneRepository::new(pool.clone()).find_all().await?;
        let work_items = WorkItemRepository::find_all(pool, true).await?;
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
                .map(|w| {
                    let key = project_keys
                        .get(&w.project_id)
                        .map(|k| k.as_str())
                        .unwrap_or("UNKNOWN");
                    WorkItemDto::from_work_item(w, key)
                })
                .collect(),
            comments: comments.into_iter().map(CommentDto::from).collect(),
            dependencies: dependencies.into_iter().map(DependencyDto::from).collect(),
            time_entries: time_entries.into_iter().map(TimeEntryDto::from).collect(),
        };

        return Ok(Json(data));
    }

    // Scoped export: filter to a specific work item (+ optional descendants/related data)
    let work_item_id_str = query.work_item.as_ref().unwrap();
    let root_id = Uuid::parse_str(work_item_id_str).map_err(|_| crate::ApiError::Validation {
        message: format!("Invalid work_item UUID: {}", work_item_id_str),
        field: Some("work_item".to_string()),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // Load all work items to support descendant traversal
    let all_work_items = WorkItemRepository::find_all(pool, true).await?;

    // Collect the set of work item IDs to export
    let mut export_ids = std::collections::HashSet::new();
    export_ids.insert(root_id);

    if query.descendant_levels > 0 {
        let mut frontier = vec![root_id];
        for _level in 0..query.descendant_levels {
            let mut next_frontier = Vec::new();
            for parent in &frontier {
                for item in &all_work_items {
                    if item.parent_id == Some(*parent) && !export_ids.contains(&item.id) {
                        export_ids.insert(item.id);
                        next_frontier.push(item.id);
                    }
                }
            }
            frontier = next_frontier;
            if frontier.is_empty() {
                break;
            }
        }
    }

    // Filter work items
    let work_items: Vec<WorkItemDto> = all_work_items
        .into_iter()
        .filter(|w| export_ids.contains(&w.id))
        .map(|w| {
            let key = project_keys
                .get(&w.project_id)
                .map(|k| k.as_str())
                .unwrap_or("UNKNOWN");
            WorkItemDto::from_work_item(w, key)
        })
        .collect();

    // Collect sprint IDs from matched work items (for opt-in sprint export)
    let sprint_ids: std::collections::HashSet<Uuid> = if query.sprints {
        work_items
            .iter()
            .filter_map(|w| w.sprint_id.as_ref())
            .filter_map(|s| Uuid::parse_str(s).ok())
            .collect()
    } else {
        std::collections::HashSet::new()
    };

    // Load and filter related data based on opt-in flags
    let comments = if query.comments {
        let all_comments = CommentRepository::new(pool.clone()).find_all().await?;
        all_comments
            .into_iter()
            .filter(|c| export_ids.contains(&c.work_item_id))
            .map(CommentDto::from)
            .collect()
    } else {
        vec![]
    };

    let sprints = if query.sprints && !sprint_ids.is_empty() {
        let all_sprints = SprintRepository::new(pool.clone()).find_all().await?;
        all_sprints
            .into_iter()
            .filter(|s| sprint_ids.contains(&s.id))
            .map(SprintDto::from)
            .collect()
    } else {
        vec![]
    };

    let dependencies = if query.dependencies {
        let all_deps = DependencyRepository::new(pool.clone()).find_all().await?;
        all_deps
            .into_iter()
            .filter(|d| {
                export_ids.contains(&d.blocking_item_id) || export_ids.contains(&d.blocked_item_id)
            })
            .map(DependencyDto::from)
            .collect()
    } else {
        vec![]
    };

    let time_entries = if query.time_entries {
        let all_entries = TimeEntryRepository::new(pool.clone()).find_all().await?;
        all_entries
            .into_iter()
            .filter(|t| export_ids.contains(&t.work_item_id))
            .map(TimeEntryDto::from)
            .collect()
    } else {
        vec![]
    };

    let data = ExportData {
        schema_version: 1,
        exported_at: Utc::now().to_rfc3339(),
        exported_by: "pm-server".to_string(),
        projects: vec![],
        sprints,
        swim_lanes: vec![],
        work_items,
        comments,
        dependencies,
        time_entries,
    };

    Ok(Json(data))
}
