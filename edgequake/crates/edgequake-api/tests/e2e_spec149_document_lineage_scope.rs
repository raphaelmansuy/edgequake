//! SPEC-149: document-scoped lineage after durable committer + projection.
//!
//! Proves `GET /api/v1/lineage/documents/{id}` returns entities for a document
//! ingested via the P0 authority path (not the legacy merger), and that delete
//! of one document (projection-owned, as on P0 boot) leaves a sibling
//! document's lineage intact — including an entity both documents share.
//!
//! Run:
//!   EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1 \
//!     DATABASE_URL="postgresql://edgequake:edgequake_secret@localhost:5432/edgequake" \
//!     cargo test -p edgequake-api --features postgres \
//!     --test e2e_spec149_document_lineage_scope -- --nocapture

#![cfg(feature = "postgres")]

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use edgequake_api::cache_manager::CacheManager;
use edgequake_api::state::StorageMode;
use edgequake_api::{AppState, Server, ServerConfig};
use edgequake_auth::AuthConfig;
use edgequake_core::{
    ConversationService, InMemoryConversationService, InMemoryWorkspaceService, WorkspaceService,
};
use edgequake_llm::{MockProvider, ModelsConfig};
use edgequake_pipeline::Pipeline;
use edgequake_query::{QueryEngine, QueryEngineConfig};
use edgequake_rate_limiter::{RateLimitConfig as TokenBucketConfig, RateLimiter};
use edgequake_storage::contracts::{
    AccessScope, DocumentId, IngestionCommitter, PreparedIngestionBatch, PreparedRecord, TenantId,
    WorkspaceId,
};
use edgequake_storage::kv_keys;
use edgequake_storage::traits::{GraphStorage, KVStorage, VectorStorage};
use edgequake_storage::{
    AgeGraphProjectionApplier, MemoryWorkspaceVectorRegistry, PgChunkEmbeddingIndex,
    PgIngestionCommitter, PgProjectionLedger, PgVectorStorage, PgvectorProjectionApplier,
    PostgresAGEGraphStorage, PostgresConfig, PostgresKVStorage, ProjectionWorkerConfig,
    ProjectionWorkerRuntime,
};
use serial_test::serial;
use sha2::{Digest as _, Sha256};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

#[path = "common/test_db.rs"]
mod test_db;

/// Canonical default tenant/workspace UUIDs (mirror `seed_default_workspace`).
const DEFAULT_TENANT_ID: &str = "00000000-0000-0000-0000-000000000002";
const DEFAULT_WORKSPACE_ID: &str = "00000000-0000-0000-0000-000000000003";

fn get_database_url() -> Option<String> {
    let base = std::env::var("DATABASE_URL").ok().or_else(|| {
        let password = std::env::var("POSTGRES_PASSWORD").ok()?;
        let host = std::env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = std::env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());
        let db = std::env::var("POSTGRES_DB").unwrap_or_else(|_| "edgequake".to_string());
        let user = std::env::var("POSTGRES_USER").unwrap_or_else(|_| "edgequake".to_string());
        Some(format!(
            "postgresql://{}:{}@{}:{}/{}",
            user, password, host, port, db
        ))
    })?;
    Some(test_db::isolated_test_url(&base))
}

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

async fn create_pool() -> Option<PgPool> {
    let database_url = get_database_url()?;
    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .ok()
}

