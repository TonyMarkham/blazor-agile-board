use crate::{AuthError, RateLimitConfig, Result as AuthErrorResult};

use std::num::NonZeroU32;
use std::panic::Location;

use error_location::ErrorLocation;
use governor::{
    Quota, RateLimiter,
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
};

/// Per-connection rate limiter                                                                                                                                                  
pub struct ConnectionRateLimiter {
    limiter: RateLimiter<NotKeyed, InMemoryState, DefaultClock>,
    config: RateLimitConfig,
}

impl ConnectionRateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        let quota = Quota::per_second(
            NonZeroU32::new(config.max_requests / config.window_secs.max(1) as u32)
                .unwrap_or(NonZeroU32::new(1).unwrap()),
        );

        Self {
            limiter: RateLimiter::direct(quota),
            config,
        }
    }

    /// Check if request is allowed, returns error if rate limited                                                                                                               
    #[track_caller]
    pub fn check(&self) -> AuthErrorResult<()> {
        self.limiter
            .check()
            .map_err(|_| AuthError::RateLimitExceeded {
                limit: self.config.max_requests,
                window_secs: self.config.window_secs,
                location: ErrorLocation::from(Location::caller()),
            })
    }
}
