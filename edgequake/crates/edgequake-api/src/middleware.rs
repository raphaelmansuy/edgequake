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
    http::{HeaderValue, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use edgequake_rate_limiter::RateLimiter;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, warn};

/// Legacy request logging middleware.
///
/// **Deprecated:** use `observability_middleware` (SPEC-018) — single layer for
/// request ID, spans, metrics, and levelled completion logs without duplicate body errors.
#[deprecated(note = "use observability_middleware instead")]
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
        self.api_keys
            .iter()
            .any(|k| crate::services::identity_storage::constant_time_str_eq(k, key))
    }
}

/// Authentication error response.
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthError {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
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
                    request_id: edgequake_observability::current_request_id(),
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
                    request_id: edgequake_observability::current_request_id(),
                }),
            )
                .into_response()
        }
    }
}

/// Extract API key from request headers.
pub async fn protected_api_auth(
    axum::extract::State(state): axum::extract::State<crate::state::AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    if !state.auth.config.auth_enabled {
        return next.run(request).await;
    }

    let path = request.uri().path();
    let method = request.method().clone();

    if is_public_request(&state, &method, path) {
        return next.run(request).await;
    }

    if let Some(token) = extract_api_key(&request) {
        match crate::services::auth_validation::validate_presented_token(&state, &token).await {
            Ok(Some(authenticated)) => {
                if let Some(response) =
                    apply_authenticated_context(&state, &mut request, authenticated)
                {
                    return response;
                }
                #[cfg(feature = "postgres")]
                {
                    if state.security.strict_tenant_bind {
                        if let Some(scope) = membership_bind_scope(&state, &request) {
                            if let Some(response) = enforce_membership_bind(scope).await {
                                return response;
                            }
                        }
                    }
                }
                return next.run(request).await;
            }
            Ok(None) => {}
            Err(e) => {
                return e.into_response();
            }
        }
    }

    unauthorized_response(&method, path)
}

/// Gate Ollama-compatible routes when disabled — SPEC-027 IMP-005.
pub async fn ollama_compat_gate(
    axum::extract::State(state): axum::extract::State<crate::state::AppState>,
    request: Request,
    next: Next,
) -> Response {
    if state.security.enable_ollama_compat {
        return next.run(request).await;
    }

    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({
            "code": "SERVICE_UNAVAILABLE",
            "message": "Ollama compatibility API is disabled (set EDGEQUAKE_OLLAMA_COMPAT_ENABLED=true)"
        })),
    )
        .into_response()
}

/// Rate limiting wrapper reading AppState security flag — SPEC-027 IMP-008.
pub async fn tenant_rate_limit_from_state(
    axum::extract::State(state): axum::extract::State<crate::state::AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let rate_state = RateLimitState::new(
        state.rate_limiter.clone(),
        state.security.rate_limit_enabled,
    );
    tenant_rate_limit(axum::extract::State(rate_state), request, next).await
}

pub(crate) fn apply_authenticated_context(
    state: &crate::state::AppState,
    request: &mut Request,
    authenticated: crate::services::auth_validation::AuthenticatedRequest,
) -> Option<Response> {
    let mut tenant_ctx = TenantContext::from_headers(request.headers());

    if let Some(jwt_tid) = &authenticated.jwt_tenant_id {
        if let Some(response) =
            merge_claim_into_context(state, &mut tenant_ctx.tenant_id, jwt_tid, "tenant_id")
        {
            return Some(response);
        }
    }
    if let Some(jwt_wid) = &authenticated.jwt_workspace_id {
        if let Some(response) =
            merge_claim_into_context(state, &mut tenant_ctx.workspace_id, jwt_wid, "workspace_id")
        {
            return Some(response);
        }
    }

    crate::services::tenant_isolation::attach_pg_isolation_scope(
        request,
        &tenant_ctx,
        Some(&authenticated.auth.user_id),
    );
    request.extensions_mut().insert(authenticated.auth);
    request.extensions_mut().insert(tenant_ctx);
    None
}

/// Resolved tenant/workspace membership scope for strict bind (SPEC-027 phase 34).
#[cfg(feature = "postgres")]
struct MembershipBindScope {
    pool: sqlx::PgPool,
    security: crate::state::ApiSecurityConfig,
    user_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    workspace_id: uuid::Uuid,
}

