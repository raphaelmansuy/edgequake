//! MCP tool argument validation (structural SSOT aligned with tool-schemas.json).

use edgequake_auth::Role;
use serde_json::Value;

use crate::error::ApiError;

use super::json_rpc::GatewayError;

const SEARCH_MODES: &[&str] = &["naive", "local", "global", "hybrid", "mix"];
const GRANULARITIES: &[&str] = &["citation", "agent", "debug"];
const MAX_RESULTS_CAP: i64 = 50;

/// Validate tool name + arguments before execution.
pub fn validate_tool_call(name: &str, arguments: &Value) -> Result<(), GatewayError> {
    validate_tool_call_with_role(name, arguments, None)
}

/// Validate tool args with optional caller role (debug granularity policy).
pub fn validate_tool_call_with_role(
    name: &str,
    arguments: &Value,
    role: Option<Role>,
) -> Result<(), GatewayError> {
    match name {
        "edgequake_search" => validate_search(arguments),
        "edgequake_fetch" => {
            validate_fetch(arguments)?;
            enforce_debug_granularity(arguments, role)?;
            Ok(())
        }
        "edgequake_retrieve" => {
            validate_retrieve(arguments)?;
            enforce_debug_granularity(arguments, role)?;
            Ok(())
        }
        other => Err(GatewayError::Api(ApiError::BadRequest(format!(
            "Unknown tool: {other}"
        )))),
    }
}

/// EC-MCP-29: debug granularity requires admin when auth is enforced.
pub fn enforce_debug_granularity(
    arguments: &Value,
    role: Option<Role>,
) -> Result<(), GatewayError> {
    let wants_debug = arguments
        .get("content_granularity")
        .and_then(|v| v.as_str())
        .is_some_and(|g| g.eq_ignore_ascii_case("debug"));
    if !wants_debug {
        return Ok(());
    }
    match role {
        Some(Role::Admin) => Ok(()),
        Some(_) => Err(GatewayError::Api(ApiError::forbidden_reason(
            "content_granularity debug requires admin role",
        ))),
        None => Ok(()),
    }
}

fn validate_search(arguments: &Value) -> Result<(), GatewayError> {
    require_non_empty_query(arguments)?;
    validate_mode(arguments)?;
    validate_max_results(arguments)?;
    Ok(())
}

fn validate_retrieve(arguments: &Value) -> Result<(), GatewayError> {
    require_non_empty_query(arguments)?;
    validate_mode(arguments)?;
    validate_max_results(arguments)?;
    if let Some(g) = arguments
        .get("content_granularity")
        .and_then(|v| v.as_str())
    {
        validate_granularity(g)?;
    }
    Ok(())
}

fn validate_fetch(arguments: &Value) -> Result<(), GatewayError> {
    let id = arguments
        .get("retrieval_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if !id.starts_with("ret_") {
        return Err(GatewayError::Api(ApiError::BadRequest(
            "Invalid retrieval_id".into(),
        )));
    }
    if let Some(g) = arguments
        .get("content_granularity")
        .and_then(|v| v.as_str())
    {
        validate_granularity(g)?;
    }
    Ok(())
}

fn require_non_empty_query(arguments: &Value) -> Result<(), GatewayError> {
    let query = arguments
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if query.trim().is_empty() {
        return Err(GatewayError::Api(ApiError::BadRequest(
            "query is required".into(),
        )));
    }
    Ok(())
}

fn validate_mode(arguments: &Value) -> Result<(), GatewayError> {
    let Some(mode) = arguments.get("mode").and_then(|v| v.as_str()) else {
        return Ok(());
    };
    if mode.eq_ignore_ascii_case("bypass") {
        return Err(GatewayError::Api(ApiError::BadRequest(
            "bypass mode is not allowed on MCP tools".into(),
        )));
    }
    if !SEARCH_MODES.iter().any(|m| m.eq_ignore_ascii_case(mode)) {
        return Err(GatewayError::Api(ApiError::BadRequest(format!(
            "Invalid mode: {mode}"
        ))));
    }
    Ok(())
}

fn validate_granularity(value: &str) -> Result<(), GatewayError> {
    if GRANULARITIES.iter().any(|g| g.eq_ignore_ascii_case(value)) {
        Ok(())
    } else {
        Err(GatewayError::Api(ApiError::BadRequest(format!(
            "Invalid content_granularity: {value}"
        ))))
    }
}

fn validate_max_results(arguments: &Value) -> Result<(), GatewayError> {
    let Some(n) = arguments.get("max_results") else {
        return Ok(());
    };
    let Some(v) = n.as_i64() else {
        return Err(GatewayError::Api(ApiError::BadRequest(
            "max_results must be an integer".into(),
        )));
    };
    if !(1..=MAX_RESULTS_CAP).contains(&v) {
        return Err(GatewayError::Api(ApiError::BadRequest(format!(
            "max_results must be between 1 and {MAX_RESULTS_CAP}"
        ))));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_bypass_mode() {
        let err = validate_search(&json!({"query": "x", "mode": "bypass"})).unwrap_err();
        assert!(err.json_rpc_error().message.contains("bypass"));
    }

    #[test]
    fn rejects_invalid_mode() {
        assert!(validate_search(&json!({"query": "x", "mode": "invalid"})).is_err());
    }

    #[test]
    fn rejects_max_results_over_cap() {
        assert!(validate_search(&json!({"query": "x", "max_results": 100})).is_err());
    }

    #[test]
    fn debug_granularity_requires_admin_when_role_user() {
        let err =
            enforce_debug_granularity(&json!({"content_granularity": "debug"}), Some(Role::User))
                .unwrap_err();
        assert_eq!(
            err.json_rpc_http_status(),
            axum::http::StatusCode::FORBIDDEN
        );
    }

    #[test]
    fn debug_granularity_allowed_for_admin() {
        enforce_debug_granularity(&json!({"content_granularity": "debug"}), Some(Role::Admin))
            .expect("admin may use debug");
    }
}
