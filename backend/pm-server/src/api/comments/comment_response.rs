use crate::CommentDto;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CommentResponse {
    pub comment: CommentDto,
}
