use crate::Result as DbErrorResult;

use pm_core::Comment;

use chrono::DateTime;
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

        Ok(row.map(|r| Comment {
            id: Uuid::parse_str(r.id.as_ref().unwrap()).unwrap(),
            work_item_id: Uuid::parse_str(&r.work_item_id).unwrap(),
            content: r.content,
            created_at: DateTime::from_timestamp(r.created_at, 0).unwrap(),
            updated_at: DateTime::from_timestamp(r.updated_at, 0).unwrap(),
            created_by: Uuid::parse_str(&r.created_by).unwrap(),
            updated_by: Uuid::parse_str(&r.updated_by).unwrap(),
            deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
        }))
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

        Ok(rows
            .into_iter()
            .map(|r| Comment {
                id: Uuid::parse_str(r.id.as_ref().unwrap()).unwrap(),
                work_item_id: Uuid::parse_str(&r.work_item_id).unwrap(),
                content: r.content,
                created_at: DateTime::from_timestamp(r.created_at, 0).unwrap(),
                updated_at: DateTime::from_timestamp(r.updated_at, 0).unwrap(),
                created_by: Uuid::parse_str(&r.created_by).unwrap(),
                updated_by: Uuid::parse_str(&r.updated_by).unwrap(),
                deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
            })
            .collect())
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
}
