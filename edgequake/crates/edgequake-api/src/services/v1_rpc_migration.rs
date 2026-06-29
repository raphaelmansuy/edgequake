//! v1 RPC → v2 job migration headers (SPEC-027 REST-024/025, ascending-compat).
//!
//! Additive HTTP headers on v1 async RPC responses. REST-025: optional 202 when
//! REST-025: default 202 when async job id present; opt-out via `EDGEQUAKE_V1_RPC_RETURN_202=0`.

use axum::http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::error::{ApiError, ApiResult};

/// Indicative Sunset for v1 RPC async paths (RFC 8594). Successor is Level 4 v2 jobs.
pub const V1_RPC_SUNSET_RFC7231: &str = "Sat, 31 Dec 2028 23:59:59 GMT";

/// Location for a v1 async job accepted for processing (task track id).
pub fn v1_task_location(job_id: &str) -> String {
    format!("/api/v1/tasks/{job_id}")
}

/// RFC 8288 Link + RFC 8594 Sunset headers pointing integrators at v2 workspace jobs.
pub fn v1_rpc_migration_headers(workspace_id: &str) -> ApiResult<HeaderMap> {
    let base = format!("/api/v2/workspaces/{workspace_id}/jobs");
    let link =
        format!("<{base}/catalog>; rel=\"describedby\", <{base}>; rel=\"successor-version\"");
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

/// JSON body with v1→v2 migration headers (REST-024, legacy 200).
pub fn json_with_v1_rpc_migration<T: serde::Serialize>(
    workspace_id: &str,
    body: T,
) -> ApiResult<impl IntoResponse> {
    let headers = v1_rpc_migration_headers(workspace_id)?;
    Ok((headers, Json(body)))
}

/// Respond to v1 async RPC — 202 (default) or 200 when REST-025 legacy opt-out.
pub fn respond_v1_async_rpc<T: serde::Serialize>(
    workspace_id: &str,
    async_job_id: Option<&str>,
    return_202: bool,
    body: T,
) -> ApiResult<Response> {
    let mut headers = v1_rpc_migration_headers(workspace_id)?;

    if return_202 {
        if let Some(job_id) = async_job_id.filter(|id| !id.is_empty()) {
            let location = v1_task_location(job_id);
            headers.insert(
                header::LOCATION,
                HeaderValue::from_str(&location)
                    .map_err(|e| ApiError::Internal(format!("invalid Location: {e}")))?,
            );
            let migration_link = headers
                .get(header::LINK)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            let combined = format!("{migration_link}, <{location}>; rel=\"self\"");
            headers.insert(
                header::LINK,
                HeaderValue::from_str(&combined)
                    .map_err(|e| ApiError::Internal(format!("invalid Link self: {e}")))?,
            );
            return Ok((StatusCode::ACCEPTED, headers, Json(body)).into_response());
        }
    }

    Ok((headers, Json(body)).into_response())
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
        let link = headers
            .get("link")
            .and_then(|v| v.to_str().ok())
            .expect("link");
        assert!(link.contains(ws));
        assert!(link.contains("successor-version"));
        assert!(link.contains("/jobs/catalog"));
    }

    #[test]
    fn respond_202_when_opt_in_and_job_id() {
        let response = respond_v1_async_rpc(
            "ws-1",
            Some("track-abc"),
            true,
            serde_json::json!({"status": "processing"}),
        )
        .expect("response");
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        assert_eq!(
            response
                .headers()
                .get(header::LOCATION)
                .and_then(|v| v.to_str().ok()),
            Some("/api/v1/tasks/track-abc")
        );
        let link = response
            .headers()
            .get(header::LINK)
            .and_then(|v| v.to_str().ok())
            .expect("link");
        assert!(link.contains("successor-version"));
        assert!(link.contains("rel=\"self\""));
    }

    #[test]
    fn respond_200_when_opt_in_but_no_job_id() {
        let response = respond_v1_async_rpc(
            "ws-1",
            None,
            true,
            serde_json::json!({"status": "no_change"}),
        )
        .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
    }
}
