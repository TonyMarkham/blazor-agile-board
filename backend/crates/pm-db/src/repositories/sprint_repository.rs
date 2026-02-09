use crate::{DbError, Result as DbErrorResult};

use pm_core::{Sprint, SprintStatus};

use std::panic::Location;
use std::str::FromStr;

use chrono::DateTime;
use error_location::ErrorLocation;
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SprintRepository {
    pool: SqlitePool,
}

impl SprintRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, sprint: &Sprint) -> DbErrorResult<()> {
        let id = sprint.id.to_string();
        let project_id = sprint.project_id.to_string();
        let status = sprint.status.as_str();
        let start_date = sprint.start_date.timestamp();
        let end_date = sprint.end_date.timestamp();
        let created_at = sprint.created_at.timestamp();
        let updated_at = sprint.updated_at.timestamp();
        let created_by = sprint.created_by.to_string();
        let updated_by = sprint.updated_by.to_string();
        let deleted_at = sprint.deleted_at.map(|dt| dt.timestamp());

        sqlx::query!(
            r#"
                INSERT INTO pm_sprints (
                                        id, project_id, name, goal,
                                        start_date, end_date, status, version,
                                        created_at, updated_at, created_by, updated_by, deleted_at
                                        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            id,
            project_id,
            sprint.name,
            sprint.goal,
            start_date,
            end_date,
            status,
            sprint.version,
            created_at,
            updated_at,
            created_by,
            updated_by,
            deleted_at,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn find_by_id(&self, id: Uuid) -> DbErrorResult<Option<Sprint>> {
        let id_str = id.to_string();

        let row = sqlx::query!(
            r#"
                SELECT id, project_id, name, goal, start_date, end_date, status, version,
                       created_at, updated_at, created_by, updated_by, deleted_at
                FROM pm_sprints
                WHERE id = ? AND deleted_at IS NULL
            "#,
            id_str
        )
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| -> DbErrorResult<Sprint> {
            Ok(Sprint {
                id: Uuid::parse_str(r.id.as_ref().ok_or_else(|| DbError::Initialization {
                    message: "sprint.id is NULL".to_string(),
                    location: ErrorLocation::from(Location::caller()),
                })?)
                .map_err(|e| DbError::Initialization {
                    message: format!("Invalid UUID in sprint.id: {}", e),
                    location: ErrorLocation::from(Location::caller()),
                })?,
                project_id: Uuid::parse_str(&r.project_id).map_err(|e| {
                    DbError::Initialization {
                        message: format!("Invalid UUID in sprint.project_id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                name: r.name,
                goal: r.goal,
                start_date: DateTime::from_timestamp(r.start_date, 0).ok_or_else(|| {
                    DbError::Initialization {
                        message: "Invalid timestamp in sprint.start_date".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                end_date: DateTime::from_timestamp(r.end_date, 0).ok_or_else(|| {
                    DbError::Initialization {
                        message: "Invalid timestamp in sprint.end_date".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                status: SprintStatus::from_str(&r.status).map_err(|e| DbError::Initialization {
                    message: format!("Invalid SprintStatus in sprint.status: {}", e),
                    location: ErrorLocation::from(Location::caller()),
                })?,
                version: r.version as i32,
                created_at: DateTime::from_timestamp(r.created_at, 0).ok_or_else(|| {
                    DbError::Initialization {
                        message: "Invalid timestamp in sprint.created_at".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                updated_at: DateTime::from_timestamp(r.updated_at, 0).ok_or_else(|| {
                    DbError::Initialization {
                        message: "Invalid timestamp in sprint.updated_at".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                created_by: Uuid::parse_str(&r.created_by).map_err(|e| {
                    DbError::Initialization {
                        message: format!("Invalid UUID in sprint.created_by: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                updated_by: Uuid::parse_str(&r.updated_by).map_err(|e| {
                    DbError::Initialization {
                        message: format!("Invalid UUID in sprint.updated_by: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
            })
        })
        .transpose()
    }

    pub async fn find_by_project(&self, project_id: Uuid) -> DbErrorResult<Vec<Sprint>> {
        let project_id_str = project_id.to_string();

        let rows = sqlx::query!(
            r#"
                SELECT id, project_id, name, goal, start_date, end_date, status, version,
                       created_at, updated_at, created_by, updated_by, deleted_at
                FROM pm_sprints
                WHERE project_id = ? AND deleted_at IS NULL
                ORDER BY start_date DESC
            "#,
            project_id_str
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| -> DbErrorResult<Sprint> {
                Ok(Sprint {
                    id: Uuid::parse_str(r.id.as_ref().ok_or_else(|| DbError::Initialization {
                        message: "sprint.id is NULL".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    })?)
                    .map_err(|e| DbError::Initialization {
                        message: format!("Invalid UUID in sprint.id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    })?,
                    project_id: Uuid::parse_str(&r.project_id).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in sprint.project_id: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    name: r.name,
                    goal: r.goal,
                    start_date: DateTime::from_timestamp(r.start_date, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in sprint.start_date".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    end_date: DateTime::from_timestamp(r.end_date, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in sprint.end_date".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    status: SprintStatus::from_str(&r.status).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid SprintStatus in sprint.status: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    version: r.version as i32,
                    created_at: DateTime::from_timestamp(r.created_at, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in sprint.created_at".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    updated_at: DateTime::from_timestamp(r.updated_at, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in sprint.updated_at".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    created_by: Uuid::parse_str(&r.created_by).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in sprint.created_by: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    updated_by: Uuid::parse_str(&r.updated_by).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in sprint.updated_by: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
                })
            })
            .collect::<DbErrorResult<Vec<_>>>()
    }

    pub async fn find_active_by_project(&self, project_id: Uuid) -> DbErrorResult<Option<Sprint>> {
        let project_id_str = project_id.to_string();
        let active_status = SprintStatus::Active.as_str();

        let row = sqlx::query!(
            r#"
                SELECT id, project_id, name, goal, start_date, end_date, status, version,
                       created_at, updated_at, created_by, updated_by, deleted_at
                FROM pm_sprints
                WHERE project_id = ? AND status = ? AND deleted_at IS NULL
            "#,
            project_id_str,
            active_status
        )
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| -> DbErrorResult<Sprint> {
            Ok(Sprint {
                id: Uuid::parse_str(r.id.as_ref().ok_or_else(|| DbError::Initialization {
                    message: "sprint.id is NULL".to_string(),
                    location: ErrorLocation::from(Location::caller()),
                })?)
                .map_err(|e| DbError::Initialization {
                    message: format!("Invalid UUID in sprint.id: {}", e),
                    location: ErrorLocation::from(Location::caller()),
                })?,
                project_id: Uuid::parse_str(&r.project_id).map_err(|e| {
                    DbError::Initialization {
                        message: format!("Invalid UUID in sprint.project_id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                name: r.name,
                goal: r.goal,
                start_date: DateTime::from_timestamp(r.start_date, 0).ok_or_else(|| {
                    DbError::Initialization {
                        message: "Invalid timestamp in sprint.start_date".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                end_date: DateTime::from_timestamp(r.end_date, 0).ok_or_else(|| {
                    DbError::Initialization {
                        message: "Invalid timestamp in sprint.end_date".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                status: SprintStatus::from_str(&r.status).map_err(|e| DbError::Initialization {
                    message: format!("Invalid SprintStatus in sprint.status: {}", e),
                    location: ErrorLocation::from(Location::caller()),
                })?,
                version: r.version as i32,
                created_at: DateTime::from_timestamp(r.created_at, 0).ok_or_else(|| {
                    DbError::Initialization {
                        message: "Invalid timestamp in sprint.created_at".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                updated_at: DateTime::from_timestamp(r.updated_at, 0).ok_or_else(|| {
                    DbError::Initialization {
                        message: "Invalid timestamp in sprint.updated_at".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                created_by: Uuid::parse_str(&r.created_by).map_err(|e| {
                    DbError::Initialization {
                        message: format!("Invalid UUID in sprint.created_by: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                updated_by: Uuid::parse_str(&r.updated_by).map_err(|e| {
                    DbError::Initialization {
                        message: format!("Invalid UUID in sprint.updated_by: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
            })
        })
        .transpose()
    }

    pub async fn update(&self, sprint: &Sprint) -> DbErrorResult<()> {
        let id = sprint.id.to_string();
        let project_id = sprint.project_id.to_string();
        let status = sprint.status.as_str();
        let start_date = sprint.start_date.timestamp();
        let end_date = sprint.end_date.timestamp();
        let updated_at = sprint.updated_at.timestamp();
        let updated_by = sprint.updated_by.to_string();

        sqlx::query!(
            r#"
                UPDATE pm_sprints
                SET project_id = ?, name = ?, goal = ?,
                    start_date = ?, end_date = ?, status = ?, version = ?,
                    updated_at = ?, updated_by = ?
                WHERE id = ? AND deleted_at IS NULL
            "#,
            project_id,
            sprint.name,
            sprint.goal,
            start_date,
            end_date,
            status,
            sprint.version,
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
              UPDATE pm_sprints
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

    pub async fn find_all(&self) -> DbErrorResult<Vec<Sprint>> {
        let rows = sqlx::query!(
            r#"
                SELECT id, project_id, name, goal, start_date, end_date, status, version, created_at, updated_at, created_by, updated_by, deleted_at
                FROM pm_sprints
                WHERE deleted_at IS NULL
                ORDER BY start_date DESC
            "#
        )
            .fetch_all(&self.pool)
            .await?;

        rows.into_iter()
            .map(|r| -> DbErrorResult<Sprint> {
                Ok(Sprint {
                    id: Uuid::parse_str(r.id.as_ref().ok_or_else(|| DbError::Initialization {
                        message: "sprint.id is NULL".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    })?)
                    .map_err(|e| DbError::Initialization {
                        message: format!("Invalid UUID in sprint.id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    })?,
                    project_id: Uuid::parse_str(&r.project_id).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in sprint.project_id: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    name: r.name,
                    goal: r.goal,
                    start_date: DateTime::from_timestamp(r.start_date, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in sprint.start_date".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    end_date: DateTime::from_timestamp(r.end_date, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in sprint.end_date".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    status: SprintStatus::from_str(&r.status).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid SprintStatus in sprint.status: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    version: r.version as i32,
                    created_at: DateTime::from_timestamp(r.created_at, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in sprint.created_at".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    updated_at: DateTime::from_timestamp(r.updated_at, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in sprint.updated_at".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    created_by: Uuid::parse_str(&r.created_by).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in sprint.created_by: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    updated_by: Uuid::parse_str(&r.updated_by).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in sprint.updated_by: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
                })
            })
            .collect::<DbErrorResult<Vec<_>>>()
    }
}
