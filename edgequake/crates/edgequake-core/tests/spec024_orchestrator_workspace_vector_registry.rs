//! SPEC-024 pass 14 (W7) — library `EdgeQuake::insert` uses per-workspace vector registry.

#![cfg(feature = "pipeline")]

use std::sync::Arc;

use edgequake_core::{
    EdgeQuake, EdgeQuakeConfig, InMemoryWorkspaceService, StorageBackend, StorageConfig, Tenant,
    WorkspaceService,
};
use edgequake_llm::MockProvider;
use edgequake_storage::adapters::memory::{
    MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage, MemoryWorkspaceVectorRegistry,
};
use edgequake_storage::traits::{GraphStorage, KVStorage, VectorStorage, WorkspaceVectorRegistry};
use uuid::Uuid;

const EXTRACTION_JSON: &str = edgequake_pipeline::SPEC021_SARAH_CHEN_EXTRACTION_JSON;

async fn seed_workspace(
    service: &InMemoryWorkspaceService,
    tenant_id: Uuid,
    workspace_id: Uuid,
    name: &str,
    slug: &str,
    embedding_dimension: usize,
) {
    let now = chrono::Utc::now();
    let ws = edgequake_core::Workspace {
        workspace_id,
        tenant_id,
        name: name.to_string(),
        slug: slug.to_string(),
        description: None,
        is_active: true,
        created_at: now,
        updated_at: now,
        metadata: std::collections::HashMap::new(),
        llm_model: "mock".to_string(),
        llm_provider: "mock".to_string(),
        embedding_model: "mock".to_string(),
        embedding_provider: "mock".to_string(),
        embedding_dimension,
        vision_llm_provider: None,
        vision_llm_model: None,
        pdf_parser_backend: None,
    };
    service
        .insert_workspace(ws)
        .await
        .expect("insert workspace");
}

#[tokio::test]
async fn spec024_orchestrator_insert_uses_workspace_vector_registry() {
    let mock = Arc::new(MockProvider::new());
    mock.add_response(EXTRACTION_JSON).await;

    let tenant_id = Uuid::new_v4();
    let ws_a = Uuid::new_v4();
    let ws_b = Uuid::new_v4();

    let workspace_service = Arc::new(InMemoryWorkspaceService::new());
    let mut tenant = Tenant::new("Test Tenant", "test-tenant");
    tenant.tenant_id = tenant_id;
    workspace_service
        .create_tenant(tenant)
        .await
        .expect("tenant");
    seed_workspace(
        &workspace_service,
        tenant_id,
        ws_a,
        "Workspace A",
        "workspace-a",
        1536,
    )
    .await;
    seed_workspace(
        &workspace_service,
        tenant_id,
        ws_b,
        "Workspace B",
        "workspace-b",
        768,
    )
    .await;

    let default_vector: Arc<dyn VectorStorage> =
        Arc::new(MemoryVectorStorage::new("global-default", 1536));
    default_vector.initialize().await.unwrap();
    let registry = Arc::new(MemoryWorkspaceVectorRegistry::new(Arc::clone(
        &default_vector,
    )));

    let kv = Arc::new(MemoryKVStorage::new("orch-ws"));
    let graph: Arc<dyn GraphStorage> = Arc::new(MemoryGraphStorage::new("orch-ws"));
    graph.initialize().await.unwrap();

    let mut config = EdgeQuakeConfig::new()
        .with_namespace("orch-ws")
        .with_gleaning(false, 0)
        .with_storage(StorageConfig {
            backend: StorageBackend::Memory,
            ..Default::default()
        });
    config.workspace_id = Some(ws_a.to_string());

    let mut eq = EdgeQuake::new(config)
        .with_storage_backends(
            Arc::clone(&kv) as Arc<dyn KVStorage>,
            Arc::clone(&default_vector),
            Arc::clone(&graph),
        )
        .with_workspace_vector_support(
            Arc::clone(&registry) as Arc<dyn WorkspaceVectorRegistry>,
            Arc::clone(&workspace_service) as Arc<dyn WorkspaceService>,
            true,
        )
        .with_providers(
            Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::LLMProvider>,
            Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
        );

    eq.initialize().await.expect("init");

    let out = eq
        .insert("Sarah Chen leads EdgeQuake in Zurich.", Some("doc-ws-a"))
        .await
        .expect("insert");

    assert!(out.success);
    assert!(out.chunks_created > 0);

    let storage_a = registry
        .get(&ws_a)
        .await
        .expect("workspace A vector storage must exist after insert");
    assert_eq!(storage_a.dimension(), 1536);

    let count_a = storage_a.count().await.expect("count ws_a");
    assert!(
        count_a > 0,
        "chunk vectors must land in workspace A storage, not global default"
    );

    let count_default = default_vector.count().await.expect("count default");
    assert!(
        count_default == 0,
        "global default storage must stay empty when registry is wired (strict mode)"
    );

    if let Some(storage_b) = registry.get(&ws_b).await {
        let count_b = storage_b.count().await.unwrap_or(0);
        assert_eq!(
            count_b, 0,
            "workspace B must not receive workspace A vectors"
        );
    }
}
