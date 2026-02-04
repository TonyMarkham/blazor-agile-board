//! Project repository for CRUD operations on projects.
//!
//! ## Work Item Number Counter
//!
//! The `next_work_item_number` field is an atomic counter for assigning
//! sequential numbers to work items within a project. Numbers are assigned
//! when work items are created via `get_and_increment_work_item_number()`.
//!
//! **IMPORTANT: Counter gaps are EXPECTED and CORRECT behavior.**
//!
//! Gaps occur when:
//! - Transaction rolls back after incrementing counter (e.g., validation fails)
//! - Work item is soft-deleted (item_number is preserved, gap in active items)
//!
//! Example timeline:
//! 1. Create work item → assigned #5, counter becomes 6
//! 2. Transaction fails (e.g., circular reference detected)
//! 3. Counter is still at 6 (gap at #5)
//! 4. Next work item → assigned #6 (gap at #5 remains)
//!
//! This is INTENTIONAL. Work item numbers are unique identifiers, not a
//! sequential count. Users see "TEST-1, TEST-2, TEST-6" and this is correct.

use crate::{DbError, Result as DbErrorResult};

use pm_core::{Project, ProjectStatus};

use std::panic::Location;
use std::str::FromStr;

use chrono::DateTime;
use error_location::ErrorLocation;
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct ProjectRepository {
    pool: SqlitePool,
}

impl ProjectRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, project: &Project) -> DbErrorResult<()> {
        let id = project.id.to_string();
        let status = project.status.as_str();
        let created_at = project.created_at.timestamp();
        let updated_at = project.updated_at.timestamp();
        let created_by = project.created_by.to_string();
        let updated_by = project.updated_by.to_string();
        let deleted_at = project.deleted_at.map(|dt| dt.timestamp());

