//! SPEC-098 F-098-20 / LAW-098-12: shared-entity cascade prune must stick on AGE.
//!
//! Native upsert defaults to `eq_merge_graph_properties` (union). Cascade prune
//! must use Replace mode so remaining `source_ids` are not re-unioned with the
//! deleted document's chunks — otherwise post-proof fails closed.
#![cfg(feature = "postgres")]

#[path = "common/test_db.rs"]
mod test_db;

use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::Duration;

use edgequake_api::middleware::TenantContext;
use edgequake_api::services::{
    cascade_remove_document_sources, find_document_nodes, perform_document_deletion,
    DocumentSourceScope,
};
use edgequake_api::AppState;
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
use edgequake_tasks::DeletionTaskData;
use serde_json::{json, Value};
use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

fn get_database_url() -> Option<String> {
    let base = env::var("DATABASE_URL").ok().or_else(|| {
        let password = env::var("POSTGRES_PASSWORD").ok()?;
        let host = env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());
        let db = env::var("POSTGRES_DB").unwrap_or_else(|_| "edgequake".to_string());
        let user = env::var("POSTGRES_USER").unwrap_or_else(|_| "edgequake".to_string());
        Some(format!("postgresql://{user}:{password}@{host}:{port}/{db}"))
    })?;
    Some(test_db::isolated_test_url(&base))
}

