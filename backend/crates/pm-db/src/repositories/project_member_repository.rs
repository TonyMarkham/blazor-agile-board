use crate::error::Result as DbErrorResult;

use pm_core::ProjectMember;

use sqlx::SqlitePool;
use uuid::Uuid;

pub struct ProjectMemberRepository {
    pool: SqlitePool,
}

impl ProjectMemberRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn find_by_user_and_project(
        &self,
        user_id: Uuid,
        project_id: Uuid,
    ) -> DbErrorResult<Option<ProjectMember>> {
        let user_id_str = user_id.to_string();
        let project_id_str = project_id.to_string();

        let row = sqlx::query!(
            r#"
              SELECT id, project_id, user_id, role, created_at
              FROM pm_project_members
              WHERE user_id = ? AND project_id = ?
              "#,
            user_id_str,
            project_id_str
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| ProjectMember {
            id: Uuid::parse_str(r.id.as_ref().unwrap()).unwrap(),
            project_id: Uuid::parse_str(&r.project_id).unwrap(),
            user_id: Uuid::parse_str(&r.user_id).unwrap(),
            role: r.role,
            created_at: chrono::DateTime::from_timestamp(r.created_at, 0).unwrap(),
        }))
    }

    pub async fn find_by_project(&self, project_id: Uuid) -> DbErrorResult<Vec<ProjectMember>> {
        let project_id_str = project_id.to_string();

        let rows = sqlx::query!(
            r#"
              SELECT id, project_id, user_id, role, created_at
              FROM pm_project_members
              WHERE project_id = ?
              "#,
            project_id_str
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ProjectMember {
                id: Uuid::parse_str(r.id.as_ref().unwrap()).unwrap(),
                project_id: Uuid::parse_str(&r.project_id).unwrap(),
                user_id: Uuid::parse_str(&r.user_id).unwrap(),
                role: r.role,
                created_at: chrono::DateTime::from_timestamp(r.created_at, 0).unwrap(),
            })
            .collect())
    }

    pub async fn create(&self, member: &ProjectMember) -> DbErrorResult<()> {
        let id_str = member.id.to_string();
        let project_id_str = member.project_id.to_string();
        let user_id_str = member.user_id.to_string();
        let created_at = member.created_at.timestamp();

        sqlx::query!(
            r#"
              INSERT INTO pm_project_members (id, project_id, user_id, role, created_at)
              VALUES (?, ?, ?, ?, ?)
              "#,
            id_str,
            project_id_str,
            user_id_str,
            member.role,
            created_at
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete(&self, id: Uuid) -> DbErrorResult<bool> {
        let id_str = id.to_string();

        let result = sqlx::query!("DELETE FROM pm_project_members WHERE id = ?", id_str)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
