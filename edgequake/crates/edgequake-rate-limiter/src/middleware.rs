//! Axum middleware for rate limiting.
//!
//! This module provides Axum-compatible middleware that enforces rate limits
//! on incoming requests based on tenant and workspace context.

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use tracing::{debug, warn};

use crate::limiter::RateLimiter;

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
        let headers = response.headers_mut();
        headers.insert(
            "X-RateLimit-Limit",
            state.capacity.to_string().parse().unwrap(),
        );
        headers.insert(
            "X-RateLimit-Remaining",
            state.available_tokens.to_string().parse().unwrap(),
        );
        headers.insert(
            "X-RateLimit-Reset",
            "60".parse().unwrap(), // TODO: Calculate actual reset time
        );
    }

    response
}

/// Create a 429 Too Many Requests response
fn create_rate_limit_response(retry_after: Option<u64>) -> Response {
    let retry_after = retry_after.unwrap_or(60);

    let body = serde_json::json!({
        "error": "Rate limit exceeded",
        "message": format!("Too many requests. Please retry after {} seconds.", retry_after),
        "retry_after_seconds": retry_after,
    });

    let mut response = (
        StatusCode::TOO_MANY_REQUESTS,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        serde_json::to_string(&body).unwrap(),
    )
        .into_response();

    // Add Retry-After header (RFC 6585)
    response
        .headers_mut()
        .insert("Retry-After", retry_after.to_string().parse().unwrap());

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RateLimitConfig;
    use axum::{body::Body, routing::get, Router};
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
}
