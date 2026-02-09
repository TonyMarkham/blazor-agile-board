use crate::{CommentDto, CoreError, CoreResult, parse_timestamp, parse_uuid};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: Uuid,
    pub work_item_id: Uuid,

    pub content: String,

    // Audit
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Uuid,
    pub updated_by: Uuid,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Comment {
    pub fn new(work_item_id: Uuid, content: String, created_by: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            work_item_id,
            content,
            created_at: now,
            updated_at: now,
            created_by,
            updated_by: created_by,
            deleted_at: None,
        }
    }
}

impl TryFrom<CommentDto> for Comment {
    type Error = CoreError;

    fn try_from(dto: CommentDto) -> CoreResult<Self> {
        Ok(Comment {
            id: parse_uuid(&dto.id, "comment.id")?,
            work_item_id: parse_uuid(&dto.work_item_id, "comment.work_item_id")?,
            content: dto.content,
            created_at: parse_timestamp(dto.created_at, "comment.created_at")?,
            updated_at: parse_timestamp(dto.updated_at, "comment.updated_at")?,
            created_by: parse_uuid(&dto.created_by, "comment.created_by")?,
            updated_by: parse_uuid(&dto.updated_by, "comment.updated_by")?,
            deleted_at: None,
        })
    }
}
