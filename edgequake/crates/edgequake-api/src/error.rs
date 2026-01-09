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
/// @implements FEAT0803
//! | `Forbidden` | 403 | No | Check permissions |
//! | `NotFound` | 404 | No | Use valid resource ID |
//! | `Conflict` | 409 | No | Resolve conflict |
//! | `RateLimited` | 429 | Yes | Wait and retry |
//! | `Internal` | 500 | Maybe | Report if persistent |
//! | `ServiceUnavailable` | 503 | Yes | Wait and retry |

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Result type for API operations.
pub type ApiResult<T> = std::result::Result<T, ApiError>;

/// API error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error code.
    pub code: String,

    /// Error message.
    pub message: String,

    /// Additional details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ErrorResponse {
    /// Create a new error response.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
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

    /// Unauthorized.
    #[error("Unauthorized")]
    Unauthorized,

    /// Forbidden.
    #[error("Forbidden")]
    Forbidden,

    /// Conflict.
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Unprocessable entity.
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Rate limited.
    #[error("Rate limited")]
    RateLimited,

    /// Not implemented.
    #[error("Not implemented: {feature}")]
    NotImplemented {
        /// Feature name.
        feature: String,
    },

    /// Internal server error.
    #[error("Internal error: {0}")]
    Internal(String),

    /// Storage error.
    #[error("Storage error: {0}")]
    Storage(#[from] edgequake_storage::error::StorageError),

    /// LLM error.
    #[error("LLM error: {0}")]
    Llm(#[from] edgequake_llm::error::LlmError),

    /// Pipeline error.
    #[error("Pipeline error: {0}")]
    Pipeline(#[from] edgequake_pipeline::error::PipelineError),

    /// Query error.
    #[error("Query error: {0}")]
    Query(#[from] edgequake_query::error::QueryError),
}

impl ApiError {
    /// Get the HTTP status code for this error.
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::ValidationError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::NotImplemented { .. } => StatusCode::NOT_IMPLEMENTED,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Llm(_) => StatusCode::BAD_GATEWAY,
            Self::Pipeline(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Query(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Get the error code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::BadRequest(_) => "BAD_REQUEST",
            Self::NotFound(_) => "NOT_FOUND",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Forbidden => "FORBIDDEN",
            Self::Conflict(_) => "CONFLICT",
            Self::ValidationError(_) => "VALIDATION_ERROR",
            Self::RateLimited => "RATE_LIMITED",
            Self::NotImplemented { .. } => "NOT_IMPLEMENTED",
            Self::Internal(_) => "INTERNAL_ERROR",
            Self::Storage(_) => "STORAGE_ERROR",
            Self::Llm(_) => "LLM_ERROR",
            Self::Pipeline(_) => "PIPELINE_ERROR",
            Self::Query(_) => "QUERY_ERROR",
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error = ErrorResponse::new(self.code(), self.to_string());

        (status, Json(error)).into_response()
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
            ApiError::Unauthorized.status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(ApiError::Forbidden.status_code(), StatusCode::FORBIDDEN);
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
        assert_eq!(ApiError::Unauthorized.code(), "UNAUTHORIZED");
        assert_eq!(ApiError::Forbidden.code(), "FORBIDDEN");
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

        let error = ApiError::Unauthorized;
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
        let query_err = QueryError::InvalidQuery("bad query".to_string());
        let api_err = ApiError::Query(query_err);
        assert_eq!(api_err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(api_err.code(), "QUERY_ERROR");
    }

    #[test]
    fn test_error_response_without_details() {
        let error = ErrorResponse::new("CODE", "Message");
        assert!(error.details.is_none());

        // Verify serialization skips None details
        let json = serde_json::to_value(&error).unwrap();
        assert!(!json.get("details").is_some());
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
            ApiError::Unauthorized,
            ApiError::Forbidden,
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
}
