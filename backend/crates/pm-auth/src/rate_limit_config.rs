use crate::{AuthError, Result};

use pm_core::ErrorLocation;

use std::num::NonZeroU32;
use std::panic::Location;

use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};

/// Configuration for rate limiting                                                                                                                                              
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum messages per window                                                                                                                                              
    pub max_requests: u32,
    /// Window duration in seconds                                                                                                                                               
    pub window_secs: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100, // 100 messages                                                                                                                                   
            window_secs: 60,   // per minute                                                                                                                                     
        }
    }
} 