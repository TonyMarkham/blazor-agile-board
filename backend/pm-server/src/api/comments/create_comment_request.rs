use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CreateCommentRequest {
    pub content: String,
}
