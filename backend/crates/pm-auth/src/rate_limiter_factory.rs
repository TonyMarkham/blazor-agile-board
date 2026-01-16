use crate::{ConnectionRateLimiter, RateLimitConfig};

/// Factory for creating per-connection rate limiters                                                                                                                            
#[derive(Clone)]
pub struct RateLimiterFactory {
    config: RateLimitConfig,
}

impl RateLimiterFactory {
    pub fn new(config: RateLimitConfig) -> Self {
        Self { config }
    }

    pub fn create(&self) -> ConnectionRateLimiter {
        ConnectionRateLimiter::new(self.config.clone())
    }
}

impl Default for RateLimiterFactory {
    fn default() -> Self {
        Self::new(RateLimitConfig::default())
    }
}
