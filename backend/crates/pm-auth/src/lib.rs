pub mod claims;
pub mod connection_rate_limiter;
pub mod error;
pub mod jwt_algorithm;
pub mod jwt_validator;
pub mod rate_limit_config;
pub mod rate_limiter_factory;

pub use claims::Claims;
pub use connection_rate_limiter::ConnectionRateLimiter;
pub use error::{AuthError, Result};
pub use jwt_algorithm::JwtAlgorithm;
pub use jwt_validator::JwtValidator;
pub use rate_limit_config::RateLimitConfig;
pub use rate_limiter_factory::RateLimiterFactory;

#[cfg(test)]
mod tests;
