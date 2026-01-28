use crate::{DbError, Result as DbErrorResult};

use pm_core::{Dependency, DependencyType};

use std::{panic::Location, str::FromStr};

use chrono::DateTime;
use error_location::ErrorLocation;
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct DependencyRepository {
    pool: SqlitePool,
}

impl DependencyRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, dependency: &Dependency) -> DbErrorResult<()> {
        let id = dependency.id.to_string();
        let blocking_item_id = dependency.blocking_item_id.to_string();
        let blocked_item_id = dependency.blocked_item_id.to_string();
        let dependency_type = dependency.dependency_type.as_str();
        let created_at = dependency.created_at.timestamp();
        let created_by = dependency.created_by.to_string();
        let deleted_at = dependency.deleted_at.map(|dt| dt.timestamp());

        sqlx::query!(
            r#"
              INSERT INTO pm_dependencies (
                  id, blocking_item_id, blocked_item_id, dependency_type,
                  created_at, created_by, deleted_at
              ) VALUES (?, ?, ?, ?, ?, ?, ?)
              "#,
            id,
            blocking_item_id,
            blocked_item_id,
            dependency_type,
            created_at,
            created_by,
            deleted_at,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn find_by_id(&self, id: Uuid) -> DbErrorResult<Option<Dependency>> {
        let id_str = id.to_string();

        let row = sqlx::query!(
            r#"
                SELECT id, blocking_item_id, blocked_item_id, dependency_type,
                       created_at, created_by, deleted_at
                FROM pm_dependencies
                WHERE id = ? AND deleted_at IS NULL
                "#,
            id_str
        )
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| -> DbErrorResult<Dependency> {
            Ok(Dependency {
                id: Uuid::parse_str(r.id.as_ref().ok_or_else(|| DbError::Initialization {
                    message: "dependency.id is NULL".to_string(),
                    location: ErrorLocation::from(Location::caller()),
                })?)
                .map_err(|e| DbError::Initialization {
                    message: format!("Invalid UUID in dependency.id: {}", e),
                    location: ErrorLocation::from(Location::caller()),
                })?,
                blocking_item_id: Uuid::parse_str(&r.blocking_item_id).map_err(|e| {
                    DbError::Initialization {
                        message: format!("Invalid UUID in dependency.blocking_item_id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                blocked_item_id: Uuid::parse_str(&r.blocked_item_id).map_err(|e| {
                    DbError::Initialization {
                        message: format!("Invalid UUID in dependency.blocked_item_id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                dependency_type: DependencyType::from_str(&r.dependency_type).map_err(|e| {
                    DbError::Initialization {
                        message: format!("Invalid dependency_type: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                created_at: DateTime::from_timestamp(r.created_at, 0).ok_or_else(|| {
                    DbError::Initialization {
                        message: "Invalid timestamp in dependency.created_at".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                created_by: Uuid::parse_str(&r.created_by).map_err(|e| {
                    DbError::Initialization {
                        message: format!("Invalid UUID in dependency.created_by: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
            })
        })
        .transpose()
    }

    pub async fn find_blocking(&self, blocked_item_id: Uuid) -> DbErrorResult<Vec<Dependency>> {
        let blocked_item_id_str = blocked_item_id.to_string();

        let rows = sqlx::query!(
            r#"
                SELECT id, blocking_item_id, blocked_item_id, dependency_type,
                       created_at, created_by, deleted_at
                FROM pm_dependencies
                WHERE blocked_item_id = ? AND deleted_at IS NULL
                "#,
            blocked_item_id_str
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| -> DbErrorResult<Dependency> {
                Ok(Dependency {
                    id: Uuid::parse_str(r.id.as_ref().ok_or_else(|| DbError::Initialization {
                        message: "dependency.id is NULL".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    })?)
                    .map_err(|e| DbError::Initialization {
                        message: format!("Invalid UUID in dependency.id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    })?,
                    blocking_item_id: Uuid::parse_str(&r.blocking_item_id).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in dependency.blocking_item_id: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    blocked_item_id: Uuid::parse_str(&r.blocked_item_id).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in dependency.blocked_item_id: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    dependency_type: DependencyType::from_str(&r.dependency_type).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid dependency_type: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    created_at: DateTime::from_timestamp(r.created_at, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in dependency.created_at".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    created_by: Uuid::parse_str(&r.created_by).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in dependency.created_by: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
                })
            })
            .collect::<DbErrorResult<Vec<_>>>()
    }

    pub async fn find_blocked(&self, blocking_item_id: Uuid) -> DbErrorResult<Vec<Dependency>> {
        let blocking_item_id_str = blocking_item_id.to_string();

        let rows = sqlx::query!(
            r#"
                SELECT id, blocking_item_id, blocked_item_id, dependency_type,
                       created_at, created_by, deleted_at
                FROM pm_dependencies
                WHERE blocking_item_id = ? AND deleted_at IS NULL
                "#,
            blocking_item_id_str
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| -> DbErrorResult<Dependency> {
                Ok(Dependency {
                    id: Uuid::parse_str(r.id.as_ref().ok_or_else(|| DbError::Initialization {
                        message: "dependency.id is NULL".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    })?)
                    .map_err(|e| DbError::Initialization {
                        message: format!("Invalid UUID in dependency.id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    })?,
                    blocking_item_id: Uuid::parse_str(&r.blocking_item_id).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in dependency.blocking_item_id: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    blocked_item_id: Uuid::parse_str(&r.blocked_item_id).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in dependency.blocked_item_id: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    dependency_type: DependencyType::from_str(&r.dependency_type).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid dependency_type: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    created_at: DateTime::from_timestamp(r.created_at, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in dependency.created_at".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    created_by: Uuid::parse_str(&r.created_by).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in dependency.created_by: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
                })
            })
            .collect::<DbErrorResult<Vec<_>>>()
    }

    /// Find existing dependency by pair (for duplicate check).
    /// Always filters deleted_at IS NULL.
    pub async fn find_by_pair(
        &self,
        blocking_item_id: Uuid,
        blocked_item_id: Uuid,
    ) -> DbErrorResult<Option<Dependency>> {
        let blocking_str = blocking_item_id.to_string();
        let blocked_str = blocked_item_id.to_string();

        let row = sqlx::query!(
            r#"SELECT id, blocking_item_id, blocked_item_id, dependency_type,
                        created_at, created_by, deleted_at
                 FROM pm_dependencies
                 WHERE blocking_item_id = ? AND blocked_item_id = ? AND deleted_at IS NULL"#,
            blocking_str,
            blocked_str
        )
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| -> DbErrorResult<Dependency> {
            Ok(Dependency {
                id: Uuid::parse_str(r.id.as_ref().ok_or_else(|| DbError::Initialization {
                    message: "dependency.id is NULL".to_string(),
                    location: ErrorLocation::from(Location::caller()),
                })?)
                .map_err(|e| DbError::Initialization {
                    message: format!("Invalid UUID in dependency.id: {}", e),
                    location: ErrorLocation::from(Location::caller()),
                })?,
                blocking_item_id: Uuid::parse_str(&r.blocking_item_id).map_err(|e| {
                    DbError::Initialization {
                        message: format!("Invalid UUID in dependency.blocking_item_id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                blocked_item_id: Uuid::parse_str(&r.blocked_item_id).map_err(|e| {
                    DbError::Initialization {
                        message: format!("Invalid UUID in dependency.blocked_item_id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                dependency_type: DependencyType::from_str(&r.dependency_type).map_err(|e| {
                    DbError::Initialization {
                        message: format!("Invalid dependency_type: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                created_at: DateTime::from_timestamp(r.created_at, 0).ok_or_else(|| {
                    DbError::Initialization {
                        message: "Invalid timestamp in dependency.created_at".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                created_by: Uuid::parse_str(&r.created_by).map_err(|e| {
                    DbError::Initialization {
                        message: format!("Invalid UUID in dependency.created_by: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
            })
        })
        .transpose()
    }

    /// Count how many items are blocking this item.
    /// Used for limit enforcement (max 50 blocking per item).
    pub async fn count_blocking(&self, blocked_item_id: Uuid) -> DbErrorResult<usize> {
        let id_str = blocked_item_id.to_string();
        let row = sqlx::query!(
            r#"SELECT COUNT(*) as "count: i32" FROM pm_dependencies
                 WHERE blocked_item_id = ? AND deleted_at IS NULL"#,
            id_str
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.count as usize)
    }

    /// Count how many items this item blocks.
    /// Used for limit enforcement (max 50 blocked per item).
    pub async fn count_blocked(&self, blocking_item_id: Uuid) -> DbErrorResult<usize> {
        let id_str = blocking_item_id.to_string();
        let row = sqlx::query!(
            r#"SELECT COUNT(*) as "count: i32" FROM pm_dependencies
                 WHERE blocking_item_id = ? AND deleted_at IS NULL"#,
            id_str
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.count as usize)
    }

    pub async fn delete(&self, id: Uuid, deleted_at: i64) -> DbErrorResult<()> {
        let id_str = id.to_string();

        sqlx::query!(
            r#"
              UPDATE pm_dependencies
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
}