/// Extract membership scope synchronously — no request borrow across await.
#[cfg(feature = "postgres")]
fn membership_bind_scope(
    state: &crate::state::AppState,
    request: &Request,
) -> Option<MembershipBindScope> {
    let pool = state.pg_pool.clone()?;
    let auth = request
        .extensions()
        .get::<crate::handlers::auth::RequestAuthContext>();
    let tenant_ctx = request.extensions().get::<TenantContext>();

    match (auth, tenant_ctx) {
        (Some(auth), Some(ctx)) if auth.user_id != "master-api-key" => {
            let tenant_id = resolve_tenant_uuid(ctx.tenant_id.as_deref());
            let workspace_id = resolve_workspace_uuid(ctx.workspace_id.as_deref());
            let user_id = uuid::Uuid::parse_str(&auth.user_id).ok();
            match (tenant_id, workspace_id, user_id) {
                (Some(tenant_id), Some(workspace_id), Some(user_id)) => Some(MembershipBindScope {
                    pool,
                    security: state.security.clone(),
                    user_id,
                    tenant_id,
                    workspace_id,
                }),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Verify membership asynchronously (owns scope — no request borrow).
#[cfg(feature = "postgres")]
async fn enforce_membership_bind(scope: MembershipBindScope) -> Option<Response> {
    match crate::services::identity_storage::verify_membership_active(
        &scope.pool,
        &scope.security,
        scope.user_id,
        scope.tenant_id,
        scope.workspace_id,
    )
    .await
    {
        Ok(true) => None,
        Ok(false) => Some(
            (
                StatusCode::FORBIDDEN,
                Json(AuthError {
                    error: "forbidden".to_string(),
                    message: "No active membership for tenant/workspace scope".to_string(),
                    request_id: edgequake_observability::current_request_id(),
                }),
            )
                .into_response(),
        ),
        Err(e) => Some(e.into_response()),
    }
}

fn merge_claim_into_context(
    state: &crate::state::AppState,
    header_value: &mut Option<String>,
    claim_value: &str,
    field: &str,
) -> Option<Response> {
    match header_value.as_deref() {
        None | Some("") => {
            *header_value = Some(claim_value.to_string());
        }
        Some(existing) if existing == claim_value => {}
        Some(existing) => {
            tracing::warn!(
                field = field,
                header = existing,
                claim = claim_value,
                "JWT tenant claim differs from request header (SPEC-027 IMP-004)"
            );
            if state.security.strict_tenant_bind {
                return Some(
                    (
                        StatusCode::FORBIDDEN,
                        Json(AuthError {
                            error: "forbidden".to_string(),
                            message: format!(
                                "Header {field} does not match authenticated token claims"
                            ),
                            request_id: edgequake_observability::current_request_id(),
                        }),
                    )
                        .into_response(),
                );
            }
        }
    }
    None
}

fn unauthorized_response(method: &Method, path: &str) -> Response {
    let request_id =
        edgequake_observability::current_request_id().unwrap_or_else(|| "unknown".to_string());
    tracing::warn!(
        request_id = %request_id,
        method = %method,
        path = %path,
        error.code = "UNAUTHORIZED",
        http.status = 401u16,
        error.source = "auth_middleware",
        "Authentication required"
    );

    (
        StatusCode::UNAUTHORIZED,
        Json(AuthError {
            error: "unauthorized".to_string(),
            message: "Authentication required".to_string(),
            request_id: Some(request_id),
        }),
    )
        .into_response()
}

pub async fn ws_validate_token(state: &crate::state::AppState, token: Option<&str>) -> bool {
    if !state.auth.config.auth_enabled {
        return true;
    }
    let Some(token) = token.filter(|value| !value.is_empty()) else {
        return false;
    };
    matches!(
        crate::services::auth_validation::validate_presented_token(state, token).await,
        Ok(Some(_))
    )
}

fn is_public_request(state: &crate::state::AppState, method: &Method, path: &str) -> bool {
    let normalized_path = path.strip_prefix("/api/v1").unwrap_or(path);

    if is_public_documentation_path(normalized_path) {
        return true;
    }

    matches!(
        normalized_path,
        "/health"
            | "/ready"
            | "/live"
            | "/auth/login"
            | "/auth/refresh"
            | "/auth/oidc/login"
            | "/auth/oidc/callback"
    ) || (*method == Method::POST
        && normalized_path == "/users"
        && state.auth.config.allow_registration)
}

/// OpenAPI / Swagger documentation paths (including static assets under subpaths).
pub(crate) fn is_public_documentation_path(path: &str) -> bool {
    path == "/swagger-ui"
        || path.starts_with("/swagger-ui/")
        || path == "/api-docs"
        || path.starts_with("/api-docs/")
}

pub(crate) fn extract_api_key(request: &Request) -> Option<String> {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub error_code: String,
    pub retryable: bool,
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
        let request_id =
            edgequake_observability::current_request_id().unwrap_or_else(|| "unknown".to_string());

        edgequake_observability::ErrorEvent::log_rate_limit_exceeded(
            &request_id,
            tenant_id,
            retry_after,
        );
        edgequake_observability::record_http_error(
            429,
            "RATE_LIMITED",
            &format!("Too many requests for tenant '{tenant_id}'"),
            Some("rate_limiter"),
            Some(true),
        );
        edgequake_observability::record_rate_limit_exceeded("tenant");

        let mut response = (
            StatusCode::TOO_MANY_REQUESTS,
            Json(RateLimitError {
                error: "rate_limit_exceeded".to_string(),
                message: format!("Too many requests for tenant '{}'", tenant_id),
                retry_after_seconds: retry_after,
                request_id: Some(request_id),
                error_code: "RATE_LIMITED".to_string(),
                retryable: true,
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
// Default identity helpers
// ============================================================================

/// Stable UUID used for the built-in default user in non-authenticated flows.
pub fn default_user_uuid() -> uuid::Uuid {
    uuid::Uuid::from_u128(1)
}

/// Stable UUID used for the built-in default tenant in non-authenticated flows.
pub fn default_tenant_uuid() -> uuid::Uuid {
    uuid::Uuid::from_u128(2)
}

/// Stable UUID used for the built-in default workspace in non-authenticated flows.
pub fn default_workspace_uuid() -> uuid::Uuid {
    edgequake_core::default_workspace_uuid()
}

fn resolve_context_uuid(
    raw_id: Option<&str>,
    default_uuid: Option<uuid::Uuid>,
) -> Option<uuid::Uuid> {
    match raw_id.map(str::trim) {
        None | Some("") => None,
        Some("default") => default_uuid,
        Some(value) => uuid::Uuid::parse_str(value).ok(),
    }
}

/// Resolve a tenant identifier while preserving the legacy `default` alias.
pub fn resolve_tenant_uuid(tenant_id: Option<&str>) -> Option<uuid::Uuid> {
    resolve_context_uuid(tenant_id, Some(default_tenant_uuid()))
}

/// Resolve a workspace identifier while preserving the legacy `default` alias.
pub fn resolve_workspace_uuid(workspace_id: Option<&str>) -> Option<uuid::Uuid> {
    resolve_context_uuid(workspace_id, Some(default_workspace_uuid()))
}

/// Parse a workspace ID string into a UUID, honoring the `default` alias (SPEC-017 API-DRY-005).
pub fn parse_workspace_id(workspace_id: &str) -> Result<uuid::Uuid, crate::error::ApiError> {
    resolve_workspace_uuid(Some(workspace_id)).ok_or_else(|| {
        crate::error::ApiError::BadRequest(format!("Invalid workspace ID: {}", workspace_id))
    })
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
        resolve_tenant_uuid(self.tenant_id.as_deref())
    }

    /// Get tenant ID, falling back to the built-in default tenant.
    pub fn tenant_id_or_default(&self) -> String {
        self.tenant_id_uuid()
            .unwrap_or_else(default_tenant_uuid)
            .to_string()
    }

    /// Get workspace ID as UUID.
    pub fn workspace_id_uuid(&self) -> Option<uuid::Uuid> {
        resolve_workspace_uuid(self.workspace_id.as_deref())
    }

    /// Get workspace ID, falling back to the built-in default workspace.
    pub fn workspace_id_or_default(&self) -> String {
        self.workspace_id_uuid()
            .unwrap_or_else(default_workspace_uuid)
            .to_string()
    }

    /// Get user ID as UUID.
    pub fn user_id_uuid(&self) -> Option<uuid::Uuid> {
        resolve_context_uuid(self.user_id.as_deref(), None)
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
        if let Some(ctx) = parts.extensions.get::<TenantContext>() {
            return Ok(ctx.clone());
        }
        Ok(TenantContext::from_headers(&parts.headers))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_workspace_uuid_maps_default_alias() {
        assert_eq!(
            resolve_workspace_uuid(Some("default")),
            Some(default_workspace_uuid())
        );
        assert_eq!(
            resolve_workspace_uuid(Some("00000000-0000-0000-0000-000000000003")),
            Some(default_workspace_uuid())
        );
    }

    #[test]
    fn test_tenant_context_uuid_helpers_handle_default_aliases() {
        let ctx = TenantContext {
            tenant_id: Some("default".to_string()),
            workspace_id: Some("default".to_string()),
            user_id: None,
        };

        assert_eq!(ctx.tenant_id_uuid(), Some(default_tenant_uuid()));
        assert_eq!(ctx.workspace_id_uuid(), Some(default_workspace_uuid()));
        assert_eq!(
            ctx.tenant_id_or_default(),
            default_tenant_uuid().to_string()
        );
        assert_eq!(
            ctx.workspace_id_or_default(),
            default_workspace_uuid().to_string()
        );
    }

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
            request_id: Some("test-req".to_string()),
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
    fn test_public_documentation_subpaths_for_jwt_middleware() {
        assert!(super::is_public_documentation_path("/swagger-ui"));
        assert!(super::is_public_documentation_path("/swagger-ui/"));
        assert!(super::is_public_documentation_path(
            "/swagger-ui/index.html"
        ));
        assert!(super::is_public_documentation_path("/api-docs"));
        assert!(super::is_public_documentation_path(
            "/api-docs/openapi.json"
        ));
        assert!(!super::is_public_documentation_path("/api/v1/documents"));
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
            request_id: None,
        };
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("test"));
        assert!(debug_str.contains("test message"));
    }
}
