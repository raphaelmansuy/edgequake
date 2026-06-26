#![allow(dead_code)]

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

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use edgequake_tasks::worker::{WorkerPool, WorkerPoolConfig};
use serde_json::{json, Value};
use std::time::Duration;
use tower::ServiceExt;

#[cfg(feature = "postgres")]
pub mod spec013_postgres;

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
    "EDGEQUAKE_CHAT_BASE_URL",
    "EDGEQUAKE_CHAT_API_KEY",
    "EDGEQUAKE_CHAT_MODEL",
    "MODEL_PROVIDER",
    "CHAT_PROVIDER",
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
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_BASE_URL",
    "ANTHROPIC_MODEL",
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
    "NVIDIA_API_KEY",
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
pub fn create_test_app() -> axum::Router {
    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: true,
    };
    let server = Server::new(config, AppState::test_state());
    server.build_router()
}

// ============================================================================
// Worker-backed test app (P-G2b)
// ============================================================================

/// Global serialization guard for worker-backed tests. WHY: each test spawns a
/// real `WorkerPool` bound to its own `AppState`. If two such tests run in
/// parallel, one shuts the other's pool down mid-flight (the global slot is
/// single-occupancy) and the orphaned task times out. Holding this async mutex
/// for the test's lifetime makes worker-backed tests run sequentially while
/// leaving non-worker tests parallel.
static TEST_WORKER_GUARD: std::sync::LazyLock<tokio::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

/// Single-occupancy slot for the currently-running worker pool. The next
/// [`create_test_app_with_workers`] call shuts the prior pool down (async)
/// before starting a fresh one.
static TEST_WORKER_POOL: std::sync::OnceLock<std::sync::Mutex<Option<WorkerPool>>> =
    std::sync::OnceLock::new();

async fn shutdown_test_worker_pool() {
    let slot = TEST_WORKER_POOL.get_or_init(|| std::sync::Mutex::new(None));
    let pool = { slot.lock().expect("test worker pool mutex").take() };
    if let Some(pool) = pool {
        pool.shutdown().await;
        // Let in-flight tasks drain before the next test builds a fresh AppState.
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
}

/// RAII guard returned by [`create_test_app_with_workers`]. Dropping it
/// releases the global serialization mutex so the next worker-backed test can
/// run. The worker pool itself is shut down at the start of the next call.
pub struct WorkerAppGuard {
    _serialize: tokio::sync::MutexGuard<'static, ()>,
    pub router: axum::Router,
}

impl WorkerAppGuard {
    /// Borrow the router.
    pub fn app(&self) -> &axum::Router {
        &self.router
    }
}

/// Create a test app whose background `WorkerPool` is started, so enqueued
/// upload tasks actually get processed. Use this (plus
/// [`wait_for_document_processed`]) for any test that needs to observe a
/// document in a terminal (`completed`/`processed`/`partial_failure`) state.
///
/// Returns a [`WorkerAppGuard`] whose `Drop` releases the serialization mutex;
/// keep it alive for the whole test.
pub async fn create_test_app_with_workers() -> WorkerAppGuard {
    // Serialize worker-backed tests (see TEST_WORKER_GUARD).
    let serialize = TEST_WORKER_GUARD.lock().await;

    // Shut down any pool left over from the previous worker-backed test.
    shutdown_test_worker_pool().await;

    let mut state = AppState::test_state();

    // P-G2b: seed the built-in default workspace so the async upload path's
    // strict workspace resolution succeeds (mirrors the production bootstrap).
    state.workspace_service.seed_default_workspace().await;

    let processor = edgequake_api::DocumentTaskProcessor::with_workspace_support_strict(
        std::sync::Arc::clone(&state.query.pipeline),
        std::sync::Arc::clone(&state.query.llm_provider),
        std::sync::Arc::clone(&state.storage.kv_storage),
        std::sync::Arc::clone(&state.storage.vector_storage),
        std::sync::Arc::clone(&state.storage.vector_registry),
        std::sync::Arc::clone(&state.storage.graph_storage),
        state.tasks.pipeline_state.clone(),
        std::sync::Arc::clone(&state.workspace_service),
        std::sync::Arc::clone(&state.query.models_config),
    )
    .with_progress_broadcaster(state.tasks.progress_broadcaster.clone());
    let processor = std::sync::Arc::new(processor);

    let worker_config = WorkerPoolConfig {
        num_workers: 2,
        auto_retry: false,
        initial_retry_delay_ms: 100,
        max_retry_delay_ms: 1_000,
        backoff_multiplier: 2.0,
        max_tasks_per_tenant: 4,
        processing_timeout_secs: 120,
    };

    let mut worker_pool = WorkerPool::new(
        worker_config,
        std::sync::Arc::clone(&state.tasks.queue) as std::sync::Arc<dyn edgequake_tasks::TaskQueue>,
        std::sync::Arc::clone(&state.tasks.storage)
            as std::sync::Arc<dyn edgequake_tasks::TaskStorage>,
        processor,
    );

    state.tasks.cancellation_registry = worker_pool.cancellation_registry();

    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: true,
    };
    let server = Server::new(config, state);
    let router = server.build_router();

    worker_pool.start();
    let slot = TEST_WORKER_POOL.get_or_init(|| std::sync::Mutex::new(None));
    *slot.lock().expect("test worker pool mutex") = Some(worker_pool);

    WorkerAppGuard {
        _serialize: serialize,
        router,
    }
}

