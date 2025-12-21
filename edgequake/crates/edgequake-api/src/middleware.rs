//! HTTP middleware.

use axum::{
    extract::Request,
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
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

    request.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&request_id).unwrap(),
    );

    let mut response = next.run(request).await;

    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&request_id).unwrap(),
    );

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
}

