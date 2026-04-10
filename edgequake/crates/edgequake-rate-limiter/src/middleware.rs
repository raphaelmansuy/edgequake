//! Axum middleware for rate limiting.
//!
//! This module provides Axum-compatible middleware that enforces rate limits
//! on incoming requests based on tenant and workspace context.

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, HeaderValue, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::limiter::RateLimiter;

const X_RATE_LIMIT_LIMIT: &str = "X-RateLimit-Limit";
const X_RATE_LIMIT_REMAINING: &str = "X-RateLimit-Remaining";
const X_RATE_LIMIT_RESET: &str = "X-RateLimit-Reset";
const RETRY_AFTER: &str = "Retry-After";

#[derive(Serialize)]
struct RateLimitExceededBody {
    error: &'static str,
    message: String,
    retry_after_seconds: u64,
}

/// Rate limiting middleware for Axum
///
/// Extracts tenant/workspace context from request headers and applies rate limiting.
/// Returns 429 Too Many Requests if rate limit exceeded.
pub async fn rate_limit_middleware(
    State(limiter): State<Arc<RateLimiter>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    // Extract tenant context from headers
    let tenant_id = request
        .headers()
        .get("X-Tenant-ID")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("default");

    let workspace_id = request
        .headers()
        .get("X-Workspace-ID")
        .and_then(|v| v.to_str().ok());

    // Create rate limit key
    let key = match workspace_id {
        Some(workspace) => format!("{}:{}", tenant_id, workspace),
        None => tenant_id.to_string(),
    };

    debug!(
        tenant_id = tenant_id,
        workspace_id = workspace_id,
        key = key.as_str(),
        "Checking rate limit"
    );

    // Check rate limit
    let (allowed, retry_after) = limiter.check_rate_limit(&key);

    if !allowed {
        warn!(
            tenant_id = tenant_id,
            workspace_id = workspace_id,
            retry_after = retry_after,
            "Rate limit exceeded"
        );

        return create_rate_limit_response(retry_after);
    }

    // Add rate limit headers to response
    let mut response = next.run(request).await;

    if let Some(state) = limiter.get_state(&key) {
        insert_rate_limit_headers(
            response.headers_mut(),
            state.capacity,
            state.available_tokens,
            state.reset_after_seconds,
        );
    }

    response
}

/// Create a 429 Too Many Requests response
fn create_rate_limit_response(retry_after: Option<u64>) -> Response {
    let retry_after = retry_after.unwrap_or(60);

    let mut response = (
        StatusCode::TOO_MANY_REQUESTS,
        Json(RateLimitExceededBody {
            error: "Rate limit exceeded",
            message: format!(
                "Too many requests. Please retry after {} seconds.",
                retry_after
            ),
            retry_after_seconds: retry_after,
        }),
    )
        .into_response();

    insert_u64_header(response.headers_mut(), RETRY_AFTER, retry_after);
    insert_u64_header(response.headers_mut(), X_RATE_LIMIT_REMAINING, 0);

    response
}

fn insert_rate_limit_headers(
    headers: &mut HeaderMap,
    capacity: u32,
    remaining: u32,
    reset_after_seconds: u64,
) {
    insert_u64_header(headers, X_RATE_LIMIT_LIMIT, capacity as u64);
    insert_u64_header(headers, X_RATE_LIMIT_REMAINING, remaining as u64);
    insert_u64_header(headers, X_RATE_LIMIT_RESET, reset_after_seconds);
}

