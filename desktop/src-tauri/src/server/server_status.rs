use crate::server::health_info::HealthInfo;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ServerStatus {
    pub state: String,
    pub port: Option<u16>,
    pub websocket_url: Option<String>,
    pub health: Option<HealthInfo>,
    pub error: Option<String>,
    pub recovery_hint: Option<String>,
    pub is_healthy: bool,
    pub pid: Option<u32>,
}
