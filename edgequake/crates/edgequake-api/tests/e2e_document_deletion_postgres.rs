//! PostgreSQL Integration Tests for Document Deletion
//!
//! These tests verify the document deletion cascade behavior works correctly
//! with PostgreSQL storage backend.
//!
//! @implements UC0005: Delete Document (PostgreSQL verification)
//! @tests Mission requirement: "Ensure it working with postgres provider and memory provider"
//!
//! Run with:
//!   DATABASE_URL="postgresql://edgequake:edgequake_secret@localhost:5432/edgequake" \
//!   cargo test --package edgequake-api --test e2e_document_deletion_postgres --features postgres
//!
//! Or with individual environment variables:
//!   POSTGRES_PASSWORD=edgequake_secret \
//!   cargo test --package edgequake-api --test e2e_document_deletion_postgres --features postgres

#![cfg(feature = "postgres")]

use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::{postgres::PgPoolOptions, PgPool};
use tower::ServiceExt;
use uuid::Uuid;

use edgequake_api::{AppState, Server, ServerConfig};
use edgequake_core::{
    ConversationService, InMemoryConversationService, InMemoryWorkspaceService, WorkspaceService,
};
use edgequake_llm::MockProvider;
use edgequake_pipeline::Pipeline;
use edgequake_query::{QueryEngine, QueryEngineConfig};
use edgequake_storage::{
    GraphStorage, KVStorage, MemoryWorkspaceVectorRegistry, PgVectorStorage,
    PostgresAGEGraphStorage, PostgresConfig, PostgresKVStorage, VectorStorage,
};

// WHY: route these tests to a dedicated scratch database (`{db}_test`) instead
// of the shared dev DB — this suite inserts many `documents` rows (the
// historical "Wipe Scale"/duplicate-title dev pollution). See common/test_db.rs.
#[path = "common/test_db.rs"]
mod test_db;

// ============================================================================
// Test Infrastructure
// ============================================================================

/// Canonical default tenant/workspace UUIDs (mirror `seed_default_workspace`).
const DEFAULT_TENANT_ID: &str = "00000000-0000-0000-0000-000000000002";
const DEFAULT_WORKSPACE_ID: &str = "00000000-0000-0000-0000-000000000003";

/// Get database URL from environment variables, redirected to the dedicated
/// scratch test database (see `common/test_db.rs`).
fn get_database_url() -> Option<String> {
    let base = env::var("DATABASE_URL").ok().or_else(|| {
        let password = env::var("POSTGRES_PASSWORD").ok()?;
        let host = env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());
        let db = env::var("POSTGRES_DB").unwrap_or_else(|_| "edgequake".to_string());
        let user = env::var("POSTGRES_USER").unwrap_or_else(|_| "edgequake".to_string());
        Some(format!(
            "postgresql://{}:{}@{}:{}/{}",
            user, password, host, port, db
        ))
    })?;
    Some(test_db::isolated_test_url(&base))
}

/// Serialize every test in this binary: they share ONE PostgreSQL database
/// (global `documents` / `ingestion_dedup` tables) and `create_postgres_test_state`
/// purges dedup rows, so parallel execution races (one test's setup wipes
/// another's in-flight dedup reservation). A process-wide async mutex makes the
/// suite deterministic regardless of `RUST_TEST_THREADS`.
static DB_TEST_SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn db_test_serial() -> tokio::sync::MutexGuard<'static, ()> {
    DB_TEST_SERIAL.lock().await
}

/// Create test database pool.
async fn create_test_pool() -> Option<PgPool> {
    let database_url = get_database_url()?;
    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .ok()
}

/// Skip test if PostgreSQL is not available.
macro_rules! require_postgres {
    () => {
        match create_test_pool().await {
            Some(pool) => pool,
            None => {
                eprintln!("⚠️ Skipping test: DATABASE_URL or POSTGRES_PASSWORD not set");
                return;
            }
        }
    };
}

/// Create a PostgreSQL test config with unique namespace.
fn create_pg_config(namespace: &str) -> PostgresConfig {
    let database_url = get_database_url().expect("DATABASE_URL required");
    let url = url::Url::parse(&database_url).expect("Valid DATABASE_URL");

    PostgresConfig {
        host: url.host_str().unwrap_or("localhost").to_string(),
        port: url.port().unwrap_or(5432),
        database: url.path().trim_start_matches('/').to_string(),
        user: url.username().to_string(),
        password: url.password().unwrap_or("").to_string(),
        namespace: namespace.to_string(),
        max_connections: 5,
        min_connections: 1,
        connect_timeout: Duration::from_secs(10),
        idle_timeout: Duration::from_secs(60),
        ..Default::default()
    }
}

