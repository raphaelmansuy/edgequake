//! API error types.
//!
//! ## Implements
//!
//! - [`FEAT0401`]: Consistent API error format
//! - [`FEAT0402`]: HTTP status code mapping
//! - [`FEAT0403`]: Structured error details
//!
//! ## Use Cases
//!
//! - [`UC2001`]: System returns structured error response
//! - [`UC2002`]: Client handles retryable vs non-retryable errors
//!
//! ## Enforces
//!
//! - [`BR0401`]: JSON error response structure
//! - [`BR0402`]: Consistent error code naming
//!
//! # Error Response Format
//!
//! All API errors return JSON with consistent structure:
//!
//! ```json
//! {
//!   "code": "NOT_FOUND",
//!   "message": "Document not found: doc-123",
//!   "details": { "document_id": "doc-123" }
//! }
//! ```
//!
//! # HTTP Status Code Mapping
//!
//! | Error | Status | Retry? | User Action |
//! |-------|--------|--------|-------------|
//! | `BadRequest` | 400 | No | Fix request parameters |
//! | `Unauthorized` | 401 | No | Provide valid API key |
//! | `Forbidden` | 403 | No | Check permissions |
//! | `NotFound` | 404 | No | Use valid resource ID |
//! | `Conflict` | 409 | No | Resolve conflict |
//! | `RateLimited` | 429 | Yes | Wait and retry |
//! | `Internal` | 500 | Maybe | Report if persistent |
//! | `ServiceUnavailable` | 503 | Yes | Wait and retry |
//!
//! @implements FEAT0803

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;

use edgequake_observability::ErrorEvent;
use serde_json::{json, Value};

use crate::providers::ProviderResolutionError;

mod problem_details;

/// Result type for API operations.
pub type ApiResult<T> = std::result::Result<T, ApiError>;

/// Optional context for auth-related 401 responses (single structured log via `into_response`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthFailureContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
}

/// API error response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    /// Error code.
    pub code: String,

    /// Error message.
    pub message: String,

    /// Additional details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,

    /// RFC 7807 problem type URI (additive, SPEC-027 IMP-028).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub problem_type: Option<String>,

    /// RFC 7807 short title (additive).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// RFC 7807 HTTP status echo (additive).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
}

impl ErrorResponse {
    /// Create a new error response.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        let code = code.into();
        Self {
            problem_type: Some(problem_details::problem_type_for_code(&code)),
            title: Some(problem_details::problem_title_for_code(&code).to_string()),
            status: None,
            code,
            message: message.into(),
            details: None,
        }
    }

    /// Add details to the error.
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

/// API errors.
#[derive(Debug, Error)]
pub enum ApiError {
    /// Bad request.
    #[error("Bad request: {0}")]
    BadRequest(String),

    /// Not found.
    #[error("Not found: {0}")]
    NotFound(String),

    /// Resource existed but is no longer available (HTTP 410).
    #[error("Gone: {0}")]
    Gone(String),

    /// Unauthorized (optional auth context for explicit login/refresh diagnostics).
    #[error("Unauthorized")]
    Unauthorized(Option<AuthFailureContext>),

    /// Forbidden (optional reason, e.g. `account_inactive`).
    #[error("Forbidden")]
    Forbidden(Option<String>),

    /// Account locked after too many failed logins (HTTP 423).
    #[error("Account locked")]
    AccountLocked,

    /// Conflict.
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Unprocessable entity.
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Rate limited.
    #[error("Rate limited")]
    RateLimited,

    /// Request timeout.
    /// @implements OODA-01: HTTP-level timeout for document processing
    #[error("Request timeout: {0}")]
    Timeout(String),

    /// Service temporarily unavailable (resource or upstream pressure).
    /// SPEC-006: BR-006-014 — graph timeout must not fall back to full-graph load.
    #[error("Service unavailable: {message}")]
    ServiceUnavailable {
        message: String,
        retry_after_secs: u64,
    },

    /// Not implemented.
    #[error("Not implemented: {feature}")]
    NotImplemented {
        /// Feature name.
        feature: String,
    },

    /// Internal server error.
    #[error("Internal error: {0}")]
    Internal(String),

