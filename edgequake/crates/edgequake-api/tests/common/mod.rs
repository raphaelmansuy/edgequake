//! Shared test helpers for E2E tests (OODA-10 through OODA-18+).
//!
//! WHY: All OODA E2E test files duplicate the same helper functions:
//! create_test_app, extract_json, post_json, with_timeout, etc.
//! Extracting them into a single module reduces duplication and ensures
//! consistent behavior across all tests (DRY principle).
//!
//! ## Usage
//! ```ignore
//! mod common;
//! use common::*;
//! ```

#![allow(dead_code)]
// WHY: this module is a shared E2E harness. Individual binaries pull only the
// helpers they need, so unused items here are an intentional reuse surface.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use edgequake_core::types::CreateWorkspaceRequest;
use edgequake_core::{Tenant, Workspace};
use serde_json::{json, Value};
use std::time::Duration;
use tower::ServiceExt;
use uuid::Uuid;

// ============================================================================
// Constants
// ============================================================================

/// Default test tenant ID (valid UUID for endpoints requiring tenant context).
pub const TEST_TENANT_ID: &str = "aaaaaaaa-0019-0019-0019-aaaaaaaaaaaa";
/// Default test user ID (valid UUID for conversation/auth endpoints).
pub const TEST_USER_ID: &str = "bbbbbbbb-0019-0019-0019-bbbbbbbbbbbb";
/// Default test workspace ID (valid UUID for workspace-scoped operations).
pub const TEST_WORKSPACE_ID: &str = "cccccccc-0019-0019-0019-cccccccccccc";

/// Environment variables that influence provider auto-detection.
///
/// WHY: E2E tests must not depend on whatever third-party credentials happen to
/// exist on the developer machine or CI runner. Keeping this list centralized
/// makes provider-related tests deterministic and easier to maintain.
const PROVIDER_DETECTION_ENV_VARS: &[&str] = &[
    "EDGEQUAKE_LLM_PROVIDER",
    "EDGEQUAKE_LLM_MODEL",
    "EDGEQUAKE_EMBEDDING_PROVIDER",
    "EDGEQUAKE_EMBEDDING_MODEL",
    "EDGEQUAKE_EMBEDDING_DIMENSION",
    "EDGEQUAKE_DEFAULT_LLM_PROVIDER",
    "EDGEQUAKE_DEFAULT_LLM_MODEL",
    "EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER",
    "EDGEQUAKE_DEFAULT_EMBEDDING_MODEL",
    "EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION",
    "MODEL_PROVIDER",
    "CHAT_MODEL",
    "EMBEDDING_PROVIDER",
    "EMBEDDING_MODEL",
    "EMBEDDING_DIMENSION",
    "OLLAMA_HOST",
    "OLLAMA_MODEL",
    "LMSTUDIO_HOST",
    "LMSTUDIO_MODEL",
    "OPENAI_API_KEY",
    "OPENAI_BASE_URL",
    "ANTHROPIC_API_KEY",
    "GEMINI_API_KEY",
    "GOOGLE_API_KEY",
    "GOOGLE_ACCESS_TOKEN",
    "GOOGLE_CLOUD_PROJECT",
    "GOOGLE_CLOUD_REGION",
    "GOOGLE_CLOUD_LOCATION",
    "GOOGLE_APPLICATION_CREDENTIALS",
    "MISTRAL_API_KEY",
    "XAI_API_KEY",
    "OPENROUTER_API_KEY",
    "AZURE_OPENAI_API_KEY",
    "AZURE_OPENAI_ENDPOINT",
    "AZURE_OPENAI_CONTENTGEN_API_KEY",
    "AZURE_OPENAI_CONTENTGEN_API_ENDPOINT",
    "MINIMAX_API_KEY",
    "HF_TOKEN",
    "HUGGINGFACE_TOKEN",
];

// ============================================================================
// App Setup
// ============================================================================

/// Create a fresh test app with in-memory state and mock pipeline.
///
/// WHY: Each test gets an isolated state to avoid cross-test interference.
/// The mock provider returns "Mock response" for LLM calls and vec![0.1; 1536]
/// for embeddings, which means entity extraction produces 0 entities.
pub fn create_test_config() -> ServerConfig {
    ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: true,
    }
}

/// Create isolated in-memory application state for a single test.
pub fn create_test_state() -> AppState {
    AppState::test_state()
}

/// Create a server instance backed by isolated in-memory test state.
pub fn create_test_server() -> Server {
    Server::new(create_test_config(), create_test_state())
}

pub fn create_test_app() -> axum::Router {
    let server = create_test_server();
    server.build_router()
}

