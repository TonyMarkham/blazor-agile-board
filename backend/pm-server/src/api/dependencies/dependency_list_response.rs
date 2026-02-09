use pm_core::DependencyDto;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct DependencyListResponse {
    pub dependencies: Vec<DependencyDto>,
}
