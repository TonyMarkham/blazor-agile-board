use crate::Result as DbErrorResult;

use pm_core::{Project, ProjectStatus};

use std::str::FromStr;

use chrono::DateTime;
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct ProjectRepository {
    pool: SqlitePool,
}

impl ProjectRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, project: &Project) -> DbErrorResult<()> {
        let id = project.id.to_string();
        let status = project.status.as_str();
        let created_at = project.created_at.timestamp();
        let updated_at = project.updated_at.timestamp();
        let created_by = project.created_by.to_string();
        let updated_by = project.updated_by.to_string();
        let deleted_at = project.deleted_at.map(|dt| dt.timestamp());

        sqlx::query!(
            r#"
                INSERT INTO pm_projects (
                    id, title, description, key, status, version,
                    created_at, updated_at, created_by, updated_by, deleted_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            id,
            project.title,
            project.description,
            project.key,
            status,
            project.version,
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

    pub async fn find_by_id(&self, id: Uuid) -> DbErrorResult<Option<Project>> {
        let id_str = id.to_string();

        let row = sqlx::query!(
            r#"
                SELECT id, title, description, key, status, version,
                       created_at, updated_at, created_by, updated_by, deleted_at
                FROM pm_projects
                WHERE id = ? AND deleted_at IS NULL
                "#,
            id_str
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Project {
            id: Uuid::parse_str(r.id.as_ref().unwrap()).unwrap(),
            title: r.title,
            description: r.description,
            key: r.key,
            status: ProjectStatus::from_str(&r.status).unwrap(),
            version: r.version as i32,
            created_at: DateTime::from_timestamp(r.created_at, 0).unwrap(),
            updated_at: DateTime::from_timestamp(r.updated_at, 0).unwrap(),
            created_by: Uuid::parse_str(&r.created_by).unwrap(),
            updated_by: Uuid::parse_str(&r.updated_by).unwrap(),
            deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
        }))
    }

    pub async fn find_by_key(&self, key: &str) -> DbErrorResult<Option<Project>> {
        let row = sqlx::query!(
            r#"
                SELECT id, title, description, key, status, version,
                       created_at, updated_at, created_by, updated_by, deleted_at
                FROM pm_projects
                WHERE key = ? AND deleted_at IS NULL
                "#,
            key
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Project {
            id: Uuid::parse_str(r.id.as_ref().unwrap()).unwrap(),
            title: r.title,
            description: r.description,
            key: r.key,
            status: ProjectStatus::from_str(&r.status).unwrap(),
            version: r.version as i32,
            created_at: DateTime::from_timestamp(r.created_at, 0).unwrap(),
            updated_at: DateTime::from_timestamp(r.updated_at, 0).unwrap(),
            created_by: Uuid::parse_str(&r.created_by).unwrap(),
            updated_by: Uuid::parse_str(&r.updated_by).unwrap(),
            deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
        }))
    }

    pub async fn find_all(&self) -> DbErrorResult<Vec<Project>> {
        let rows = sqlx::query!(
            r#"
                SELECT id, title, description, key, status, version,
                       created_at, updated_at, created_by, updated_by, deleted_at
                FROM pm_projects
                WHERE deleted_at IS NULL
                ORDER BY title
                "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| Project {
                id: Uuid::parse_str(r.id.as_ref().unwrap()).unwrap(),
                title: r.title,
                description: r.description,
                key: r.key,
                status: ProjectStatus::from_str(&r.status).unwrap(),
                version: r.version as i32,
                created_at: DateTime::from_timestamp(r.created_at, 0).unwrap(),
                updated_at: DateTime::from_timestamp(r.updated_at, 0).unwrap(),
                created_by: Uuid::parse_str(&r.created_by).unwrap(),
                updated_by: Uuid::parse_str(&r.updated_by).unwrap(),
                deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
            })
            .collect())
    }

    pub async fn find_active(&self) -> DbErrorResult<Vec<Project>> {
        let rows = sqlx::query!(
            r#"
                SELECT id, title, description, key, status, version,
                       created_at, updated_at, created_by, updated_by, deleted_at
                FROM pm_projects
                WHERE status = 'active' AND deleted_at IS NULL
                ORDER BY title
                "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| Project {
                id: Uuid::parse_str(r.id.as_ref().unwrap()).unwrap(),
                title: r.title,
                description: r.description,
                key: r.key,
                status: ProjectStatus::from_str(&r.status).unwrap(),
                version: r.version as i32,
                created_at: DateTime::from_timestamp(r.created_at, 0).unwrap(),
                updated_at: DateTime::from_timestamp(r.updated_at, 0).unwrap(),
                created_by: Uuid::parse_str(&r.created_by).unwrap(),
                updated_by: Uuid::parse_str(&r.updated_by).unwrap(),
                deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
            })
            .collect())
    }

    pub async fn update(&self, project: &Project) -> DbErrorResult<()> {
        let id = project.id.to_string();
        let status = project.status.as_str();
        let updated_at = project.updated_at.timestamp();
        let updated_by = project.updated_by.to_string();

        sqlx::query!(
            r#"
                UPDATE pm_projects
                SET title = ?, description = ?, key = ?, status = ?,
                    version = ?, updated_at = ?, updated_by = ?
                WHERE id = ? AND deleted_at IS NULL
                "#,
            project.title,
            project.description,
            project.key,
            status,
            project.version,
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
                UPDATE pm_projects
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
