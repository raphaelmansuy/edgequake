//! RFC 7807 Problem Details helpers (SPEC-027 IMP-028).
//!
//! Adds optional `type`, `title`, and `status` fields to v1 error JSON (additive, AC-2).

use axum::{
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};

use super::ErrorResponse;

/// RFC 7807 media type (additive — legacy `code`/`message` fields preserved).
pub const PROBLEM_JSON_MEDIA_TYPE: &str = "application/problem+json";

/// Base URI for machine-readable problem types.
pub const PROBLEM_TYPE_BASE: &str = "https://edgequake.dev/problems";

/// RFC 7807 `type` URI for an API error code.
pub fn problem_type_for_code(code: &str) -> String {
    let slug = code.to_ascii_lowercase().replace('_', "-");
    format!("{PROBLEM_TYPE_BASE}/{slug}")
}

/// Human-readable RFC 7807 `title` for an API error code.
pub fn problem_title_for_code(code: &str) -> &'static str {
    match code {
        "BAD_REQUEST" => "Bad Request",
        "NOT_FOUND" => "Not Found",
        "UNAUTHORIZED" => "Unauthorized",
        "FORBIDDEN" => "Forbidden",
        "ACCOUNT_LOCKED" => "Account Locked",
        "CONFLICT" => "Conflict",
        "VALIDATION_ERROR" => "Validation Error",
        "RATE_LIMITED" => "Too Many Requests",
        "REQUEST_TIMEOUT" => "Request Timeout",
        "SERVICE_UNAVAILABLE" => "Service Unavailable",
        "NOT_IMPLEMENTED" => "Not Implemented",
        "INTERNAL_ERROR" => "Internal Server Error",
        "CONFIG_ERROR" => "Configuration Error",
        "STORAGE_ERROR" => "Storage Error",
        "LLM_ERROR" => "LLM Provider Error",
        "PIPELINE_ERROR" => "Pipeline Error",
        _ => "Error",
    }
}

/// Build an Axum response with RFC 7807 Content-Type (ascending-compat hybrid JSON).
pub fn into_problem_json_response(
    status: StatusCode,
    error: ErrorResponse,
    extra_headers: &[(&str, String)],
) -> Response {
    let mut response = (status, Json(error)).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(PROBLEM_JSON_MEDIA_TYPE),
    );
    for (name, value) in extra_headers {
        if let (Ok(header_name), Ok(header_value)) = (
            name.parse::<axum::http::HeaderName>(),
            HeaderValue::from_str(value),
        ) {
            response.headers_mut().insert(header_name, header_value);
        }
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn problem_type_uri_is_stable() {
        assert_eq!(
            problem_type_for_code("NOT_FOUND"),
            "https://edgequake.dev/problems/not-found"
        );
    }
}