/// Create a test state with PostgreSQL storage.
///
/// Uses a unique namespace for test isolation.
async fn create_postgres_test_state(pool: &PgPool) -> AppState {
    create_postgres_test_state_named(pool).await.0
}

/// Same as [`create_postgres_test_state`] plus KV namespace + concrete KV handle.
async fn create_postgres_test_state_named(
    pool: &PgPool,
) -> (AppState, String, Arc<PostgresKVStorage>) {
    use edgequake_api::cache_manager::CacheManager;
    use edgequake_api::state::StorageMode;
    use edgequake_auth::AuthConfig;
    use edgequake_llm::ModelsConfig;
    use edgequake_rate_limiter::{RateLimitConfig as TokenBucketConfig, RateLimiter};

    // Generate unique namespace for this test run
    let namespace = format!(
        "test_{}",
        &Uuid::new_v4().to_string().replace('-', "")[..12]
    );
    let pg_config = create_pg_config(&namespace);

    // Create PostgreSQL-backed storages
    let kv_storage = Arc::new(PostgresKVStorage::new(pg_config.clone()));
    kv_storage
        .initialize()
        .await
        .expect("Failed to initialize KV storage");

    // Vector storage with 1536 dimensions (matches MockProvider)
    let vector_storage = Arc::new(PgVectorStorage::new(pg_config.clone()));
    vector_storage
        .initialize()
        .await
        .expect("Failed to initialize vector storage");

    // Graph storage with AGE
    let graph_storage = Arc::new(PostgresAGEGraphStorage::new(pg_config.clone()));
    graph_storage
        .initialize()
        .await
        .expect("Failed to initialize graph storage");

    // Mock LLM provider (same as memory tests)
    let mock_provider = Arc::new(MockProvider::new());

    // Pipeline
    let pipeline = Arc::new(Pipeline::default_pipeline());

    // Services (use in-memory for simplicity)
    let workspace_service: Arc<dyn WorkspaceService> = Arc::new(InMemoryWorkspaceService::new());
    // SPEC-091 queue/admission gates require the workspace row to exist —
    // seed the canonical default tenant + workspace (...-0002 / ...-0003).
    workspace_service.seed_default_workspace().await;
    // Register the PG pool for relational workspace-membership reads (the
    // wsdoc Wave B3 cutover) — production does this in `state/postgres.rs`,
    // but this harness builds `AppState` manually.
    edgequake_api::services::workspace_document_index::register_membership_pool(pool.clone());
    // Admission inserts `documents` rows referencing the default tenant /
    // workspace FK — production seeds these at startup
    // (`ensure_default_tenant_workspace`); replicate on a fresh schema.
    edgequake_api::services::identity_storage::ensure_default_tenant_workspace(
        pool,
        &edgequake_api::state::ApiSecurityConfig::default(),
    )
    .await
    .expect("seed default tenant/workspace rows");
    let conversation_service: Arc<dyn ConversationService> =
        Arc::new(InMemoryConversationService::new());

    // Idempotent reruns: purge dedup reservations left by previous
    // (possibly aborted) runs — fixture content hashes are deterministic.
    sqlx::query("DELETE FROM public.ingestion_dedup")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM public.eq_eq_default_kv WHERE key LIKE 'doc:hash:%' OR key LIKE 'staging:hash:%'")
        .execute(pool)
        .await
        .ok(); // 42P01 post-Wave-D drop: nothing to purge.

    // Task infrastructure
    let task_storage = Arc::new(edgequake_tasks::memory::MemoryTaskStorage::new());
    let task_queue = Arc::new(edgequake_tasks::queue::ChannelTaskQueue::new(100));

    // Query engine (SOTA — the single production engine; legacy engine deleted P-G6a)
    let engine_impl = Arc::new(QueryEngine::with_mock_keywords(
        QueryEngineConfig::default(),
        Arc::clone(&vector_storage) as Arc<dyn edgequake_storage::traits::VectorStorage>,
        Arc::clone(&graph_storage) as Arc<dyn edgequake_storage::traits::GraphStorage>,
        Arc::clone(&mock_provider) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
        Arc::clone(&mock_provider) as Arc<dyn edgequake_llm::traits::LLMProvider>,
    ));

    // Auth services — disable for local cascade/wipe contract tests.
    let auth_config = AuthConfig {
        auth_enabled: false,
        ..AuthConfig::default()
    };
    let vector_registry: Arc<dyn edgequake_storage::traits::WorkspaceVectorRegistry> =
        Arc::new(MemoryWorkspaceVectorRegistry::new(
            Arc::clone(&vector_storage) as Arc<dyn edgequake_storage::traits::VectorStorage>,
        ));

    let state = AppState {
        storage: edgequake_api::state::StorageRuntime {
            kv_storage: Arc::clone(&kv_storage) as Arc<dyn edgequake_storage::traits::KVStorage>,
            vector_storage: Arc::clone(&vector_storage)
                as Arc<dyn edgequake_storage::traits::VectorStorage>,
            vector_registry,
            graph_storage: Arc::clone(&graph_storage)
                as Arc<dyn edgequake_storage::traits::GraphStorage>,
            auth_memory: Arc::new(
                edgequake_api::services::auth_memory_store::AuthMemoryStore::new(),
            ),
            pdf_storage: None,
            original_storage: None,
            mm_asset_storage: None,
            page_layout_storage: None,
            mode: StorageMode::Memory,
        },
        query: edgequake_api::state::QueryRuntime {
            llm_provider: Arc::clone(&mock_provider) as Arc<dyn edgequake_llm::traits::LLMProvider>,
            vision_llm_provider: None,
            embedding_provider: Arc::clone(&mock_provider)
                as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
            engine_impl,
            pipeline,
            models_config: Arc::new(ModelsConfig::builtin_defaults()),
            model_catalog: Arc::new(edgequake_api::model_catalog::ModelCatalog::new()),
        },
        auth: edgequake_api::state::AuthRuntime::new(auth_config),
        tasks: edgequake_api::state::TaskRuntime::new(task_storage, task_queue),
        workspace_service,
        conversation_service,
        config: edgequake_api::state::AppConfig::default(),
        cache_manager: CacheManager::with_defaults(),
        rate_limiter: RateLimiter::new(TokenBucketConfig::strict(100, 60)),
        pg_pool: Some(pool.clone()),
        pool_bundle: None,
        pool_budget: None,
        start_time: std::time::Instant::now(),
        path_validation_config: edgequake_api::path_validation::PathValidationConfig {
            allow_any_path: true,
            ..Default::default()
        },
        audit_logger: None,
        migration_bootstrap: None,
        security: edgequake_api::state::ApiSecurityConfig::default(),
        allow_set_provider: None,
        resource_guard: edgequake_core::ResourceGuard::default(),
        graph_materialize: std::sync::Arc::new(edgequake_core::GraphMaterializationSemaphore::new(
            4,
        )),
        pdf_vision: std::sync::Arc::new(edgequake_core::PdfVisionSemaphore::new(2)),
        parse_jobs: edgequake_api::handlers::parse::ParseJobStore::from_env(),
        read_path_db: std::sync::Arc::new(edgequake_api::read_path::ReadPathDbPermit::from_env()),
        postgres_capabilities: None,
        server_config: edgequake_api::server_config_store::ServerConfigStore::new(),
    };
    (state, namespace, kv_storage)
}

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

