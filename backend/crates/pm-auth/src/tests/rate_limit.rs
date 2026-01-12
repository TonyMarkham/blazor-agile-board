use crate::{ConnectionRateLimiter, RateLimitConfig};

#[test]
fn given_rate_limiter_when_under_limit_then_allows_requests() {
    let config = RateLimitConfig {
        max_requests: 10,
        window_secs: 1,
    };
    let limiter = ConnectionRateLimiter::new(config);

    // First few requests should succeed
    for _ in 0..5 {
        assert!(limiter.check().is_ok());
    }
}

#[test]
fn given_rate_limiter_when_burst_exceeds_limit_then_rejects() {
    let config = RateLimitConfig {
        max_requests: 2,
        window_secs: 1,
    };
    let limiter = ConnectionRateLimiter::new(config);

    // Exhaust the limit
    let _ = limiter.check();
    let _ = limiter.check();
    let _ = limiter.check();

    // Should eventually hit rate limit
    let mut hit_limit = false;
    for _ in 0..10 {
        if limiter.check().is_err() {
            hit_limit = true;
            break;
        }
    }
    assert!(hit_limit, "Expected rate limit to be enforced");
}