    /// Configuration error (e.g., missing API keys for workspace provider).
    /// @implements OODA-229: Better error messages for missing API keys
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Storage error.
    #[error("Storage error: {0}")]
    Storage(#[from] edgequake_storage::error::StorageError),

    /// LLM error.
    #[error("LLM error: {0}")]
    Llm(#[from] edgequake_llm::error::LlmError),

    /// Pipeline error.
    #[error("Pipeline error: {0}")]
    Pipeline(#[from] edgequake_pipeline::error::PipelineError),
}

impl ApiError {
    /// Get the HTTP status code for this error.
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Gone(_) => StatusCode::GONE,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::AccountLocked => StatusCode::LOCKED,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::ValidationError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::Timeout(_) => StatusCode::REQUEST_TIMEOUT,
            Self::ServiceUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            Self::NotImplemented { .. } => StatusCode::NOT_IMPLEMENTED,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::ConfigError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Llm(_) => StatusCode::BAD_GATEWAY,
            Self::Pipeline(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Whether clients should retry (rate limits, timeouts, transient upstream errors).
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::RateLimited | Self::Timeout(_) | Self::ServiceUnavailable { .. } => true,
            Self::Storage(e) => storage_error_retryable(e),
            Self::Llm(e) => llm_error_retryable(e),
            Self::Pipeline(e) => pipeline_error_retryable(e),
            _ => false,
        }
    }

    /// Upstream layer that produced the failure (for logs and `details`).
    pub fn source_layer(&self) -> &'static str {
        match self {
            Self::Storage(_) => "storage",
            Self::Llm(_) => "llm",
            Self::Pipeline(_) => "pipeline",
            Self::Unauthorized(Some(_)) | Self::Forbidden(Some(_)) | Self::AccountLocked => "auth",
            _ => "api",
        }
    }

    /// Generic 401 without auth-specific context.
    pub fn unauthorized() -> Self {
        Self::Unauthorized(None)
    }

    /// 401 with explicit login/refresh failure context (one log line in `into_response`).
    pub fn auth_unauthorized(action: &str, reason: &str, subject: Option<&str>) -> Self {
        Self::Unauthorized(Some(AuthFailureContext {
            action: Some(action.into()),
            reason: Some(reason.into()),
            subject: subject.map(|s| s.to_string()),
        }))
    }

    /// Generic 403 without reason.
    pub fn forbidden() -> Self {
        Self::Forbidden(None)
    }

    /// 403 with explicit reason (e.g. inactive account).
    pub fn forbidden_reason(reason: impl Into<String>) -> Self {
        Self::Forbidden(Some(reason.into()))
    }

    /// HTTP 423 — account locked after failed login attempts (SPEC-027 SEC-011).
    pub fn account_locked() -> Self {
        Self::AccountLocked
    }

    /// Variant-specific diagnostics (explicit, not only Display).
    pub fn diagnostic_details(&self) -> Value {
        match self {
            Self::BadRequest(msg) => json!({ "kind": "bad_request", "message": msg }),
            Self::NotFound(msg) => json!({ "kind": "not_found", "resource": msg }),
            Self::Gone(msg) => json!({ "kind": "gone", "resource": msg }),
            Self::Unauthorized(ctx) => auth_failure_diagnostic("unauthorized", ctx.as_ref()),
            Self::Forbidden(reason) => {
                let mut d = json!({ "kind": "forbidden" });
                if let Some(r) = reason {
                    d["reason"] = json!(r);
                }
                d
            }
            Self::AccountLocked => json!({ "kind": "account_locked" }),
            Self::Conflict(msg) => json!({ "kind": "conflict", "message": msg }),
            Self::ValidationError(msg) => json!({ "kind": "validation", "message": msg }),
            Self::RateLimited => json!({ "kind": "rate_limited", "retryable": true }),
            Self::Timeout(msg) => json!({ "kind": "timeout", "message": msg, "retryable": true }),
            Self::ServiceUnavailable {
                message,
                retry_after_secs,
            } => json!({
                "kind": "service_unavailable",
                "message": message,
                "retry_after_secs": retry_after_secs,
                "retryable": true,
            }),
            Self::NotImplemented { feature } => {
                json!({ "kind": "not_implemented", "feature": feature })
            }
            Self::Internal(msg) => json!({ "kind": "internal", "message": msg }),
            Self::ConfigError(msg) => json!({ "kind": "config", "message": msg }),
            Self::Storage(e) => storage_error_diagnostic(e),
            Self::Llm(e) => {
                let retryable = llm_error_retryable(e);
                let mut diag = json!({
                    "kind": "llm",
                    "error": e.to_string(),
                    "retryable": retryable,
                });
                if let Some(provider) = edgequake_observability::current_llm_provider() {
                    diag["provider"] = json!(provider);
                }
                diag
            }
            Self::Pipeline(e) => pipeline_error_diagnostic(e),
        }
    }

    /// Build structured error event for logging and traces.
    pub fn to_error_event(&self, request_id: String) -> ErrorEvent {
        ErrorEvent {
            request_id,
            error_code: self.code().to_string(),
            http_status: self.status_code().as_u16(),
            message: self.to_string(),
            source: Some(self.source_layer().to_string()),
            retryable: self.is_retryable(),
            details: self.diagnostic_details(),
        }
    }

    /// Get the error code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::BadRequest(_) => "BAD_REQUEST",
            Self::NotFound(_) => "NOT_FOUND",
            Self::Gone(_) => "GONE",
            Self::Unauthorized(_) => "UNAUTHORIZED",
            Self::Forbidden(_) => "FORBIDDEN",
            Self::AccountLocked => "ACCOUNT_LOCKED",
            Self::Conflict(_) => "CONFLICT",
            Self::ValidationError(_) => "VALIDATION_ERROR",
            Self::RateLimited => "RATE_LIMITED",
            Self::Timeout(_) => "REQUEST_TIMEOUT",
            Self::ServiceUnavailable { .. } => "SERVICE_UNAVAILABLE",
            Self::NotImplemented { .. } => "NOT_IMPLEMENTED",
            Self::Internal(_) => "INTERNAL_ERROR",
            Self::ConfigError(_) => "CONFIG_ERROR",
            Self::Storage(_) => "STORAGE_ERROR",
            Self::Llm(_) => "LLM_ERROR",
            Self::Pipeline(_) => "PIPELINE_ERROR",
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let request_id =
            edgequake_observability::current_request_id().unwrap_or_else(|| "unknown".to_string());

        let event = self.to_error_event(request_id.clone());
        event.record_on_current_span();
        event.log();

        edgequake_observability::record_http_error(
            status.as_u16(),
            self.code(),
            &self.to_string(),
            Some(self.source_layer()),
            Some(self.is_retryable()),
        );

        if let Self::Storage(e) = &self {
            edgequake_observability::record_storage_error(storage_error_category(e), self.code());
        }

        if let Self::Pipeline(e) = &self {
            let category = pipeline_error_category(e);
            edgequake_observability::record_pipeline_error(category, self.code());
            tracing::debug!(
                request_id = %request_id,
                error.code = %self.code(),
                pipeline.category = %category,
                pipeline.error = %e,
                "Pipeline error mapped to API response"
            );
        }

        if let Self::Llm(e) = &self {
            let provider = edgequake_observability::current_llm_provider()
                .unwrap_or_else(|| "unknown".to_string());
            edgequake_observability::record_llm_request(
                &provider,
                "query_generation",
                "failure",
                0.0,
            );
            tracing::debug!(
                request_id = %request_id,
                error.code = %self.code(),
                llm.error = %e,
                "LLM error mapped to API response"
            );
        }

        let mut error = ErrorResponse::new(self.code(), self.to_string());
        error.status = Some(status.as_u16());
        error.details = Some(event.into_api_details());

        if let Self::ServiceUnavailable {
            retry_after_secs, ..
        } = &self
        {
            return problem_details::into_problem_json_response(
                status,
                error,
                &[(
                    axum::http::header::RETRY_AFTER.as_str(),
                    retry_after_secs.to_string(),
                )],
            );
        }

        problem_details::into_problem_json_response(status, error, &[])
    }
}

