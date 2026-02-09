//! Project entity - organizational container for work items.

use crate::{CoreError, CoreResult, ProjectDto, ProjectStatus, parse_timestamp, parse_uuid};

use std::panic::Location;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use error_location::ErrorLocation;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A project is a top-level organizational container.
/// Unlike work items, projects have a unique key and status (active/archived).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    /// Unique short identifier (e.g., "PROJ", "WEBAPP")
    pub key: String,
    pub status: ProjectStatus,
    /// Optimistic locking version
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Uuid,
    pub updated_by: Uuid,
    pub deleted_at: Option<DateTime<Utc>>,
    /// Atomic counter for assigning sequential work item numbers
    pub next_work_item_number: i32,
}

impl Project {
    /// Create a new project with default values
    pub fn new(title: String, key: String, created_by: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title,
            description: None,
            key,
            status: ProjectStatus::Active,
            version: 1,
            created_at: now,
            updated_at: now,
            created_by,
            updated_by: created_by,
            deleted_at: None,
            next_work_item_number: 1,
        }
    }

    /// Check if project is deleted (soft delete)
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    /// Check if project is archived
    pub fn is_archived(&self) -> bool {
        self.status == ProjectStatus::Archived
    }
}

impl TryFrom<ProjectDto> for Project {
    type Error = CoreError;

    fn try_from(dto: ProjectDto) -> CoreResult<Self> {
        Ok(Project {
            id: parse_uuid(&dto.id, "project.id")?,
            title: dto.title,
            description: dto.description,
            key: dto.key,
            status: ProjectStatus::from_str(&dto.status).map_err(|_| CoreError::Validation {
                message: format!("Invalid project status: {}", dto.status),
                field: Some("status".into()),
                location: ErrorLocation::from(Location::caller()),
            })?,
            version: dto.version,
            created_at: parse_timestamp(dto.created_at, "project.created_at")?,
            updated_at: parse_timestamp(dto.updated_at, "project.updated_at")?,
            created_by: parse_uuid(&dto.created_by, "project.created_by")?,
            updated_by: parse_uuid(&dto.updated_by, "project.updated_by")?,
            deleted_at: None,
            next_work_item_number: dto.next_work_item_number,
        })
    }
}
