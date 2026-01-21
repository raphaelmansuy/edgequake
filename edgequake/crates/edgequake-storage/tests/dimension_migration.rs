//! Integration tests for vector dimension migration (OODA-228).
//!
//! These tests verify that switching embedding providers (and thus dimensions)
//! works correctly by dropping and recreating vector tables.
//!
//! @implements OODA-228: Fix vector dimension mismatch after provider switch

use edgequake_storage::adapters::memory::MemoryVectorStorage;
use edgequake_storage::traits::{VectorStorage, WorkspaceVectorConfig, WorkspaceVectorRegistry};
use uuid::Uuid;

/// Test that MemoryVectorStorage correctly handles dimension isolation.
///
/// This simulates the scenario where:
/// 1. User creates workspace with OpenAI (1536 dimensions)
/// 2. User switches to Ollama (768 dimensions) and rebuilds
/// 3. Query with 768-dim embedding should work
#[tokio::test]
async fn test_memory_storage_dimension_isolation() {
    // Create storage with OpenAI dimension (1536)
    let storage = MemoryVectorStorage::new("test_workspace", 1536);
    storage.initialize().await.unwrap();

    // Store some vectors with 1536 dimensions
    let embedding_1536 = vec![0.1; 1536];
    storage
        .upsert(&[(
            "doc1".to_string(),
            embedding_1536.clone(),
            serde_json::json!({"title": "test doc"}),
        )])
        .await
        .unwrap();

    // Verify the dimension is 1536
    assert_eq!(storage.dimension(), 1536);

    // Verify storage is not empty
    assert_eq!(storage.count().await.unwrap(), 1);

    // Query with matching dimension should work
    let results = storage.query(&embedding_1536, 10, None).await.unwrap();
    assert_eq!(results.len(), 1);
}

/// Test that WorkspaceVectorConfig generates correct table names.
#[test]
fn test_workspace_vector_config_table_names() {
    let workspace_id = Uuid::parse_str("4e32a055-9722-40f9-b03e-ade870b07604").unwrap();

    // OpenAI dimension
    let config_openai = WorkspaceVectorConfig::new(workspace_id, 1536);
    assert_eq!(config_openai.table_name(), "eq_default_ws_4e32a055_vectors");
    assert_eq!(config_openai.dimension, 1536);

    // Ollama dimension
    let config_ollama = WorkspaceVectorConfig::new(workspace_id, 768);
    assert_eq!(config_ollama.table_name(), "eq_default_ws_4e32a055_vectors");
    assert_eq!(config_ollama.dimension, 768);

    // Same workspace ID generates same table name regardless of dimension
    // This is important: the table needs to be recreated, not a new one created
    assert_eq!(config_openai.table_name(), config_ollama.table_name());
}

/// Test dimension mismatch detection in WorkspaceVectorRegistry.
///
/// Note: MemoryWorkspaceVectorRegistry creates fresh storage on each get_or_create
/// and doesn't check for dimension mismatch like PgWorkspaceVectorRegistry.
/// This test verifies the eviction workflow works correctly.
#[tokio::test]
async fn test_workspace_registry_eviction_workflow() {
    use edgequake_storage::adapters::memory::MemoryWorkspaceVectorRegistry;

    // Create registry with default 1536 dimension
    let default_storage = std::sync::Arc::new(MemoryVectorStorage::new("default", 1536));
    let registry = MemoryWorkspaceVectorRegistry::new(default_storage.clone());

    let workspace_id = Uuid::new_v4();

    // First: create workspace storage with 1536 dimensions
    let config_1536 = WorkspaceVectorConfig::new(workspace_id, 1536);
    let storage_1536 = registry.get_or_create(config_1536).await.unwrap();
    assert_eq!(storage_1536.dimension(), 1536);

    // Store some data
    storage_1536.initialize().await.unwrap();
    let embedding_1536 = vec![0.1; 1536];
    storage_1536
        .upsert(&[(
            "doc1".to_string(),
            embedding_1536.clone(),
            serde_json::json!({"title": "test"}),
        )])
        .await
        .unwrap();

    // Without eviction, getting with same workspace_id returns cached storage
    let config_768 = WorkspaceVectorConfig::new(workspace_id, 768);
    let storage_cached = registry.get_or_create(config_768).await.unwrap();
    // Still returns cached 1536-dim storage (memory impl doesn't validate dimension)
    assert_eq!(storage_cached.dimension(), 1536);

    // Evict the cache
    registry.evict(&workspace_id).await;

    // After eviction, should get new storage with correct dimension
    let config_768_retry = WorkspaceVectorConfig::new(workspace_id, 768);
    let storage_768 = registry.get_or_create(config_768_retry).await.unwrap();
    assert_eq!(storage_768.dimension(), 768);
}

