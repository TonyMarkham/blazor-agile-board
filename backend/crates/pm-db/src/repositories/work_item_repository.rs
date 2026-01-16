use crate::error::Result as DbErrorResult;

use pm_core::{WorkItem, WorkItemType};

use std::str::FromStr;

use chrono::DateTime;
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct WorkItemRepository {
    pool: SqlitePool,
}

impl WorkItemRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, work_item: &WorkItem) -> DbErrorResult<()> {
        let id = work_item.id.to_string();
        let item_type = work_item.item_type.as_str();
        let parent_id = work_item.parent_id.map(|id| id.to_string());
        let project_id = work_item.project_id.to_string();
        let assignee_id = work_item.assignee_id.map(|id| id.to_string());
        let sprint_id = work_item.sprint_id.map(|id| id.to_string());
        let created_at = work_item.created_at.timestamp();
        let updated_at = work_item.updated_at.timestamp();
        let created_by = work_item.created_by.to_string();
        let updated_by = work_item.updated_by.to_string();
        let deleted_at = work_item.deleted_at.map(|dt| dt.timestamp());

        sqlx::query!(
            r#"
              INSERT INTO pm_work_items (
                  id, item_type, parent_id, project_id, position,
                  title, description, status, assignee_id, sprint_id,
                  created_at, updated_at, created_by, updated_by, deleted_at
              ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
              "#,
            id,
            item_type,
            parent_id,
            project_id,
            work_item.position,
            work_item.title,
            work_item.description,
            work_item.status,
            assignee_id,
            sprint_id,
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

    pub async fn find_by_id(&self, id: Uuid) -> DbErrorResult<Option<WorkItem>> {
        let id_str = id.to_string();

        let row = sqlx::query!(
            r#"
              SELECT
                  id, item_type, parent_id, project_id, position,
                  title, description, status, assignee_id, sprint_id,
                  created_at, updated_at, created_by, updated_by, deleted_at
              FROM pm_work_items
              WHERE id = ? AND deleted_at IS NULL
              "#,
            id_str
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| WorkItem {
            id: Uuid::parse_str(r.id.as_ref().unwrap()).unwrap(),
            item_type: WorkItemType::from_str(&r.item_type).unwrap(),
            parent_id: r.parent_id.as_ref().and_then(|s| Uuid::parse_str(s).ok()),
            project_id: Uuid::parse_str(&r.project_id).unwrap(),
            position: r.position as i32,
            title: r.title,
            description: r.description,
            status: r.status,
            assignee_id: r.assignee_id.as_ref().and_then(|s| Uuid::parse_str(s).ok()),
            sprint_id: r.sprint_id.as_ref().and_then(|s| Uuid::parse_str(s).ok()),
            created_at: DateTime::from_timestamp(r.created_at, 0).unwrap(),
            updated_at: DateTime::from_timestamp(r.updated_at, 0).unwrap(),
            created_by: Uuid::parse_str(&r.created_by).unwrap(),
            updated_by: Uuid::parse_str(&r.updated_by).unwrap(),
            deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
        }))
    }

    pub async fn find_by_project(&self, project_id: Uuid) -> DbErrorResult<Vec<WorkItem>> {
        let project_id_str = project_id.to_string();

        let rows = sqlx::query!(
            r#"
              SELECT
                  id, item_type, parent_id, project_id, position,
                  title, description, status, assignee_id, sprint_id,
                  created_at, updated_at, created_by, updated_by, deleted_at
              FROM pm_work_items
              WHERE project_id = ? AND deleted_at IS NULL
              ORDER BY position
              "#,
            project_id_str
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| WorkItem {
                id: Uuid::parse_str(r.id.as_ref().unwrap()).unwrap(),
                item_type: WorkItemType::from_str(&r.item_type).unwrap(),
                parent_id: r.parent_id.as_ref().and_then(|s| Uuid::parse_str(s).ok()),
                project_id: Uuid::parse_str(&r.project_id).unwrap(),
                position: r.position as i32,
                title: r.title,
                description: r.description,
                status: r.status,
                assignee_id: r.assignee_id.as_ref().and_then(|s| Uuid::parse_str(s).ok()),
                sprint_id: r.sprint_id.as_ref().and_then(|s| Uuid::parse_str(s).ok()),
                created_at: DateTime::from_timestamp(r.created_at, 0).unwrap(),
                updated_at: DateTime::from_timestamp(r.updated_at, 0).unwrap(),
                created_by: Uuid::parse_str(&r.created_by).unwrap(),
                updated_by: Uuid::parse_str(&r.updated_by).unwrap(),
                deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
            })
            .collect())
    }

    pub async fn update(&self, work_item: &WorkItem) -> DbErrorResult<()> {
        let id = work_item.id.to_string();
        let item_type = work_item.item_type.as_str();
        let parent_id = work_item.parent_id.map(|id| id.to_string());
        let project_id = work_item.project_id.to_string();
        let assignee_id = work_item.assignee_id.map(|id| id.to_string());
        let sprint_id = work_item.sprint_id.map(|id| id.to_string());
        let updated_at = work_item.updated_at.timestamp();
        let updated_by = work_item.updated_by.to_string();

        sqlx::query!(
            r#"
              UPDATE pm_work_items
              SET item_type = ?, parent_id = ?, project_id = ?, position = ?,
                  title = ?, description = ?, status = ?, assignee_id = ?, sprint_id = ?,
                  updated_at = ?, updated_by = ?
              WHERE id = ? AND deleted_at IS NULL
              "#,
            item_type,
            parent_id,
            project_id,
            work_item.position,
            work_item.title,
            work_item.description,
            work_item.status,
            assignee_id,
            sprint_id,
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
              UPDATE pm_work_items
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