impl ApiError {
    /// SPEC-006: graph popular-nodes query exceeded budget — never fall back to full scan.
    pub fn graph_query_timeout() -> Self {
        Self::ServiceUnavailable {
            message: "Graph query exceeded time budget. Retry later.".into(),
            retry_after_secs: 30,
        }
    }

    /// SPEC-006 P3: full-graph operation rejected at admission (community detection, etc.).
    pub fn graph_too_large(node_count: usize, threshold: usize) -> Self {
        Self::ServiceUnavailable {
            message: format!(
                "Graph has {node_count} nodes, exceeding scan threshold of {threshold}. \
                 Operation requires bounded graph size."
            ),
            retry_after_secs: 60,
        }
    }

    /// SPEC-006 P1: concurrent graph materialization cap reached.
    pub fn graph_materialization_busy() -> Self {
        Self::ServiceUnavailable {
            message: "Graph materialization capacity reached. Retry shortly.".into(),
            retry_after_secs: 5,
        }
    }
}

/// SPEC-021 R2: single source of truth for the transient-congestion payload.
///
/// Both the HTTP 503 path (`ApiError::graph_materialization_busy`) and the
/// SSE `error` event path (`graph_stream.rs`) describe the *same* condition.
/// Without a shared struct, the SSE path re-typed the literal message string
/// and silently dropped `retry_after_secs` — a DRY violation that made the
/// streaming transport lossy vs the REST transport. This struct is the one
/// place that defines the reason code + retry hint, so the two transports
/// can never drift again.
#[derive(Debug, Clone, Copy)]
pub struct TransientCongestion {
    /// Machine-readable reason code (e.g. `"transient_congestion"`).
    pub reason: &'static str,
    /// Seconds the client should wait before retrying.
    pub retry_after_secs: u64,
}

impl TransientCongestion {
    /// The transient-congestion payload for graph materialization capacity.
    pub fn graph_materialization_busy() -> Self {
        Self {
            reason: "transient_congestion",
            retry_after_secs: 5,
        }
    }

    /// Build the SSE `error` event fields from this payload.
    pub fn sse_error_fields(
        self,
        message: impl Into<String>,
    ) -> (String, Option<String>, Option<u64>) {
        (
            message.into(),
            Some(self.reason.to_string()),
            Some(self.retry_after_secs),
        )
    }
}