        sqlx::query!(
            r#"
                INSERT INTO pm_projects (
                    id, title, description, key, status, version,
                    created_at, updated_at, created_by, updated_by, deleted_at,
                    next_work_item_number
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            id,
            project.title,
            project.description,
            project.key,
            status,
            project.version,
            created_at,
            updated_at,
            created_by,
            updated_by,
            deleted_at,
            project.next_work_item_number,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn find_by_id(&self, id: Uuid) -> DbErrorResult<Option<Project>> {
        let id_str = id.to_string();

        let row = sqlx::query!(
            r#"
                SELECT id, title, description, key, status, version,
                    created_at, updated_at, created_by, updated_by, deleted_at,
                    next_work_item_number
                FROM pm_projects
                WHERE id = ? AND deleted_at IS NULL
                "#,
            id_str
        )
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| -> DbErrorResult<Project> {
            Ok(Project {
                id: Uuid::parse_str(r.id.as_ref().ok_or_else(|| DbError::Initialization {
                    message: "project.id is NULL".to_string(),
                    location: ErrorLocation::from(Location::caller()),
                })?)
                .map_err(|e| DbError::Initialization {
                    message: format!("Invalid UUID in project.id: {}", e),
                    location: ErrorLocation::from(Location::caller()),
                })?,
                title: r.title,
                description: r.description,
                key: r.key,
                status: ProjectStatus::from_str(&r.status).map_err(|e| {
                    DbError::Initialization {
                        message: format!("Invalid ProjectStatus in project.status: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                version: r.version as i32,
                created_at: DateTime::from_timestamp(r.created_at, 0).ok_or_else(|| {
                    DbError::Initialization {
                        message: "Invalid timestamp in project.created_at".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                updated_at: DateTime::from_timestamp(r.updated_at, 0).ok_or_else(|| {
                    DbError::Initialization {
                        message: "Invalid timestamp in project.updated_at".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                created_by: Uuid::parse_str(&r.created_by).map_err(|e| {
                    DbError::Initialization {
                        message: format!("Invalid UUID in project.created_by: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                updated_by: Uuid::parse_str(&r.updated_by).map_err(|e| {
                    DbError::Initialization {
                        message: format!("Invalid UUID in project.updated_by: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
                next_work_item_number: r.next_work_item_number as i32,
            })
        })
        .transpose()
    }

    pub async fn find_by_key(&self, key: &str) -> DbErrorResult<Option<Project>> {
        let row = sqlx::query!(
            r#"
                SELECT id, title, description, key, status, version,
                    created_at, updated_at, created_by, updated_by, deleted_at,
                    next_work_item_number
                FROM pm_projects
                WHERE key = ? AND deleted_at IS NULL
                "#,
            key
        )
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| -> DbErrorResult<Project> {
            Ok(Project {
                id: Uuid::parse_str(r.id.as_ref().ok_or_else(|| DbError::Initialization {
                    message: "project.id is NULL".to_string(),
                    location: ErrorLocation::from(Location::caller()),
                })?)
                .map_err(|e| DbError::Initialization {
                    message: format!("Invalid UUID in project.id: {}", e),
                    location: ErrorLocation::from(Location::caller()),
                })?,
                title: r.title,
                description: r.description,
                key: r.key,
                status: ProjectStatus::from_str(&r.status).map_err(|e| {
                    DbError::Initialization {
                        message: format!("Invalid ProjectStatus in project.status: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                version: r.version as i32,
                created_at: DateTime::from_timestamp(r.created_at, 0).ok_or_else(|| {
                    DbError::Initialization {
                        message: "Invalid timestamp in project.created_at".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                updated_at: DateTime::from_timestamp(r.updated_at, 0).ok_or_else(|| {
                    DbError::Initialization {
                        message: "Invalid timestamp in project.updated_at".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                created_by: Uuid::parse_str(&r.created_by).map_err(|e| {
                    DbError::Initialization {
                        message: format!("Invalid UUID in project.created_by: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                updated_by: Uuid::parse_str(&r.updated_by).map_err(|e| {
                    DbError::Initialization {
                        message: format!("Invalid UUID in project.updated_by: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
                next_work_item_number: r.next_work_item_number as i32,
            })
        })
        .transpose()
    }

    pub async fn find_all(&self) -> DbErrorResult<Vec<Project>> {
        let rows = sqlx::query!(
            r#"
                SELECT id, title, description, key, status, version,
                    created_at, updated_at, created_by, updated_by, deleted_at,
                    next_work_item_number
                FROM pm_projects
                WHERE deleted_at IS NULL
                ORDER BY title
                "#
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| -> DbErrorResult<Project> {
                Ok(Project {
                    id: Uuid::parse_str(r.id.as_ref().ok_or_else(|| DbError::Initialization {
                        message: "project.id is NULL".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    })?)
                    .map_err(|e| DbError::Initialization {
                        message: format!("Invalid UUID in project.id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    })?,
                    title: r.title,
                    description: r.description,
                    key: r.key,
                    status: ProjectStatus::from_str(&r.status).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid ProjectStatus in project.status: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    version: r.version as i32,
                    created_at: DateTime::from_timestamp(r.created_at, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in project.created_at".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    updated_at: DateTime::from_timestamp(r.updated_at, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in project.updated_at".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    created_by: Uuid::parse_str(&r.created_by).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in project.created_by: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    updated_by: Uuid::parse_str(&r.updated_by).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in project.updated_by: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
                    next_work_item_number: r.next_work_item_number as i32,
                })
            })
            .collect::<DbErrorResult<Vec<_>>>()
    }

    pub async fn find_active(&self) -> DbErrorResult<Vec<Project>> {
        let rows = sqlx::query!(
            r#"
                SELECT id, title, description, key, status, version,
                    created_at, updated_at, created_by, updated_by, deleted_at,
                    next_work_item_number
                FROM pm_projects
                WHERE status = 'active' AND deleted_at IS NULL
                ORDER BY title
                "#
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| -> DbErrorResult<Project> {
                Ok(Project {
                    id: Uuid::parse_str(r.id.as_ref().ok_or_else(|| DbError::Initialization {
                        message: "project.id is NULL".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    })?)
                    .map_err(|e| DbError::Initialization {
                        message: format!("Invalid UUID in project.id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    })?,
                    title: r.title,
                    description: r.description,
                    key: r.key,
                    status: ProjectStatus::from_str(&r.status).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid ProjectStatus in project.status: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    version: r.version as i32,
                    created_at: DateTime::from_timestamp(r.created_at, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in project.created_at".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    updated_at: DateTime::from_timestamp(r.updated_at, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in project.updated_at".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    created_by: Uuid::parse_str(&r.created_by).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in project.created_by: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    updated_by: Uuid::parse_str(&r.updated_by).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in project.updated_by: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
                    next_work_item_number: r.next_work_item_number as i32,
                })
            })
            .collect::<DbErrorResult<Vec<_>>>()
    }

    pub async fn update(&self, project: &Project) -> DbErrorResult<()> {
        let id = project.id.to_string();
        let status = project.status.as_str();
        let updated_at = project.updated_at.timestamp();
        let updated_by = project.updated_by.to_string();

        sqlx::query!(
            r#"
                UPDATE pm_projects
                SET title = ?, description = ?, key = ?, status = ?,
                    version = ?, updated_at = ?, updated_by = ?
                WHERE id = ? AND deleted_at IS NULL
                "#,
            project.title,
            project.description,
            project.key,
            status,
            project.version,
            updated_at,
            updated_by,
            id,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete(&self, id: Uuid, deleted_at: i64) -> DbErrorResult<()> {
        let id_str = id.to_string();

        sqlx::query!(
            r#"
                UPDATE pm_projects
                SET deleted_at = ?
                WHERE id = ? AND deleted_at IS NULL
                "#,
            deleted_at,
            id_str
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Atomically get and increment the work item number for a project.
    /// Returns the number to assign to the new work item.
    ///
    /// CRITICAL: This method REQUIRES a Transaction. The type system enforces this.
    ///
    /// **Counter Gap Behavior:**
    /// If the transaction rolls back after incrementing, the counter will have a gap.
    /// This is EXPECTED and CORRECT behavior. Work item numbers are NOT guaranteed to be
    /// sequential without gaps - they are only guaranteed to be unique within a project.
    ///
    /// Example: Create work item #5 fails → counter is at 6 → next work item is #6 (gap at #5).
    pub async fn get_and_increment_work_item_number(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        project_id: Uuid,
    ) -> DbErrorResult<i32> {
        let project_id_str = project_id.to_string();

        // SQLite doesn't support RETURNING, so we need two queries
        // The transaction isolation ensures atomicity

        // First, get the current value
        let current = sqlx::query_scalar!(
            r#"
                  SELECT next_work_item_number as "next_work_item_number!"
                  FROM pm_projects
                  WHERE id = ? AND deleted_at IS NULL
                  "#,
            project_id_str
        )
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => DbError::Initialization {
                message: format!(
                    "Project {} not found when incrementing work item counter",
                    project_id
                ),
                location: ErrorLocation::from(Location::caller()),
            },
            _ => DbError::from(e),
        })?;

        let item_number = current as i32;
        let next_number = item_number + 1;

        // Then increment
        sqlx::query!(
            r#"
                  UPDATE pm_projects
                  SET next_work_item_number = ?
                  WHERE id = ? AND deleted_at IS NULL
                  "#,
            next_number,
            project_id_str
        )
        .execute(&mut **tx)
        .await?;

        Ok(item_number)
    }
}
