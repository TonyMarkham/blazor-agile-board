use crate::Result as DbErrorResult;

use pm_core::TimeEntry;

use chrono::DateTime;
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

        Ok(row.map(|r| TimeEntry {
            id: Uuid::parse_str(r.id.as_ref().unwrap()).unwrap(),
            work_item_id: Uuid::parse_str(&r.work_item_id).unwrap(),
            user_id: Uuid::parse_str(&r.user_id).unwrap(),
            started_at: DateTime::from_timestamp(r.started_at, 0).unwrap(),
            ended_at: r.ended_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
            duration_seconds: r.duration_seconds.map(|d| d as i32),
            description: r.description,
            created_at: DateTime::from_timestamp(r.created_at, 0).unwrap(),
            updated_at: DateTime::from_timestamp(r.updated_at, 0).unwrap(),
            deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
        }))
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

        Ok(rows
            .into_iter()
            .map(|r| TimeEntry {
                id: Uuid::parse_str(r.id.as_ref().unwrap()).unwrap(),
                work_item_id: Uuid::parse_str(&r.work_item_id).unwrap(),
                user_id: Uuid::parse_str(&r.user_id).unwrap(),
                started_at: DateTime::from_timestamp(r.started_at, 0).unwrap(),
                ended_at: r.ended_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
                duration_seconds: r.duration_seconds.map(|d| d as i32),
                description: r.description,
                created_at: DateTime::from_timestamp(r.created_at, 0).unwrap(),
                updated_at: DateTime::from_timestamp(r.updated_at, 0).unwrap(),
                deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
            })
            .collect())
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

        Ok(rows
            .into_iter()
            .map(|r| TimeEntry {
                id: Uuid::parse_str(r.id.as_ref().unwrap()).unwrap(),
                work_item_id: Uuid::parse_str(&r.work_item_id).unwrap(),
                user_id: Uuid::parse_str(&r.user_id).unwrap(),
                started_at: DateTime::from_timestamp(r.started_at, 0).unwrap(),
                ended_at: r.ended_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
                duration_seconds: r.duration_seconds.map(|d| d as i32),
                description: r.description,
                created_at: DateTime::from_timestamp(r.created_at, 0).unwrap(),
                updated_at: DateTime::from_timestamp(r.updated_at, 0).unwrap(),
                deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
            })
            .collect())
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
