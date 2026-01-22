//! Project entity - organizational container for work items.

use crate::ProjectStatus;

use chrono::{DateTime, Utc};
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
