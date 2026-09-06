//! SPEC-098 Symptom F / LAW-098-13: edge provenance SSOT + multigraph cascade.
//!
//! science_one shape: topology `source_id` + poisoned `source_ids` + singular
//! `source_chunk_id` / `source_document_id`. Cascade must delete exclusive edges
//! and prune both multigraph sisters keyed by `(src, tgt, rel)`.
#![cfg(feature = "postgres")]

#[path = "common/test_db.rs"]
mod test_db;

use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::Duration;

use edgequake_api::middleware::TenantContext;
use edgequake_api::services::{
    cascade_remove_document_sources, find_document_edges, DocumentSourceScope,
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
use serde_json::json;
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

fn science_one_edge_props(
    tenant: &str,
    workspace: &str,
    src_entity: &str,
    doc_id: &str,
    chunk: &str,
    rel: &str,
) -> HashMap<String, serde_json::Value> {
    let mut props = HashMap::new();
    props.insert("tenant_id".into(), json!(tenant));
    props.insert("workspace_id".into(), json!(workspace));
    props.insert("relation_type".into(), json!(rel));
    props.insert("source_id".into(), json!(src_entity));
    props.insert("source_ids".into(), json!([src_entity])); // poisoned
    props.insert("source_chunk_ids".into(), json!([src_entity])); // poisoned
    props.insert("source_chunk_id".into(), json!(chunk));
    props.insert("source_document_id".into(), json!(doc_id));
    props.insert("source_document_ids".into(), json!([]));
    props.insert("description".into(), json!("edge from science_one shape"));
    props
}

#[tokio::test]
async fn e2e_spec098_science_one_edge_provenance_cascade() {
    let Some(pool) = create_test_pool().await else {
        eprintln!("skip: DATABASE_URL unset");
        return;
    };
    let state = create_postgres_test_state(&pool).await;

    let tenant = "00000000-0000-0000-0000-000000000002";
    let workspace = "00000000-0000-0000-0000-000000000003";
    let doc = Uuid::new_v4().to_string();
    let chunk = format!("{doc}-chunk-27");
    let src = format!("{workspace}::JSON");
    let tgt = format!("{workspace}::CONFIG");

    for id in [&src, &tgt] {
        let mut n = HashMap::new();
        n.insert("entity_type".into(), json!("CONCEPT"));
        n.insert("tenant_id".into(), json!(tenant));
        n.insert("workspace_id".into(), json!(workspace));
        n.insert("source_ids".into(), json!([&format!("{doc}-chunk-0")]));
        state
            .storage
            .graph_storage
            .upsert_node(id, n)
            .await
            .expect("seed node");
    }

    // Exclusive edge: only science_one provenance (singular) + poisoned arrays.
    // Nodes also reference doc so DETACH path may apply — use a different doc on
    // nodes so the edge is exclusive and endpoints survive.
    let other = Uuid::new_v4().to_string();
    for id in [&src, &tgt] {
        let mut n = HashMap::new();
        n.insert("entity_type".into(), json!("CONCEPT"));
        n.insert("tenant_id".into(), json!(tenant));
        n.insert("workspace_id".into(), json!(workspace));
        n.insert("source_ids".into(), json!([&format!("{other}-chunk-0")]));
        state
            .storage
            .graph_storage
            .upsert_node(id, n)
            .await
            .expect("reseed node other-doc");
    }

    // Mixed-case accent like production French extracts (prop é vs eq_rel É).
    let props = science_one_edge_props(tenant, workspace, &src, &doc, &chunk, "REPRéSENTE");
    state
        .storage
        .graph_storage
        .upsert_edge(&src, &tgt, props)
        .await
        .expect("seed edge");

    let scope = DocumentSourceScope::from_document_id(doc.clone());
    let tenant_ctx = TenantContext {
        tenant_id: Some(tenant.to_string()),
        workspace_id: Some(workspace.to_string()),
        user_id: None,
    };

    // Discovery must find the edge via singular citation despite poisoned arrays.
    let found = find_document_edges(&state.storage.graph_storage, Some(&tenant_ctx), &scope)
        .await
        .expect("find");
    assert_eq!(found.len(), 1, "singular orphan discovery: {found:?}");

    let stats = cascade_remove_document_sources(
        &state.storage.graph_storage,
        None,
        Some(&tenant_ctx),
        &scope,
    )
    .await
    .expect("cascade");
    assert_eq!(
        stats.relationships_removed, 1,
        "exclusive edge must delete, not false-shared update: {stats:?}"
    );

    let leftover = find_document_edges(&state.storage.graph_storage, Some(&tenant_ctx), &scope)
        .await
        .expect("post-proof find");
    assert!(
        leftover.is_empty(),
        "post-proof must be empty; leftover={leftover:?}"
    );
    assert!(
        state
            .storage
            .graph_storage
            .get_node(&src)
            .await
            .expect("get")
            .is_some(),
        "endpoint nodes must survive exclusive edge delete"
    );
}

#[tokio::test]
async fn e2e_spec098_multigraph_edge_cascade_prunes_both_rels() {
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
    let src = format!("MG_SRC_{}", &doc_a[..8]);
    let tgt = format!("MG_TGT_{}", &doc_a[..8]);

    for id in [&src, &tgt] {
        let mut n = HashMap::new();
        n.insert("entity_type".into(), json!("CONCEPT"));
        n.insert("tenant_id".into(), json!(tenant));
        n.insert("workspace_id".into(), json!(workspace));
        n.insert("source_ids".into(), json!([&chunk_b]));
        state
            .storage
            .graph_storage
            .upsert_node(id, n)
            .await
            .expect("seed node");
    }

    for rel in ["KNOWS", "WORKS_WITH"] {
        let mut props = HashMap::new();
        props.insert("tenant_id".into(), json!(tenant));
        props.insert("workspace_id".into(), json!(workspace));
        props.insert("relation_type".into(), json!(rel));
        props.insert("source_ids".into(), json!([&chunk_a, &chunk_b]));
        props.insert("source_chunk_ids".into(), json!([&chunk_a, &chunk_b]));
        props.insert("source_document_ids".into(), json!([&doc_a, &doc_b]));
        state
            .storage
            .graph_storage
            .upsert_edge(&src, &tgt, props)
            .await
            .expect("seed multigraph edge");
    }

    let scope = DocumentSourceScope::from_document_id(doc_a.clone());
    let tenant_ctx = TenantContext {
        tenant_id: Some(tenant.to_string()),
        workspace_id: Some(workspace.to_string()),
        user_id: None,
    };

    let before = find_document_edges(&state.storage.graph_storage, Some(&tenant_ctx), &scope)
        .await
        .expect("find before");
    assert_eq!(
        before.len(),
        2,
        "both rel types must be discovered: {before:?}"
    );

    let stats = cascade_remove_document_sources(
        &state.storage.graph_storage,
        None,
        Some(&tenant_ctx),
        &scope,
    )
    .await
    .expect("cascade");
    assert_eq!(
        stats.relationships_updated, 2,
        "both sisters must be pruned: {stats:?}"
    );

    let leftover = find_document_edges(&state.storage.graph_storage, Some(&tenant_ctx), &scope)
        .await
        .expect("post-proof");
    assert!(
        leftover.is_empty(),
        "post-proof empty after multigraph prune; leftover={leftover:?}"
    );

    // Surviving sisters still discoverable via doc_b and must not cite doc_a.
    let scope_b = DocumentSourceScope::from_document_id(doc_b.clone());
    let surviving = find_document_edges(&state.storage.graph_storage, Some(&tenant_ctx), &scope_b)
        .await
        .expect("find doc_b edges");
    assert_eq!(
        surviving.len(),
        2,
        "both multigraph sisters must survive shared prune: {surviving:?}"
    );
    let mut rels: Vec<String> = surviving
        .iter()
        .map(|e| edgequake_storage::normalize_rel_type(&e.properties))
        .collect();
    rels.sort();
    assert_eq!(rels, vec!["KNOWS".to_string(), "WORKS_WITH".to_string()]);
    for edge in &surviving {
        let ids = edge
            .properties
            .get("source_ids")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        assert!(
            ids.iter().any(|s| s == &chunk_b || s == &doc_b),
            "must keep doc_b provenance: {ids:?}"
        );
        assert!(
            !ids.iter()
                .any(|s| s == &chunk_a || s.starts_with(&format!("{doc_a}-"))),
            "doc_a must be gone: {ids:?}"
        );
    }
}
