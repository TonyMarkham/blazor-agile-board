/// Configuration for broadcast channels
#[derive(Debug, Clone)]
pub struct BroadcastConfig {
    /// Channel capacity per tenant (bounded to prevent memory exhaustion)
    pub channel_capacity: usize,
}

impl Default for BroadcastConfig {
    fn default() -> Self {
        Self {
            channel_capacity: 1000, // 1000 messages buffered per tenant
        }
    }
}
