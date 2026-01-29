use crate::{DbError, Result as DbErrorResult};

use pm_core::ActivityLog;

use std::panic::Location;

use chrono::DateTime;
use error_location::ErrorLocation;
use uuid::Uuid;

pub struct ActivityLogRepository;

impl ActivityLogRepository {
    pub async fn create<'e, E>(executor: E, log: &ActivityLog) -> DbErrorResult<()>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        let id = log.id.to_string();
        let entity_id = log.entity_id.to_string();
        let user_id = log.user_id.to_string();
        let timestamp = log.timestamp.timestamp();

        sqlx::query!(
            r#"
              INSERT INTO pm_activity_log (
                  id, entity_type, entity_id, action,
                  field_name, old_value, new_value,
                  user_id, timestamp, comment
              ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
              "#,
            id,
            log.entity_type,
            entity_id,
            log.action,
            log.field_name,
            log.old_value,
            log.new_value,
            user_id,
            timestamp,
            log.comment,
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn find_by_entity<'e, E>(
        executor: E,
        entity_type: &str,
        entity_id: Uuid,
    ) -> DbErrorResult<Vec<ActivityLog>>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        let entity_id_str = entity_id.to_string();

        let rows = sqlx::query!(
            r#"
                SELECT id, entity_type, entity_id, action,
                       field_name, old_value, new_value,
                       user_id, timestamp, comment
                FROM pm_activity_log
                WHERE entity_type = ? AND entity_id = ?
                ORDER BY timestamp DESC
                "#,
            entity_type,
            entity_id_str
        )
        .fetch_all(executor)
        .await?;

        rows.into_iter()
            .map(|r| -> DbErrorResult<ActivityLog> {
                Ok(ActivityLog {
                    id: Uuid::parse_str(r.id.as_ref().ok_or_else(|| DbError::Initialization {
                        message: "activity_log.id is NULL".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    })?)
                    .map_err(|e| DbError::Initialization {
                        message: format!("Invalid UUID in activity_log.id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    })?,
                    entity_type: r.entity_type,
                    entity_id: Uuid::parse_str(&r.entity_id).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in activity_log.entity_id: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    action: r.action,
                    field_name: r.field_name,
                    old_value: r.old_value,
                    new_value: r.new_value,
                    user_id: Uuid::parse_str(&r.user_id).map_err(|e| DbError::Initialization {
                        message: format!("Invalid UUID in activity_log.user_id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    })?,
                    timestamp: DateTime::from_timestamp(r.timestamp, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in activity_log.timestamp".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    comment: r.comment,
                })
            })
            .collect::<DbErrorResult<Vec<_>>>()
    }

    pub async fn find_by_user<'e, E>(
        executor: E,
        user_id: Uuid,
        limit: i64,
    ) -> DbErrorResult<Vec<ActivityLog>>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        let user_id_str = user_id.to_string();

        let rows = sqlx::query!(
            r#"
                SELECT id, entity_type, entity_id, action,
                       field_name, old_value, new_value,
                       user_id, timestamp, comment
                FROM pm_activity_log
                WHERE user_id = ?
                ORDER BY timestamp DESC
                LIMIT ?
                "#,
            user_id_str,
            limit
        )
        .fetch_all(executor)
        .await?;

        rows.into_iter()
            .map(|r| -> DbErrorResult<ActivityLog> {
                Ok(ActivityLog {
                    id: Uuid::parse_str(r.id.as_ref().ok_or_else(|| DbError::Initialization {
                        message: "activity_log.id is NULL".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    })?)
                    .map_err(|e| DbError::Initialization {
                        message: format!("Invalid UUID in activity_log.id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    })?,
                    entity_type: r.entity_type,
                    entity_id: Uuid::parse_str(&r.entity_id).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in activity_log.entity_id: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    action: r.action,
                    field_name: r.field_name,
                    old_value: r.old_value,
                    new_value: r.new_value,
                    user_id: Uuid::parse_str(&r.user_id).map_err(|e| DbError::Initialization {
                        message: format!("Invalid UUID in activity_log.user_id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    })?,
                    timestamp: DateTime::from_timestamp(r.timestamp, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in activity_log.timestamp".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    comment: r.comment,
                })
            })
            .collect::<DbErrorResult<Vec<_>>>()
    }

    pub async fn find_by_entity_paginated<'e, E>(
        executor: &'e E,
        entity_type: &str,
        entity_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> DbErrorResult<(Vec<ActivityLog>, i64)>
    where
        &'e E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        let entity_id_str = entity_id.to_string();

        let total_count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!: i64"
            FROM pm_activity_log
            WHERE entity_type = ? AND entity_id = ?
            "#,
            entity_type,
            entity_id_str
        )
        .fetch_one(executor)
        .await?;

        let rows = sqlx::query!(
            r#"
            SELECT id, entity_type, entity_id, action,
                   field_name, old_value, new_value,
                   user_id, timestamp, comment
            FROM pm_activity_log
            WHERE entity_type = ? AND entity_id = ?
            ORDER BY timestamp DESC
            LIMIT ? OFFSET ?
            "#,
            entity_type,
            entity_id_str,
            limit,
            offset
        )
        .fetch_all(executor)
        .await?;

        let logs = rows
            .into_iter()
            .map(|r| {
                Ok(ActivityLog {
                    id: Uuid::parse_str(r.id.as_ref().ok_or_else(|| DbError::Initialization {
                        message: "activity_log.id is NULL".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    })?)
                    .map_err(|e| DbError::Initialization {
                        message: format!("Invalid UUID in activity_log.id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    })?,
                    entity_type: r.entity_type,
                    entity_id: Uuid::parse_str(&r.entity_id).map_err(|e| {
                        DbError::Initialization {
                            message: format!("Invalid UUID in activity_log.entity_id: {}", e),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    action: r.action,
                    field_name: r.field_name,
                    old_value: r.old_value,
                    new_value: r.new_value,
                    user_id: Uuid::parse_str(&r.user_id).map_err(|e| DbError::Initialization {
                        message: format!("Invalid UUID in activity_log.user_id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    })?,
                    timestamp: DateTime::from_timestamp(r.timestamp, 0).ok_or_else(|| {
                        DbError::Initialization {
                            message: "Invalid timestamp in activity_log.timestamp".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    comment: r.comment,
                })
            })
            .collect::<DbErrorResult<Vec<_>>>()?;

        Ok((logs, total_count))
    }

    /// Delete activity older than the given cutoff date (retention policy)
    pub async fn delete_older_than<'e, E>(
        executor: E,
        cutoff: DateTime<chrono::Utc>,
    ) -> DbErrorResult<u64>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        let cutoff_ts = cutoff.timestamp();

        let result = sqlx::query!(
            r#"
            DELETE FROM pm_activity_log
            WHERE timestamp < ?
            "#,
            cutoff_ts
        )
        .execute(executor)
        .await?;

        Ok(result.rows_affected())
    }
}
