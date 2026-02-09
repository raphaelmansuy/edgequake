//! HTTP middleware.
//!
//! Provides cross-cutting concerns for API request processing.
//!
//! ## Implements
//!
//! - [`FEAT0410`]: Request logging with timing
//! - [`FEAT0411`]: Request ID tracking
//! - [`FEAT0412`]: Rate limiting enforcement
//! - [`FEAT0413`]: CORS configuration
//!
//! ## Use Cases
//!
//! - [`UC2010`]: System logs all API requests with timing
//! - [`UC2011`]: System assigns unique ID to each request
//! - [`UC2012`]: System enforces rate limits per client
//!
//! ## Enforces
//!
//! - [`BR0410`]: All requests logged with method, URI, status, duration
//! - [`BR0411`]: X-Request-ID header propagation
//! - [`BR0412`]: Rate limit headers in response

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use edgequake_rate_limiter::RateLimiter;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, warn};

/// Request logging middleware.
pub async fn request_logging(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = Instant::now();

    let response = next.run(request).await;

    let duration = start.elapsed();
    let status = response.status();

    if status.is_success() {
        info!(
            method = %method,
            uri = %uri,
            status = %status.as_u16(),
            duration_ms = %duration.as_millis(),
            "Request completed"
        );
    } else {
        warn!(
            method = %method,
            uri = %uri,
            status = %status.as_u16(),
            duration_ms = %duration.as_millis(),
            "Request failed"
        );
    }

    response
}

/// Add request ID header.
pub async fn request_id(mut request: Request, next: Next) -> Response {
    let request_id = uuid::Uuid::new_v4().to_string();

    // SAFETY: UUID hyphenated format is always valid ASCII, so unwrap is safe
    request
        .headers_mut()
        .insert("x-request-id", HeaderValue::from_str(&request_id).unwrap());

    let mut response = next.run(request).await;

    // SAFETY: Same UUID, still valid ASCII
    response
        .headers_mut()
        .insert("x-request-id", HeaderValue::from_str(&request_id).unwrap());

    response
}

/// Authentication configuration.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// Whether authentication is enabled.
    pub enabled: bool,

    /// API keys that are allowed (for simple auth).
    pub api_keys: Vec<String>,

    /// Paths that don't require authentication.
    pub public_paths: Vec<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_keys: Vec::new(),
            public_paths: vec![
                "/health".to_string(),
                "/ready".to_string(),
                "/live".to_string(),
                "/swagger-ui".to_string(),
                "/api-docs".to_string(),
            ],
        }
    }
}

impl AuthConfig {
    /// Create auth config with API keys.
    pub fn with_api_keys(api_keys: Vec<String>) -> Self {
        Self {
            enabled: true,
            api_keys,
            ..Default::default()
        }
    }

    /// Check if a path is public (doesn't require auth).
    pub fn is_public_path(&self, path: &str) -> bool {
        self.public_paths.iter().any(|p| path.starts_with(p))
    }

    /// Validate an API key.
    pub fn validate_api_key(&self, key: &str) -> bool {
        self.api_keys.iter().any(|k| k == key)
    }
}

/// Authentication error response.
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthError {
    pub error: String,
    pub message: String,
}

/// Authentication middleware state.
#[derive(Clone)]
pub struct AuthState {
    pub config: Arc<AuthConfig>,
}

impl AuthState {
    /// Create new auth state.
    pub fn new(config: AuthConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }
}

/// API key authentication middleware.
///
/// Checks for a valid API key in the `Authorization` header or `X-API-Key` header.
/// Format: `Bearer <api-key>` or just the key in `X-API-Key`.
/// @implements FEAT0808
pub async fn api_key_auth(
    axum::extract::State(auth_state): axum::extract::State<AuthState>,
    request: Request,
    next: Next,
) -> Response {
    let config = &auth_state.config;

    // Skip auth if disabled
    if !config.enabled {
        return next.run(request).await;
    }

    // Skip auth for public paths
    let path = request.uri().path();
    if config.is_public_path(path) {
        return next.run(request).await;
    }

    // Try to get API key from headers
    let api_key = extract_api_key(&request);

    match api_key {
        Some(key) if config.validate_api_key(&key) => {
            // Valid API key, proceed
            next.run(request).await
        }
        Some(_) => {
            // Invalid API key
            (
                StatusCode::UNAUTHORIZED,
                Json(AuthError {
                    error: "unauthorized".to_string(),
                    message: "Invalid API key".to_string(),
                }),
            )
                .into_response()
        }
        None => {
            // No API key provided
            (
                StatusCode::UNAUTHORIZED,
                Json(AuthError {
                    error: "unauthorized".to_string(),
                    message: "Missing API key. Provide via Authorization header (Bearer <key>) or X-API-Key header".to_string(),
                }),
            )
                .into_response()
        }
    }
}

