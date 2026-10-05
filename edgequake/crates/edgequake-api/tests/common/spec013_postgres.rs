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
#![allow(dead_code)] // shared harness symbols used by mistral vs github issue test binaries

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

/// Default Ollama chat model for SPEC-114 live extract (thinking MoE; use reasoning none).
pub const OLLAMA_LLM_MODEL: &str = "qwen3.6:35b-a3b";
/// Preferred Ollama embedding model (768 dimensions).
pub const OLLAMA_EMBEDDING_MODEL: &str = "embeddinggemma:latest";
/// Fallback embed if preferred tag is missing.
pub const OLLAMA_EMBEDDING_MODEL_FALLBACK: &str = "nomic-embed-text";
pub const OLLAMA_EMBEDDING_DIMENSION: usize = 768;
pub const OLLAMA_DEFAULT_HOST: &str = "http://localhost:11434";

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
    mistral_kg_schema_workspace_json(name, entity_types, &[], &[], true, true)
}

/// Provider-agnostic workspace create payload with dual allowlists + typed edges.
#[allow(clippy::too_many_arguments)] // mirrors workspace create JSON field surface
pub fn kg_schema_workspace_json(
    name: impl AsRef<str>,
    llm_provider: &str,
    llm_model: &str,
    embedding_provider: &str,
    embedding_model: &str,
    embedding_dimension: usize,
    entity_types: &[&str],
    relation_types: &[&str],
    relation_edges: &[(&str, &str, &str)],
    entity_types_strict: bool,
    relation_types_strict: bool,
) -> Value {
    let edges: Vec<Value> = relation_edges
        .iter()
        .map(|(source, relation, target)| {
            json!({
                "source": source,
                "relation": relation,
                "target": target,
            })
        })
        .collect();
    json!({
        "name": name.as_ref(),
        "llm_provider": llm_provider,
        "llm_model": llm_model,
        "embedding_provider": embedding_provider,
        "embedding_model": embedding_model,
        "embedding_dimension": embedding_dimension,
        "entity_types": entity_types,
        "entity_types_strict": entity_types_strict,
        "relation_types": relation_types,
        "relation_types_strict": relation_types_strict,
        "relation_edges": edges,
        "kg_schema_preset": if relation_types.is_empty() && relation_edges.is_empty() {
            "blank"
        } else {
            "custom"
        },
        // Disable think-heavy extract for Qwen3.6 / reasoning models (SPEC-109 / SPEC-113).
        "default_reasoning_effort": "none",
    })
}

/// Mistral workspace with optional dual allowlists + typed edges (SPEC-114).
///
/// Empty `relation_types` / `relation_edges` keep free-form relation extraction
/// (EC-114-01 / EC-114-18). Prefer [`mistral_kg_schema_workspace_json`] for
/// G-114-17 live extract gates that pin WORKS_AT / LOCATED_IN.
pub fn mistral_kg_schema_workspace_json(
    name: impl AsRef<str>,
    entity_types: &[&str],
    relation_types: &[&str],
    relation_edges: &[(&str, &str, &str)],
    entity_types_strict: bool,
    relation_types_strict: bool,
) -> Value {
    kg_schema_workspace_json(
        name,
        "mistral",
        MISTRAL_LLM_MODEL,
        "mistral",
        MISTRAL_EMBEDDING_MODEL,
        MISTRAL_EMBEDDING_DIMENSION,
        entity_types,
        relation_types,
        relation_edges,
        entity_types_strict,
        relation_types_strict,
    )
}

/// Default SPEC-114 Mistral extract schema (PERSON/ORG/OTHER + WORKS_AT/LOCATED_IN).
pub fn mistral_spec114_extract_workspace_json(name: impl AsRef<str>) -> Value {
    mistral_kg_schema_workspace_json(
        name,
        &["PERSON", "ORGANIZATION", "OTHER"],
        &["WORKS_AT", "LOCATED_IN"],
        &[("PERSON", "WORKS_AT", "ORGANIZATION")],
        true,
        true,
    )
}

