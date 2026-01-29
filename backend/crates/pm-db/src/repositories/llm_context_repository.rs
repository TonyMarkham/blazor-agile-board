use crate::{DbError, Result as DbErrorResult};

use pm_core::{LlmContext, LlmContextType};

use std::panic::Location;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use error_location::ErrorLocation;
use uuid::Uuid;

pub struct LlmContextRepository;

impl LlmContextRepository {
    pub async fn list_all<'e, E>(executor: E) -> DbErrorResult<Vec<LlmContext>>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        let rows = sqlx::query!(
            r#"
            SELECT id, context_type, category, title, content,
                   example_sql, example_description, priority,
                   created_at, updated_at, deleted_at
            FROM pm_llm_context
            WHERE deleted_at IS NULL
            ORDER BY priority DESC, title ASC
            "#
        )
        .fetch_all(executor)
        .await?;

        rows.into_iter()
            .map(|r| {
                let id = r.id.ok_or_else(|| DbError::Initialization {
                    message: "llm_context.id is NULL".to_string(),
                    location: ErrorLocation::from(Location::caller()),
                })?;

                Ok(LlmContext {
                    id: Uuid::parse_str(&id).map_err(|e| DbError::Initialization {
                        message: format!("Invalid UUID in llm_context.id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    })?,
                    context_type: LlmContextType::from_str(&r.context_type).map_err(|_| {
                        DbError::Initialization {
                            message: format!("Invalid context_type: {}", r.context_type),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    category: r.category,
                    title: r.title,
                    content: r.content,
                    example_sql: r.example_sql,
                    example_description: r.example_description,
                    priority: r.priority as i32,
                    created_at: DateTime::<Utc>::from_timestamp(r.created_at, 0).ok_or_else(
                        || DbError::Initialization {
                            message: "Invalid created_at timestamp".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        },
                    )?,
                    updated_at: DateTime::<Utc>::from_timestamp(r.updated_at, 0).ok_or_else(
                        || DbError::Initialization {
                            message: "Invalid updated_at timestamp".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        },
                    )?,
                    deleted_at: r
                        .deleted_at
                        .and_then(|ts| DateTime::<Utc>::from_timestamp(ts, 0)),
                })
            })
            .collect()
    }

    pub async fn list_filtered<'e, E>(
        executor: E,
        category: Option<&str>,
        context_type: Option<&str>,
        min_priority: Option<i32>,
    ) -> DbErrorResult<Vec<LlmContext>>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        let rows = sqlx::query!(
            r#"
            SELECT id, context_type, category, title, content,
                   example_sql, example_description, priority,
                   created_at, updated_at, deleted_at
            FROM pm_llm_context
            WHERE deleted_at IS NULL
              AND (?1 IS NULL OR category = ?1)
              AND (?2 IS NULL OR context_type = ?2)
              AND (?3 IS NULL OR priority >= ?3)
            ORDER BY priority DESC, title ASC
            "#,
            category,
            context_type,
            min_priority
        )
        .fetch_all(executor)
        .await?;

        rows.into_iter()
            .map(|r| {
                let id = r.id.ok_or_else(|| DbError::Initialization {
                    message: "llm_context.id is NULL".to_string(),
                    location: ErrorLocation::from(Location::caller()),
                })?;

                Ok(LlmContext {
                    id: Uuid::parse_str(&id).map_err(|e| DbError::Initialization {
                        message: format!("Invalid UUID in llm_context.id: {}", e),
                        location: ErrorLocation::from(Location::caller()),
                    })?,
                    context_type: LlmContextType::from_str(&r.context_type).map_err(|_| {
                        DbError::Initialization {
                            message: format!("Invalid context_type: {}", r.context_type),
                            location: ErrorLocation::from(Location::caller()),
                        }
                    })?,
                    category: r.category,
                    title: r.title,
                    content: r.content,
                    example_sql: r.example_sql,
                    example_description: r.example_description,
                    priority: r.priority as i32,
                    created_at: DateTime::<Utc>::from_timestamp(r.created_at, 0).ok_or_else(
                        || DbError::Initialization {
                            message: "Invalid created_at timestamp".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        },
                    )?,
                    updated_at: DateTime::<Utc>::from_timestamp(r.updated_at, 0).ok_or_else(
                        || DbError::Initialization {
                            message: "Invalid updated_at timestamp".to_string(),
                            location: ErrorLocation::from(Location::caller()),
                        },
                    )?,
                    deleted_at: r
                        .deleted_at
                        .and_then(|ts| DateTime::<Utc>::from_timestamp(ts, 0)),
                })
            })
            .collect()
    }
}
