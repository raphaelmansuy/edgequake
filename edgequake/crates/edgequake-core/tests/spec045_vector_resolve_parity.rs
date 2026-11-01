//! SPEC-045 — Vector resolve parity between ingest and query paths.

use std::sync::Arc;

use edgequake_core::workspace_vector_resolve::{
    is_dimension_mismatch_error, resolve_workspace_vector_storage, WorkspaceVectorResolveInput,
    WorkspaceVectorResolvePolicy,
};
use edgequake_core::{InMemoryWorkspaceService, Tenant, Workspace, WorkspaceService};
use edgequake_storage::adapters::memory::{MemoryVectorStorage, MemoryWorkspaceVectorRegistry};
use edgequake_storage::traits::{VectorStorage, WorkspaceVectorConfig, WorkspaceVectorRegistry};
use uuid::Uuid;

async fn seed_workspace(service: &InMemoryWorkspaceService, workspace_id: Uuid, dimension: usize) {
    let now = chrono::Utc::now();
    let tenant_id = Uuid::new_v4();
    let mut tenant = Tenant::new("Spec045 Tenant", "spec045-tenant");
    tenant.tenant_id = tenant_id;
    service.create_tenant(tenant).await.expect("tenant");
    let ws = Workspace {
        workspace_id,
        tenant_id,
        name: "parity-ws".to_string(),
        slug: "parity-ws".to_string(),
        description: None,
        is_active: true,
        created_at: now,
        updated_at: now,
        metadata: Default::default(),
        llm_model: "mock".to_string(),
        llm_provider: "mock".to_string(),
        embedding_model: "mock".to_string(),
        embedding_provider: "mock".to_string(),
        embedding_dimension: dimension,
        vision_llm_provider: None,
        vision_llm_model: None,
        pdf_parser_backend: None,
        rag_profile: Default::default(),
        embedding_modalities: Default::default(),
        fts_regconfig: Default::default(),
        archived_at: None,
        archived_by: None,
    };
    service.insert_workspace(ws).await.expect("insert ws");
}

#[tokio::test]
async fn spec045_ingest_evicts_stale_cache_on_dimension_change() {
    let ws_id = Uuid::new_v4();
    let workspace_service = Arc::new(InMemoryWorkspaceService::new());
    seed_workspace(&workspace_service, ws_id, 1536).await;

    let default_vector: Arc<dyn VectorStorage> =
        Arc::new(MemoryVectorStorage::new("default", 1536));
    default_vector.initialize().await.unwrap();
    let registry = Arc::new(MemoryWorkspaceVectorRegistry::new(Arc::clone(
        &default_vector,
    )));

    let stale_config = WorkspaceVectorConfig::new(ws_id, 768);
    registry.get_or_create(stale_config).await.unwrap();
    assert_eq!(registry.get_dimension(&ws_id).await, Some(768));

    let storage = resolve_workspace_vector_storage(
        registry.as_ref(),
        Arc::clone(&default_vector),
        Some(workspace_service.as_ref()),
        1536,
        WorkspaceVectorResolveInput::new(Some(&ws_id.to_string()), "default"),
        WorkspaceVectorResolvePolicy::Strict,
    )
    .await
    .expect("resolve after dimension change");

    assert_eq!(storage.dimension(), 1536);
    assert_eq!(registry.get_dimension(&ws_id).await, Some(1536));
}

#[test]
fn spec045_query_and_ingest_share_dimension_mismatch_detector() {
    let msg = "Dimension mismatch for workspace: cached=768, requested=1536";
    assert!(is_dimension_mismatch_error(msg));
    assert!(msg.contains("Dimension mismatch") || msg.contains("cached="));
}
