//! Rate limit configuration structures.
//!
//! This module defines the configuration options for rate limiting,
//! including per-tier settings and token bucket parameters.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum number of requests allowed in the window
    pub requests_per_window: u32,

    /// Time window for rate limiting (in seconds)
    pub window_seconds: u64,

    /// Maximum burst allowance (additional requests beyond normal rate)
    pub burst_size: u32,

    /// Token refill rate (tokens per second)
    pub refill_rate: f64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_window: 100,  // 100 requests
            window_seconds: 60,        // per minute
            burst_size: 20,            // with burst of 20
            refill_rate: 100.0 / 60.0, // ~1.67 tokens/second
        }
    }
}

impl RateLimitConfig {
    /// Create a new rate limit configuration
    pub fn new(requests_per_window: u32, window_seconds: u64) -> Self {
        let refill_rate = requests_per_window as f64 / window_seconds as f64;
        Self {
            requests_per_window,
            window_seconds,
            burst_size: requests_per_window / 5, // Default burst is 20% of limit
            refill_rate,
        }
    }

    /// Create a strict rate limit (no burst)
    pub fn strict(requests_per_window: u32, window_seconds: u64) -> Self {
        let refill_rate = requests_per_window as f64 / window_seconds as f64;
        Self {
            requests_per_window,
            window_seconds,
            burst_size: 0, // No burst allowed
            refill_rate,
        }
    }

    /// Create a lenient rate limit (large burst)
    pub fn lenient(requests_per_window: u32, window_seconds: u64) -> Self {
        let refill_rate = requests_per_window as f64 / window_seconds as f64;
        Self {
            requests_per_window,
            window_seconds,
            burst_size: requests_per_window / 2, // 50% burst allowance
            refill_rate,
        }
    }

    /// Get the window duration
    pub fn window_duration(&self) -> Duration {
        Duration::from_secs(self.window_seconds)
    }

    /// Get the maximum capacity (normal + burst)
    pub fn max_capacity(&self) -> u32 {
        self.requests_per_window + self.burst_size
    }
}

/// Tier-based rate limit configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierConfig {
    pub free: RateLimitConfig,
    pub basic: RateLimitConfig,
    pub premium: RateLimitConfig,
    pub enterprise: RateLimitConfig,
}

impl Default for TierConfig {
    fn default() -> Self {
        Self {
            free: RateLimitConfig::new(10, 60),          // 10 req/min
            basic: RateLimitConfig::new(100, 60),        // 100 req/min
            premium: RateLimitConfig::new(1000, 60),     // 1000 req/min
            enterprise: RateLimitConfig::new(10000, 60), // 10k req/min
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RateLimitConfig::default();
        assert_eq!(config.requests_per_window, 100);
        assert_eq!(config.window_seconds, 60);
        assert_eq!(config.burst_size, 20);
    }

    #[test]
    fn test_strict_config() {
        let config = RateLimitConfig::strict(50, 30);
        assert_eq!(config.requests_per_window, 50);
        assert_eq!(config.burst_size, 0);
    }

    #[test]
    fn test_lenient_config() {
        let config = RateLimitConfig::lenient(100, 60);
        assert_eq!(config.burst_size, 50);
        assert_eq!(config.max_capacity(), 150);
    }
}
