use crate::error::Result as DbErrorResult;

use chrono::Utc;
use sqlx::SqlitePool;

pub struct IdempotencyRepository {
    pool: SqlitePool,
}

impl IdempotencyRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn find_by_message_id(&self, message_id: &str) -> DbErrorResult<Option<String>> {
        let result = sqlx::query_scalar!(
            r#"
              SELECT result_json
              FROM pm_idempotency_keys
              WHERE message_id = ?
              "#,
            message_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    pub async fn create(
        &self,
        message_id: &str,
        operation: &str,
        result_json: &str,
    ) -> DbErrorResult<()> {
        let created_at = Utc::now().timestamp();

        sqlx::query!(
            r#"
              INSERT INTO pm_idempotency_keys (message_id, operation, result_json, created_at)
              VALUES (?, ?, ?, ?)
              ON CONFLICT(message_id) DO NOTHING
              "#,
            message_id,
            operation,
            result_json,
            created_at
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn cleanup_old_entries(&self, max_age_seconds: i64) -> DbErrorResult<u64> {
        let cutoff = Utc::now().timestamp() - max_age_seconds;

        let result = sqlx::query!(
            "DELETE FROM pm_idempotency_keys WHERE created_at < ?",
            cutoff
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}