/// Convert ProviderResolutionError to ApiError.
///
/// This implementation provides a unified way to convert provider resolution
/// failures into appropriate HTTP errors with clear error codes.
///
/// ## Mapping
///
/// | ProviderResolutionError | ApiError | Status |
/// |------------------------|----------|--------|
/// | WorkspaceNotFound | NotFound | 404 |
/// | InvalidWorkspaceId | BadRequest | 400 |
/// | InvalidProviderName | BadRequest | 400 |
/// | ProviderCreationFailed (api_key) | ConfigError | 422 |
/// | ProviderCreationFailed (other) | BadRequest | 400 |
/// | WorkspaceServiceError | Internal | 500 |
///
/// @implements OODA-234: Unified error conversion for provider resolution
fn auth_failure_diagnostic(kind: &str, ctx: Option<&AuthFailureContext>) -> Value {
    let mut d = json!({ "kind": kind });
    if let Some(ctx) = ctx {
        if let Some(a) = &ctx.action {
            d["action"] = json!(a);
        }
        if let Some(r) = &ctx.reason {
            d["reason"] = json!(r);
        }
        if let Some(s) = &ctx.subject {
            d["subject"] = json!(s);
        }
    }
    d
}

fn storage_error_category(e: &edgequake_storage::error::StorageError) -> &'static str {
    use edgequake_storage::error::StorageError;
    match e {
        StorageError::Connection(_) => "connection",
        StorageError::NotFound(_) => "not_found",
        StorageError::AlreadyExists(_) => "already_exists",
        StorageError::Conflict(_) => "conflict",
        StorageError::InvalidQuery(_) => "invalid_query",
        StorageError::InvalidInput(_) => "invalid_input",
        StorageError::Transaction(_) => "transaction",
        StorageError::Serialization(_) => "serialization",
        StorageError::Database(_) => "database",
        StorageError::Io(_) => "io",
        StorageError::NotInitialized => "not_initialized",
        StorageError::InvalidConfig(_) => "invalid_config",
        StorageError::InvalidData(_) => "invalid_data",
    }
}

fn storage_error_retryable(e: &edgequake_storage::error::StorageError) -> bool {
    use edgequake_storage::error::StorageError;
    matches!(
        e,
        StorageError::Connection(_) | StorageError::Database(_) | StorageError::Transaction(_)
    )
}

fn storage_error_diagnostic(e: &edgequake_storage::error::StorageError) -> Value {
    json!({
        "kind": "storage",
        "category": storage_error_category(e),
        "error": e.to_string(),
        "retryable": storage_error_retryable(e),
    })
}

fn llm_error_retryable(e: &edgequake_llm::error::LlmError) -> bool {
    let msg = e.to_string().to_ascii_lowercase();
    msg.contains("timeout")
        || msg.contains("rate limit")
        || msg.contains("rate_limit")
        || msg.contains("429")
        || msg.contains("503")
        || msg.contains("overloaded")
        || msg.contains("unavailable")
}

fn pipeline_error_retryable(e: &edgequake_pipeline::error::PipelineError) -> bool {
    use edgequake_pipeline::error::PipelineError;
    matches!(
        e,
        PipelineError::ExtractionTimeout { .. } | PipelineError::CircuitBreakerOpen { .. }
    )
}

fn pipeline_error_category(e: &edgequake_pipeline::error::PipelineError) -> &'static str {
    use edgequake_pipeline::error::PipelineError;
    match e {
        PipelineError::DocumentError(_) => "document",
        PipelineError::ChunkingError(_) => "chunking",
        PipelineError::ExtractionError(_) => "extraction",
        PipelineError::EmbeddingError(_) => "embedding",
        PipelineError::GraphError(_) => "graph",
        PipelineError::StorageError(_) => "storage",
        PipelineError::LlmError(_) => "llm",
        PipelineError::ConfigError(_) => "config",
        PipelineError::NotFound(_) => "not_found",
        PipelineError::InvalidFormat(_) => "invalid_format",
        PipelineError::ExtractionTimeout { .. } => "extraction_timeout",
        PipelineError::RetryExhausted { .. } => "retry_exhausted",
        PipelineError::CircuitBreakerOpen { .. } => "circuit_breaker_open",
        PipelineError::Validation(_) => "validation",
    }
}

