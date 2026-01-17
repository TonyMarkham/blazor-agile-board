use uuid::Uuid;

/// Information about what to broadcast after a successful operation                                   
#[derive(Debug, Clone)]
pub struct BroadcastInfo {
    /// Project ID to broadcast to                                                                     
    pub project_id: Uuid,
    /// Event type for metrics/logging                                                                 
    pub event_type: String,
}

impl BroadcastInfo {
    pub fn new(project_id: Uuid, event_type: impl Into<String>) -> Self {
        Self {
            project_id,
            event_type: event_type.into(),
        }
    }
}
