use crate::CommentDto;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CommentListResponse {
    pub comments: Vec<CommentDto>,
}