/// Test that clearing workspace vectors works correctly.
#[tokio::test]
async fn test_clear_workspace_after_dimension_change() {
    let storage = MemoryVectorStorage::new("test_clear", 1536);
    storage.initialize().await.unwrap();

    // Add some vectors
    let embedding = vec![0.1; 1536];
    storage
        .upsert(&[(
            "doc1".to_string(),
            embedding.clone(),
            serde_json::json!({"workspace_id": "ws1"}),
        )])
        .await
        .unwrap();

    assert_eq!(storage.count().await.unwrap(), 1);

    // Clear all vectors
    storage.clear().await.unwrap();

    // Verify storage is empty
    assert_eq!(storage.count().await.unwrap(), 0);

    // Now we could reinitialize with different dimension
    // (In production, this would be a new PgVectorStorage instance)
}

/// Test the OODA-228 scenario end-to-end with memory storage.
///
/// Scenario:
/// 1. Create workspace with OpenAI (1536 dim)
/// 2. Ingest documents (store embeddings)
/// 3. Switch to Ollama (768 dim)
/// 4. Rebuild embeddings (clears old, stores new with 768 dim)
/// 5. Query with 768-dim embedding should work
#[tokio::test]
async fn test_provider_switch_workflow() {
    use edgequake_storage::adapters::memory::MemoryWorkspaceVectorRegistry;

    // Step 1: Create registry and workspace storage with OpenAI (1536)
    let default_storage = std::sync::Arc::new(MemoryVectorStorage::new("default", 1536));
    let registry = MemoryWorkspaceVectorRegistry::new(default_storage.clone());

    let workspace_id = Uuid::new_v4();
    let config_openai = WorkspaceVectorConfig::new(workspace_id, 1536);
    let storage_openai = registry.get_or_create(config_openai).await.unwrap();

    // Step 2: Ingest documents with 1536-dim embeddings
    storage_openai.initialize().await.unwrap();
    let embedding_openai = vec![0.5; 1536];
    storage_openai
        .upsert(&[
            (
                "doc1".to_string(),
                embedding_openai.clone(),
                serde_json::json!({"title": "Doc 1", "workspace_id": workspace_id.to_string()}),
            ),
            (
                "doc2".to_string(),
                embedding_openai.clone(),
                serde_json::json!({"title": "Doc 2", "workspace_id": workspace_id.to_string()}),
            ),
        ])
        .await
        .unwrap();

    assert_eq!(storage_openai.count().await.unwrap(), 2);

    // Query works with 1536 dimensions
    let results = storage_openai
        .query(&embedding_openai, 10, None)
        .await
        .unwrap();
    assert_eq!(results.len(), 2);

    // Step 3: User switches to Ollama (768 dimensions)
    // This triggers rebuild_embeddings API which:
    // 3a. Evicts the cache
    registry.evict(&workspace_id).await;

    // 3b. Gets storage with new dimension (this creates a fresh storage instance)
    let config_ollama = WorkspaceVectorConfig::new(workspace_id, 768);
    let storage_ollama = registry.get_or_create(config_ollama).await.unwrap();

    // Step 4: Rebuild re-ingests documents with 768-dim embeddings
    storage_ollama.initialize().await.unwrap();
    let embedding_ollama = vec![0.5; 768];
    storage_ollama
        .upsert(&[
            (
                "doc1".to_string(),
                embedding_ollama.clone(),
                serde_json::json!({"title": "Doc 1", "workspace_id": workspace_id.to_string()}),
            ),
            (
                "doc2".to_string(),
                embedding_ollama.clone(),
                serde_json::json!({"title": "Doc 2", "workspace_id": workspace_id.to_string()}),
            ),
        ])
        .await
        .unwrap();

    // Step 5: Query with 768-dim embedding works
    let results = storage_ollama
        .query(&embedding_ollama, 10, None)
        .await
        .unwrap();
    assert_eq!(results.len(), 2);

    // Verify dimension is correct
    assert_eq!(storage_ollama.dimension(), 768);
}
