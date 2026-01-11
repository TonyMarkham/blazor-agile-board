use crate::Result;
use chrono::DateTime;
use pm_core::ActivityLog;
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct ActivityLogRepository {
    pool: SqlitePool,
}

impl ActivityLogRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, log: &ActivityLog) -> Result<()> {
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
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn find_by_entity(
        &self,
        entity_type: &str,
        entity_id: Uuid,
    ) -> Result<Vec<ActivityLog>> {
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
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ActivityLog {
                id: Uuid::parse_str(r.id.as_ref().unwrap()).unwrap(),
                entity_type: r.entity_type,
                entity_id: Uuid::parse_str(&r.entity_id).unwrap(),
                action: r.action,
                field_name: r.field_name,
                old_value: r.old_value,
                new_value: r.new_value,
                user_id: Uuid::parse_str(&r.user_id).unwrap(),
                timestamp: DateTime::from_timestamp(r.timestamp, 0).unwrap(),
                comment: r.comment,
            })
            .collect())
    }

    pub async fn find_by_user(&self, user_id: Uuid, limit: i64) -> Result<Vec<ActivityLog>> {
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
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ActivityLog {
                id: Uuid::parse_str(r.id.as_ref().unwrap()).unwrap(),
                entity_type: r.entity_type,
                entity_id: Uuid::parse_str(&r.entity_id).unwrap(),
                action: r.action,
                field_name: r.field_name,
                old_value: r.old_value,
                new_value: r.new_value,
                user_id: Uuid::parse_str(&r.user_id).unwrap(),
                timestamp: DateTime::from_timestamp(r.timestamp, 0).unwrap(),
                comment: r.comment,
            })
            .collect())
    }
}
