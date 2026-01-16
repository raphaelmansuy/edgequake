//! Token bucket rate limiter implementation.
//!
//! This module provides the core `RateLimiter` that uses a token bucket
//! algorithm with concurrent access support via `DashMap`.
//!
//! ## WHY Token Bucket Algorithm?
//!
//! We chose token bucket over alternatives for these reasons:
//! - **vs Fixed Window**: Avoids burst spikes at window boundaries
//! - **vs Sliding Window**: Lower memory overhead (O(1) per key vs O(n) requests)
//! - **vs Leaky Bucket**: Allows controlled bursts while maintaining avg rate
//!
//! The token bucket naturally smooths traffic while permitting brief bursts,
//! which is ideal for RAG workloads that may batch multiple queries.

use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time;
use tracing::{debug, warn};

use crate::config::RateLimitConfig;

/// Token bucket state for a single key (tenant/workspace)
#[derive(Debug)]
struct TokenBucket {
    /// Current number of tokens available
    tokens: f64,
    /// Maximum number of tokens (capacity)
    capacity: f64,
    /// Rate at which tokens are refilled (tokens per second)
    refill_rate: f64,
    /// Last time the bucket was updated
    last_refill: Instant,
}

impl TokenBucket {
    fn new(config: &RateLimitConfig) -> Self {
        let capacity = config.max_capacity() as f64;
        Self {
            tokens: capacity,
            capacity,
            refill_rate: config.refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();

        // Calculate new tokens
        let new_tokens = elapsed * self.refill_rate;
        self.tokens = (self.tokens + new_tokens).min(self.capacity);
        self.last_refill = now;

        debug!(
            tokens = self.tokens,
            capacity = self.capacity,
            elapsed_secs = elapsed,
            "Refilled token bucket"
        );
    }

    /// Try to consume tokens
    /// Returns true if successful, false if insufficient tokens
    fn try_consume(&mut self, amount: f64) -> bool {
        self.refill();

        if self.tokens >= amount {
            self.tokens -= amount;
            debug!(
                consumed = amount,
                remaining = self.tokens,
                "Consumed tokens"
            );
            true
        } else {
            warn!(
                requested = amount,
                available = self.tokens,
                "Insufficient tokens"
            );
            false
        }
    }

    /// Get time until next token is available
    fn time_until_available(&self, amount: f64) -> Duration {
        let deficit = (amount - self.tokens).max(0.0);
        let seconds = deficit / self.refill_rate;
        Duration::from_secs_f64(seconds)
    }
}

/// Rate limiter using token bucket algorithm
#[derive(Clone)]
pub struct RateLimiter {
    // WHY DashMap: Provides lock-free concurrent access without needing
    // external synchronization. Each bucket can be updated independently,
    // which is critical for high-throughput API servers.
    /// Buckets per key (tenant_id:workspace_id)
    buckets: Arc<DashMap<String, TokenBucket>>,
    /// Configuration
    config: Arc<RateLimitConfig>,
}

impl RateLimiter {
    /// Create a new rate limiter with given configuration
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            buckets: Arc::new(DashMap::new()),
            config: Arc::new(config),
        }
    }

    /// Check if a request is allowed for the given key
    /// Returns (allowed, retry_after_seconds)
    pub fn check_rate_limit(&self, key: &str) -> (bool, Option<u64>) {
        self.check_rate_limit_with_cost(key, 1.0)
    }

    /// Check rate limit with custom cost
    /// Some operations may cost more tokens than others
    pub fn check_rate_limit_with_cost(&self, key: &str, cost: f64) -> (bool, Option<u64>) {
        let mut entry = self
            .buckets
            .entry(key.to_string())
            .or_insert_with(|| TokenBucket::new(&self.config));

        let bucket = entry.value_mut();

        if bucket.try_consume(cost) {
            (true, None)
        } else {
            let retry_after = bucket.time_until_available(cost);
            (false, Some(retry_after.as_secs()))
        }
    }

    /// Get current state for a key (for monitoring/debugging)
    pub fn get_state(&self, key: &str) -> Option<RateLimitState> {
        self.buckets.get(key).map(|entry| {
            let bucket = entry.value();
            RateLimitState {
                available_tokens: bucket.tokens as u32,
                capacity: bucket.capacity as u32,
                refill_rate: bucket.refill_rate,
            }
        })
    }

    /// Reset rate limit for a key (admin operation)
    pub fn reset(&self, key: &str) {
        self.buckets.remove(key);
        debug!(key = key, "Reset rate limit");
    }

    /// Clear old buckets that haven't been used recently
    /// Should be called periodically to prevent memory leaks
    pub fn cleanup_stale_buckets(&self, max_age: Duration) {
        let now = Instant::now();
        let mut removed = 0;

        self.buckets.retain(|_key, bucket| {
            let age = now.duration_since(bucket.last_refill);
            let keep = age < max_age;
            if !keep {
                removed += 1;
            }
            keep
        });

        if removed > 0 {
            debug!(removed = removed, "Cleaned up stale rate limit buckets");
        }
    }

    /// Start background task to cleanup stale buckets
    pub fn start_cleanup_task(self, interval: Duration, max_age: Duration) {
        tokio::spawn(async move {
            let mut interval_timer = time::interval(interval);
            loop {
                interval_timer.tick().await;
                self.cleanup_stale_buckets(max_age);
            }
        });
    }
}

