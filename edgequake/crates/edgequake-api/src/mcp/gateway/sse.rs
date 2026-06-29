//! MCP Streamable HTTP SSE response encoding (2026-07-28).

use std::convert::Infallible;
use std::pin::Pin;

use axum::http::HeaderMap;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use futures::Stream;
use serde_json::{json, Value};
use tokio::task::AbortHandle;

use super::dispatch::{execute_tool_call, DispatchContext};
use super::json_rpc::{error_response, success_response, GatewayError, JsonRpcResponse};

pub const HEADER_MCP_STREAM: &str = "mcp-stream";
pub const HEADER_ACCEL_BUFFERING: &str = "x-accel-buffering";

pub type SseBody = Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>>;

/// Whether this request should use an SSE response stream (MCP-E / EC-MCP-09).
pub fn wants_sse_response(headers: &HeaderMap, method: &str, params: Option<&Value>) -> bool {
    if method != "tools/call" {
        return false;
    }
    let tool = params.and_then(|p| p.get("name")).and_then(|v| v.as_str());
    if tool != Some("edgequake_retrieve") {
        return false;
    }
    header_truthy(headers, HEADER_MCP_STREAM) || meta_stream_enabled(params)
}

fn header_truthy(headers: &HeaderMap, name: &str) -> bool {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| matches!(v.trim(), "1" | "true" | "yes"))
}

fn meta_stream_enabled(params: Option<&Value>) -> bool {
    let Some(params) = params else {
        return false;
    };
    stream_flag(params.get("_meta"))
        || params
            .get("arguments")
            .and_then(|a| a.get("_meta"))
            .is_some_and(|m| stream_flag(Some(m)))
}

fn stream_flag(meta: Option<&Value>) -> bool {
    meta.and_then(|m| m.get("stream"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// JSON-RPC progress notification for SSE stream (request-scoped).
pub fn progress_notification(progress_token: &Value, progress: u8, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "notifications/progress",
        "params": {
            "progressToken": progress_token,
            "progress": progress,
            "total": 100,
            "message": message
        }
    })
}

struct CancelOnDrop(AbortHandle);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Build SSE stream for `edgequake_retrieve` with progress + final JSON-RPC response.
pub fn retrieve_sse_stream(ctx: &DispatchContext<'_>, params: Value, request_id: Value) -> SseBody {
    let task_ctx = ctx.clone_for_task();
    let progress_token = request_id.clone();

    Box::pin(async_stream::stream! {
        yield Ok(Event::default().json_data(progress_notification(
            &progress_token,
            10,
            "Resolving workspace",
        )).expect("sse progress event"));

        let handle = tokio::spawn(async move {
            execute_tool_call(task_ctx, params).await
        });
        let _cancel = CancelOnDrop(handle.abort_handle());

        yield Ok(Event::default().json_data(progress_notification(
            &progress_token,
            50,
            "Running retrieval",
        )).expect("sse progress event"));

        let tool_result: Result<Value, GatewayError> = match handle.await {
            Ok(inner) => inner,
            Err(_) => Err(GatewayError::transport(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                -32603,
                "Retrieval cancelled or failed",
            )),
        };

        yield Ok(Event::default().json_data(progress_notification(
            &progress_token,
            90,
            "Finalizing context bundle",
        )).expect("sse progress event"));

        let final_body: JsonRpcResponse = match tool_result {
            Ok(value) => success_response(request_id, value),
            Err(err) => error_response(request_id, &err),
        };

        yield Ok(Event::default().json_data(final_body).expect("sse final event"));
    })
}

/// Axum SSE response with MCP-required keep-alive comments.
pub fn sse_response(body: SseBody) -> axum::response::Response {
    Sse::new(body)
        .keep_alive(
            KeepAlive::new()
                .interval(std::time::Duration::from_secs(15))
                .text(":"),
        )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    #[test]
    fn mcp_stream_header_enables_sse_for_retrieve() {
        let mut h = HeaderMap::new();
        h.insert(HEADER_MCP_STREAM, "true".parse().unwrap());
        let params = json!({
            "name": "edgequake_retrieve",
            "arguments": { "query": "test" }
        });
        assert!(wants_sse_response(&h, "tools/call", Some(&params)));
    }

    #[test]
    fn sse_not_used_for_search() {
        let mut h = HeaderMap::new();
        h.insert(HEADER_MCP_STREAM, "true".parse().unwrap());
        let params = json!({ "name": "edgequake_search", "arguments": { "query": "x" } });
        assert!(!wants_sse_response(&h, "tools/call", Some(&params)));
    }
}