async fn create_postgres_state(pool: &PgPool) -> (AppState, PostgresConfig) {
    let namespace = format!("lin_{}", &Uuid::new_v4().to_string().replace('-', "")[..12]);
    let pg_config = create_pg_config(&namespace);

    let kv_storage = Arc::new(PostgresKVStorage::new(pg_config.clone()));
    kv_storage.initialize().await.expect("init KV");
    let vector_storage = Arc::new(PgVectorStorage::new(pg_config.clone()));
    vector_storage.initialize().await.expect("init vectors");
    let graph_storage = Arc::new(PostgresAGEGraphStorage::new(pg_config.clone()));
    graph_storage.initialize().await.expect("init AGE");

    let mock_provider = Arc::new(MockProvider::new());
    let pipeline = Arc::new(Pipeline::default_pipeline());
    let workspace_service: Arc<dyn WorkspaceService> = Arc::new(InMemoryWorkspaceService::new());
    workspace_service.seed_default_workspace().await;
    edgequake_api::services::workspace_document_index::register_membership_pool(pool.clone());
    edgequake_api::services::identity_storage::ensure_default_tenant_workspace(
        pool,
        &edgequake_api::state::ApiSecurityConfig::default(),
    )
    .await
    .expect("seed default tenant/workspace");
    let conversation_service: Arc<dyn ConversationService> =
        Arc::new(InMemoryConversationService::new());

    let task_storage = Arc::new(edgequake_tasks::memory::MemoryTaskStorage::new());
    let task_queue = Arc::new(edgequake_tasks::queue::ChannelTaskQueue::new(100));
    let engine_impl = Arc::new(QueryEngine::with_mock_keywords(
        QueryEngineConfig::default(),
        Arc::clone(&vector_storage) as Arc<dyn VectorStorage>,
        Arc::clone(&graph_storage) as Arc<dyn GraphStorage>,
        Arc::clone(&mock_provider) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
        Arc::clone(&mock_provider) as Arc<dyn edgequake_llm::traits::LLMProvider>,
    ));
    let auth_config = AuthConfig {
        auth_enabled: false,
        ..AuthConfig::default()
    };
    let vector_registry: Arc<dyn edgequake_storage::traits::WorkspaceVectorRegistry> = Arc::new(
        MemoryWorkspaceVectorRegistry::new(Arc::clone(&vector_storage) as Arc<dyn VectorStorage>),
    );
    let committer = Arc::new(PgIngestionCommitter::new(pool.clone()));
    let ingestion_committer: Arc<dyn edgequake_storage::contracts::IngestionCommitter> =
        Arc::clone(&committer) as _;
    let lifecycle_committer: Arc<dyn edgequake_storage::contracts::LifecycleCommitter> =
        Arc::clone(&committer) as _;
    let document_reader: Arc<dyn edgequake_storage::contracts::DocumentReader> =
        Arc::clone(&committer) as _;
    let projection_ledger: Arc<dyn edgequake_storage::ProjectionWorkLedger> =
        Arc::new(PgProjectionLedger::new(pool.clone()));
    // Mirrors P0 boot: the runtime owns graph/vector writes, so delete takes
    // the projection path (`projection_owns_cleanup`), not the legacy cascade.
    let projection_worker = Arc::new(ProjectionWorkerRuntime::spawn(
        Uuid::new_v4(),
        Arc::clone(&projection_ledger),
        Arc::new(AgeGraphProjectionApplier::new(
            Arc::clone(&graph_storage) as Arc<dyn GraphStorage>,
            pool.clone(),
        )),
        Arc::new(PgvectorProjectionApplier::new(
            Arc::new(PgChunkEmbeddingIndex::new(
                pool.clone(),
                "api-lineage-scope",
            )),
            None,
            pool.clone(),
        )),
        ProjectionWorkerConfig {
            batch_size: 16,
            ..ProjectionWorkerConfig::default()
        },
        Duration::from_millis(50),
    ));

    let state = AppState {
        storage: edgequake_api::state::StorageRuntime {
            kv_storage: Arc::clone(&kv_storage) as Arc<dyn KVStorage>,
            vector_storage: Arc::clone(&vector_storage) as Arc<dyn VectorStorage>,
            vector_registry,
            graph_storage: Arc::clone(&graph_storage) as Arc<dyn GraphStorage>,
            auth_memory: Arc::new(
                edgequake_api::services::auth_memory_store::AuthMemoryStore::new(),
            ),
            pdf_storage: None,
            original_storage: None,
            mm_asset_storage: None,
            page_layout_storage: None,
            mode: StorageMode::PostgreSQL,
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
        ingestion_committer: Some(ingestion_committer),
        lifecycle_committer: Some(lifecycle_committer),
        document_reader: Some(document_reader),
        projection_ledger: Some(projection_ledger),
        projection_worker: Some(projection_worker),
        operational_stores: edgequake_api::state::OperationalStores::default(),
        start_time: std::time::Instant::now(),
        path_validation_config: edgequake_api::path_validation::PathValidationConfig {
            allow_any_path: true,
            ..Default::default()
        },
        audit_logger: None,
        migration_bootstrap: None,
        security: edgequake_api::state::ApiSecurityConfig::default(),
        resource_guard: edgequake_core::ResourceGuard::default(),
        graph_materialize: Arc::new(edgequake_core::GraphMaterializationSemaphore::new(4)),
        pdf_vision: Arc::new(edgequake_core::PdfVisionSemaphore::new(2)),
        parse_jobs: edgequake_api::handlers::parse::ParseJobStore::from_env(),
        read_path_db: Arc::new(edgequake_api::read_path::ReadPathDbPermit::from_env()),
        postgres_capabilities: None,
        server_config: edgequake_api::server_config_store::ServerConfigStore::new(),
    };
    (state, pg_config)
}

fn prepared_record(id: Uuid, payload: serde_json::Value) -> PreparedRecord {
    let bytes = serde_json::to_vec(&payload).expect("encode");
    PreparedRecord {
        id,
        revision: 1,
        digest: Sha256::digest(&bytes).into(),
        payload: bytes,
    }
}

/// Delivery states for `document_id` events with `operation` prefix.
async fn delivery_states(pool: &PgPool, document_id: Uuid, operation: &str) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT d.state FROM projection_deliveries d \
         JOIN projection_events e USING (event_id) \
         WHERE e.object_id = $1 AND e.operation LIKE $2 || '%'",
    )
    .bind(document_id)
    .bind(operation)
    .fetch_all(pool)
    .await
    .expect("delivery states")
}

