use crate::Result as DbErrorResult;

use pm_core::ActivityLog;

use chrono::DateTime;
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