async fn create_test_pool() -> Option<PgPool> {
    let database_url = get_database_url()?;
    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .ok()
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

async fn create_postgres_test_state(pool: &PgPool) -> AppState {
    use edgequake_api::cache_manager::CacheManager;
    use edgequake_api::state::StorageMode;
    use edgequake_auth::AuthConfig;
    use edgequake_llm::ModelsConfig;
    use edgequake_rate_limiter::{RateLimitConfig as TokenBucketConfig, RateLimiter};

    let namespace = format!(
        "test_{}",
        &Uuid::new_v4().to_string().replace('-', "")[..12]
    );
    let pg_config = create_pg_config(&namespace);

    let kv_storage = Arc::new(PostgresKVStorage::new(pg_config.clone()));
    kv_storage.initialize().await.expect("kv init");
    let vector_storage = Arc::new(PgVectorStorage::new(pg_config.clone()));
    vector_storage.initialize().await.expect("vector init");
    let graph_storage = Arc::new(PostgresAGEGraphStorage::new(pg_config.clone()));
    graph_storage.initialize().await.expect("graph init");

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
    let vector_registry: Arc<dyn edgequake_storage::traits::WorkspaceVectorRegistry> = Arc::new(
        MemoryWorkspaceVectorRegistry::new(Arc::clone(&vector_storage) as Arc<dyn VectorStorage>),
    );

    AppState {
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
        auth: edgequake_api::state::AuthRuntime::new(AuthConfig {
            auth_enabled: false,
            ..AuthConfig::default()
        }),
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
        graph_materialize: Arc::new(edgequake_core::GraphMaterializationSemaphore::new(4)),
        pdf_vision: Arc::new(edgequake_core::PdfVisionSemaphore::new(2)),
        parse_jobs: edgequake_api::handlers::parse::ParseJobStore::from_env(),
        read_path_db: Arc::new(edgequake_api::read_path::ReadPathDbPermit::from_env()),
        postgres_capabilities: None,
        server_config: edgequake_api::server_config_store::ServerConfigStore::new(),
    }
}

fn source_ids_of(node: &edgequake_storage::GraphNode) -> Vec<String> {
    node.properties
        .get("source_ids")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

#[tokio::test]
async fn e2e_spec098_cascade_shared_prune_survives_eq_merge() {
    let Some(pool) = create_test_pool().await else {
        eprintln!("skip: DATABASE_URL unset");
        return;
    };
    let state = create_postgres_test_state(&pool).await;

    let tenant = "00000000-0000-0000-0000-000000000002";
    let workspace = "00000000-0000-0000-0000-000000000003";
    let doc_a = Uuid::new_v4().to_string();
    let doc_b = Uuid::new_v4().to_string();
    let chunk_a = format!("{doc_a}-chunk-0");
    let chunk_b = format!("{doc_b}-chunk-0");
    let shared_id = format!("SHARED_PRUNE_{}", &doc_a[..8]);

    // Seed shared AGE node via MergeSources upsert (ingest path).
    let mut props = HashMap::new();
    props.insert("entity_type".into(), json!("CONCEPT"));
    props.insert("tenant_id".into(), json!(tenant));
    props.insert("workspace_id".into(), json!(workspace));
    props.insert("source_ids".into(), json!([&chunk_a, &chunk_b]));
    props.insert("source_chunk_ids".into(), json!([&chunk_a, &chunk_b]));
    props.insert("source_document_ids".into(), json!([&doc_a, &doc_b]));
    props.insert(
        "description".into(),
        json!(format!(
            "From A [chunk_id={chunk_a}]\n\nFrom B [chunk_id={chunk_b}]"
        )),
    );
    state
        .storage
        .graph_storage
        .upsert_node(&shared_id, props)
        .await
        .expect("seed shared node");

    // KV metadata for docA so full deletion can wipe list surfaces.
    state
        .storage
        .kv_storage
        .upsert(&[(
            format!("{doc_a}-metadata"),
            json!({
                "id": doc_a,
                "title": "docA prune",
                "status": "completed",
                "tenant_id": tenant,
                "workspace_id": workspace,
                "created_at": "2026-01-01T00:00:00Z",
            }),
        )])
        .await
        .expect("seed meta");
    state
        .storage
        .kv_storage
        .upsert(&[(format!("{doc_a}-content"), json!({ "content": "body A" }))])
        .await
        .expect("seed content");

    let scope = DocumentSourceScope::from_document_id(doc_a.clone());
    let tenant_ctx = TenantContext {
        tenant_id: Some(tenant.to_string()),
        workspace_id: Some(workspace.to_string()),
        user_id: None,
    };

    let stats = cascade_remove_document_sources(
        &state.storage.graph_storage,
        None,
        Some(&tenant_ctx),
        &scope,
    )
    .await
    .expect("cascade prune");
    assert_eq!(stats.entities_updated, 1, "shared node should be pruned");
    assert_eq!(stats.entities_removed, 0, "shared node must survive");

    let node = state
        .storage
        .graph_storage
        .get_node(&shared_id)
        .await
        .expect("get")
        .expect("shared node remains");
    let ids = source_ids_of(&node);
    assert_eq!(
        ids,
        vec![chunk_b.clone()],
        "Replace mode must persist pruned source_ids (not re-union docA): {ids:?}"
    );
    assert!(
        !ids.iter().any(|s| s.starts_with(&doc_a)),
        "docA sources must be gone"
    );

    // Post-proof discovery for docA must be empty.
    let leftover = find_document_nodes(&state.storage.graph_storage, Some(&tenant_ctx), &scope)
        .await
        .expect("find");
    assert!(
        leftover.is_empty(),
        "post-proof must pass; leftover={leftover:?}"
    );

    // Full delete path should complete (KV wipe) without delete_failed.
    let del_data = DeletionTaskData {
        document_id: doc_a.clone(),
        key_prefix: doc_a.clone(),
        workspace_id: workspace.to_string(),
        tenant_id: tenant.to_string(),
        deletion_track_id: format!("spec098-prune-{doc_a}"),
        metadata_key: Some(format!("{doc_a}-metadata")),
        chunk_ids: vec![],
        has_content: true,
        content_hash: None,
        pdf_id: None,
        ingest_track_id: None,
        document_status: Some("completed".into()),
    };
    perform_document_deletion(&state, &del_data, &tenant_ctx)
        .await
        .expect("full delete must succeed after prune sticks");

    let meta: Option<Value> = state
        .storage
        .kv_storage
        .get_by_id(&format!("{doc_a}-metadata"))
        .await
        .expect("kv");
    assert!(meta.is_none(), "docA metadata must be wiped");

    // Surviving shared node still references only docB.
    let node_after = state
        .storage
        .graph_storage
        .get_node(&shared_id)
        .await
        .expect("get")
        .expect("shared survives full delete");
    assert_eq!(source_ids_of(&node_after), vec![chunk_b]);
}