fn pipeline_error_diagnostic(e: &edgequake_pipeline::error::PipelineError) -> Value {
    use edgequake_pipeline::error::PipelineError;
    match e {
        PipelineError::StorageError(inner) => json!({
            "kind": "pipeline",
            "category": "storage",
            "storage": storage_error_diagnostic(inner),
            "error": e.to_string(),
        }),
        PipelineError::ExtractionTimeout {
            chunk_index,
            timeout_secs,
            message,
        } => json!({
            "kind": "pipeline",
            "category": "extraction_timeout",
            "chunk_index": chunk_index,
            "timeout_secs": timeout_secs,
            "message": message,
            "retryable": true,
        }),
        PipelineError::RetryExhausted {
            chunk_index,
            attempts,
            message,
        } => json!({
            "kind": "pipeline",
            "category": "retry_exhausted",
            "chunk_index": chunk_index,
            "attempts": attempts,
            "message": message,
        }),
        PipelineError::CircuitBreakerOpen {
            failures,
            retry_after_secs,
        } => json!({
            "kind": "pipeline",
            "category": "circuit_breaker_open",
            "failures": failures,
            "retry_after_secs": retry_after_secs,
            "retryable": true,
        }),
        other => {
            let category = match other {
                PipelineError::DocumentError(_) => "document",
                PipelineError::ChunkingError(_) => "chunking",
                PipelineError::ExtractionError(_) => "extraction",
                PipelineError::EmbeddingError(_) => "embedding",
                PipelineError::GraphError(_) => "graph",
                PipelineError::LlmError(_) => "llm",
                PipelineError::ConfigError(_) => "config",
                PipelineError::NotFound(_) => "not_found",
                PipelineError::InvalidFormat(_) => "invalid_format",
                PipelineError::Validation(_) => "validation",
                _ => "unknown",
            };
            json!({
                "kind": "pipeline",
                "category": category,
                "error": e.to_string(),
            })
        }
    }
}

impl From<ProviderResolutionError> for ApiError {
    fn from(err: ProviderResolutionError) -> Self {
        match err {
            ProviderResolutionError::WorkspaceNotFound { workspace_id } => {
                ApiError::NotFound(format!("Workspace not found: {}", workspace_id))
            }
            ProviderResolutionError::InvalidWorkspaceId(msg) => {
                ApiError::BadRequest(format!("Invalid workspace ID: {}", msg))
            }
            ProviderResolutionError::InvalidProviderName(msg) => {
                ApiError::BadRequest(format!("Invalid provider name: {}", msg))
            }
            ProviderResolutionError::ProviderCreationFailed {
                provider,
                model,
                reason,
                is_api_key_error,
            } => {
                if is_api_key_error {
                    // API key errors are configuration issues
                    ApiError::ConfigError(format!(
                        "Provider '{}' requires API key configuration for model '{}': {}",
                        provider, model, reason
                    ))
                } else {
                    // Other creation failures are bad requests
                    ApiError::BadRequest(format!(
                        "Cannot use provider '{}' with model '{}': {}",
                        provider, model, reason
                    ))
                }
            }
            ProviderResolutionError::WorkspaceServiceError(msg) => {
                ApiError::Internal(format!("Workspace service error: {}", msg))
            }
        }
    }
}

/// Convert query engine errors to semantic HTTP API errors (SPEC-017 P1-07).
impl From<edgequake_query::error::QueryError> for ApiError {
    fn from(e: edgequake_query::error::QueryError) -> Self {
        use edgequake_query::error::QueryError;
        match e {
            QueryError::InvalidQuery(msg) => ApiError::BadRequest(msg),
            QueryError::NoResults => ApiError::NotFound("No results found for query".to_string()),
            QueryError::ContextLimitExceeded { max, got } => ApiError::BadRequest(format!(
                "Context limit exceeded: max {} tokens, got {}",
                max, got
            )),
            QueryError::StorageError(se) => ApiError::Storage(se),
            QueryError::LlmError(le) => ApiError::Llm(le),
            QueryError::ConfigError(msg) => ApiError::ConfigError(msg),
            QueryError::Timeout(ms) => ApiError::Timeout(format!("Query timed out after {}ms", ms)),
            QueryError::Internal(msg) => ApiError::Internal(msg),
        }
    }
}

