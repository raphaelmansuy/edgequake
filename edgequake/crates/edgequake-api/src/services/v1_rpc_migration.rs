//! v1 RPC → v2 job migration headers (SPEC-027 REST-024, ascending-compat).
//!
//! Additive HTTP headers on v1 async RPC responses; does not change status codes or paths.

use axum::http::{HeaderMap, HeaderName, HeaderValue};
use axum::response::IntoResponse;
use axum::Json;

use crate::error::{ApiError, ApiResult};

/// Indicative Sunset for v1 RPC async paths (RFC 8594). Successor is Level 4 v2 jobs.
pub const V1_RPC_SUNSET_RFC7231: &str = "Sat, 31 Dec 2028 23:59:59 GMT";

/// RFC 8288 Link + RFC 8594 Sunset headers pointing integrators at v2 workspace jobs.
pub fn v1_rpc_migration_headers(workspace_id: &str) -> ApiResult<HeaderMap> {
    let base = format!("/api/v2/workspaces/{workspace_id}/jobs");
    let link = format!(
        "<{base}/catalog>; rel=\"describedby\", <{base}>; rel=\"successor-version\""
    );
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("link"),
        HeaderValue::from_str(&link)
            .map_err(|e| ApiError::Internal(format!("invalid Link header: {e}")))?,
    );
    headers.insert(
        HeaderName::from_static("sunset"),
        HeaderValue::from_static(V1_RPC_SUNSET_RFC7231),
    );
    Ok(headers)
}

/// JSON body with v1→v2 migration headers (REST-024).
pub fn json_with_v1_rpc_migration<T: serde::Serialize>(
    workspace_id: &str,
    body: T,
) -> ApiResult<impl IntoResponse> {
    let headers = v1_rpc_migration_headers(workspace_id)?;
    Ok((headers, Json(body)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_headers_include_link_and_sunset() {
        let ws = "cccccccc-0027-0027-0027-cccccccccccc";
        let headers = v1_rpc_migration_headers(ws).expect("headers");
        assert_eq!(
            headers.get("sunset").and_then(|v| v.to_str().ok()),
            Some(V1_RPC_SUNSET_RFC7231)
        );
        let link = headers.get("link").and_then(|v| v.to_str().ok()).expect("link");
        assert!(link.contains(ws));
        assert!(link.contains("successor-version"));
        assert!(link.contains("/jobs/catalog"));
    }
}
