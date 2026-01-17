use crate::error::Result as DbErrorResult;

use pm_core::{WorkItem, WorkItemType};

use std::str::FromStr;

use chrono::DateTime;
use uuid::Uuid;

pub struct WorkItemRepository;

impl WorkItemRepository {
    pub async fn create<'e, E>(executor: E, work_item: &WorkItem) -> DbErrorResult<()>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
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
                  title, description, status, priority, assignee_id,
                  story_points, sprint_id, version,
                  created_at, updated_at, created_by, updated_by, deleted_at
              ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
              "#,
            id,
            item_type,
            parent_id,
            project_id,
            work_item.position,
            work_item.title,
            work_item.description,
            work_item.status,
            work_item.priority,
            assignee_id,
            work_item.story_points,
            sprint_id,
            work_item.version,
            created_at,
            updated_at,
            created_by,
            updated_by,
            deleted_at,
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn find_by_id<'e, E>(executor: E, id: Uuid) -> DbErrorResult<Option<WorkItem>>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        let id_str = id.to_string();

        let row = sqlx::query!(
            r#"
              SELECT
                  id, item_type, parent_id, project_id, position,
                  title, description, status, priority, assignee_id,
                  story_points, sprint_id, version,
                  created_at, updated_at, created_by, updated_by, deleted_at
              FROM pm_work_items
              WHERE id = ? AND deleted_at IS NULL
              "#,
            id_str
        )
        .fetch_optional(executor)
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
            priority: r.priority,
            assignee_id: r.assignee_id.as_ref().and_then(|s| Uuid::parse_str(s).ok()),
            story_points: r.story_points.map(|sp| sp as i32),
            sprint_id: r.sprint_id.as_ref().and_then(|s| Uuid::parse_str(s).ok()),
            version: r.version as i32,
            created_at: DateTime::from_timestamp(r.created_at, 0).unwrap(),
            updated_at: DateTime::from_timestamp(r.updated_at, 0).unwrap(),
            created_by: Uuid::parse_str(&r.created_by).unwrap(),
            updated_by: Uuid::parse_str(&r.updated_by).unwrap(),
            deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
        }))
    }

    pub async fn find_by_project<'e, E>(
        executor: E,
        project_id: Uuid,
    ) -> DbErrorResult<Vec<WorkItem>>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        let project_id_str = project_id.to_string();

        let rows = sqlx::query!(
            r#"
              SELECT
                  id, item_type, parent_id, project_id, position,
                  title, description, status, priority, assignee_id,
                  story_points, sprint_id, version,
                  created_at, updated_at, created_by, updated_by, deleted_at
              FROM pm_work_items
              WHERE project_id = ? AND deleted_at IS NULL
              ORDER BY position
              "#,
            project_id_str
        )
        .fetch_all(executor)
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
                priority: r.priority,
                assignee_id: r.assignee_id.as_ref().and_then(|s| Uuid::parse_str(s).ok()),
                story_points: r.story_points.map(|sp| sp as i32),
                sprint_id: r.sprint_id.as_ref().and_then(|s| Uuid::parse_str(s).ok()),
                version: r.version as i32,
                created_at: DateTime::from_timestamp(r.created_at, 0).unwrap(),
                updated_at: DateTime::from_timestamp(r.updated_at, 0).unwrap(),
                created_by: Uuid::parse_str(&r.created_by).unwrap(),
                updated_by: Uuid::parse_str(&r.updated_by).unwrap(),
                deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
            })
            .collect())
    }

    pub async fn update<'e, E>(executor: E, work_item: &WorkItem) -> DbErrorResult<()>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
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
                  title = ?, description = ?, status = ?, priority = ?, assignee_id = ?,
                  story_points = ?, sprint_id = ?, version = ?,
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
            work_item.priority,
            assignee_id,
            work_item.story_points,
            sprint_id,
            work_item.version,
            updated_at,
            updated_by,
            id,
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn soft_delete<'e, E>(executor: E, id: Uuid, user_id: Uuid) -> DbErrorResult<()>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        let id_str = id.to_string();
        let user_id_str = user_id.to_string();
        let deleted_at = chrono::Utc::now().timestamp();

        sqlx::query!(
            r#"
              UPDATE pm_work_items
              SET deleted_at = ?, updated_by = ?, updated_at = ?
              WHERE id = ? AND deleted_at IS NULL
              "#,
            deleted_at,
            user_id_str,
            deleted_at,
            id_str
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn find_children<'e, E>(executor: E, parent_id: Uuid) -> DbErrorResult<Vec<WorkItem>>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        let parent_id_str = parent_id.to_string();

        let rows = sqlx::query!(
            r#"
                SELECT
                    id, item_type, parent_id, project_id, position,
                    title, description, status, priority, assignee_id,
                    story_points, sprint_id, version,
                    created_at, updated_at, created_by, updated_by, deleted_at
                FROM pm_work_items
                WHERE parent_id = ? AND deleted_at IS NULL
                "#,
            parent_id_str
        )
        .fetch_all(executor)
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
                priority: r.priority,
                assignee_id: r.assignee_id.as_ref().and_then(|s| Uuid::parse_str(s).ok()),
                story_points: r.story_points.map(|sp| sp as i32),
                sprint_id: r.sprint_id.as_ref().and_then(|s| Uuid::parse_str(s).ok()),
                version: r.version as i32,
                created_at: DateTime::from_timestamp(r.created_at, 0).unwrap(),
                updated_at: DateTime::from_timestamp(r.updated_at, 0).unwrap(),
                created_by: Uuid::parse_str(&r.created_by).unwrap(),
                updated_by: Uuid::parse_str(&r.updated_by).unwrap(),
                deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
            })
            .collect())
    }

    pub async fn find_max_position<'e, E>(
        executor: E,
        project_id: Uuid,
        parent_id: Option<Uuid>,
    ) -> DbErrorResult<i32>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        let project_id_str = project_id.to_string();
        let parent_id_str = parent_id.map(|id| id.to_string());

        let result = sqlx::query_scalar!(
            r#"
                SELECT COALESCE(MAX(position), 0) as "max_position!"
                FROM pm_work_items
                WHERE project_id = ? AND parent_id IS ? AND deleted_at IS NULL
                "#,
            project_id_str,
            parent_id_str
        )
        .fetch_one(executor)
        .await?;

        Ok(result as i32)
    }

    pub async fn find_by_project_since<'e, E>(
        executor: E,
        project_id: Uuid,
        since_timestamp: i64,
    ) -> DbErrorResult<Vec<WorkItem>>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        let project_id_str = project_id.to_string();

        let rows = sqlx::query!(
            r#"
                SELECT
                    id, item_type, parent_id, project_id, position,
                    title, description, status, priority, assignee_id,
                    story_points, sprint_id, version,
                    created_at, updated_at, created_by, updated_by, deleted_at
                FROM pm_work_items
                WHERE project_id = ? AND updated_at > ?
                ORDER BY position
                "#,
            project_id_str,
            since_timestamp
        )
        .fetch_all(executor)
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
                priority: r.priority,
                assignee_id: r.assignee_id.as_ref().and_then(|s| Uuid::parse_str(s).ok()),
                story_points: r.story_points.map(|sp| sp as i32),
                sprint_id: r.sprint_id.as_ref().and_then(|s| Uuid::parse_str(s).ok()),
                version: r.version as i32,
                created_at: DateTime::from_timestamp(r.created_at, 0).unwrap(),
                updated_at: DateTime::from_timestamp(r.updated_at, 0).unwrap(),
                created_by: Uuid::parse_str(&r.created_by).unwrap(),
                updated_by: Uuid::parse_str(&r.updated_by).unwrap(),
                deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
            })
            .collect())
    }
}