/// Ollama workspace with dual allowlists (qwen3.6:35b-a3b + local embed).
pub fn ollama_kg_schema_workspace_json(
    name: impl AsRef<str>,
    entity_types: &[&str],
    relation_types: &[&str],
    relation_edges: &[(&str, &str, &str)],
    entity_types_strict: bool,
    relation_types_strict: bool,
) -> Value {
    let embed = env::var("EDGEQUAKE_EMBEDDING_MODEL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| OLLAMA_EMBEDDING_MODEL.to_string());
    kg_schema_workspace_json(
        name,
        "ollama",
        OLLAMA_LLM_MODEL,
        "ollama",
        &embed,
        OLLAMA_EMBEDDING_DIMENSION,
        entity_types,
        relation_types,
        relation_edges,
        entity_types_strict,
        relation_types_strict,
    )
}

/// Default SPEC-114 Ollama extract schema.
pub fn ollama_spec114_extract_workspace_json(name: impl AsRef<str>) -> Value {
    ollama_kg_schema_workspace_json(
        name,
        &["PERSON", "ORGANIZATION", "OTHER"],
        &["WORKS_AT", "LOCATED_IN"],
        &[("PERSON", "WORKS_AT", "ORGANIZATION")],
        true,
        true,
    )
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

/// Assert workspace API response uses Ollama providers (model pins may vary on embed fallback).
pub fn assert_workspace_uses_ollama(ws: &Value) {
    assert_eq!(
        ws["llm_provider"].as_str(),
        Some("ollama"),
        "llm_provider: {ws:?}"
    );
    assert_eq!(
        ws["embedding_provider"].as_str(),
        Some("ollama"),
        "embedding_provider: {ws:?}"
    );
    assert_eq!(
        ws["llm_model"].as_str(),
        Some(OLLAMA_LLM_MODEL),
        "llm_model: {ws:?}"
    );
    let embed = ws["embedding_model"].as_str().unwrap_or("");
    assert!(
        embed.contains("embeddinggemma") || embed.contains("nomic-embed"),
        "expected ollama embed model, got {embed}"
    );
}

/// Resolve OLLAMA_HOST (default localhost:11434).
pub fn ollama_host() -> String {
    env::var("OLLAMA_HOST").unwrap_or_else(|_| OLLAMA_DEFAULT_HOST.to_string())
}

/// Probe `/api/tags` for a model name substring (e.g. `qwen3.6:35b-a3b`).
pub async fn ollama_has_model(host: &str, model_substr: &str) -> bool {
    let url = format!("{}/api/tags", host.trim_end_matches('/'));
    let Ok(resp) = reqwest::Client::new()
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await
    else {
        return false;
    };
    if !resp.status().is_success() {
        return false;
    }
    let Ok(body) = resp.json::<Value>().await else {
        return false;
    };
    body.get("models")
        .and_then(|m| m.as_array())
        .map(|models| {
            models.iter().any(|m| {
                m.get("name")
                    .and_then(|n| n.as_str())
                    .map(|n| n.contains(model_substr) || model_substr.contains(n))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

/// Pick an available Ollama embedding model tag, or None.
pub async fn resolve_ollama_embed_model(host: &str) -> Option<&'static str> {
    if ollama_has_model(host, "embeddinggemma").await {
        return Some(OLLAMA_EMBEDDING_MODEL);
    }
    if ollama_has_model(host, "nomic-embed-text").await {
        return Some(OLLAMA_EMBEDDING_MODEL_FALLBACK);
    }
    None
}

/// True when DATABASE_URL + Ollama host has chat + embed models for SPEC-114 live.
pub async fn ollama_live_extract_available() -> bool {
    if database_url().is_none() {
        return false;
    }
    let host = ollama_host();
    ollama_has_model(&host, OLLAMA_LLM_MODEL).await
        && resolve_ollama_embed_model(&host).await.is_some()
}

/// Resolve PostgreSQL connection URL from environment.
///
/// The resolved database is redirected to a dedicated scratch test database and
/// auto-provisioned once per process (see [`super::test_db`]) so tests run fully
/// isolated from the dev database.
pub fn database_url() -> Option<String> {
    let base = env::var("DATABASE_URL")
        .ok()
        .filter(|url| !url.trim().is_empty())
        .or_else(|| {
            let password = env::var("POSTGRES_PASSWORD")
                .ok()
                .filter(|value| !value.is_empty())?;
            let host = env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
            let port = env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());
            let db = env::var("POSTGRES_DB").unwrap_or_else(|_| "edgequake".to_string());
            let user = env::var("POSTGRES_USER").unwrap_or_else(|_| "edgequake".to_string());
            Some(format!("postgresql://{user}:{password}@{host}:{port}/{db}"))
        })?;
    Some(super::test_db::isolated_test_url(&base))
}

pub fn require_database_url() -> String {
    database_url().unwrap_or_else(|| {
        panic!(
            "DATABASE_URL (or POSTGRES_PASSWORD) required for SPEC-013 Postgres E2E. \
             Example: export DATABASE_URL=postgres://edgequake:edgequake_secret@localhost:5432/edgequake"
        )
    })
}

/// Like `require_database_url` but returns `None` instead of panicking.
///
/// WHY: SPEC-013 Postgres E2E tests need a live database. When no database is
/// configured (e.g. running `cargo test --workspace` without `make postgres-start`),
/// the tests should SKIP instead of failing the whole suite. Each test calls
/// `create_postgres_mock_app_or_skip()` and returns early when it yields `None`.
pub fn try_database_url() -> Option<String> {
    database_url()
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
    let pool = {
        let mut guard = slot.lock().expect("SPEC013 worker pool mutex");
        guard.take()
    };
    if let Some(pool) = pool {
        pool.shutdown().await;
        // Let in-flight PDF tasks finish cancellation before next test's AppState.
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

/// Start worker pool for this test's `AppState` (shuts down any prior pool first).
pub async fn start_worker_pool(state: &mut AppState) {
    shutdown_worker_pool().await;

    let mut processor = DocumentTaskProcessor::with_workspace_support_strict(
        Arc::clone(&state.query.pipeline),
        Arc::clone(&state.query.llm_provider),
        Arc::clone(&state.storage.kv_storage),
        Arc::clone(&state.storage.vector_storage),
        Arc::clone(&state.storage.vector_registry),
        Arc::clone(&state.storage.graph_storage),
        state.tasks.pipeline_state.clone(),
        Arc::clone(&state.workspace_service),
        Arc::clone(&state.query.models_config),
    )
    .with_app_state(state.clone())
    .with_progress_broadcaster(state.tasks.progress_broadcaster.clone());

    // SPEC-091 / SPEC-021: fleet FK spine requires PostgresEntitySink under typed vectors.
    #[cfg(feature = "postgres")]
    if let Some(ref pool) = state.pg_pool {
        let entity_sink =
            edgequake_api::postgres_entity_sink::PostgresEntitySink::create_for_runtime(Arc::new(
                pool.clone(),
            ))
            .await;
        processor = processor.with_relational_sink(entity_sink);
        let lineage_sink =
            edgequake_api::postgres_lineage_sink::PostgresLineageSink::create_if_migration_applied(
                Arc::new(pool.clone()),
            )
            .await;
        processor = processor.with_lineage_sink(lineage_sink);
    }

    #[cfg(feature = "postgres")]
    if let Some(ref pdf_storage) = state.storage.pdf_storage {
        processor = processor.with_pdf_storage(Arc::clone(pdf_storage));
    }
    #[cfg(feature = "postgres")]
    if let Some(ref mm_asset_storage) = state.storage.mm_asset_storage {
        processor = processor.with_mm_asset_storage(Arc::clone(mm_asset_storage));
    }
    #[cfg(feature = "postgres")]
    if let Some(ref page_layout_storage) = state.storage.page_layout_storage {
        processor = processor.with_page_layout_storage(Arc::clone(page_layout_storage));
    }
    // SPEC-091: typed vector persist needs pg_pool on the processor (same as main.rs).
    #[cfg(feature = "postgres")]
    if let (Some(pool), Some(caps)) = (state.pg_pool.clone(), state.postgres_capabilities.clone()) {
        processor = processor.with_postgres_id_allocation(pool, caps);
    }

    let processor = Arc::new(processor);

    let worker_config = WorkerPoolConfig {
        num_workers: 2,
        auto_retry: true,
        initial_retry_delay_ms: 1000,
        max_retry_delay_ms: 10_000,
        backoff_multiplier: 2.0,
        max_tasks_per_tenant: 4,
        max_lifecycle_tasks_per_tenant: 4,
        processing_timeout_secs: 900,
        provider_budget: 0,
        tenant_lane_weight: 1,
    };

    let mut worker_pool = WorkerPool::new(
        worker_config,
        Arc::clone(&state.tasks.queue) as Arc<dyn TaskQueue>,
        Arc::clone(&state.tasks.storage) as Arc<dyn TaskStorage>,
        processor,
    );

    state.tasks.cancellation_registry = worker_pool.cancellation_registry();
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
        matches!(state.storage.mode, edgequake_api::StorageMode::PostgreSQL),
        "SPEC-013 E2E must use PostgreSQL storage, got {:?}",
        state.storage.mode
    );
    state
        .initialize_defaults()
        .await
        .expect("SPEC-013 server default identity initialization");
    start_worker_pool(&mut state).await;
    let router = Server::new(test_server_config(), state).build_router();
    wait_until_app_ready(&router).await;
    router
}

/// Disable auth for SPEC-013 postgres e2e harness (matches `make` defaults).
///
/// `AuthConfig::from_env` defaults `auth_enabled=true` when unset, which
/// 401s tenant/workspace bootstrap and masks the behaviors under test.
fn configure_postgres_e2e_auth_env() {
    if env::var("EDGEQUAKE_AUTH_ENABLED").is_err() && env::var("AUTH_ENABLED").is_err() {
        env::set_var("EDGEQUAKE_AUTH_ENABLED", "false");
    }
}

/// Opt in to mock LLM for postgres e2e harness (forbidden as server default).
fn configure_postgres_e2e_mock_provider_env() {
    env::set_var("EDGEQUAKE_LLM_PROVIDER", "mock");
    env::set_var("EDGEQUAKE_EMBEDDING_PROVIDER", "mock");
    env::set_var("EDGEQUAKE_ALLOW_MOCK_PROVIDER", "1");
}

/// Build an Axum app backed by PostgreSQL with mock LLM (deterministic, no API keys).
pub async fn create_postgres_mock_app() -> axum::Router {
    super::clear_provider_detection_env();
    configure_postgres_e2e_auth_env();
    configure_postgres_e2e_mock_provider_env();

    let url = require_database_url();
    let state = AppState::new_postgres(url, "")
        .await
        .unwrap_or_else(|e| panic!("PostgreSQL AppState failed: {e}"));

    build_postgres_router(state).await
}

/// Like `create_postgres_mock_app` but returns `None` when no database is
/// configured, so callers can skip the test instead of panicking.
pub async fn create_postgres_mock_app_or_skip() -> Option<axum::Router> {
    super::clear_provider_detection_env();
    configure_postgres_e2e_auth_env();
    configure_postgres_e2e_mock_provider_env();

    let required = env::var("EDGEQUAKE_REQUIRE_POSTGRES_TESTS")
        .is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"));
    let Some(url) = try_database_url() else {
        assert!(
            !required,
            "SPEC-013 requires a configured PostgreSQL database"
        );
        return None;
    };
    let state = match AppState::new_postgres(url, "").await {
        Ok(state) => state,
        Err(error) => {
            assert!(!required, "SPEC-013 PostgreSQL AppState failed: {error}");
            eprintln!("SKIP: PostgreSQL AppState failed: {error}");
            return None;
        }
    };
    Some(build_postgres_router(state).await)
}

/// Build an Axum app backed by PostgreSQL with Mistral providers (live API calls).
pub async fn create_postgres_mistral_app() -> axum::Router {
    let mistral_key =
        env::var("MISTRAL_API_KEY").expect("MISTRAL_API_KEY required for Mistral live tests");
    super::clear_provider_detection_env();
    configure_postgres_e2e_auth_env();
    env::set_var("MISTRAL_API_KEY", &mistral_key);
    env::set_var("EDGEQUAKE_LLM_PROVIDER", "mistral");
    env::set_var("EDGEQUAKE_EMBEDDING_PROVIDER", "mistral");
    env::set_var("MISTRAL_EMBEDDING_MODEL", "mistral-embed");
    env::set_var("EDGEQUAKE_EMBEDDING_BATCH_SIZE", "16");
    env::set_var("EDGEQUAKE_EXTRACT_REASONING_EFFORT", "none");
    env::set_var("EDGEQUAKE_REASONING_EFFORT", "none");

    let url = require_database_url();
    let state = AppState::new_postgres(url, "")
        .await
        .unwrap_or_else(|e| panic!("PostgreSQL Mistral AppState failed: {e}"));

    build_postgres_router(state).await
}

/// Like `create_postgres_mistral_app` but returns `None` when no database or
/// no `MISTRAL_API_KEY` is configured, so callers can skip instead of panicking.
pub async fn create_postgres_mistral_app_or_skip() -> Option<axum::Router> {
    let mistral_key = env::var("MISTRAL_API_KEY").ok()?;
    super::clear_provider_detection_env();
    configure_postgres_e2e_auth_env();
    env::set_var("MISTRAL_API_KEY", &mistral_key);
    env::set_var("EDGEQUAKE_LLM_PROVIDER", "mistral");
    env::set_var("EDGEQUAKE_EMBEDDING_PROVIDER", "mistral");
    env::set_var("MISTRAL_EMBEDDING_MODEL", "mistral-embed");
    env::set_var("EDGEQUAKE_EMBEDDING_BATCH_SIZE", "16");
    env::set_var("EDGEQUAKE_EXTRACT_REASONING_EFFORT", "none");
    env::set_var("EDGEQUAKE_REASONING_EFFORT", "none");

    let url = try_database_url()?;
    let state = AppState::new_postgres(url, "")
        .await
        .map_err(|e| eprintln!("SKIP: PostgreSQL Mistral AppState failed: {e}"))
        .ok()?;
    Some(build_postgres_router(state).await)
}

/// Build PostgreSQL app with Ollama (`qwen3.6:35b-a3b`) for SPEC-114 live extract.
///
/// Returns `None` when DATABASE_URL missing, Ollama unreachable, or required
/// models are not pulled.
pub async fn create_postgres_ollama_app_or_skip() -> Option<axum::Router> {
    let host = ollama_host();
    if !ollama_has_model(&host, OLLAMA_LLM_MODEL).await {
        eprintln!(
            "SKIP: Ollama model `{OLLAMA_LLM_MODEL}` not found at {host} — run `ollama pull {OLLAMA_LLM_MODEL}`"
        );
        return None;
    }
    let embed_model = match resolve_ollama_embed_model(&host).await {
        Some(m) => m,
        None => {
            eprintln!(
                "SKIP: no Ollama embed model ({OLLAMA_EMBEDDING_MODEL} or {OLLAMA_EMBEDDING_MODEL_FALLBACK}) at {host}"
            );
            return None;
        }
    };

    super::clear_provider_detection_env();
    configure_postgres_e2e_auth_env();
    env::set_var("OLLAMA_HOST", &host);
    env::set_var("EDGEQUAKE_LLM_PROVIDER", "ollama");
    env::set_var("EDGEQUAKE_EMBEDDING_PROVIDER", "ollama");
    env::set_var("OLLAMA_MODEL", OLLAMA_LLM_MODEL);
    env::set_var("EDGEQUAKE_LLM_MODEL", OLLAMA_LLM_MODEL);
    env::set_var("OLLAMA_EMBEDDING_MODEL", embed_model);
    env::set_var("EDGEQUAKE_EMBEDDING_MODEL", embed_model);
    env::set_var(
        "EDGEQUAKE_EMBEDDING_DIMENSION",
        OLLAMA_EMBEDDING_DIMENSION.to_string(),
    );
    env::set_var("EDGEQUAKE_EXTRACT_REASONING_EFFORT", "none");
    env::set_var("EDGEQUAKE_REASONING_EFFORT", "none");

    let url = try_database_url()?;
    let state = AppState::new_postgres(url, "")
        .await
        .map_err(|e| eprintln!("SKIP: PostgreSQL Ollama AppState failed: {e}"))
        .ok()?;
    Some(build_postgres_router(state).await)
}
