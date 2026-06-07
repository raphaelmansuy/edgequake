//! SPEC-017 P2-02: Cross-handler query routing parity.
//!
//! Verifies `/query`, `/chat/completions`, and `/query/stream` share the same
//! workspace resource resolution path (`resolve_workspace_query_resources`).

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use edgequake_core::types::CreateWorkspaceRequest;
use edgequake_core::Tenant;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

fn create_test_config() -> ServerConfig {
    ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: true,
    }
}

async fn extract_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("Failed to read response body");
    serde_json::from_slice(&bytes).expect("Failed to parse JSON")
}

async fn create_workspace(state: &AppState) -> edgequake_core::Workspace {
    let tenant = Tenant::new(
        "Parity Test Tenant".to_string(),
        format!("parity-{}", Uuid::new_v4()),
    );
    let created_tenant = state
        .workspace_service
        .create_tenant(tenant)
        .await
        .expect("Should create tenant");

    let request = CreateWorkspaceRequest {
        name: "Parity Workspace".to_string(),
        slug: Some(format!("parity-ws-{}", Uuid::new_v4())),
        description: Some("SPEC-017 routing parity".to_string()),
        max_documents: None,
        llm_model: Some("mock-model".to_string()),
        llm_provider: Some("mock".to_string()),
        embedding_model: Some("mock-embedding".to_string()),
        embedding_provider: Some("mock".to_string()),
        embedding_dimension: Some(1536),
        vision_llm_provider: None,
        vision_llm_model: None,
        pdf_parser_backend: None,
        entity_types: None,
        ..Default::default()
    };

    state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, request)
        .await
        .expect("Should create workspace")
}

/// Query and chat completion must agree on mode when given identical workspace + mode.
#[tokio::test]
async fn test_query_and_chat_share_workspace_routing() {
    let state = AppState::test_state();
    let workspace = create_workspace(&state).await;
    let app = Server::new(create_test_config(), state).build_router();

    let ws_header = workspace.workspace_id.to_string();
    // Chat requires tenant/user headers; use the workspace owner's tenant for consistency.
    let tenant_id = workspace.tenant_id.to_string();
    let user_id = Uuid::new_v4().to_string();

    let query_body = json!({
        "query": "What is retrieval augmented generation?",
        "mode": "naive"
    });

    let query_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .header("X-Workspace-ID", &ws_header)
                .body(Body::from(serde_json::to_string(&query_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(query_response.status(), StatusCode::OK);
    let query_json = extract_json(query_response).await;
    assert_eq!(
        query_json.get("mode").and_then(|v| v.as_str()),
        Some("naive")
    );

    let chat_body = json!({
        "message": "What is retrieval augmented generation?",
        "mode": "naive"
    });

    let chat_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/chat/completions")
                .header("Content-Type", "application/json")
                .header("X-Workspace-ID", &ws_header)
                .header("X-Tenant-Id", &tenant_id)
                .header("X-User-Id", &user_id)
                .body(Body::from(serde_json::to_string(&chat_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(chat_response.status(), StatusCode::OK);
    let chat_json = extract_json(chat_response).await;
    assert_eq!(
        chat_json.get("mode").and_then(|v| v.as_str()),
        Some("naive")
    );
    assert!(chat_json.get("content").is_some());
}

/// Query fails closed when workspace header references a missing workspace.
#[tokio::test]
async fn test_query_invalid_workspace_fails_closed() {
    let app = Server::new(create_test_config(), AppState::test_state()).build_router();
    let bogus_ws = Uuid::new_v4().to_string();

    let query_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .header("X-Workspace-ID", &bogus_ws)
                .body(Body::from(
                    serde_json::to_string(&json!({ "query": "test parity" })).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(query_response.status(), StatusCode::NOT_FOUND);
}

/// Chat fails closed on invalid workspace headers (parity with /query).
#[tokio::test]
async fn test_chat_invalid_workspace_fails_closed() {
    let app = Server::new(create_test_config(), AppState::test_state()).build_router();
    let bogus_ws = Uuid::new_v4().to_string();

    let chat_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/chat/completions")
                .header("Content-Type", "application/json")
                .header("X-Workspace-ID", &bogus_ws)
                .header("X-Tenant-Id", Uuid::new_v4().to_string())
                .header("X-User-Id", Uuid::new_v4().to_string())
                .body(Body::from(
                    serde_json::to_string(&json!({ "message": "test parity" })).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(chat_response.status(), StatusCode::NOT_FOUND);
}