async fn wait_until_applied(pool: &PgPool, document_id: Uuid, operation: &str) {
    for _ in 0..200 {
        let states = delivery_states(pool, document_id, operation).await;
        if !states.is_empty() && states.iter().all(|s| s == "applied") {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("{operation} deliveries not applied for {document_id}");
}

fn string_set(value: Option<&serde_json::Value>) -> Vec<String> {
    let mut out: Vec<String> = value
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

fn node_fact(
    node_id: &str,
    document_id: Uuid,
    tenant_id: Uuid,
    workspace_id: Uuid,
    entity_type: &str,
    description: &str,
) -> serde_json::Value {
    serde_json::json!({
        "schema": "edgequake.graph.fact.v1",
        "kind": "node",
        "node_id": node_id,
        "properties": {
            "entity_type": entity_type,
            "description": description,
            "source_chunk_ids": [format!("{document_id}-chunk-0")],
            "source_document_id": document_id,
            "tenant_id": tenant_id,
            "workspace_id": workspace_id
        }
    })
}

fn edge_fact(
    source: &str,
    target: &str,
    document_id: Uuid,
    tenant_id: Uuid,
    workspace_id: Uuid,
) -> serde_json::Value {
    serde_json::json!({
        "schema": "edgequake.graph.fact.v1",
        "kind": "edge",
        "source": source,
        "target": target,
        "properties": {
            "relation_type": "RELATED_TO",
            "description": "knows",
            "weight": 0.5,
            "source_chunk_ids": [format!("{document_id}-chunk-0")],
            "source_document_id": document_id,
            "tenant_id": tenant_id,
            "workspace_id": workspace_id
        }
    })
}

#[allow(clippy::too_many_arguments)] // test harness packs tenant/workspace/doc/node labels
async fn commit_and_project_document(
    pool: &PgPool,
    state: &AppState,
    tenant_id: Uuid,
    workspace_id: Uuid,
    document_id: Uuid,
    node_a: &str,
    node_b: &str,
    title: &str,
    kv_status: &str,
) {
    let chunk_uuid = Uuid::new_v4();
    let fact_a = node_fact(
        node_a,
        document_id,
        tenant_id,
        workspace_id,
        "PERSON",
        "Alice",
    );
    let fact_b = node_fact(
        node_b,
        document_id,
        tenant_id,
        workspace_id,
        "CONCEPT",
        "Bob",
    );
    let fact_e = edge_fact(node_a, node_b, document_id, tenant_id, workspace_id);

    sqlx::query(
        "INSERT INTO documents (id, tenant_id, workspace_id, title, content, status, chunk_count)
         VALUES ($1, $2, $3, $4, 'Alice knows Bob', 'indexed', 1)
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(document_id)
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(title)
    .execute(pool)
    .await
    .expect("seed document row");

    let command = PreparedIngestionBatch {
        scope: AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id)),
        document_id: DocumentId::new(document_id),
        ingest_generation: 1,
        batch_ordinal: 0,
        expected_revision: Some(0),
        idempotency_key: format!("api-lineage-scope-{document_id}"),
        schema_version: 1,
        canonical_digest: Sha256::digest(format!("api-lin:{document_id}").as_bytes()).into(),
        chunks: vec![prepared_record(
            chunk_uuid,
            serde_json::json!({
                "chunk_index": 0,
                "content": "Alice knows Bob",
                "metadata": {"legacy_chunk_key": format!("{document_id}-chunk-0")}
            }),
        )],
        facts: vec![
            prepared_record(Uuid::new_v4(), fact_a.clone()),
            prepared_record(Uuid::new_v4(), fact_b.clone()),
            prepared_record(Uuid::new_v4(), fact_e.clone()),
        ],
        contributions: vec![
            prepared_record(Uuid::new_v4(), fact_a),
            prepared_record(Uuid::new_v4(), fact_b),
            prepared_record(Uuid::new_v4(), fact_e),
        ],
        embeddings: vec![prepared_record(
            chunk_uuid,
            serde_json::json!({
                "schema": "edgequake.embedding.v1",
                "family": "chunk",
                "subject_id": chunk_uuid,
                "workspace_id": workspace_id,
                "model_id": "api-lineage-scope",
                "dimensions": 3,
                "embedding": [0.1, 0.2, 0.3]
            }),
        )],
    };
    PgIngestionCommitter::new(pool.clone())
        .commit_batch(&command)
        .await
        .expect("commit durable batch");

    let metadata = serde_json::json!({
        "id": document_id.to_string(),
        "status": kv_status,
        "current_stage": kv_status,
        "tenant_id": tenant_id.to_string(),
        "workspace_id": workspace_id.to_string(),
        "file_path": title,
        "content_summary": "Alice knows Bob",
    });
    state
        .storage
        .kv_storage
        .upsert(&[(kv_keys::doc_metadata(&document_id.to_string()), metadata)])
        .await
        .expect("seed kv metadata");
    state
        .storage
        .kv_storage
        .upsert(&[(
            kv_keys::doc_chunk(&document_id.to_string(), 0),
            serde_json::json!({"content": "Alice knows Bob", "chunk_index": 0}),
        )])
        .await
        .expect("seed kv chunk");

    wait_until_applied(pool, document_id, "ingest").await;
}

async fn get_lineage(app: &axum::Router, document_id: Uuid) -> (StatusCode, serde_json::Value) {
    get_json(app, &format!("/api/v1/lineage/documents/{document_id}")).await
}

async fn get_json(app: &axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .header("X-Tenant-ID", DEFAULT_TENANT_ID)
                .header("X-Workspace-ID", DEFAULT_WORKSPACE_ID)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("lineage request");
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap_or(serde_json::json!({}));
    (status, json)
}

#[tokio::test]
#[serial]
async fn durable_ingest_lineage_endpoint_returns_entities_and_delete_isolates() {
    std::env::set_var("EDGEQUAKE_ALLOW_MOCK_PROVIDER", "1");
    std::env::set_var("EDGEQUAKE_LLM_PROVIDER", "mock");
    std::env::set_var("EDGEQUAKE_EMBEDDING_PROVIDER", "mock");

    let Some(pool) = create_pool().await else {
        let strict = std::env::var("EDGEQUAKE_REQUIRE_POSTGRES_TESTS")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        if strict {
            panic!("EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1 but PostgreSQL unavailable");
        }
        eprintln!("SKIP: PostgreSQL unavailable");
        return;
    };

    let (state, _pg_config) = create_postgres_state(&pool).await;
    let tenant_id = Uuid::parse_str(DEFAULT_TENANT_ID).unwrap();
    let workspace_id = Uuid::parse_str(DEFAULT_WORKSPACE_ID).unwrap();

    // Unique per run: the scratch DB graph is reused across runs.
    let run = &Uuid::new_v4().simple().to_string()[..8].to_uppercase();
    let node_a = format!("API_LIN_A_{run}");
    let node_b = format!("API_LIN_B_{run}");
    let shared = format!("API_LIN_SHARED_{run}");

    let doc_a = Uuid::new_v4();
    let doc_b = Uuid::new_v4();
    commit_and_project_document(
        &pool,
        &state,
        tenant_id,
        workspace_id,
        doc_a,
        &node_a,
        &shared,
        "lineage-scope-a.md",
        "completed",
    )
    .await;
    commit_and_project_document(
        &pool,
        &state,
        tenant_id,
        workspace_id,
        doc_b,
        &node_b,
        &shared,
        "lineage-scope-b.md",
        // Durable commit leaves `projecting` until deliveries settle.
        "projecting",
    )
    .await;

    let app = Server::new(
        ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            enable_cors: false,
            enable_compression: false,
            enable_swagger: false,
        },
        state.clone(),
    )
    .build_router();

    // Detail polls must settle `projecting` once deliveries are applied.
    let (detail_status, detail_b) = get_json(&app, &format!("/api/v1/documents/{doc_b}")).await;
    assert_eq!(detail_status, StatusCode::OK, "detail B body={detail_b}");
    assert_eq!(
        detail_b["status"], "completed",
        "detail GET must promote applied projecting doc: {detail_b}"
    );

    let (status_a, json_a) = get_lineage(&app, doc_a).await;
    assert_eq!(status_a, StatusCode::OK, "lineage A body={json_a}");
    let entities_a = json_a["entities"].as_array().expect("entities A");
    assert!(
        !entities_a.is_empty(),
        "durable-path document A must return entities: {json_a}"
    );
    let rels_a = json_a["relationships"].as_array().expect("rels A");
    assert!(
        !rels_a.is_empty(),
        "durable-path document A must return relationships: {json_a}"
    );

    let (status_b, json_b) = get_lineage(&app, doc_b).await;
    assert_eq!(status_b, StatusCode::OK, "lineage B body={json_b}");
    assert!(
        !json_b["entities"]
            .as_array()
            .expect("entities B")
            .is_empty(),
        "document B must return entities before delete: {json_b}"
    );

    // Delete document A (admit + drain deletion task inline).
    let delete_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/documents/{doc_a}"))
                .header("X-Tenant-ID", DEFAULT_TENANT_ID)
                .header("X-Workspace-ID", DEFAULT_WORKSPACE_ID)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("delete request");
    assert_eq!(
        delete_resp.status(),
        StatusCode::ACCEPTED,
        "delete admits with 202"
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
    edgequake_api::services::perform_document_deletion(&state, &data, &tenant)
        .await
        .expect("perform_document_deletion");

    // Projection-owned delete: tombstone events exist and were applied.
    let delete_states = delivery_states(&pool, doc_a, "delete").await;
    assert!(
        !delete_states.is_empty() && delete_states.iter().all(|s| s == "applied"),
        "delete must go through applied projection deliveries: {delete_states:?}"
    );

    let graph = &state.storage.graph_storage;
    let node_id = |name: &str| edgequake_storage::canonical_graph_node_id(workspace_id, name);
    assert!(
        graph
            .get_node(&node_id(&node_a))
            .await
            .expect("read A")
            .is_none(),
        "A-only entity must be removed"
    );
    let shared_node = graph
        .get_node(&node_id(&shared))
        .await
        .expect("read shared")
        .expect("shared entity must survive for document B");
    let b_chunk = format!("{doc_b}-chunk-0");
    for key in edgequake_storage::INDEXED_LINEAGE_ARRAY_KEYS {
        assert_eq!(
            string_set(shared_node.properties.get(key)),
            vec![b_chunk.clone()],
            "shared entity {key} must keep only B's chunk"
        );
    }
    assert_eq!(
        string_set(shared_node.properties.get("source_document_ids")),
        vec![doc_b.to_string()],
        "shared entity must list only document B"
    );

    let (status_a_after, json_a_after) = get_lineage(&app, doc_a).await;
    assert!(
        status_a_after == StatusCode::NOT_FOUND
            || json_a_after["entities"]
                .as_array()
                .map(|e| e.is_empty())
                .unwrap_or(true),
        "deleted document A must be empty or 404: status={status_a_after} body={json_a_after}"
    );

    let (status_b_after, json_b_after) = get_lineage(&app, doc_b).await;
    assert_eq!(
        status_b_after,
        StatusCode::OK,
        "sibling B must remain after A delete: {json_b_after}"
    );
    let entities_b_after = json_b_after["entities"]
        .as_array()
        .expect("entities B after");
    assert_eq!(
        entities_b_after.len(),
        json_b["entities"].as_array().expect("entities B").len(),
        "sibling B lineage must stay intact: {json_b_after}"
    );
}
