//! Rate limiting for EdgeQuake API.
//!
//! This crate provides token bucket-based rate limiting with support for
//! multi-tenant configurations and Axum middleware integration.
//!
//! ## Implements
//!
//! - [`FEAT0801`]: Token bucket rate limiting algorithm
//! - [`FEAT0802`]: Tiered rate limits (free, standard, premium)
//! - [`FEAT0803`]: Per-tenant and per-workspace isolation
//! - [`FEAT0804`]: Axum middleware integration
//!
//! ## Enforces
//!
//! - [`BR0801`]: Rate limits applied per tenant/workspace
//! - [`BR0802`]: Exceeded limits return 429 Too Many Requests
//! - [`BR0803`]: Limit headers included in responses
//!
//! ## Use Cases
//!
//! - [`UC0801`]: API protects against request flooding
//! - [`UC0802`]: Premium users get higher limits
//!
//! # Features
//!
//! - Token bucket algorithm with configurable refill rates
//! - Tiered rate limits (free, standard, premium)
//! - Per-tenant and per-workspace isolation
//! - Axum middleware for seamless integration
//!
//! # Example
//!
//! ```ignore
//! use edgequake_rate_limiter::{RateLimiter, RateLimitConfig};
//!
//! let config = RateLimitConfig::default();
//! let limiter = RateLimiter::new(config);
//!
//! if limiter.check("tenant-123", "workspace-456") {
//!     // Request allowed
//! }
//! ```

pub mod config;
pub mod limiter;
pub mod middleware;

pub use config::{RateLimitConfig, TierConfig};
pub use limiter::{RateLimitState, RateLimiter};
pub use middleware::rate_limit_middleware;
