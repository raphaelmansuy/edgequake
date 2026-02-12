#![cfg(feature = "pipeline")]

//! End-to-End Multi-Tenancy Isolation Tests
//!
//! These tests verify that the TenantRAGManager correctly isolates data
//! between different tenants and workspaces (knowledge bases).

use std::sync::Arc;
use tempfile::tempdir;

use edgequake_core::{
    tenant_manager::{InMemoryTenantService, TenantConfig, TenantRAGManager},
    EdgeQuakeConfig,
};
use edgequake_llm::MockProvider;
use edgequake_storage::{MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage};

/// Create a smart mock provider that returns valid extraction JSON.
async fn create_smart_mock_provider() -> Arc<MockProvider> {
    let provider = Arc::new(MockProvider::new());

    // Add valid extraction JSON response for document A
    let extraction_a = r#"{
  "entities": [
    {"name": "PROJECT_ALPHA", "type": "PROJECT", "description": "Secret project for Tenant A"}
  ],
  "relationships": []
}"#;

    // Add valid extraction JSON response for document B
    let extraction_b = r#"{
  "entities": [
    {"name": "PROJECT_BETA", "type": "PROJECT", "description": "Secret project for Tenant B"}
  ],
  "relationships": []
}"#;

    provider.add_response(extraction_a).await;
    provider.add_response(extraction_b).await;
    provider
}

/// Tests tenant isolation in E2E scenario.
/// Currently ignored due to insufficient mock responses for multi-tenant document insertion.
/// TODO: Add more mock responses to cover all LLM calls during document processing.
#[tokio::test]
#[ignore = "Mock provider needs more responses for multi-tenant document processing"]
async fn test_tenant_isolation_e2e() {
    // 1. Setup environment
    let base_dir = tempdir().unwrap();
    let tenant_service = Arc::new(InMemoryTenantService::new());

    // Register tenants
    tenant_service
        .add_tenant(TenantConfig::new("tenant_a"))
        .await;
    tenant_service
        .add_tenant(TenantConfig::new("tenant_b"))
        .await;

    // Grant access to a test user
    tenant_service.grant_access("user_1", "tenant_a").await;
    tenant_service.grant_access("user_1", "tenant_b").await;

    let mock_llm = create_smart_mock_provider().await;

    let config = EdgeQuakeConfig::default();

    // Shared storage for testing filtering logic
    let shared_kv = Arc::new(MemoryKVStorage::new("shared_kv"));
    let shared_vector = Arc::new(MemoryVectorStorage::new("shared_vector", 1536));
    let shared_graph = Arc::new(MemoryGraphStorage::new("shared_graph"));

    let manager = TenantRAGManager::new(base_dir.path(), tenant_service.clone(), config, 10)
        .with_auth_required(true);

    // 2. Tenant A uploads a document
    let instance_a = manager
        .get_instance("tenant_a", "default", Some("user_1"))
        .await
        .unwrap();
    {
        let mut rag = instance_a.write().await;

        // Use shared storage
        rag.set_storage_backends(
            shared_kv.clone(),
            shared_vector.clone(),
            shared_graph.clone(),
        );
        rag.set_providers(mock_llm.clone(), mock_llm.clone());

        rag.initialize().await.expect("Failed to initialize RAG A");

        rag.insert(
            "This is a secret document for Project Alpha belonging to Tenant A.",
            Some("doc_a"),
        )
        .await
        .expect("Failed to upload doc A");
    }

    // 3. Tenant B uploads a different document
    let instance_b = manager
        .get_instance("tenant_b", "default", Some("user_1"))
        .await
        .unwrap();
    {
        let mut rag = instance_b.write().await;

        // Use SAME shared storage
        rag.set_storage_backends(
            shared_kv.clone(),
            shared_vector.clone(),
            shared_graph.clone(),
        );
        rag.set_providers(mock_llm.clone(), mock_llm.clone());

        rag.initialize().await.expect("Failed to initialize RAG B");

        rag.insert(
            "This is a secret document for Project Beta belonging to Tenant B.",
            Some("doc_b"),
        )
        .await
        .expect("Failed to upload doc B");
    }

    // 4. Verify Isolation: Tenant A queries for its content
    {
        let rag = instance_a.read().await;
        let mut params = edgequake_core::types::QueryParams::default();
        params.mode = edgequake_core::types::QueryMode::Mix;

        let result = rag
            .query("What is Project Alpha?", Some(params))
            .await
            .unwrap();

        let entities: Vec<String> = result
            .context
            .entities
            .iter()
            .map(|e| e.name.clone())
            .collect();
        println!("Tenant A retrieved entities: {:?}", entities);
        assert!(
            entities.contains(&"PROJECTALPHA".to_string()),
            "Tenant A should see its own project"
        );
        assert!(
            !entities.contains(&"PROJECTBETA".to_string()),
            "Tenant A should NOT see Tenant B's project"
        );

        // Verify chunk isolation
        let chunks: Vec<String> = result
            .context
            .chunks
            .iter()
            .map(|c| c.content.clone())
            .collect();
        println!("Tenant A retrieved chunks: {:?}", chunks);
        assert!(
            chunks.iter().any(|c| c.contains("Project Alpha")),
            "Tenant A should see its own chunks"
        );
        assert!(
            !chunks.iter().any(|c| c.contains("Project Beta")),
            "Tenant A should NOT see Tenant B's chunks"
        );
    }

    // 5. Verify Isolation: Tenant B queries for its content
    {
        let rag = instance_b.read().await;
        let mut params = edgequake_core::types::QueryParams::default();
        params.mode = edgequake_core::types::QueryMode::Mix;

        let result = rag
            .query("What is Project Beta?", Some(params))
            .await
            .unwrap();

        let entities: Vec<String> = result
            .context
            .entities
            .iter()
            .map(|e| e.name.clone())
            .collect();
        println!("Tenant B retrieved entities: {:?}", entities);
        assert!(
            entities.contains(&"PROJECTBETA".to_string()),
            "Tenant B should see its own project"
        );
        assert!(
            !entities.contains(&"PROJECTALPHA".to_string()),
            "Tenant B should NOT see Tenant A's project"
        );

        // Verify chunk isolation
        let chunks: Vec<String> = result
            .context
            .chunks
            .iter()
            .map(|c| c.content.clone())
            .collect();
        println!("Tenant B retrieved chunks: {:?}", chunks);
        assert!(
            chunks.iter().any(|c| c.contains("Project Beta")),
            "Tenant B should see its own chunks"
        );
        assert!(
            !chunks.iter().any(|c| c.contains("Project Alpha")),
            "Tenant B should NOT see Tenant A's chunks"
        );
    }

    // 6. Verify Security: User without access to Tenant C
    tenant_service
        .add_tenant(TenantConfig::new("tenant_c"))
        .await;
    let result = manager
        .get_instance("tenant_c", "default", Some("user_1"))
        .await;
    assert!(result.is_err(), "User 1 should not have access to Tenant C");
}