/// Extract API key from request headers.
fn extract_api_key(request: &Request) -> Option<String> {
    // Try Authorization header first (Bearer token)
    if let Some(auth_header) = request.headers().get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(key) = auth_str.strip_prefix("Bearer ") {
                return Some(key.trim().to_string());
            }
        }
    }

    // Try X-API-Key header
    if let Some(api_key_header) = request.headers().get("x-api-key") {
        if let Ok(key) = api_key_header.to_str() {
            return Some(key.trim().to_string());
        }
    }

    None
}

/// Rate limiting configuration.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Whether rate limiting is enabled.
    pub enabled: bool,

    /// Maximum requests per window.
    pub max_requests: usize,

    /// Window duration in seconds.
    pub window_seconds: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_requests: 100,
            window_seconds: 60,
        }
    }
}

/// Rate limiting middleware state.
#[derive(Clone)]
pub struct RateLimitState {
    pub limiter: RateLimiter,
    pub enabled: bool,
}

impl RateLimitState {
    /// Create a new rate limit state.
    pub fn new(limiter: RateLimiter, enabled: bool) -> Self {
        Self { limiter, enabled }
    }
}

/// Rate limiting error response.
#[derive(Debug, Serialize, Deserialize)]
pub struct RateLimitError {
    pub error: String,
    pub message: String,
    pub retry_after_seconds: Option<u64>,
}

