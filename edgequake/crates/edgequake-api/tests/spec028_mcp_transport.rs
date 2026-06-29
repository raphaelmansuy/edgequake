//! SPEC-028 MCP Streamable HTTP transport contract + E2E tests.

mod common;

use std::path::PathBuf;

use axum::http::StatusCode;
use common::spec028_mcp::{
    default_mcp_app, mcp_post_bytes, mcp_post_legacy, mcp_post_modern, mcp_post_stream, parse_json,
    MCP_ACCEPT, MCP_PROTOCOL,
};
use edgequake_api::mcp::gateway::body::MCP_MAX_BODY_BYTES;
use serde_json::json;
use tower::ServiceExt;

fn read_crate_src(rel: &str) -> String {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(manifest.join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

#[test]
fn spec028_mcp_gateway_modules_exist() {
    let validate = read_crate_src("src/mcp/gateway/validate.rs");
    assert!(validate.contains("validate_accept"));
    assert!(validate.contains("reject_session_header"));
    let sse = read_crate_src("src/mcp/gateway/sse.rs");
    assert!(sse.contains("notifications/progress"));
    assert!(sse.contains("wants_sse_response"));
    let auth = read_crate_src("src/mcp/auth/protected_resource.rs");
    assert!(auth.contains("authorization_servers"));
    let meta = read_crate_src("src/mcp/gateway/meta.rs");
    assert!(meta.contains("propagation_from_meta"));
    let body_mod = read_crate_src("src/mcp/gateway/body.rs");
    assert!(body_mod.contains("parse_mcp_body"));
    assert!(body_mod.contains("MCP_MAX_BODY_BYTES"));
}

#[tokio::test]
async fn ec_mcp_01_missing_accept_returns_406_on_modern_request() {
    let app = default_mcp_app();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("Content-Type", "application/json")
                .header("MCP-Protocol-Version", MCP_PROTOCOL)
                .body(axum::body::Body::from(
                    json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_ACCEPTABLE);
}

#[tokio::test]
async fn ec_mcp_02_mcp_method_header_mismatch() {
    let app = default_mcp_app();
    let response = app
        .oneshot(mcp_post_modern(
            "/mcp",
            "tools/call",
            json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = parse_json(response).await;
    assert!(body["error"]["message"]
        .as_str()
        .unwrap_or("")
        .contains("HeaderMismatch"));
}

#[tokio::test]
async fn ec_mcp_04_unsupported_protocol_version() {
    let app = default_mcp_app();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("Content-Type", "application/json")
                .header("Accept", MCP_ACCEPT)
                .header("MCP-Protocol-Version", "2099-01-01")
                .header("Mcp-Method", "tools/list")
                .body(axum::body::Body::from(
                    json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn ec_mcp_05_legacy_initialize_shim() {
    let app = default_mcp_app();
    let response = app
        .oneshot(mcp_post_legacy(
            "/mcp",
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": { "protocolVersion": "2025-11-25" }
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await;
    assert_eq!(body["result"]["serverInfo"]["name"], "edgequake-mcp");
}

#[tokio::test]
async fn ec_mcp_03_missing_protocol_version_defaults_legacy() {
    let app = default_mcp_app();
    let response = app
        .oneshot(mcp_post_legacy(
            "/mcp",
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {}
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await;
    assert_eq!(
        body["result"]["protocolVersion"].as_str(),
        Some("2025-11-25"),
        "compat default when MCP-Protocol-Version omitted"
    );
}

#[tokio::test]
async fn ec_mcp_06_legacy_session_header_ignored_without_modern_headers() {
    let app = default_mcp_app();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("Content-Type", "application/json")
                .header("Mcp-Session-Id", "legacy-session-123")
                .body(axum::body::Body::from(
                    json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn ec_mcp_06_modern_session_header_rejected() {
    let app = default_mcp_app();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("Content-Type", "application/json")
                .header("Accept", MCP_ACCEPT)
                .header("MCP-Protocol-Version", MCP_PROTOCOL)
                .header("Mcp-Method", "tools/list")
                .header("Mcp-Session-Id", "must-not-use")
                .body(axum::body::Body::from(
                    json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn ec_mcp_41_sampling_rejected() {
    let app = default_mcp_app();
    let response = app
        .oneshot(mcp_post_legacy(
            "/mcp",
            json!({"jsonrpc":"2.0","id":1,"method":"sampling/createMessage"}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = parse_json(response).await;
    assert_eq!(body["error"]["code"], -32601);
}

#[tokio::test]
async fn ec_mcp_42_roots_list_rejected() {
    let app = default_mcp_app();
    let response = app
        .oneshot(mcp_post_legacy(
            "/mcp",
            json!({"jsonrpc":"2.0","id":1,"method":"roots/list"}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn ec_mcp_43_logging_set_level_rejected() {
    let app = default_mcp_app();
    let response = app
        .oneshot(mcp_post_legacy(
            "/mcp",
            json!({"jsonrpc":"2.0","id":1,"method":"logging/setLevel"}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn ec_mcp_prm_endpoint_public() {
    let app = default_mcp_app();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/.well-known/oauth-protected-resource")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn ec_mcp_tools_list_cache_scope() {
    let app = default_mcp_app();
    let response = app
        .oneshot(mcp_post_legacy(
            "/mcp",
            json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await;
    assert_eq!(body["result"]["cacheScope"], "public");
    assert!(body["result"]["ttlMs"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn ec_mcp_ping_returns_empty_object() {
    let app = default_mcp_app();
    let response = app
        .oneshot(mcp_post_legacy(
            "/mcp",
            json!({"jsonrpc":"2.0","id":1,"method":"ping"}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await;
    assert_eq!(body["result"], json!({}));
}

#[tokio::test]
async fn ec_mcp_server_discover() {
    let app = default_mcp_app();
    let response = app
        .oneshot(mcp_post_legacy(
            "/mcp",
            json!({"jsonrpc":"2.0","id":1,"method":"server/discover"}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await;
    assert!(body["result"]["supportedProtocolVersions"].is_array());
}

#[tokio::test]
async fn ec_mcp_07_batch_array_rejected() {
    let app = default_mcp_app();
    let batch = json!([
        {"jsonrpc":"2.0","id":1,"method":"tools/list"},
        {"jsonrpc":"2.0","id":2,"method":"ping"}
    ]);
    let response = app.oneshot(mcp_post_legacy("/mcp", batch)).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn ec_mcp_08_notification_returns_202() {
    let app = default_mcp_app();
    let response = app
        .oneshot(mcp_post_legacy(
            "/mcp",
            json!({"jsonrpc":"2.0","method":"ping"}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
}

#[tokio::test]
async fn ec_mcp_10_oversized_body_returns_413() {
    let app = default_mcp_app();
    let mut payload = vec![b' '];
    payload.resize(MCP_MAX_BODY_BYTES + 1, b'x');
    let response = app.oneshot(mcp_post_bytes("/mcp", payload)).await.unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn ec_mcp_35_rate_limit_returns_429_on_mcp() {
    use axum::body::Body;
    use axum::http::Request;
    use common::spec028_mcp::build_mcp_app;
    use edgequake_api::AppState;
    use edgequake_rate_limiter::{RateLimitConfig as TokenBucketConfig, RateLimiter};

    let mut state = AppState::test_state();
    state.security.rate_limit_enabled = true;
    state.rate_limiter = RateLimiter::new(TokenBucketConfig::strict(1, 60));
    let app = build_mcp_app(state);
    let tenant = "spec028-mcp-rate-limit";

    let req = || {
        Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("Content-Type", "application/json")
            .header("X-Tenant-ID", tenant)
            .header("X-Workspace-ID", "default")
            .body(Body::from(
                json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
            ))
            .unwrap()
    };

    let first = app.clone().oneshot(req()).await.unwrap();
    assert_eq!(first.status(), StatusCode::OK);

    let second = app.oneshot(req()).await.unwrap();
    assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(
        second.headers().get("retry-after").is_some(),
        "429 must include Retry-After"
    );
}

#[tokio::test]
async fn ec_mcp_registry_well_known_server_json() {
    let app = default_mcp_app();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/.well-known/mcp/server.json")
                .header("Host", "api.edgequake.test:8080")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await;
    assert_eq!(body["name"], "io.github.raphaelmansuy/edgequake");
    assert_eq!(body["remotes"][0]["type"], "streamable-http");
    assert!(body["remotes"][0]["url"]
        .as_str()
        .unwrap()
        .ends_with("/mcp"));
}

#[tokio::test]
async fn ec_mcp_09_retrieve_sse_stream_returns_progress_and_result() {
    let app = default_mcp_app();
    let body = common::spec028_mcp::tools_call_body(
        "edgequake_retrieve",
        json!({ "query": "SSE retrieve test", "mode": "naive", "content_granularity": "agent" }),
    );
    let response = app
        .oneshot(mcp_post_stream(
            "/mcp",
            "tools/call",
            "edgequake_retrieve",
            body,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        content_type.contains("text/event-stream"),
        "expected SSE content-type, got {content_type}"
    );
    assert_eq!(
        response
            .headers()
            .get("x-accel-buffering")
            .and_then(|v| v.to_str().ok()),
        Some("no")
    );
    let raw = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("read sse body");
    let text = String::from_utf8_lossy(&raw);
    assert!(
        text.contains("notifications/progress"),
        "SSE must include progress notifications: {text}"
    );
    assert!(
        text.contains("ret_"),
        "SSE final event must include retrieval_id: {text}"
    );
}
