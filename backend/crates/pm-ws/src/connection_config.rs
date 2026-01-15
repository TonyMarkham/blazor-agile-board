/// Configuration for WebSocket connections
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// Send buffer size (bounded to handle backpressure)                                                                                                                        
    pub send_buffer_size: usize,
    /// Heartbeat interval in seconds                                                                                                                                            
    pub heartbeat_interval_secs: u64,
    /// Heartbeat timeout in seconds                                                                                                                                             
    pub heartbeat_timeout_secs: u64,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            send_buffer_size: 100,
            heartbeat_interval_secs: 30,
            heartbeat_timeout_secs: 60,
        }
    }
} 