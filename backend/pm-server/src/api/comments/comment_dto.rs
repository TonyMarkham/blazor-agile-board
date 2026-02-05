use pm_core::Comment;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CommentDto {
    pub id: String,
    pub work_item_id: String,
    pub content: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub created_by: String,
    pub updated_by: String,
}

impl From<Comment> for CommentDto {
    fn from(c: Comment) -> Self {
        Self {
            id: c.id.to_string(),
            work_item_id: c.work_item_id.to_string(),
            content: c.content,
            created_at: c.created_at.timestamp(),
            updated_at: c.updated_at.timestamp(),
            created_by: c.created_by.to_string(),
            updated_by: c.updated_by.to_string(),
        }
    }
}
