use crate::Result;
use chrono::DateTime;
use pm_core::{Sprint, SprintStatus};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SprintRepository {
    pool: SqlitePool,
}

impl SprintRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, sprint: &Sprint) -> Result<()> {
        let id = sprint.id.to_string();
        let project_id = sprint.project_id.to_string();
        let status = sprint.status.as_str();
        let start_date = sprint.start_date.timestamp();
        let end_date = sprint.end_date.timestamp();
        let created_at = sprint.created_at.timestamp();
        let updated_at = sprint.updated_at.timestamp();
        let created_by = sprint.created_by.to_string();
        let updated_by = sprint.updated_by.to_string();
        let deleted_at = sprint.deleted_at.map(|dt| dt.timestamp());

        sqlx::query!(
            r#"
              INSERT INTO pm_sprints (
                  id, project_id, name, goal,
                  start_date, end_date, status,
                  created_at, updated_at, created_by, updated_by, deleted_at
              ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
              "#,
            id,
            project_id,
            sprint.name,
            sprint.goal,
            start_date,
            end_date,
            status,
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

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Sprint>> {
        let id_str = id.to_string();

        let row = sqlx::query!(
            r#"
              SELECT id, project_id, name, goal, start_date, end_date, status,
                     created_at, updated_at, created_by, updated_by, deleted_at
              FROM pm_sprints
              WHERE id = ? AND deleted_at IS NULL
              "#,
            id_str
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Sprint {
            id: Uuid::parse_str(r.id.as_ref().unwrap()).unwrap(),
            project_id: Uuid::parse_str(&r.project_id).unwrap(),
            name: r.name,
            goal: r.goal,
            start_date: DateTime::from_timestamp(r.start_date, 0).unwrap(),
            end_date: DateTime::from_timestamp(r.end_date, 0).unwrap(),
            status: SprintStatus::from_str(&r.status).unwrap(),
            created_at: DateTime::from_timestamp(r.created_at, 0).unwrap(),
            updated_at: DateTime::from_timestamp(r.updated_at, 0).unwrap(),
            created_by: Uuid::parse_str(&r.created_by).unwrap(),
            updated_by: Uuid::parse_str(&r.updated_by).unwrap(),
            deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
        }))
    }

    pub async fn find_by_project(&self, project_id: Uuid) -> Result<Vec<Sprint>> {
        let project_id_str = project_id.to_string();

        let rows = sqlx::query!(
            r#"
              SELECT id, project_id, name, goal, start_date, end_date, status,
                     created_at, updated_at, created_by, updated_by, deleted_at
              FROM pm_sprints
              WHERE project_id = ? AND deleted_at IS NULL
              ORDER BY start_date DESC
              "#,
            project_id_str
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| Sprint {
                id: Uuid::parse_str(r.id.as_ref().unwrap()).unwrap(),
                project_id: Uuid::parse_str(&r.project_id).unwrap(),
                name: r.name,
                goal: r.goal,
                start_date: DateTime::from_timestamp(r.start_date, 0).unwrap(),
                end_date: DateTime::from_timestamp(r.end_date, 0).unwrap(),
                status: SprintStatus::from_str(&r.status).unwrap(),
                created_at: DateTime::from_timestamp(r.created_at, 0).unwrap(),
                updated_at: DateTime::from_timestamp(r.updated_at, 0).unwrap(),
                created_by: Uuid::parse_str(&r.created_by).unwrap(),
                updated_by: Uuid::parse_str(&r.updated_by).unwrap(),
                deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
            })
            .collect())
    }

    pub async fn update(&self, sprint: &Sprint) -> Result<()> {
        let id = sprint.id.to_string();
        let project_id = sprint.project_id.to_string();
        let status = sprint.status.as_str();
        let start_date = sprint.start_date.timestamp();
        let end_date = sprint.end_date.timestamp();
        let updated_at = sprint.updated_at.timestamp();
        let updated_by = sprint.updated_by.to_string();

        sqlx::query!(
            r#"
              UPDATE pm_sprints
              SET project_id = ?, name = ?, goal = ?,
                  start_date = ?, end_date = ?, status = ?,
                  updated_at = ?, updated_by = ?
              WHERE id = ? AND deleted_at IS NULL
              "#,
            project_id,
            sprint.name,
            sprint.goal,
            start_date,
            end_date,
            status,
            updated_at,
            updated_by,
            id,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete(&self, id: Uuid, deleted_at: i64) -> Result<()> {
        let id_str = id.to_string();

        sqlx::query!(
            r#"
              UPDATE pm_sprints
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
