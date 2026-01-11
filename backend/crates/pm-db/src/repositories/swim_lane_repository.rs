use crate::Result;
use chrono::DateTime;
use pm_core::SwimLane;
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SwimLaneRepository {
    pool: SqlitePool,
}

impl SwimLaneRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, lane: &SwimLane) -> Result<()> {
        let id = lane.id.to_string();
        let project_id = lane.project_id.to_string();
        let is_default = if lane.is_default { 1 } else { 0 };
        let created_at = lane.created_at.timestamp();
        let updated_at = lane.updated_at.timestamp();
        let deleted_at = lane.deleted_at.map(|dt| dt.timestamp());

        sqlx::query!(
            r#"
              INSERT INTO pm_swim_lanes (
                  id, project_id, name, status_value, position, is_default,
                  created_at, updated_at, deleted_at
              ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
              "#,
            id,
            project_id,
            lane.name,
            lane.status_value,
            lane.position,
            is_default,
            created_at,
            updated_at,
            deleted_at,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<SwimLane>> {
        let id_str = id.to_string();

        let row = sqlx::query!(
            r#"
              SELECT id, project_id, name, status_value, position, is_default,
                     created_at, updated_at, deleted_at
              FROM pm_swim_lanes
              WHERE id = ? AND deleted_at IS NULL
              "#,
            id_str
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| SwimLane {
            id: Uuid::parse_str(r.id.as_ref().unwrap()).unwrap(),
            project_id: Uuid::parse_str(&r.project_id).unwrap(),
            name: r.name,
            status_value: r.status_value,
            position: r.position as i32,
            is_default: r.is_default,
            created_at: DateTime::from_timestamp(r.created_at, 0).unwrap(),
            updated_at: DateTime::from_timestamp(r.updated_at, 0).unwrap(),
            deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
        }))
    }

    pub async fn find_by_project(&self, project_id: Uuid) -> Result<Vec<SwimLane>> {
        let project_id_str = project_id.to_string();

        let rows = sqlx::query!(
            r#"
              SELECT id, project_id, name, status_value, position, is_default,
                     created_at, updated_at, deleted_at
              FROM pm_swim_lanes
              WHERE project_id = ? AND deleted_at IS NULL
              ORDER BY position ASC
              "#,
            project_id_str
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| SwimLane {
                id: Uuid::parse_str(r.id.as_ref().unwrap()).unwrap(),
                project_id: Uuid::parse_str(&r.project_id).unwrap(),
                name: r.name,
                status_value: r.status_value,
                position: r.position as i32,
                is_default: r.is_default,
                created_at: DateTime::from_timestamp(r.created_at, 0).unwrap(),
                updated_at: DateTime::from_timestamp(r.updated_at, 0).unwrap(),
                deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
            })
            .collect())
    }

    pub async fn update(&self, lane: &SwimLane) -> Result<()> {
        let id = lane.id.to_string();
        let updated_at = lane.updated_at.timestamp();

        sqlx::query!(
            r#"
              UPDATE pm_swim_lanes
              SET name = ?, status_value = ?, position = ?, updated_at = ?
              WHERE id = ? AND deleted_at IS NULL
              "#,
            lane.name,
            lane.status_value,
            lane.position,
            updated_at,
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
              UPDATE pm_swim_lanes
              SET deleted_at = ?
              WHERE id = ? AND deleted_at IS NULL AND is_default = 0
              "#,
            deleted_at,
            id_str
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
