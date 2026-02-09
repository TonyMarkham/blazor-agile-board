use crate::{DbError, Result as DbErrorResult};

use pm_core::Comment;

use std::panic::Location;

use chrono::DateTime;
use error_location::ErrorLocation;
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct CommentRepository {
    pool: SqlitePool,
}

impl CommentRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, comment: &Comment) -> DbErrorResult<()> {
        let id = comment.id.to_string();
        let work_item_id = comment.work_item_id.to_string();
        let created_at = comment.created_at.timestamp();
        let updated_at = comment.updated_at.timestamp();
        let created_by = comment.created_by.to_string();
        let updated_by = comment.updated_by.to_string();
        let deleted_at = comment.deleted_at.map(|dt| dt.timestamp());

        sqlx::query!(
            r#"
              INSERT INTO pm_comments (
                  id, work_item_id, content,
                  created_at, updated_at, created_by, updated_by, deleted_at
              ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
              "#,
            id,
            work_item_id,
            comment.content,
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

    pub async fn find_by_id(&self, id: Uuid) -> DbErrorResult<Option<Comment>> {
        let id_str = id.to_string();

        let row = sqlx::query!(
            r#"
                SELECT id, work_item_id, content,
                       created_at, updated_at, created_by, updated_by, deleted_at
                FROM pm_comments
                WHERE id = ? AND deleted_at IS NULL
                "#,
            id_str
        )
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| -> DbErrorResult<Comment> {
            Ok(Comment {
                id: Uuid::parse_str(r.id.as_ref().ok_or_else(|| DbError::Initialization {
                    message: "comment.id is NULL".to_string(),
                    location: ErrorLocation::from(Location::caller()),
                })?)
                .map_err(|e| DbError::Initialization {
                    message: format!("Invalid UUID in comment.id: {}", e),
                    location: ErrorLocation::from(Location::caller()),
                })?,
                work_item_id: Uuid::parse_str(&r.work_item_id).map_err(|e| {
                    DbError::Initialization {
                        message: format!("Invalid UUID in comment.work_item_id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                content: r.content,
                created_at: DateTime::from_timestamp(r.created_at, 0).ok_or_else(|| {
                    DbError::Initialization {
                        message: "Invalid timestamp in comment.created_at".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                updated_at: DateTime::from_timestamp(r.updated_at, 0).ok_or_else(|| {
                    DbError::Initialization {
                        message: "Invalid timestamp in comment.updated_at".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                created_by: Uuid::parse_str(&r.created_by).map_err(|e| {
                    DbError::Initialization {
                        message: format!("Invalid UUID in comment.created_by: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                updated_by: Uuid::parse_str(&r.updated_by).map_err(|e| {
                    DbError::Initialization {
                        message: format!("Invalid UUID in comment.updated_by: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
            })
        })
        .transpose()
    }

    pub async fn find_by_work_item(&self, work_item_id: Uuid) -> DbErrorResult<Vec<Comment>> {
        let work_item_id_str = work_item_id.to_string();

        let rows = sqlx::query!(
            r#"
                SELECT id, work_item_id, content,
                       created_at, updated_at, created_by, updated_by, deleted_at
                FROM pm_comments
                WHERE work_item_id = ? AND deleted_at IS NULL
                ORDER BY created_at ASC
                "#,
            work_item_id_str
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| -> DbErrorResult<Comment> {
                Ok(Comment {
                    id: Uuid::parse_str(r.id.as_ref().ok_or_else(|| DbError::Initialization {
                        message: "comment.id is NULL".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    })?)
                    .map_err(|e| DbError::Initialization {
                        message: format!("Invalid UUID in comment.id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    })?,
                    work_item_id: Uuid::parse_str(&r.work_item_id).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in comment.work_item_id: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    content: r.content,
                    created_at: DateTime::from_timestamp(r.created_at, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in comment.created_at".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    updated_at: DateTime::from_timestamp(r.updated_at, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in comment.updated_at".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    created_by: Uuid::parse_str(&r.created_by).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in comment.created_by: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    updated_by: Uuid::parse_str(&r.updated_by).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in comment.updated_by: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
                })
            })
            .collect::<DbErrorResult<Vec<_>>>()
    }

    pub async fn update(&self, comment: &Comment) -> DbErrorResult<()> {
        let id = comment.id.to_string();
        let updated_at = comment.updated_at.timestamp();
        let updated_by = comment.updated_by.to_string();

        sqlx::query!(
            r#"
              UPDATE pm_comments
              SET content = ?, updated_at = ?, updated_by = ?
              WHERE id = ? AND deleted_at IS NULL
              "#,
            comment.content,
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
              UPDATE pm_comments
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

    pub async fn find_all(&self) -> DbErrorResult<Vec<Comment>> {
        let rows = sqlx::query!(
            r#"
              SELECT id, work_item_id, content,
                     created_at, updated_at, created_by, updated_by, deleted_at
              FROM pm_comments
              WHERE deleted_at IS NULL
              ORDER BY created_at ASC
          "#
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| -> DbErrorResult<Comment> {
                Ok(Comment {
                    id: Uuid::parse_str(r.id.as_ref().ok_or_else(|| DbError::Initialization {
                        message: "comment.id is NULL".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    })?)
                    .map_err(|e| DbError::Initialization {
                        message: format!("Invalid UUID in comment.id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    })?,
                    work_item_id: Uuid::parse_str(&r.work_item_id).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in comment.work_item_id: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    content: r.content,
                    created_at: DateTime::from_timestamp(r.created_at, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in comment.created_at".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    updated_at: DateTime::from_timestamp(r.updated_at, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in comment.updated_at".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    created_by: Uuid::parse_str(&r.created_by).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in comment.created_by: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    updated_by: Uuid::parse_str(&r.updated_by).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in comment.updated_by: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
                })
            })
            .collect::<DbErrorResult<Vec<_>>>()
    }
}