/// Convert a core domain error into an appropriate HTTP API error.
///
/// ## Semantic mapping (First Principles)
///
/// | CoreError variant  | ApiError variant   | HTTP |
/// |--------------------|-------------------|------|
/// | NotFound           | NotFound          | 404  |
/// | Validation         | ValidationError   | 422  |
/// | Config             | ConfigError       | 422  |
/// | Storage            | Storage           | 500  |
/// | Llm                | Llm               | 502  |
/// | everything else    | Internal          | 500  |
///
/// This single `From` impl is the **only** place where CoreError → ApiError
/// mapping lives (DRY). Handlers just use `?` — no hand-rolled `map_err`.
impl From<edgequake_core::Error> for ApiError {
    fn from(e: edgequake_core::Error) -> Self {
        use edgequake_core::Error as CoreError;
        match e {
            CoreError::NotFound(msg) => ApiError::NotFound(msg),
            CoreError::Validation(msg) => ApiError::ValidationError(msg),
            CoreError::Config(msg) => ApiError::ConfigError(msg),
            CoreError::Storage(se) => ApiError::Storage(se),
            CoreError::Llm(le) => ApiError::Llm(le),
            other => ApiError::Internal(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_response() {
        let error = ErrorResponse::new("NOT_FOUND", "Resource not found")
            .with_details(serde_json::json!({"id": "123"}));

        assert_eq!(error.code, "NOT_FOUND");
        assert!(error.details.is_some());
    }

    #[test]
    fn test_error_status_codes() {
        assert_eq!(
            ApiError::BadRequest("test".into()).status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ApiError::NotFound("test".into()).status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            ApiError::Internal("test".into()).status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_error_response_serialization() {
        let error = ErrorResponse::new("TEST_ERROR", "Test message");
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("TEST_ERROR"));
        assert!(json.contains("Test message"));
        // details should be skipped when None
        assert!(!json.contains("details"));
    }

    #[test]
    fn test_error_response_with_details_serialization() {
        let error = ErrorResponse::new("ERROR", "Message")
            .with_details(serde_json::json!({"key": "value"}));
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("details"));
        assert!(json.contains("key"));
    }

    #[test]
    fn test_error_response_deserialization() {
        let json = r#"{"code":"NOT_FOUND","message":"Resource not found"}"#;
        let error: ErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(error.code, "NOT_FOUND");
        assert_eq!(error.message, "Resource not found");
        assert!(error.details.is_none());
    }

    #[test]
    fn test_all_error_status_codes() {
        assert_eq!(
            ApiError::unauthorized().status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(ApiError::forbidden().status_code(), StatusCode::FORBIDDEN);
        assert_eq!(
            ApiError::Conflict("c".into()).status_code(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            ApiError::ValidationError("v".into()).status_code(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            ApiError::RateLimited.status_code(),
            StatusCode::TOO_MANY_REQUESTS
        );
    }

    #[test]
    fn test_all_error_codes() {
        assert_eq!(ApiError::BadRequest("b".into()).code(), "BAD_REQUEST");
        assert_eq!(ApiError::NotFound("n".into()).code(), "NOT_FOUND");
        assert_eq!(ApiError::unauthorized().code(), "UNAUTHORIZED");
        assert_eq!(ApiError::forbidden().code(), "FORBIDDEN");
        assert_eq!(ApiError::Conflict("c".into()).code(), "CONFLICT");
        assert_eq!(
            ApiError::ValidationError("v".into()).code(),
            "VALIDATION_ERROR"
        );
        assert_eq!(ApiError::RateLimited.code(), "RATE_LIMITED");
        assert_eq!(ApiError::Internal("i".into()).code(), "INTERNAL_ERROR");
    }

    #[test]
    fn test_error_display() {
        let error = ApiError::BadRequest("invalid input".to_string());
        assert_eq!(error.to_string(), "Bad request: invalid input");

        let error = ApiError::NotFound("document".to_string());
        assert_eq!(error.to_string(), "Not found: document");

        let error = ApiError::unauthorized();
        assert_eq!(error.to_string(), "Unauthorized");
    }

    #[test]
    fn test_error_response_clone() {
        let error = ErrorResponse::new("CODE", "Message").with_details(serde_json::json!({"x": 1}));
        let cloned = error.clone();
        assert_eq!(error.code, cloned.code);
        assert_eq!(error.message, cloned.message);
    }

    #[test]
    fn test_error_response_debug() {
        let error = ErrorResponse::new("DEBUG_TEST", "Debug message");
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("DEBUG_TEST"));
        assert!(debug_str.contains("Debug message"));
    }

    #[test]
    fn test_api_error_debug() {
        let error = ApiError::BadRequest("debug test".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("BadRequest"));
        assert!(debug_str.contains("debug test"));
    }

    #[test]
    fn test_not_implemented_error() {
        let error = ApiError::NotImplemented {
            feature: "batch_delete".to_string(),
        };
        assert_eq!(error.code(), "NOT_IMPLEMENTED");
        assert_eq!(error.status_code(), StatusCode::NOT_IMPLEMENTED);
        assert!(error.to_string().contains("batch_delete"));
    }

    #[test]
    fn test_storage_error_status_code() {
        use edgequake_storage::error::StorageError;
        let storage_err = StorageError::NotFound("doc".to_string());
        let api_err = ApiError::Storage(storage_err);
        assert_eq!(api_err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(api_err.code(), "STORAGE_ERROR");
    }

    #[test]
    fn test_llm_error_status_code() {
        use edgequake_llm::error::LlmError;
        let llm_err = LlmError::ApiError("timeout".to_string());
        let api_err = ApiError::Llm(llm_err);
        assert_eq!(api_err.status_code(), StatusCode::BAD_GATEWAY);
        assert_eq!(api_err.code(), "LLM_ERROR");
    }

    #[test]
    fn test_pipeline_error_status_code() {
        use edgequake_pipeline::error::PipelineError;
        let pipeline_err = PipelineError::ChunkingError("chunk failed".to_string());
        let api_err = ApiError::Pipeline(pipeline_err);
        assert_eq!(api_err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(api_err.code(), "PIPELINE_ERROR");
    }

    #[test]
    fn test_query_error_status_code() {
        use edgequake_query::error::QueryError;
        let api_err = ApiError::from(QueryError::InvalidQuery("bad query".to_string()));
        assert_eq!(api_err.status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(api_err.code(), "BAD_REQUEST");

        let api_err = ApiError::from(QueryError::NoResults);
        assert_eq!(api_err.status_code(), StatusCode::NOT_FOUND);

        let api_err = ApiError::from(QueryError::ConfigError("missing key".into()));
        assert_eq!(api_err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn test_error_event_explicit_context() {
        let err = ApiError::NotFound("doc-abc".into());
        let event = err.to_error_event("req-1".into());
        assert_eq!(event.error_code, "NOT_FOUND");
        assert_eq!(event.http_status, 404);
        assert_eq!(event.source.as_deref(), Some("api"));
        assert_eq!(event.details["kind"], "not_found");
        let api_details = event.into_api_details();
        assert_eq!(api_details["request_id"], "req-1");
        assert_eq!(api_details["error_code"], "NOT_FOUND");
        assert!(api_details.get("diagnostics").is_some());
    }

    #[test]
    fn test_storage_error_diagnostic_layer() {
        use edgequake_storage::error::StorageError;
        let err = ApiError::Storage(StorageError::NotFound("chunk-1".into()));
        let event = err.to_error_event("req-2".into());
        assert_eq!(event.source.as_deref(), Some("storage"));
        assert_eq!(event.details["kind"], "storage");
        assert_eq!(event.details["category"], "not_found");
        assert!(event.http_status >= 500);
    }

    #[test]
    fn test_storage_connection_error_is_retryable_with_category() {
        use edgequake_storage::error::StorageError;
        let err = ApiError::Storage(StorageError::Connection("pool exhausted".into()));
        assert!(err.is_retryable());
        assert_eq!(err.diagnostic_details()["category"], "connection");
        assert_eq!(err.diagnostic_details()["retryable"], true);
    }

    #[test]
    fn test_pipeline_circuit_breaker_is_retryable() {
        use edgequake_pipeline::error::PipelineError;
        let err = ApiError::Pipeline(PipelineError::CircuitBreakerOpen {
            failures: 3,
            retry_after_secs: 60,
        });
        assert!(err.is_retryable());
        assert_eq!(err.diagnostic_details()["retryable"], true);
    }

    #[test]
    fn test_auth_failure_includes_reason_in_diagnostics() {
        let err = ApiError::auth_unauthorized("login", "invalid_password", Some("alice"));
        let d = err.diagnostic_details();
        assert_eq!(d["kind"], "unauthorized");
        assert_eq!(d["action"], "login");
        assert_eq!(d["reason"], "invalid_password");
        assert_eq!(d["subject"], "alice");
        assert_eq!(err.source_layer(), "auth");
    }

    #[test]
    fn test_forbidden_includes_reason() {
        let err = ApiError::forbidden_reason("account_inactive");
        assert_eq!(err.diagnostic_details()["reason"], "account_inactive");
        assert_eq!(err.source_layer(), "auth");
    }

    #[test]
    fn test_llm_timeout_is_retryable_not_auth_error() {
        use edgequake_llm::error::LlmError;
        let timeout = ApiError::Llm(LlmError::ApiError("request timeout after 30s".into()));
        assert!(timeout.is_retryable());
        let auth = ApiError::Llm(LlmError::ApiError("invalid_api_key: unauthorized".into()));
        assert!(!auth.is_retryable());
    }

    #[test]
    fn test_pipeline_extraction_timeout_diagnostic() {
        use edgequake_pipeline::error::PipelineError;
        let err = ApiError::Pipeline(PipelineError::ExtractionTimeout {
            chunk_index: 3,
            timeout_secs: 120,
            message: "LLM hung".into(),
        });
        let d = err.diagnostic_details();
        assert_eq!(d["category"], "extraction_timeout");
        assert_eq!(d["chunk_index"], 3);
        assert_eq!(d["retryable"], true);
    }

    #[test]
    fn test_error_response_without_details() {
        let error = ErrorResponse::new("CODE", "Message");
        assert!(error.details.is_none());

        // Verify serialization skips None details
        let json = serde_json::to_value(&error).unwrap();
        assert!(json.get("details").is_none());
    }

    #[test]
    fn test_error_response_builder_pattern() {
        let error = ErrorResponse::new("TEST", "Test")
            .with_details(serde_json::json!({"a": 1}))
            .with_details(serde_json::json!({"b": 2})); // Should overwrite

        assert_eq!(error.details.unwrap()["b"], 2);
    }

    #[test]
    fn test_all_error_variants_have_status_code() {
        // Ensure every error variant has a defined status code
        let errors = vec![
            ApiError::BadRequest("test".into()),
            ApiError::NotFound("test".into()),
            ApiError::unauthorized(),
            ApiError::forbidden(),
            ApiError::Conflict("test".into()),
            ApiError::ValidationError("test".into()),
            ApiError::RateLimited,
            ApiError::NotImplemented {
                feature: "test".into(),
            },
            ApiError::Internal("test".into()),
        ];

        for error in errors {
            let status = error.status_code();
            assert!(status.as_u16() >= 400 && status.as_u16() < 600);
        }
    }

    #[test]
    fn test_error_into_response() {
        let error = ApiError::NotFound("resource".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_api_result_type_alias() {
        fn test_function() -> ApiResult<String> {
            Ok("success".to_string())
        }

        let result = test_function();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }

    #[test]
    fn test_api_result_error() {
        fn test_function() -> ApiResult<String> {
            Err(ApiError::BadRequest("invalid".to_string()))
        }

        let result = test_function();
        assert!(result.is_err());
    }

    // OODA-234: Tests for ProviderResolutionError -> ApiError conversion
    #[test]
    fn test_provider_error_workspace_not_found() {
        let err = ProviderResolutionError::WorkspaceNotFound {
            workspace_id: "ws-123".to_string(),
        };
        let api_err: ApiError = err.into();
        assert_eq!(api_err.code(), "NOT_FOUND");
        assert_eq!(api_err.status_code(), StatusCode::NOT_FOUND);
        assert!(api_err.to_string().contains("ws-123"));
    }

    #[test]
    fn test_provider_error_invalid_workspace_id() {
        let err = ProviderResolutionError::InvalidWorkspaceId("bad-uuid".to_string());
        let api_err: ApiError = err.into();
        assert_eq!(api_err.code(), "BAD_REQUEST");
        assert_eq!(api_err.status_code(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_provider_error_api_key_missing() {
        let err = ProviderResolutionError::ProviderCreationFailed {
            provider: "openai".to_string(),
            model: "gpt-4o-mini".to_string(),
            reason: "OPENAI_API_KEY not set".to_string(),
            is_api_key_error: true,
        };
        let api_err: ApiError = err.into();
        assert_eq!(api_err.code(), "CONFIG_ERROR");
        assert_eq!(api_err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        assert!(api_err.to_string().contains("openai"));
        assert!(api_err.to_string().contains("gpt-4o-mini"));
    }

    #[test]
    fn test_provider_error_creation_failed_not_api_key() {
        let err = ProviderResolutionError::ProviderCreationFailed {
            provider: "ollama".to_string(),
            model: "llama3".to_string(),
            reason: "Connection refused".to_string(),
            is_api_key_error: false,
        };
        let api_err: ApiError = err.into();
        assert_eq!(api_err.code(), "BAD_REQUEST");
        assert_eq!(api_err.status_code(), StatusCode::BAD_REQUEST);
        assert!(api_err.to_string().contains("ollama"));
    }

    #[test]
    fn test_provider_error_service_error() {
        let err =
            ProviderResolutionError::WorkspaceServiceError("DB connection failed".to_string());
        let api_err: ApiError = err.into();
        assert_eq!(api_err.code(), "INTERNAL_ERROR");
        assert_eq!(api_err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    // From<edgequake_core::Error> for ApiError — semantic mapping tests
    #[test]
    fn test_core_not_found_maps_to_404() {
        let core_err = edgequake_core::Error::not_found("Message abc-123 not found");
        let api_err: ApiError = core_err.into();
        assert_eq!(api_err.code(), "NOT_FOUND");
        assert_eq!(api_err.status_code(), StatusCode::NOT_FOUND);
        assert!(api_err.to_string().contains("abc-123"));
    }

    #[test]
    fn test_core_validation_maps_to_422() {
        let core_err = edgequake_core::Error::validation("field 'name' is required");
        let api_err: ApiError = core_err.into();
        assert_eq!(api_err.code(), "VALIDATION_ERROR");
        assert_eq!(api_err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn test_core_config_maps_to_config_error() {
        let core_err = edgequake_core::Error::config("DATABASE_URL not set");
        let api_err: ApiError = core_err.into();
        assert_eq!(api_err.code(), "CONFIG_ERROR");
    }

    #[test]
    fn test_core_internal_maps_to_500() {
        let core_err = edgequake_core::Error::internal("unexpected state");
        let api_err: ApiError = core_err.into();
        assert_eq!(api_err.code(), "INTERNAL_ERROR");
        assert_eq!(api_err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_core_not_initialized_maps_to_500() {
        let core_err = edgequake_core::Error::not_initialized("pipeline not ready");
        let api_err: ApiError = core_err.into();
        // NotInitialized falls through to Internal
        assert_eq!(api_err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