fn insert_u64_header(headers: &mut HeaderMap, name: &'static str, value: u64) {
    match HeaderValue::from_str(&value.to_string()) {
        Ok(header_value) => {
            headers.insert(name, header_value);
        }
        Err(error) => {
            warn!(header = name, value, %error, "Skipping invalid rate limit header");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RateLimitConfig;
    use axum::{body, body::Body, routing::get, Router};
    use serde_json::Value;
    use tower::ServiceExt;

    async fn test_handler() -> &'static str {
        "OK"
    }

    #[tokio::test]
    async fn test_rate_limit_middleware_allows_requests() {
        let config = RateLimitConfig::new(10, 60);
        let limiter = Arc::new(RateLimiter::new(config));

        let app = Router::new().route("/test", get(test_handler)).layer(
            axum::middleware::from_fn_with_state(limiter, rate_limit_middleware),
        );

        // First request should succeed
        let request = Request::builder()
            .uri("/test")
            .header("X-Tenant-ID", "tenant-123")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_rate_limit_middleware_blocks_excess() {
        let config = RateLimitConfig::new(3, 60); // Only 3 requests allowed
        let limiter = Arc::new(RateLimiter::new(config));

        let app = Router::new().route("/test", get(test_handler)).layer(
            axum::middleware::from_fn_with_state(limiter, rate_limit_middleware),
        );

        // First 3 requests should succeed
        for _ in 0..3 {
            let request = Request::builder()
                .uri("/test")
                .header("X-Tenant-ID", "tenant-123")
                .body(Body::empty())
                .unwrap();

            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }

        // 4th request should fail
        let request = Request::builder()
            .uri("/test")
            .header("X-Tenant-ID", "tenant-123")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn test_tenant_isolation_in_middleware() {
        let config = RateLimitConfig::new(2, 60);
        let limiter = Arc::new(RateLimiter::new(config));

        let app = Router::new().route("/test", get(test_handler)).layer(
            axum::middleware::from_fn_with_state(limiter, rate_limit_middleware),
        );

        // Tenant A: consume all requests
        for _ in 0..2 {
            let request = Request::builder()
                .uri("/test")
                .header("X-Tenant-ID", "tenant-a")
                .body(Body::empty())
                .unwrap();

            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }

        // Tenant A: next request fails
        let request = Request::builder()
            .uri("/test")
            .header("X-Tenant-ID", "tenant-a")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

        // Tenant B: should still have full quota
        let request = Request::builder()
            .uri("/test")
            .header("X-Tenant-ID", "tenant-b")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_success_response_includes_rate_limit_headers() {
        let config = RateLimitConfig::strict(2, 10);
        let limiter = Arc::new(RateLimiter::new(config));

        let app = Router::new().route("/test", get(test_handler)).layer(
            axum::middleware::from_fn_with_state(limiter, rate_limit_middleware),
        );

        let request = Request::builder()
            .uri("/test")
            .header("X-Tenant-ID", "tenant-headers")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers().get(X_RATE_LIMIT_LIMIT).unwrap(), "2");
        assert_eq!(response.headers().get(X_RATE_LIMIT_REMAINING).unwrap(), "1");

        let reset_header = response.headers().get(X_RATE_LIMIT_RESET).unwrap();
        let reset_value: u64 = reset_header.to_str().unwrap().parse().unwrap();
        assert!(
            reset_value <= 10,
            "reset header should stay within the window"
        );
    }

    #[tokio::test]
    async fn test_rate_limited_response_rounds_retry_after_up() {
        let config = RateLimitConfig {
            requests_per_window: 1,
            window_seconds: 1,
            burst_size: 0,
            refill_rate: 100.0,
        };
        let limiter = Arc::new(RateLimiter::new(config));

        let app = Router::new().route("/test", get(test_handler)).layer(
            axum::middleware::from_fn_with_state(limiter, rate_limit_middleware),
        );

        let first_request = Request::builder()
            .uri("/test")
            .header("X-Tenant-ID", "tenant-fast")
            .body(Body::empty())
            .unwrap();
        let first_response = app.clone().oneshot(first_request).await.unwrap();
        assert_eq!(first_response.status(), StatusCode::OK);

        let second_request = Request::builder()
            .uri("/test")
            .header("X-Tenant-ID", "tenant-fast")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(second_request).await.unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers().get(RETRY_AFTER).unwrap(), "1");
        assert_eq!(response.headers().get(X_RATE_LIMIT_REMAINING).unwrap(), "0");

        let body = body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["error"], "Rate limit exceeded");
        assert_eq!(payload["retry_after_seconds"], 1);
    }
}
