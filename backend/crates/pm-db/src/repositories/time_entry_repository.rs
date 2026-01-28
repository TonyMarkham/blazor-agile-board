use crate::{DbError, Result as DbErrorResult};

use pm_core::TimeEntry;

use std::panic::Location;

use chrono::DateTime;
use error_location::ErrorLocation;
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct TimeEntryRepository {
    pool: SqlitePool,
}

impl TimeEntryRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, entry: &TimeEntry) -> DbErrorResult<()> {
        let id = entry.id.to_string();
        let work_item_id = entry.work_item_id.to_string();
        let user_id = entry.user_id.to_string();
        let started_at = entry.started_at.timestamp();
        let ended_at = entry.ended_at.map(|dt| dt.timestamp());
        let created_at = entry.created_at.timestamp();
        let updated_at = entry.updated_at.timestamp();
        let deleted_at = entry.deleted_at.map(|dt| dt.timestamp());

        sqlx::query!(
            r#"
              INSERT INTO pm_time_entries (
                  id, work_item_id, user_id,
                  started_at, ended_at, duration_seconds, description,
                  created_at, updated_at, deleted_at
              ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
              "#,
            id,
            work_item_id,
            user_id,
            started_at,
            ended_at,
            entry.duration_seconds,
            entry.description,
            created_at,
            updated_at,
            deleted_at,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn find_by_id(&self, id: Uuid) -> DbErrorResult<Option<TimeEntry>> {
        let id_str = id.to_string();

        let row = sqlx::query!(
            r#"
                SELECT id, work_item_id, user_id,
                       started_at, ended_at, duration_seconds, description,
                       created_at, updated_at, deleted_at
                FROM pm_time_entries
                WHERE id = ? AND deleted_at IS NULL
                "#,
            id_str
        )
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| -> DbErrorResult<TimeEntry> {
            Ok(TimeEntry {
                id: Uuid::parse_str(r.id.as_ref().ok_or_else(|| DbError::Initialization {
                    message: "time_entry.id is NULL".to_string(),
                    location: ErrorLocation::from(Location::caller()),
                })?)
                .map_err(|e| DbError::Initialization {
                    message: format!("Invalid UUID in time_entry.id: {}", e),
                    location: ErrorLocation::from(Location::caller()),
                })?,
                work_item_id: Uuid::parse_str(&r.work_item_id).map_err(|e| {
                    DbError::Initialization {
                        message: format!("Invalid UUID in time_entry.work_item_id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                user_id: Uuid::parse_str(&r.user_id).map_err(|e| DbError::Initialization {
                    message: format!("Invalid UUID in time_entry.user_id: {}", e),
                    location: ErrorLocation::from(Location::caller()),
                })?,
                started_at: DateTime::from_timestamp(r.started_at, 0).ok_or_else(|| {
                    DbError::Initialization {
                        message: "Invalid timestamp in time_entry.started_at".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                ended_at: r.ended_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
                duration_seconds: r.duration_seconds.map(|d| d as i32),
                description: r.description,
                created_at: DateTime::from_timestamp(r.created_at, 0).ok_or_else(|| {
                    DbError::Initialization {
                        message: "Invalid timestamp in time_entry.created_at".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                updated_at: DateTime::from_timestamp(r.updated_at, 0).ok_or_else(|| {
                    DbError::Initialization {
                        message: "Invalid timestamp in time_entry.updated_at".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    }
                })?,
                deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
            })
        })
        .transpose()
    }

    pub async fn find_by_work_item(&self, work_item_id: Uuid) -> DbErrorResult<Vec<TimeEntry>> {
        let work_item_id_str = work_item_id.to_string();

        let rows = sqlx::query!(
            r#"
                SELECT id, work_item_id, user_id,
                       started_at, ended_at, duration_seconds, description,
                       created_at, updated_at, deleted_at
                FROM pm_time_entries
                WHERE work_item_id = ? AND deleted_at IS NULL
                ORDER BY started_at DESC
                "#,
            work_item_id_str
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| -> DbErrorResult<TimeEntry> {
                Ok(TimeEntry {
                    id: Uuid::parse_str(r.id.as_ref().ok_or_else(|| DbError::Initialization {
                        message: "time_entry.id is NULL".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    })?)
                    .map_err(|e| DbError::Initialization {
                        message: format!("Invalid UUID in time_entry.id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    })?,
                    work_item_id: Uuid::parse_str(&r.work_item_id).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in time_entry.work_item_id: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    user_id: Uuid::parse_str(&r.user_id).map_err(|e| DbError::Initialization {
                        message: format!("Invalid UUID in time_entry.user_id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    })?,
                    started_at: DateTime::from_timestamp(r.started_at, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in time_entry.started_at".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    ended_at: r.ended_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
                    duration_seconds: r.duration_seconds.map(|d| d as i32),
                    description: r.description,
                    created_at: DateTime::from_timestamp(r.created_at, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in time_entry.created_at".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    updated_at: DateTime::from_timestamp(r.updated_at, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in time_entry.updated_at".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
                })
            })
            .collect::<DbErrorResult<Vec<_>>>()
    }

    /// Find time entries for a work item with pagination.
    /// Returns (entries, total_count) for pagination UI.
    /// Always filters deleted_at IS NULL.
    /// Orders by started_at DESC (most recent first).
    pub async fn find_by_work_item_paginated(
        &self,
        work_item_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> DbErrorResult<(Vec<TimeEntry>, i32)> {
        let work_item_str = work_item_id.to_string();

        // Get total count (for pagination UI)
        let count_row = sqlx::query!(
            r#"SELECT COUNT(*) as "count: i32" FROM pm_time_entries
                 WHERE work_item_id = ? AND deleted_at IS NULL"#,
            work_item_str
        )
        .fetch_one(&self.pool)
        .await?;
        let total_count = count_row.count;

        // Get paginated entries
        let rows = sqlx::query!(
            r#"SELECT id, work_item_id, user_id, started_at, ended_at,
                        duration_seconds, description, created_at, updated_at, deleted_at
                 FROM pm_time_entries
                 WHERE work_item_id = ? AND deleted_at IS NULL
                 ORDER BY started_at DESC
                 LIMIT ? OFFSET ?"#,
            work_item_str,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        let entries = rows
            .into_iter()
            .map(|row| -> DbErrorResult<TimeEntry> {
                Ok(TimeEntry {
                    id: Uuid::parse_str(row.id.as_ref().ok_or_else(|| {
                        DbError::Initialization {
                            message: "time_entry.id is NULL".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?)
                    .map_err(|e| DbError::Initialization {
                        message: format!("Invalid UUID in time_entry.id: {e}"),
                        location: ErrorLocation::from(Location::caller()),
                    })?,
                    work_item_id: Uuid::parse_str(&row.work_item_id).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in time_entry.work_item_id: {e}"),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    user_id: Uuid::parse_str(&row.user_id).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in time_entry.user_id: {e}"),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    started_at: DateTime::from_timestamp(row.started_at, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in time_entry.started_at".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    ended_at: row.ended_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
                    duration_seconds: row.duration_seconds.map(|d| d as i32),
                    description: row.description,
                    created_at: DateTime::from_timestamp(row.created_at, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in time_entry.created_at".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    updated_at: DateTime::from_timestamp(row.updated_at, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in time_entry.updated_at".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    deleted_at: row
                        .deleted_at
                        .and_then(|ts| DateTime::from_timestamp(ts, 0)),
                })
            })
            .collect::<DbErrorResult<Vec<_>>>()?;

        Ok((entries, total_count))
    }

    pub async fn find_running(&self, user_id: Uuid) -> DbErrorResult<Vec<TimeEntry>> {
        let user_id_str = user_id.to_string();

        let rows = sqlx::query!(
            r#"
                SELECT id, work_item_id, user_id,
                       started_at, ended_at, duration_seconds, description,
                       created_at, updated_at, deleted_at
                FROM pm_time_entries
                WHERE user_id = ? AND ended_at IS NULL AND deleted_at IS NULL
                "#,
            user_id_str
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| -> DbErrorResult<TimeEntry> {
                Ok(TimeEntry {
                    id: Uuid::parse_str(r.id.as_ref().ok_or_else(|| DbError::Initialization {
                        message: "time_entry.id is NULL".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    })?)
                    .map_err(|e| DbError::Initialization {
                        message: format!("Invalid UUID in time_entry.id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    })?,
                    work_item_id: Uuid::parse_str(&r.work_item_id).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in time_entry.work_item_id: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    user_id: Uuid::parse_str(&r.user_id).map_err(|e| DbError::Initialization {
                        message: format!("Invalid UUID in time_entry.user_id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    })?,
                    started_at: DateTime::from_timestamp(r.started_at, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in time_entry.started_at".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    ended_at: r.ended_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
                    duration_seconds: r.duration_seconds.map(|d| d as i32),
                    description: r.description,
                    created_at: DateTime::from_timestamp(r.created_at, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in time_entry.created_at".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    updated_at: DateTime::from_timestamp(r.updated_at, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in time_entry.updated_at".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
                })
            })
            .collect::<DbErrorResult<Vec<_>>>()
    }

    pub async fn update(&self, entry: &TimeEntry) -> DbErrorResult<()> {
        let id = entry.id.to_string();
        let ended_at = entry.ended_at.map(|dt| dt.timestamp());
        let updated_at = entry.updated_at.timestamp();

        sqlx::query!(
            r#"
              UPDATE pm_time_entries
              SET ended_at = ?, duration_seconds = ?, description = ?, updated_at = ?
              WHERE id = ? AND deleted_at IS NULL
              "#,
            ended_at,
            entry.duration_seconds,
            entry.description,
            updated_at,
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
              UPDATE pm_time_entries
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