/// Insert a workspace row for tests that use synthetic (non-default)
/// tenant/workspace UUIDs — admission gates fail-closed on unknown workspaces.
async fn seed_test_workspace(state: &AppState, tenant_id: Uuid, workspace_id: Uuid) {
    // Parent tenant first (insert_workspace validates nothing, but the
    // quota/admission reads resolve the tenant).
    if state
        .workspace_service
        .get_tenant(tenant_id)
        .await
        .ok()
        .flatten()
        .is_none()
    {
        let mut tenant = edgequake_core::Tenant::new("Test Tenant", "test");
        tenant.tenant_id = tenant_id;
        state
            .workspace_service
            .create_tenant(tenant)
            .await
            .expect("seed test tenant");
    }
    let now = chrono::Utc::now();
    let (llm_model, llm_provider) = edgequake_core::Workspace::default_llm_config();
    let (embedding_model, embedding_provider, embedding_dimension) =
        edgequake_core::Workspace::default_embedding_config();
    state
        .workspace_service
        .insert_workspace(edgequake_core::Workspace {
            workspace_id,
            tenant_id,
            name: format!("test-{workspace_id}"),
            slug: format!("test-{workspace_id}"),
            description: None,
            is_active: true,
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
            llm_model,
            llm_provider,
            embedding_model,
            embedding_provider,
            embedding_dimension,
            vision_llm_provider: None,
            vision_llm_model: None,
            pdf_parser_backend: None,
        })
        .await
        .expect("seed test workspace");
}

