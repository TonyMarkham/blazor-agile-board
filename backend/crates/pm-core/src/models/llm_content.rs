use crate::models::llm_content_type::LlmContextType;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmContext {
    pub id: Uuid,
    pub context_type: LlmContextType,

    pub category: String,
    pub title: String,
    pub content: String,

    pub example_sql: Option<String>,
    pub example_description: Option<String>,

    pub priority: i32,

    // Audit
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl LlmContext {
    pub fn new(
        context_type: LlmContextType,
        category: String,
        title: String,
        content: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            context_type,
            category,
            title,
            content,
            example_sql: None,
            example_description: None,
            priority: 0,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }
}
