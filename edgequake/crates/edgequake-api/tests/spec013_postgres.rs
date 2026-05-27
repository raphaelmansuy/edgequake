//! Shared PostgreSQL harness for SPEC-013 E2E tests.
//!
//! Requires: `cargo test -p edgequake-api --features postgres`
//! and `DATABASE_URL` (or `POSTGRES_PASSWORD` + host/port/user/db).
//!
//! ## First-principles (anti-flake)
//!
//! 1. **Worker pool** — in-process tests must start `WorkerPool` (same as `main.rs`).
//! 2. **One worker pool per test** — each `#[serial]` test gets a fresh `AppState` + router;
//!    the pool is shut down before the next test to avoid queue/tenant-limit carry-over.
//! 3. **No live API during `spec013-proof`** — leave `SPEC013_LIVE_API_URL` unset while
//!    `make dev-bg` is running or two worker pools contend on `DATABASE_URL`.

#![cfg(feature = "postgres")]

use axum::body::Body;
use axum::http::Request;
use edgequake_api::{AppState, DocumentTaskProcessor, Server, ServerConfig};
use edgequake_tasks::{TaskQueue, TaskStorage, WorkerPool, WorkerPoolConfig};
use serde_json::{json, Value};
use std::env;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tower::ServiceExt;

/// Default Mistral chat model for SPEC-013 intensive E2E.
pub const MISTRAL_LLM_MODEL: &str = "mistral-small-latest";
/// Default Mistral embedding model (1024 dimensions).
pub const MISTRAL_EMBEDDING_MODEL: &str = "mistral-embed";
pub const MISTRAL_EMBEDDING_DIMENSION: usize = 1024;

static SPEC013_WORKER_POOL: OnceLock<Mutex<Option<WorkerPool>>> = OnceLock::new();

/// Create-workspace JSON body with explicit Mistral LLM + embedding providers.
pub fn mistral_workspace_json(name: impl AsRef<str>) -> Value {
    mistral_workspace_json_with_entity_types(
        name,
        &["PERSON", "ORGANIZATION", "LOCATION", "CONCEPT", "OTHER"],
    )
}

pub fn mistral_workspace_json_with_entity_types(
    name: impl AsRef<str>,
    entity_types: &[&str],
) -> Value {
    json!({
        "name": name.as_ref(),
        "llm_provider": "mistral",
        "llm_model": MISTRAL_LLM_MODEL,
        "embedding_provider": "mistral",
        "embedding_model": MISTRAL_EMBEDDING_MODEL,
        "embedding_dimension": MISTRAL_EMBEDDING_DIMENSION,
        "entity_types": entity_types,
    })
}

/// Assert workspace API response uses Mistral providers/models.
pub fn assert_workspace_uses_mistral(ws: &Value) {
    assert_eq!(
        ws["llm_provider"].as_str(),
        Some("mistral"),
        "llm_provider: {ws:?}"
    );
    assert_eq!(
        ws["embedding_provider"].as_str(),
        Some("mistral"),
        "embedding_provider: {ws:?}"
    );
    assert_eq!(
        ws["llm_model"].as_str(),
        Some(MISTRAL_LLM_MODEL),
        "llm_model: {ws:?}"
    );
    assert_eq!(
        ws["embedding_model"].as_str(),
        Some(MISTRAL_EMBEDDING_MODEL),
        "embedding_model: {ws:?}"
    );
}

/// Resolve PostgreSQL connection URL from environment.
pub fn database_url() -> Option<String> {
    env::var("DATABASE_URL").ok().or_else(|| {
        let password = env::var("POSTGRES_PASSWORD").ok()?;
        let host = env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());
        let db = env::var("POSTGRES_DB").unwrap_or_else(|_| "edgequake".to_string());
        let user = env::var("POSTGRES_USER").unwrap_or_else(|_| "edgequake".to_string());
        Some(format!("postgresql://{user}:{password}@{host}:{port}/{db}"))
    })
}

pub fn require_database_url() -> String {
    database_url().unwrap_or_else(|| {
        panic!(
            "DATABASE_URL (or POSTGRES_PASSWORD) required for SPEC-013 Postgres E2E. \
             Example: export DATABASE_URL=postgres://edgequake:edgequake_secret@localhost:5432/edgequake"
        )
    })
}

fn test_server_config() -> ServerConfig {
    ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: true,
    }
}

