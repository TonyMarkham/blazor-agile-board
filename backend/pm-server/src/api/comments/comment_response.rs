use pm_core::CommentDto;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CommentResponse {
    pub comment: CommentDto,
}
