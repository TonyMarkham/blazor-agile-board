/// Configuration for connection limits
#[derive(Debug, Clone)]
pub struct ConnectionLimits {
    /// Maximum total connections
    pub max_total: usize,
}

impl Default for ConnectionLimits {
    fn default() -> Self {
        Self { max_total: 10000 }
    }
}
