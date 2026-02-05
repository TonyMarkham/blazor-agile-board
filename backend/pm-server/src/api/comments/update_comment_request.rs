use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct UpdateCommentRequest {
    pub content: String,
}
