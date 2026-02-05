use crate::ProjectDto;
use serde::Serialize;

/// List of projects response
#[derive(Debug, Serialize)]
pub struct ProjectListResponse {
    pub projects: Vec<ProjectDto>,
}
