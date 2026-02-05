use crate::ProjectDto;
use serde::Serialize;

/// Single project response
#[derive(Debug, Serialize)]
pub struct ProjectResponse {
    pub project: ProjectDto,
}