async fn shutdown_worker_pool() {
    let slot = SPEC013_WORKER_POOL.get_or_init(|| Mutex::new(None));
    let mut guard = slot.lock().expect("SPEC013 worker pool mutex");
    if let Some(pool) = guard.take() {
        pool.shutdown().await;
        // Let in-flight PDF tasks finish cancellation before next test's AppState.
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

/// Start worker pool for this test's `AppState` (shuts down any prior pool first).
async fn start_worker_pool(state: &mut AppState) {
    shutdown_worker_pool().await;

    let mut processor = DocumentTaskProcessor::with_workspace_support_strict(
        Arc::clone(&state.pipeline),
        Arc::clone(&state.llm_provider),
        Arc::clone(&state.kv_storage),
        Arc::clone(&state.vector_storage),
        Arc::clone(&state.vector_registry),
        Arc::clone(&state.graph_storage),
        state.pipeline_state.clone(),
        Arc::clone(&state.workspace_service),
        Arc::clone(&state.models_config),
    )
    .with_progress_broadcaster(state.progress_broadcaster.clone());

    #[cfg(feature = "postgres")]
    if let Some(ref pdf_storage) = state.pdf_storage {
        processor = processor.with_pdf_storage(Arc::clone(pdf_storage));
    }

    let processor = Arc::new(processor);

    let worker_config = WorkerPoolConfig {
        num_workers: 2,
        auto_retry: true,
        initial_retry_delay_ms: 1000,
        max_retry_delay_ms: 10_000,
        backoff_multiplier: 2.0,
        max_tasks_per_tenant: 4,
        processing_timeout_secs: 900,
    };

    let mut worker_pool = WorkerPool::new(
        worker_config,
        Arc::clone(&state.task_queue) as Arc<dyn TaskQueue>,
        Arc::clone(&state.task_storage) as Arc<dyn TaskStorage>,
        processor,
    );

    state.cancellation_registry = worker_pool.cancellation_registry();
    worker_pool.start();

    let slot = SPEC013_WORKER_POOL.get_or_init(|| Mutex::new(None));
    *slot.lock().expect("SPEC013 worker pool mutex") = Some(worker_pool);
}

/// Poll until the test router responds (workers are scheduled).
pub async fn wait_until_app_ready(app: &axum::Router) {
    for attempt in 0..40 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
        let res = response.expect("SPEC-013 /health oneshot failed");
        if res.status().is_success() {
            if attempt > 0 {
                eprintln!("SPEC013_APP_READY after {attempt} polls");
            }
            tokio::time::sleep(Duration::from_millis(150)).await;
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("SPEC-013 test app did not become ready on /health within 4s");
}

async fn build_postgres_router(mut state: AppState) -> axum::Router {
    assert!(
        matches!(state.storage_mode, edgequake_api::StorageMode::PostgreSQL),
        "SPEC-013 E2E must use PostgreSQL storage, got {:?}",
        state.storage_mode
    );
    start_worker_pool(&mut state).await;
    let router = Server::new(test_server_config(), state).build_router();
    wait_until_app_ready(&router).await;
    router
}

/// Build an Axum app backed by PostgreSQL with mock LLM (deterministic, no API keys).
pub async fn create_postgres_mock_app() -> axum::Router {
    crate::common::clear_provider_detection_env();
    env::set_var("EDGEQUAKE_LLM_PROVIDER", "mock");
    env::set_var("EDGEQUAKE_EMBEDDING_PROVIDER", "mock");

    let url = require_database_url();
    let state = AppState::new_postgres(url, "")
        .await
        .unwrap_or_else(|e| panic!("PostgreSQL AppState failed: {e}"));

    build_postgres_router(state).await
}

/// Build an Axum app backed by PostgreSQL with Mistral providers (live API calls).
pub async fn create_postgres_mistral_app() -> axum::Router {
    let mistral_key =
        env::var("MISTRAL_API_KEY").expect("MISTRAL_API_KEY required for Mistral live tests");
    crate::common::clear_provider_detection_env();
    env::set_var("MISTRAL_API_KEY", &mistral_key);
    env::set_var("EDGEQUAKE_LLM_PROVIDER", "mistral");
    env::set_var("EDGEQUAKE_EMBEDDING_PROVIDER", "mistral");
    env::set_var("MISTRAL_EMBEDDING_MODEL", "mistral-embed");
    env::set_var("EDGEQUAKE_EMBEDDING_BATCH_SIZE", "16");

    let url = require_database_url();
    let state = AppState::new_postgres(url, "")
        .await
        .unwrap_or_else(|e| panic!("PostgreSQL Mistral AppState failed: {e}"));

    build_postgres_router(state).await
}