/// Current rate limit state for a key
#[derive(Debug, Clone)]
pub struct RateLimitState {
    pub available_tokens: u32,
    pub capacity: u32,
    pub refill_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_token_bucket_basic() {
        let config = RateLimitConfig::strict(10, 10); // 10 requests, NO BURST
        let limiter = RateLimiter::new(config);

        // First 10 requests should succeed
        for i in 0..10 {
            let (allowed, _) = limiter.check_rate_limit("test-key");
            assert!(allowed, "Request {} should be allowed", i);
        }

        // 11th request should fail
        let (allowed, retry_after) = limiter.check_rate_limit("test-key");
        assert!(!allowed, "Request 11 should be blocked");
        assert!(retry_after.is_some());
    }

    #[tokio::test]
    async fn test_token_refill() {
        // WHY: Use high refill rate to minimize wait time (600ms → 50ms)
        let config = RateLimitConfig {
            requests_per_window: 2,
            window_seconds: 1,
            burst_size: 0,
            refill_rate: 100.0, // Fast refill for testing
        };
        let limiter = RateLimiter::new(config);

        // Consume all tokens
        assert!(limiter.check_rate_limit("test-key").0);
        assert!(limiter.check_rate_limit("test-key").0);
        assert!(!limiter.check_rate_limit("test-key").0);

        // Wait for fast refill (100 tokens/sec = 10ms for 1 token)
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Should have ~1 token now
        assert!(limiter.check_rate_limit("test-key").0);
    }

    #[tokio::test]
    async fn test_burst_allowance() {
        let config = RateLimitConfig {
            requests_per_window: 10,
            window_seconds: 10,
            burst_size: 5,
            refill_rate: 1.0,
        };
        let limiter = RateLimiter::new(config);

        // Should allow up to 15 requests (10 + 5 burst)
        for i in 0..15 {
            let (allowed, _) = limiter.check_rate_limit("test-key");
            assert!(allowed, "Request {} should be allowed", i);
        }

        // 16th should fail
        let (allowed, _) = limiter.check_rate_limit("test-key");
        assert!(!allowed);
    }

    #[tokio::test]
    async fn test_tenant_isolation() {
        let config = RateLimitConfig::strict(5, 10); // 5 requests, NO BURST
        let limiter = RateLimiter::new(config);

        // Tenant A consumes all tokens
        for _ in 0..5 {
            assert!(limiter.check_rate_limit("tenant-a").0);
        }
        assert!(!limiter.check_rate_limit("tenant-a").0);

        // Tenant B should still have full quota
        for i in 0..5 {
            let (allowed, _) = limiter.check_rate_limit("tenant-b");
            assert!(allowed, "Tenant B request {} should be allowed", i);
        }
    }

    #[tokio::test]
    async fn test_custom_cost() {
        let config = RateLimitConfig::strict(100, 60); // 100 tokens, NO BURST
        let limiter = RateLimiter::new(config);

        // Expensive operation costs 5 tokens
        assert!(limiter.check_rate_limit_with_cost("test-key", 5.0).0);

        // Should have 95 tokens left (100 - 5)
        // Use 95 more tokens with 1-token requests
        for _ in 0..95 {
            assert!(limiter.check_rate_limit("test-key").0);
        }

        // No tokens left
        assert!(!limiter.check_rate_limit("test-key").0);
    }

    #[test]
    fn test_cleanup_stale_buckets() {
        let config = RateLimitConfig::new(10, 60);
        let limiter = RateLimiter::new(config);

        // Create some buckets
        limiter.check_rate_limit("key-1");
        limiter.check_rate_limit("key-2");
        limiter.check_rate_limit("key-3");

        assert_eq!(limiter.buckets.len(), 3);

        // Cleanup with very short max age (nothing should be removed yet)
        limiter.cleanup_stale_buckets(Duration::from_secs(3600));
        assert_eq!(limiter.buckets.len(), 3);

        // Cleanup with zero max age (everything should be removed)
        limiter.cleanup_stale_buckets(Duration::from_secs(0));
        assert_eq!(limiter.buckets.len(), 0);
    }
}