/// Remove all environment variables that can change provider selection.
pub fn clear_provider_detection_env() {
    for key in PROVIDER_DETECTION_ENV_VARS {
        std::env::remove_var(key);
    }
}

/// Create a workspace with explicit LLM and embedding provider configuration.
///
/// WHY: Provider-routing tests all follow the same tenant -> workspace setup
/// flow. Centralizing it keeps test setup deterministic while still making
/// the provider intent explicit at each call site.
pub async fn create_workspace_with_providers(
    state: &AppState,
    name: &str,
    llm_provider: &str,
    llm_model: &str,
    embedding_provider: &str,
    embedding_model: &str,
    embedding_dimension: usize,
) -> Workspace {
    let tenant = Tenant::new(
        format!("Test Tenant {}", name),
        format!("test-{}", Uuid::new_v4()),
    );
    let created_tenant = state
        .workspace_service
        .create_tenant(tenant)
        .await
        .expect("Should create tenant");

    let request = CreateWorkspaceRequest::new(name)
        .with_llm_config(llm_model, llm_provider)
        .with_embedding_config(embedding_model, embedding_provider, embedding_dimension);

    state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, request)
        .await
        .expect("Should create workspace")
}

/// Create a workspace whose query tests only care about embedding config.
pub async fn create_workspace_with_embedding_config(
    state: &AppState,
    name: &str,
    embedding_provider: &str,
    embedding_model: &str,
    embedding_dimension: usize,
) -> Workspace {
    create_workspace_with_providers(
        state,
        name,
        "mock",
        "mock-model",
        embedding_provider,
        embedding_model,
        embedding_dimension,
    )
    .await
}

// ============================================================================
// Timeout
// ============================================================================

/// Wrap a test body with a timeout. Returns Err if the test exceeds the duration.
///
/// WHY: E2E tests must not hang indefinitely. 30s is a reasonable default.
pub async fn with_timeout<F, T>(duration: Duration, future: F) -> Result<T, String>
where
    F: std::future::Future<Output = T>,
{
    tokio::time::timeout(duration, future)
        .await
        .map_err(|_| format!("Test exceeded timeout of {:?}", duration))
}

// ============================================================================
// Response Extraction
// ============================================================================

/// Extract JSON from an Axum response body.
///
/// Returns Value::Null if body is empty or not valid JSON.
pub async fn extract_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("Failed to read response body");
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

// ============================================================================
// HTTP Helpers (no headers)
// ============================================================================

/// POST JSON to an endpoint (no tenant headers).
pub async fn post_json(app: &axum::Router, uri: &str, payload: &Value) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = extract_json(response).await;
    (status, body)
}

/// GET an endpoint (no tenant headers).
pub async fn get_endpoint(app: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = extract_json(response).await;
    (status, body)
}

/// DELETE an endpoint (no tenant headers).
pub async fn delete_endpoint(app: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = extract_json(response).await;
    (status, body)
}

// ============================================================================
// HTTP Helpers (with tenant headers)
// ============================================================================

/// POST JSON with X-Tenant-ID, X-User-ID, X-Workspace-ID headers.
///
/// WHY: Conversation and reprocess endpoints require valid UUID headers.
pub async fn post_json_with_tenant(
    app: &axum::Router,
    uri: &str,
    payload: &Value,
    tenant_id: &str,
    user_id: &str,
    workspace_id: &str,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("Content-Type", "application/json")
                .header("X-Tenant-ID", tenant_id)
                .header("X-User-ID", user_id)
                .header("X-Workspace-ID", workspace_id)
                .body(Body::from(serde_json::to_string(payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = extract_json(response).await;
    (status, body)
}

/// GET with X-Tenant-ID, X-User-ID, X-Workspace-ID headers.
pub async fn get_with_tenant(
    app: &axum::Router,
    uri: &str,
    tenant_id: &str,
    user_id: &str,
    workspace_id: &str,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .header("X-Tenant-ID", tenant_id)
                .header("X-User-ID", user_id)
                .header("X-Workspace-ID", workspace_id)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = extract_json(response).await;
    (status, body)
}

// ============================================================================
// Document Upload Helper
// ============================================================================

/// Upload a JSON document and return (status, body).
///
/// Convenience wrapper for POST /api/v1/documents.
pub async fn upload_document(
    app: &axum::Router,
    title: &str,
    content: &str,
) -> (StatusCode, Value) {
    let payload = json!({
        "content": content,
        "title": title
    });
    post_json(app, "/api/v1/documents", &payload).await
}

/// Upload a document and assert it was created (201).
pub async fn upload_document_assert(app: &axum::Router, title: &str, content: &str) -> Value {
    let (status, body) = upload_document(app, title, content).await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "Upload should return 201: {}",
        body
    );
    body
}