/// Helper to upload a document via HTTP.
async fn upload_document_http(
    app: &axum::Router,
    title: &str,
    content: &str,
) -> (StatusCode, Value) {
    let request = json!({
        "content": content,
        "title": title,
        "async_processing": false
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = extract_json(response).await;
    (status, body)
}

/// SPEC-091 queue/admission: upload ADMITS asynchronously (202 + task on the
/// durable queue). Drive the insert task inline so the test observes the
/// completed pipeline state (same drain pattern as the deletion helpers).
async fn drain_insert_task(state: &AppState) {
    let mut task = state
        .tasks
        .queue
        .try_receive()
        .await
        .expect("queue receive")
        .expect("Insert task on queue");
    for _ in 0..20 {
        if matches!(
            task.task_type,
            edgequake_tasks::TaskType::Insert | edgequake_tasks::TaskType::Upload
        ) {
            break;
        }
        task = state
            .tasks
            .queue
            .try_receive()
            .await
            .expect("queue receive")
            .expect("expected Insert task");
    }
    let processor = edgequake_api::DocumentTaskProcessor::new(
        Arc::clone(&state.query.pipeline),
        Arc::clone(&state.query.llm_provider),
        Arc::clone(&state.storage.kv_storage),
        Arc::clone(&state.storage.vector_storage),
        Arc::clone(&state.storage.vector_registry),
        Arc::clone(&state.storage.graph_storage),
        edgequake_tasks::PipelineState::default(),
    )
    .with_app_state(state.clone());
    edgequake_tasks::TaskProcessor::process(
        &processor,
        &mut task,
        tokio_util::sync::CancellationToken::new(),
    )
    .await
    .expect("drain insert task");
}

/// Upload + admit + inline-drain, returning the admitted document id.
async fn upload_and_process(
    state: &AppState,
    app: &axum::Router,
    title: &str,
    content: &str,
) -> String {
    let (status, resp) = upload_document_http(app, title, content).await;
    assert!(
        status == StatusCode::CREATED || status == StatusCode::ACCEPTED,
        "Upload should admit (201 legacy or 202 async admit), got {status}: {resp:?}"
    );
    let doc_id = resp
        .get("document_id")
        .and_then(|v| v.as_str())
        .expect("document_id")
        .to_string();
    drain_insert_task(state).await;
    doc_id
}

/// Admit delete (202) then drain durable `Deletion` task from the queue.
async fn delete_document_http(
    app: &axum::Router,
    state: &AppState,
    document_id: &str,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/documents/{}", document_id))
                .header("X-Tenant-ID", DEFAULT_TENANT_ID)
                .header("X-Workspace-ID", DEFAULT_WORKSPACE_ID)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = extract_json(response).await;
    assert_eq!(
        status,
        StatusCode::ACCEPTED,
        "delete admits with 202; body={body}"
    );

    let mut task = state
        .tasks
        .queue
        .try_receive()
        .await
        .expect("queue receive")
        .expect("Deletion task on queue");
    for _ in 0..20 {
        if task.task_type == edgequake_tasks::TaskType::Deletion {
            break;
        }
        task = state
            .tasks
            .queue
            .try_receive()
            .await
            .expect("queue receive")
            .expect("expected Deletion task");
    }
    let data: edgequake_tasks::DeletionTaskData =
        serde_json::from_value(task.task_data).expect("DeletionTaskData");
    let tenant = edgequake_api::TenantContext {
        tenant_id: Some(data.tenant_id.clone()),
        workspace_id: Some(data.workspace_id.clone()),
        user_id: None,
    };
    edgequake_api::services::perform_document_deletion(state, &data, &tenant)
        .await
        .expect("perform_document_deletion");
    (status, body)
}

/// Admit wipe-all (202) then drain durable `WorkspaceWipe`.
async fn delete_all_documents_http_and_drain(
    app: &axum::Router,
    state: &AppState,
    workspace_id: &str,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/documents")
                .header("X-Tenant-ID", "00000000-0000-0000-0000-000000000001")
                .header("X-Workspace-ID", workspace_id)
                .header("X-EdgeQuake-Confirm", "delete-all-documents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = extract_json(response).await;
    assert_eq!(status, StatusCode::ACCEPTED, "wipe admits 202; body={body}");
    assert_eq!(body["accepted"].as_bool(), Some(true));
    let mut task = state
        .tasks
        .queue
        .try_receive()
        .await
        .expect("queue receive")
        .expect("WorkspaceWipe on queue");
    for _ in 0..20 {
        if task.task_type == edgequake_tasks::TaskType::WorkspaceWipe {
            break;
        }
        task = state
            .tasks
            .queue
            .try_receive()
            .await
            .expect("queue receive")
            .expect("expected WorkspaceWipe");
    }
    let data: edgequake_tasks::WorkspaceWipeTaskData =
        serde_json::from_value(task.task_data.clone()).expect("WorkspaceWipeTaskData");
    edgequake_api::services::run_workspace_wipe_phases(state, &mut task, data)
        .await
        .expect("run_workspace_wipe_phases");
    (status, body)
}

/// Helper to query via HTTP.
async fn query_rag_http(app: &axum::Router, query_text: &str) -> (StatusCode, Value) {
    let request = json!({
        "query": query_text,
        "mode": "hybrid"
    });

    let response = app
        .clone()
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

    let status = response.status();
    let body = extract_json(response).await;
    (status, body)
}

// ============================================================================
// PostgreSQL Deletion Tests
// ============================================================================

/// Test 1: Single document deletion with PostgreSQL
///
/// Verifies basic cascade delete works with PostgreSQL constraints.
#[tokio::test]
async fn test_single_document_deletion_pg() {
    let _serial = db_test_serial().await;
    let pool = require_postgres!();
    let state = create_postgres_test_state(&pool).await;
    let server = Server::new(create_test_config(), state.clone());
    let app = server.build_router();

    // Upload document (202 admit + inline drain per SPEC-091 queue/admission)
    let doc_id = upload_and_process(
        &state,
        &app,
        "Tech Article PG",
        "Alice is a software engineer at Google. She works with Bob on AI projects. \
         They collaborate on machine learning models and data pipelines.",
    )
    .await;

    // Delete document
    let (delete_status, delete_resp) = delete_document_http(&app, &state, &doc_id).await;

    if delete_status != StatusCode::ACCEPTED {
        eprintln!(
            "Delete failed: status={}, body={:?}",
            delete_status, delete_resp
        );
    }
    assert_eq!(
        delete_status,
        StatusCode::ACCEPTED,
        "Delete should admit, got: {:?}",
        delete_resp
    );

    // Log the response for debugging
    eprintln!("Delete response: {:?}", delete_resp);

    assert_eq!(
        delete_resp.get("accepted").and_then(|v| v.as_bool()),
        Some(true),
        "Response should indicate async admit"
    );

    // Verify delete metrics are present (at top level, not nested)
    let chunks_deleted = delete_resp
        .get("chunks_deleted")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    // chunks_deleted is u64, always non-negative
    let _ = chunks_deleted;

    println!("✅ Single document deletion with PostgreSQL: PASSED");
}

/// Test 2: Shared entities preserved when one document deleted (PostgreSQL)
///
/// Verifies source_ids tracking works with PostgreSQL UPSERT.
#[tokio::test]
async fn test_delete_preserves_shared_entities_pg() {
    let _serial = db_test_serial().await;
    let pool = require_postgres!();
    let state = create_postgres_test_state(&pool).await;
    let server = Server::new(create_test_config(), state.clone());
    let app = server.build_router();

    // Upload first document
    let doc1_id = upload_and_process(
        &state,
        &app,
        "Doc1 PG",
        "John Smith is a researcher at MIT. He studies quantum computing and AI.",
    )
    .await;

    // Upload second document with overlapping entity (John Smith)
    let doc2_id = upload_and_process(
        &state,
        &app,
        "Doc2 PG",
        "John Smith published a paper on quantum algorithms. He collaborates with researchers worldwide.",
    )
    .await;

    // Delete first document
    let (delete_status, delete_resp) = delete_document_http(&app, &state, &doc1_id).await;
    assert_eq!(delete_status, StatusCode::ACCEPTED);

    // Check metrics (at top level, not nested)
    let entities_affected = delete_resp
        .get("entities_affected")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // Log for debugging
    println!("Entities affected: {}", entities_affected);

    // Delete second document to clean up
    let (cleanup_status, _) = delete_document_http(&app, &state, &doc2_id).await;
    assert_eq!(cleanup_status, StatusCode::ACCEPTED);

    println!("✅ Shared entity preservation with PostgreSQL: PASSED");
}

/// Test 3: Query works after deletion (PostgreSQL)
///
/// Verifies query engine handles missing chunks gracefully with PostgreSQL.
#[tokio::test]
async fn test_query_after_deletion_pg() {
    let _serial = db_test_serial().await;
    let pool = require_postgres!();
    let state = create_postgres_test_state(&pool).await;
    let server = Server::new(create_test_config(), state.clone());
    let app = server.build_router();

    // Upload document
    let doc_id = upload_and_process(
        &state,
        &app,
        "Queryable Doc PG",
        "EdgeQuake is a RAG framework. It uses graph-based knowledge representation.",
    )
    .await;

    // Query should work
    let (query_status1, _) = query_rag_http(&app, "What is EdgeQuake?").await;
    assert!(
        query_status1 == StatusCode::OK || query_status1 == StatusCode::NOT_FOUND,
        "Query should not error (got {})",
        query_status1
    );

    // Delete document
    let (delete_status, _) = delete_document_http(&app, &state, &doc_id).await;
    assert_eq!(delete_status, StatusCode::ACCEPTED);

    // Query should still work (no dangling references)
    let (query_status2, _) = query_rag_http(&app, "What is EdgeQuake?").await;
    assert!(
        query_status2 == StatusCode::OK || query_status2 == StatusCode::NOT_FOUND,
        "Query after deletion should not error (got {})",
        query_status2
    );

    println!("✅ Query after deletion with PostgreSQL: PASSED");
}

/// Test 4: Delete failed document cleans partial entities (PostgreSQL)
///
/// Verifies cleanup of partial data with PostgreSQL transactions.
#[tokio::test]
async fn test_delete_failed_document_cleans_partial_entities_pg() {
    let _serial = db_test_serial().await;
    let pool = require_postgres!();
    let state = create_postgres_test_state(&pool).await;
    let server = Server::new(create_test_config(), state.clone());
    let app = server.build_router();

    // Directly insert a "failed" document with some entities.
    // SPEC-091 Wave D: metadata keys route to the typed `documents` shell,
    // which requires a UUID doc id (non-UUID keys would fall back to the
    // dropped generic KV table).
    let doc_id = Uuid::new_v4().to_string();
    let metadata_key = format!("{}-metadata", doc_id);
    let metadata = json!({
        "id": doc_id,
        "title": "Failed Document PG",
        "status": "failed",
        "error": "Processing failed due to mock error",
        "created_at": "2026-01-26T00:00:00Z",
        "workspace_id": DEFAULT_WORKSPACE_ID
    });

    state
        .storage
        .kv_storage
        .upsert(&[(metadata_key.clone(), metadata)])
        .await
        .expect("Should store metadata");

    // Add some partial entities
    let mut entity_props: HashMap<String, Value> = HashMap::new();
    entity_props.insert("entity_type".to_string(), json!("PERSON"));
    entity_props.insert("source_ids".to_string(), json!([&doc_id]));
    entity_props.insert("source_chunk_ids".to_string(), json!([]));

    state
        .storage
        .graph_storage
        .upsert_node("PARTIAL_ENTITY_PG", entity_props)
        .await
        .expect("Should create entity");

    // Delete the failed document - should clean up partial data
    let (delete_status, delete_resp) = delete_document_http(&app, &state, &doc_id).await;

    assert_eq!(
        delete_status,
        StatusCode::ACCEPTED,
        "Should admit delete of failed document"
    );
    assert_eq!(
        delete_resp.get("accepted").and_then(|v| v.as_bool()),
        Some(true)
    );

    // Verify entity was cleaned up
    let entity_after = state
        .storage
        .graph_storage
        .get_node("PARTIAL_ENTITY_PG")
        .await
        .expect("Should query");

    assert!(entity_after.is_none(), "Partial entity should be deleted");

    println!("✅ Failed document cleanup with PostgreSQL: PASSED");
}

/// Test 5: Accumulated source_ids deletion (PostgreSQL)
///
/// Verifies multi-document entities are properly handled with PostgreSQL.
#[tokio::test]
async fn test_accumulated_source_ids_deletion_pg() {
    let _serial = db_test_serial().await;
    let pool = require_postgres!();
    let state = create_postgres_test_state(&pool).await;
    let server = Server::new(create_test_config(), state.clone());
    let app = server.build_router();

    // Upload three documents with shared entity
    let docs = vec![
        ("Doc A PG", "Sarah Chen is a scientist at Stanford."),
        ("Doc B PG", "Sarah Chen won the Nobel Prize."),
        ("Doc C PG", "Sarah Chen lectures on quantum physics."),
    ];

    let mut doc_ids = Vec::new();
    for (title, content) in docs {
        doc_ids.push(upload_and_process(&state, &app, title, content).await);
    }

    // Delete first document
    let (status1, resp1) = delete_document_http(&app, &state, &doc_ids[0]).await;
    assert_eq!(status1, StatusCode::ACCEPTED);

    let entities_affected_1 = resp1
        .get("entities_affected")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // Delete second document
    let (status2, resp2) = delete_document_http(&app, &state, &doc_ids[1]).await;
    assert_eq!(status2, StatusCode::ACCEPTED);

    let entities_affected_2 = resp2
        .get("entities_affected")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // Delete third document - now entities should be fully deleted
    let (status3, resp3) = delete_document_http(&app, &state, &doc_ids[2]).await;
    assert_eq!(status3, StatusCode::ACCEPTED);

    let entities_affected_3 = resp3
        .get("entities_affected")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    println!(
        "Entities affected: {} → {} → {}",
        entities_affected_1, entities_affected_2, entities_affected_3
    );

    println!("✅ Accumulated source_ids deletion with PostgreSQL: PASSED");
}

/// ISSUE-309: durable wipe admit + drain clears seeded docs without N× prefix scans
/// (wipe path uses clear_workspace once; verified by empty graph after drain).
/// Memory suite asserts exact op-counts; this PG path proves AGE clear at reporter scale.
#[tokio::test]
async fn issue309_workspace_wipe_admit_and_drain_pg() {
    let _serial = db_test_serial().await;
    let pool = require_postgres!();
    let state = create_postgres_test_state(&pool).await;
    let workspace_id = Uuid::new_v4();
    let server = Server::new(create_test_config(), state.clone());
    let app = server.build_router();

    // Reporter-scale: 200 documents + exclusive graph nodes.
    let n = 200usize;
    let tenant_id = "00000000-0000-0000-0000-000000000001";
    seed_test_workspace(&state, Uuid::parse_str(tenant_id).unwrap(), workspace_id).await;
    // SPEC-091 Wave D: metadata keys route to the typed `documents` shell,
    // which requires a UUID doc id. Collect ids for post-wipe verification.
    let mut doc_ids: Vec<String> = Vec::with_capacity(n);
    for i in 0..n {
        let doc_id = Uuid::new_v4().to_string();
        doc_ids.push(doc_id.clone());
        let meta = json!({
            "id": doc_id,
            "title": format!("Wipe Scale {i}"),
            "status": "completed",
            "workspace_id": workspace_id.to_string(),
            "tenant_id": tenant_id,
        });
        state
            .storage
            .kv_storage
            .upsert(&[(format!("{doc_id}-metadata"), meta)])
            .await
            .unwrap();
        let mut props = HashMap::new();
        props.insert("entity_type".into(), json!("CONCEPT"));
        props.insert("source_ids".into(), json!([doc_id]));
        props.insert("workspace_id".into(), json!(workspace_id.to_string()));
        state
            .storage
            .graph_storage
            .upsert_node(&format!("WIPE_ENTITY_{i}"), props)
            .await
            .unwrap();
    }

    let (status, body) =
        delete_all_documents_http_and_drain(&app, &state, &workspace_id.to_string()).await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(body["accepted"].as_bool(), Some(true));
    assert!(body["wipe_track_id"]
        .as_str()
        .is_some_and(|s| !s.is_empty()));
    assert_eq!(
        body["deleted_count"].as_u64(),
        Some(n as u64),
        "planned wipe count; body={body}"
    );

    for (i, doc_id) in doc_ids.iter().enumerate() {
        assert!(
            state
                .storage
                .kv_storage
                .get_by_id(&format!("{doc_id}-metadata"))
                .await
                .unwrap()
                .is_none(),
            "doc {doc_id} metadata must be purged"
        );
        let node = state
            .storage
            .graph_storage
            .get_node(&format!("WIPE_ENTITY_{i}"))
            .await
            .unwrap();
        assert!(node.is_none(), "graph node WIPE_ENTITY_{i} must be cleared");
    }

    println!("✅ ISSUE-309 workspace wipe admit+drain (PostgreSQL): PASSED");
}

/// SPEC-111 E2E-111-08 / #366: Clear All → GET /documents count 0.
///
/// Also proves LAW-111-9: after typed membership is empty, leftover dual-write
/// KV `-metadata` residue must **not** resurrect into the list (the v0.24.1 bug).
#[tokio::test]
async fn e2e_spec111_clear_all_list_empty_pg() {
    let _serial = db_test_serial().await;
    let pool = require_postgres!();
    let (state, kv_namespace, pg_kv) = create_postgres_test_state_named(&pool).await;
    let workspace_id = Uuid::new_v4();
    let tenant_id = "00000000-0000-0000-0000-000000000001";
    seed_test_workspace(&state, Uuid::parse_str(tenant_id).unwrap(), workspace_id).await;
    let server = Server::new(create_test_config(), state.clone());
    let app = server.build_router();

    let mut seeded_ids = Vec::new();
    for i in 0..3 {
        let doc_id = Uuid::new_v4().to_string();
        seeded_ids.push(doc_id.clone());
        let meta = json!({
            "id": doc_id,
            "title": format!("ClearAll {i}"),
            "status": "completed",
            "workspace_id": workspace_id.to_string(),
            "tenant_id": tenant_id,
        });
        state
            .storage
            .kv_storage
            .upsert(&[(format!("{doc_id}-metadata"), meta)])
            .await
            .unwrap();
    }

    let (status, _) =
        delete_all_documents_http_and_drain(&app, &state, &workspace_id.to_string()).await;
    assert_eq!(status, StatusCode::ACCEPTED);

    async fn list_total(app: &axum::Router, tenant_id: &str, workspace_id: Uuid) -> u64 {
        let list = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/documents")
                    .header("X-Tenant-ID", tenant_id)
                    .header("X-Workspace-ID", workspace_id.to_string())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(list.status(), StatusCode::OK);
        let body = extract_json(list).await;
        body["total"]
            .as_u64()
            .or_else(|| body["documents"].as_array().map(|a| a.len() as u64))
            .unwrap_or(u64::MAX)
    }

    assert_eq!(
        list_total(&app, tenant_id, workspace_id).await,
        0,
        "GET /documents must be empty after Clear All wipe"
    );

    // #366 regression: plant raw KV residue into this test's namespaced KV
    // table (create if post-125 dropped it) while `documents` stays empty.
    // Authoritative membership must not suffix-fallback into these rows.
    let kv_table = format!(
        "public.{}",
        edgequake_storage::bare_kv_table_for_namespace(&kv_namespace)
    );
    sqlx::query(&format!(
        "CREATE TABLE IF NOT EXISTS {kv_table} (
            key TEXT PRIMARY KEY,
            value JSONB NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"
    ))
    .execute(&pool)
    .await
    .expect("ensure scratch KV table for #366 residue plant");

    for doc_id in &seeded_ids {
        let ghost = json!({
            "id": doc_id,
            "title": "Ghost after wipe",
            "status": "completed",
            "workspace_id": workspace_id.to_string(),
            "tenant_id": tenant_id,
        });
        sqlx::query(&format!(
            "INSERT INTO {kv_table} (key, value, updated_at)
             VALUES ($1, $2, NOW())
             ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = NOW()"
        ))
        .bind(format!("{doc_id}-metadata"))
        .bind(&ghost)
        .execute(&pool)
        .await
        .expect("plant KV ghost");
    }

    // Reset relation-absent short-circuit so a buggy suffix fallback would
    // actually see the planted rows (regression must fail closed).
    pg_kv.seed_relation_from_dropped(false);

    assert_eq!(
        list_total(&app, tenant_id, workspace_id).await,
        0,
        "LAW-111-9: raw KV residue must not resurrect documents after wipe (#366)"
    );
    let _ = sqlx::query(&format!("DROP TABLE IF EXISTS {kv_table}"))
        .execute(&pool)
        .await;
    println!("✅ SPEC-111 clear-all list empty + #366 no-KV-resurrect (PostgreSQL): PASSED");
}

/// ISSUE-304: AUTO_RESUME=0 orphan → structured Interrupted; force entities requeues Full PDF.
#[tokio::test]
async fn issue304_interrupted_pdf_force_entities_enqueues_full_pg() {
    let _serial = db_test_serial().await;
    let pool = require_postgres!();
    let _guard = std::sync::Mutex::new(());
    // Scoped env for this test process — restore after.
    let prev = std::env::var("EDGEQUAKE_STARTUP_AUTO_RESUME").ok();
    std::env::set_var("EDGEQUAKE_STARTUP_AUTO_RESUME", "0");

    let state = create_postgres_test_state(&pool).await;
    let workspace_id = Uuid::new_v4();
    let tenant_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
    seed_test_workspace(&state, tenant_id, workspace_id).await;
    let pdf_id = Uuid::new_v4();
    // SPEC-091 Wave D: metadata keys route to the typed `documents` shell,
    // which requires a UUID doc id.
    let doc_id = pdf_id.to_string();

    let metadata = json!({
        "id": doc_id,
        "title": "interrupted.pdf",
        "status": "failed",
        "current_stage": "failed",
        "failure_code": "server_restart_interrupted",
        "error_message": "Interrupted by server restart — use Reprocess",
        "source_type": "pdf",
        "pdf_id": pdf_id.to_string(),
        "markdown": "",
        "tenant_id": tenant_id.to_string(),
        "workspace_id": workspace_id.to_string(),
    });
    state
        .storage
        .kv_storage
        .upsert(&[(format!("{doc_id}-metadata"), metadata.clone())])
        .await
        .unwrap();

    assert!(edgequake_api::services::is_interrupted_restart_metadata(
        &metadata
    ));

    let outcome = edgequake_api::services::ensure_task_for_pending_document(
        &state,
        &doc_id,
        &metadata,
        None,
        "issue304-batch",
        "reprocess_recovery_enqueue",
    )
    .await
    .expect("ensure_task");

    match outcome {
        edgequake_api::services::EnsureTaskOutcome::Enqueued { task_id } => {
            let task = state
                .tasks
                .storage
                .get_task(&task_id)
                .await
                .unwrap()
                .expect("task row");
            assert_eq!(task.task_type, edgequake_tasks::TaskType::PdfProcessing);
            let data: edgequake_tasks::PdfProcessingData =
                serde_json::from_value(task.task_data).unwrap();
            assert!(
                data.restart_from_scratch,
                "empty markdown Interrupted must upgrade to Full"
            );
            assert_eq!(
                data.reprocess_mode,
                Some(edgequake_tasks::ReprocessMode::Full)
            );
        }
        other => panic!("expected Enqueued Full PDF, got {other:?}"),
    }

    match prev {
        Some(v) => std::env::set_var("EDGEQUAKE_STARTUP_AUTO_RESUME", v),
        None => std::env::remove_var("EDGEQUAKE_STARTUP_AUTO_RESUME"),
    }
    let _ = _guard;
    println!("✅ ISSUE-304 Interrupted PDF → Full PdfProcessing (PostgreSQL): PASSED");
}