/// Remove all environment variables that can change provider selection.
pub fn clear_provider_detection_env() {
    for key in PROVIDER_DETECTION_ENV_VARS {
        std::env::remove_var(key);
    }
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

/// Upload a document and assert it was accepted (201 Created or 202 Accepted).
///
/// WHY (P-G2b / RC-7): uploads now always enqueue a background task and return
/// `202 Accepted` + `task_id` + `status: "pending"`. Older sync semantics
/// (`201 Created` + immediate `processed`) are removed. Tests that need the
/// document fully ingested must call [`wait_for_document_processed`].
pub async fn upload_document_assert(app: &axum::Router, title: &str, content: &str) -> Value {
    let (status, body) = upload_document(app, title, content).await;
    assert!(
        status == StatusCode::CREATED || status == StatusCode::ACCEPTED,
        "Upload should return 201 or 202: {} | body={}",
        status,
        body
    );
    body
}

/// Poll `GET /api/v1/documents/track/{track_id}` until `is_complete` is true,
/// then return the first document's terminal status string.
///
/// P-G2b: replaces the old "upload returns processed immediately" assumption.
/// The track-status endpoint returns `{ is_complete, documents: [{status}] }`.
pub async fn wait_for_document_processed(
    app: &axum::Router,
    track_id: &str,
    timeout: Duration,
) -> String {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let (status, body) =
            get_endpoint(app, &format!("/api/v1/documents/track/{}", track_id)).await;
        if status.is_success() {
            let complete = body
                .get("is_complete")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if complete {
                if let Some(docs) = body.get("documents").and_then(|v| v.as_array()) {
                    if let Some(first) = docs.first() {
                        return first
                            .get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                    }
                }
                return "completed".to_string();
            }
        }
        if std::time::Instant::now() >= deadline {
            return "timeout".to_string();
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Upload + wait until the document reaches a terminal processed state.
///
/// Returns `(document_id, track_id, final_status)`. Asserts the upload was
/// accepted and that the document did not fail/timeout.
pub async fn upload_and_wait(
    app: &axum::Router,
    title: &str,
    content: &str,
    timeout: Duration,
) -> (String, String, String) {
    let body = upload_document_assert(app, title, content).await;
    let document_id = body
        .get("document_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let track_id = body
        .get("track_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    assert!(
        !document_id.is_empty(),
        "upload response missing document_id"
    );
    assert!(!track_id.is_empty(), "upload response missing track_id");
    let final_status = wait_for_document_processed(app, &track_id, timeout).await;
    assert!(
        final_status == "completed"
            || final_status == "processed"
            || final_status == "indexed"
            || final_status == "partial_failure",
        "document did not reach an ingested state: {}",
        final_status
    );
    (document_id, track_id, final_status)
}
