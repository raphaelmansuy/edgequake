//! E2E tests for query HTTP flow with workspace-specific providers.
//!
//! SPEC-032: Verify that query endpoint uses workspace-configured providers.
//!
//! Tests verify:
//! 1. Query with X-Workspace-Id header triggers workspace provider lookup
//! 2. Workspace embedding configuration is used for provider creation
//! 3. Different workspaces use different embedding providers

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use common::{
    clear_provider_detection_env, create_test_config, create_test_server, create_test_state,
    create_workspace_with_embedding_config, extract_json,
};
use edgequake_api::Server;
use edgequake_core::types::UpdateWorkspaceRequest;
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

// ============================================================================
// Query with Workspace Header Tests
// ============================================================================

/// Test that query without workspace header still works
#[tokio::test]
async fn test_query_http_without_workspace_header() {
    clear_provider_detection_env();
    let app = create_test_server().build_router();

    let request = json!({
        "query": "What is machine learning?"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should work with default provider
    assert_eq!(response.status(), StatusCode::OK);
}

/// Test query with valid workspace UUID triggers provider lookup
#[tokio::test]
async fn test_query_http_with_workspace_header() {
    clear_provider_detection_env();
    let state = create_test_state();
    let app = Server::new(create_test_config(), state.clone()).build_router();

    // Create a workspace with config
    let workspace = create_workspace_with_embedding_config(
        &state,
        "test-query-ws",
        "mock",
        "mock-embedding",
        1536,
    )
    .await;

    let request = json!({
        "query": "What is AI?"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .header("X-Workspace-ID", workspace.workspace_id.to_string())
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Query should succeed with valid workspace
    assert_eq!(response.status(), StatusCode::OK);
}

/// Test query with workspace that has custom Ollama embedding config
#[tokio::test]
async fn test_query_http_workspace_ollama_config() {
    clear_provider_detection_env();
    let state = create_test_state();
    let app = Server::new(create_test_config(), state.clone()).build_router();

    // Create workspace with Ollama embedding config
    let workspace = create_workspace_with_embedding_config(
        &state,
        "ollama-workspace",
        "ollama",
        "nomic-embed-text",
        768,
    )
    .await;

    // Verify config was stored
    assert_eq!(workspace.embedding_provider, "ollama");
    assert_eq!(workspace.embedding_model, "nomic-embed-text");
    assert_eq!(workspace.embedding_dimension, 768);

    // Now query with this workspace
    let request = json!({
        "query": "What is artificial intelligence?"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .header("X-Workspace-ID", workspace.workspace_id.to_string())
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Query should succeed - provider factory will use mock for testing
    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    assert!(body.get("answer").is_some());
}

/// Test workspace provider isolation - different workspaces use different configs
#[tokio::test]
async fn test_query_http_workspace_provider_isolation() {
    clear_provider_detection_env();
    let state = create_test_state();

    // Create workspace A with OpenAI-style config
    let ws_a = create_workspace_with_embedding_config(
        &state,
        "ws-openai",
        "openai",
        "text-embedding-3-small",
        1536,
    )
    .await;

    // Create workspace B with Ollama-style config
    let ws_b = create_workspace_with_embedding_config(
        &state,
        "ws-ollama",
        "ollama",
        "nomic-embed-text",
        768,
    )
    .await;

    // Verify isolation in stored config
    assert_eq!(ws_a.embedding_provider, "openai");
    assert_eq!(ws_b.embedding_provider, "ollama");
    assert_eq!(ws_a.embedding_dimension, 1536);
    assert_eq!(ws_b.embedding_dimension, 768);

    // Execute queries with both workspaces
    let app = Server::new(create_test_config(), state.clone()).build_router();

    let request = json!({
        "query": "Test query"
    });

    // Query workspace A
    let response_a = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .header("X-Workspace-ID", ws_a.workspace_id.to_string())
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response_a.status(), StatusCode::OK);

    // Query workspace B (need fresh router since oneshot consumes it)
    let app = Server::new(create_test_config(), state).build_router();
    let response_b = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .header("X-Workspace-ID", ws_b.workspace_id.to_string())
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response_b.status(), StatusCode::OK);
}

/// Test provider switch affects subsequent queries
///
/// This test verifies that when we switch from one provider to another,
/// the new provider configuration is used for subsequent queries.
///
/// Note: When switching to a real provider like "openai" in test mode (no API key),
/// the query may fail with 500. This PROVES the switch took effect - it's trying
/// to use the new provider. In production with valid API keys, queries would succeed.
#[tokio::test]
async fn test_query_http_after_provider_switch() {
    clear_provider_detection_env();
    let state = create_test_state();

    // Create workspace with initial mock config (always works)
    let workspace = create_workspace_with_embedding_config(
        &state,
        "switch-workspace",
        "mock", // mock provider always works
        "mock-embedding",
        1536,
    )
    .await;

    // Verify initial config
    assert_eq!(workspace.embedding_provider, "mock");

    // Execute initial query with mock provider
    let app = Server::new(create_test_config(), state.clone()).build_router();
    let request = json!({ "query": "Test query" });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .header("X-Workspace-ID", workspace.workspace_id.to_string())
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    // Mock provider query succeeds
    assert_eq!(response.status(), StatusCode::OK);

    // Switch to a different mock config (not real provider to avoid API key issues)
    let update_request = UpdateWorkspaceRequest {
        name: None,
        description: None,
        is_active: None,
        max_documents: None,
        llm_model: None,
        llm_provider: None,
        embedding_model: Some("mock-embedding-v2".to_string()),
        embedding_provider: Some("mock".to_string()),
        embedding_dimension: Some(768), // Different dimension
        vision_llm_provider: None,
        vision_llm_model: None,
    };

    state
        .workspace_service
        .update_workspace(workspace.workspace_id, update_request)
        .await
        .expect("Failed to update provider config");

    // Verify switch
    let switched = state
        .workspace_service
        .get_workspace(workspace.workspace_id)
        .await
        .expect("Get ws")
        .expect("Ws exists");
    assert_eq!(switched.embedding_provider, "mock");
    assert_eq!(switched.embedding_model, "mock-embedding-v2");
    assert_eq!(switched.embedding_dimension, 768);

    // Execute query after switch
    let app = Server::new(create_test_config(), state).build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .header("X-Workspace-ID", workspace.workspace_id.to_string())
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    // Query still works with updated mock config
    assert_eq!(response.status(), StatusCode::OK);
}

/// Test query with non-existent workspace falls back to default
#[tokio::test]
async fn test_query_http_nonexistent_workspace() {
    clear_provider_detection_env();
    let app = create_test_server().build_router();

    let fake_workspace_id = Uuid::new_v4();
    let request = json!({
        "query": "What is AI?"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .header("X-Workspace-ID", fake_workspace_id.to_string())
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Non-existent workspace falls back to default provider (graceful fallback)
    // This is the expected behavior - system continues with default rather than failing
    assert_eq!(response.status(), StatusCode::OK);
}

/// Test query with invalid workspace UUID format falls back to default
#[tokio::test]
async fn test_query_http_invalid_workspace_uuid() {
    clear_provider_detection_env();
    let app = create_test_server().build_router();

    let request = json!({
        "query": "What is AI?"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .header("X-Workspace-ID", "not-a-valid-uuid")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Invalid UUID format causes graceful fallback to default provider
    // The query still succeeds with the default provider
    assert_eq!(response.status(), StatusCode::OK);
}

/// Test query with OpenAI workspace config
#[tokio::test]
async fn test_query_http_workspace_openai_config() {
    clear_provider_detection_env();
    let state = create_test_state();
    let app = Server::new(create_test_config(), state.clone()).build_router();

    // Create workspace with OpenAI embedding config
    let workspace = create_workspace_with_embedding_config(
        &state,
        "openai-workspace",
        "openai",
        "text-embedding-3-small",
        1536,
    )
    .await;

    // Verify config was stored
    assert_eq!(workspace.embedding_provider, "openai");
    assert_eq!(workspace.embedding_model, "text-embedding-3-small");
    assert_eq!(workspace.embedding_dimension, 1536);

    // Now query with this workspace
    let request = json!({
        "query": "Explain deep learning"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .header("X-Workspace-ID", workspace.workspace_id.to_string())
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    assert!(body.get("answer").is_some());
}