/// Tenant-based rate limiting middleware.
///
/// Extracts tenant ID from `X-Tenant-ID` header and applies rate limiting per tenant.
/// Returns 429 Too Many Requests when rate limit is exceeded.
pub async fn tenant_rate_limit(
    axum::extract::State(rate_state): axum::extract::State<RateLimitState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    // Skip if rate limiting is disabled
    if !rate_state.enabled {
        return next.run(request).await;
    }

    // Extract tenant ID from header
    let tenant_id = request
        .headers()
        .get("x-tenant-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("anonymous");

    // Check rate limit
    let (allowed, retry_after) = rate_state.limiter.check_rate_limit(tenant_id);

    if !allowed {
        warn!(
            tenant_id = tenant_id,
            retry_after = ?retry_after,
            "Rate limit exceeded"
        );

        let mut response = (
            StatusCode::TOO_MANY_REQUESTS,
            Json(RateLimitError {
                error: "rate_limit_exceeded".to_string(),
                message: format!("Too many requests for tenant '{}'", tenant_id),
                retry_after_seconds: retry_after,
            }),
        )
            .into_response();

        // Add rate limit headers
        if let Some(retry) = retry_after {
            response.headers_mut().insert(
                "Retry-After",
                HeaderValue::from_str(&retry.to_string()).unwrap(),
            );
        }
        response
            .headers_mut()
            .insert("X-RateLimit-Remaining", HeaderValue::from_static("0"));

        return response;
    }

    // Get remaining tokens for headers
    let state = rate_state.limiter.get_state(tenant_id);
    let mut response = next.run(request).await;

    // Add rate limit headers to successful responses
    if let Some(s) = state {
        response.headers_mut().insert(
            "X-RateLimit-Limit",
            HeaderValue::from_str(&s.capacity.to_string()).unwrap(),
        );
        response.headers_mut().insert(
            "X-RateLimit-Remaining",
            HeaderValue::from_str(&(s.available_tokens as u64).to_string()).unwrap(),
        );
    }

    response
}

// ============================================================================
// Tenant Context Extractor
// ============================================================================

/// Tenant context extracted from request headers.
///
/// Extracts `X-Tenant-ID`, `X-Workspace-ID`, and `X-User-ID` headers from the request.
/// These headers are set by the frontend when a user selects a tenant/workspace.
#[derive(Debug, Clone, Default)]
pub struct TenantContext {
    /// The tenant ID from X-Tenant-ID header.
    pub tenant_id: Option<String>,
    /// The workspace ID from X-Workspace-ID header.
    pub workspace_id: Option<String>,
    /// The user ID from X-User-ID header.
    pub user_id: Option<String>,
}

impl TenantContext {
    /// Extract tenant context from request headers.
    pub fn from_headers(headers: &axum::http::HeaderMap) -> Self {
        let tenant_id = headers
            .get("x-tenant-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let workspace_id = headers
            .get("x-workspace-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let user_id = headers
            .get("x-user-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        Self {
            tenant_id,
            workspace_id,
            user_id,
        }
    }

    /// Check if tenant context is set.
    pub fn has_tenant(&self) -> bool {
        self.tenant_id.is_some()
    }

    /// Check if workspace context is set.
    pub fn has_workspace(&self) -> bool {
        self.workspace_id.is_some()
    }

    /// Check if user context is set.
    pub fn has_user(&self) -> bool {
        self.user_id.is_some()
    }

    /// Get tenant ID as UUID.
    pub fn tenant_id_uuid(&self) -> Option<uuid::Uuid> {
        self.tenant_id
            .as_ref()
            .and_then(|s| uuid::Uuid::parse_str(s).ok())
    }

    /// Get workspace ID as UUID.
    pub fn workspace_id_uuid(&self) -> Option<uuid::Uuid> {
        self.workspace_id
            .as_ref()
            .and_then(|s| uuid::Uuid::parse_str(s).ok())
    }

    /// Get user ID as UUID.
    pub fn user_id_uuid(&self) -> Option<uuid::Uuid> {
        self.user_id
            .as_ref()
            .and_then(|s| uuid::Uuid::parse_str(s).ok())
    }
}

/// Axum extractor for TenantContext.
impl<S> axum::extract::FromRequestParts<S> for TenantContext
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(TenantContext::from_headers(&parts.headers))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_config_default() {
        let config = AuthConfig::default();
        assert!(!config.enabled);
        assert!(config.is_public_path("/health"));
        assert!(config.is_public_path("/ready"));
    }

    #[test]
    fn test_auth_config_with_keys() {
        let config = AuthConfig::with_api_keys(vec!["test-key".to_string()]);
        assert!(config.enabled);
        assert!(config.validate_api_key("test-key"));
        assert!(!config.validate_api_key("wrong-key"));
    }

    #[test]
    fn test_public_paths() {
        let config = AuthConfig::default();
        assert!(config.is_public_path("/health"));
        assert!(config.is_public_path("/swagger-ui/index.html"));
        assert!(!config.is_public_path("/api/v1/documents"));
    }

    #[test]
    fn test_auth_config_multiple_keys() {
        let config = AuthConfig::with_api_keys(vec![
            "key1".to_string(),
            "key2".to_string(),
            "key3".to_string(),
        ]);
        assert!(config.validate_api_key("key1"));
        assert!(config.validate_api_key("key2"));
        assert!(config.validate_api_key("key3"));
        assert!(!config.validate_api_key("key4"));
    }

    #[test]
    fn test_auth_config_empty_keys() {
        let config = AuthConfig::with_api_keys(vec![]);
        assert!(config.enabled);
        assert!(!config.validate_api_key("any-key"));
    }

    #[test]
    fn test_auth_error_serialization() {
        let error = AuthError {
            error: "unauthorized".to_string(),
            message: "Invalid API key".to_string(),
        };

        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("unauthorized"));
        assert!(json.contains("Invalid API key"));
    }

    #[test]
    fn test_auth_error_deserialization() {
        let json = r#"{"error":"forbidden","message":"Access denied"}"#;
        let error: AuthError = serde_json::from_str(json).unwrap();
        assert_eq!(error.error, "forbidden");
        assert_eq!(error.message, "Access denied");
    }

    #[test]
    fn test_auth_state_creation() {
        let config = AuthConfig::default();
        let state = AuthState::new(config);
        assert!(!state.config.enabled);
    }

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.max_requests, 100);
        assert_eq!(config.window_seconds, 60);
    }

    #[test]
    fn test_rate_limit_config_custom() {
        let config = RateLimitConfig {
            enabled: true,
            max_requests: 50,
            window_seconds: 30,
        };
        assert!(config.enabled);
        assert_eq!(config.max_requests, 50);
        assert_eq!(config.window_seconds, 30);
    }

    #[test]
    fn test_rate_limit_config_clone() {
        let config = RateLimitConfig::default();
        let cloned = config.clone();
        assert_eq!(config.enabled, cloned.enabled);
        assert_eq!(config.max_requests, cloned.max_requests);
    }

    #[test]
    fn test_auth_config_debug() {
        let config = AuthConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("enabled"));
        assert!(debug_str.contains("public_paths"));
    }

    #[test]
    fn test_rate_limit_config_debug() {
        let config = RateLimitConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("enabled"));
        assert!(debug_str.contains("max_requests"));
    }

    #[test]
    fn test_public_paths_variations() {
        let config = AuthConfig::default();

        // Check exact public paths
        assert!(config.is_public_path("/health"));
        assert!(config.is_public_path("/ready"));
        assert!(config.is_public_path("/live"));
        assert!(config.is_public_path("/api-docs"));
        assert!(config.is_public_path("/api-docs/openapi.json"));

        // Check swagger paths
        assert!(config.is_public_path("/swagger-ui"));
        assert!(config.is_public_path("/swagger-ui/"));
        assert!(config.is_public_path("/swagger-ui/index.html"));

        // Non-public paths
        assert!(!config.is_public_path("/api/v1/query"));
        assert!(!config.is_public_path("/api/v1/graph"));
        assert!(!config.is_public_path("/admin"));
        assert!(!config.is_public_path("/rapidoc")); // Not in default public paths
    }

    #[test]
    fn test_auth_config_clone() {
        let config = AuthConfig::with_api_keys(vec!["key".to_string()]);
        let cloned = config.clone();
        assert_eq!(config.enabled, cloned.enabled);
        assert!(cloned.validate_api_key("key"));
    }

    #[test]
    fn test_auth_error_debug() {
        let error = AuthError {
            error: "test".to_string(),
            message: "test message".to_string(),
        };
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("test"));
        assert!(debug_str.contains("test message"));
    }
}
