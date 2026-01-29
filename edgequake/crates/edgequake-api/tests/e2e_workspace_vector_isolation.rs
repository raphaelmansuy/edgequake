//! E2E test for workspace vector storage isolation (SPEC-033)
//!
//! This test validates that:
//! 1. Each workspace gets its own vector storage table
//! 2. Workspaces with different embedding dimensions are isolated
//! 3. Document processing uses workspace-specific vector storage
//! 4. Queries use workspace-specific vector storage

use std::sync::Arc;
use uuid::Uuid;

use edgequake_storage::{
    adapters::memory::{MemoryVectorStorage, MemoryWorkspaceVectorRegistry},
    traits::{WorkspaceVectorConfig, WorkspaceVectorRegistry},
};

/// Create a test registry with a default storage
fn create_test_registry(default_dim: usize) -> MemoryWorkspaceVectorRegistry {
    let default_storage = Arc::new(MemoryVectorStorage::new("default", default_dim));
    MemoryWorkspaceVectorRegistry::new(default_storage)
}

/// Test that MemoryWorkspaceVectorRegistry creates isolated storage per workspace
#[tokio::test]
async fn test_memory_workspace_vector_isolation() {
    let registry = create_test_registry(1536);

    // Create two workspaces with different dimensions
    let ws_a = Uuid::new_v4();
    let ws_b = Uuid::new_v4();

    let config_a = WorkspaceVectorConfig {
        workspace_id: ws_a,
        dimension: 1536, // OpenAI dimension
        namespace: "test".to_string(),
    };

    let config_b = WorkspaceVectorConfig {
        workspace_id: ws_b,
        dimension: 768, // Ollama dimension
        namespace: "test".to_string(),
    };

    // Get or create storage for each workspace
    let storage_a = registry.get_or_create(config_a.clone()).await.unwrap();
    let storage_b = registry.get_or_create(config_b.clone()).await.unwrap();

    // Verify dimensions are correct
    assert_eq!(
        storage_a.dimension(),
        1536,
        "Workspace A should have 1536 dimensions"
    );
    assert_eq!(
        storage_b.dimension(),
        768,
        "Workspace B should have 768 dimensions"
    );

    // Store vectors in workspace A
    let vector_a = vec![0.1_f32; 1536];
    storage_a
        .upsert(&[("doc_a".to_string(), vector_a.clone(), serde_json::json!({}))])
        .await
        .unwrap();

    // Store vectors in workspace B
    let vector_b = vec![0.2_f32; 768];
    storage_b
        .upsert(&[("doc_b".to_string(), vector_b.clone(), serde_json::json!({}))])
        .await
        .unwrap();

    // Verify isolation - workspace A should only see its vector
    let search_a = vec![0.1_f32; 1536];
    let results_a = storage_a.query(&search_a, 10, None).await.unwrap();
    assert_eq!(results_a.len(), 1, "Workspace A should have 1 result");
    assert_eq!(results_a[0].id, "doc_a", "Workspace A should find doc_a");

    // Verify isolation - workspace B should only see its vector
    let search_b = vec![0.2_f32; 768];
    let results_b = storage_b.query(&search_b, 10, None).await.unwrap();
    assert_eq!(results_b.len(), 1, "Workspace B should have 1 result");
    assert_eq!(results_b[0].id, "doc_b", "Workspace B should find doc_b");

    // Verify we can get the same storage back
    let storage_a2 = registry.get(&ws_a).await;
    assert!(
        storage_a2.is_some(),
        "Should be able to get workspace A storage"
    );

    let storage_a2 = storage_a2.unwrap();
    let results_a2 = storage_a2.query(&search_a, 10, None).await.unwrap();
    assert_eq!(
        results_a2.len(),
        1,
        "Should still find doc_a in cached storage"
    );

    println!("✅ Memory workspace vector isolation test passed!");
}

/// Test that different workspaces can have incompatible dimensions
#[tokio::test]
async fn test_dimension_independence() {
    let registry = create_test_registry(1536);

    // Create workspaces with various common embedding dimensions
    let dimensions = vec![
        (Uuid::new_v4(), 384),  // MiniLM
        (Uuid::new_v4(), 512),  // Some BERT models
        (Uuid::new_v4(), 768),  // Ollama nomic-embed-text, BGE
        (Uuid::new_v4(), 1024), // Some larger models
        (Uuid::new_v4(), 1536), // OpenAI text-embedding-3-small
        (Uuid::new_v4(), 3072), // OpenAI text-embedding-3-large
    ];

    let mut storages = Vec::new();

    for (ws_id, dim) in &dimensions {
        let config = WorkspaceVectorConfig {
            workspace_id: *ws_id,
            dimension: *dim,
            namespace: "test".to_string(),
        };
        let storage = registry.get_or_create(config).await.unwrap();
        assert_eq!(storage.dimension(), *dim, "Dimension should match config");
        storages.push((*ws_id, *dim, storage));
    }

    // Store and retrieve a vector in each workspace
    for (ws_id, dim, storage) in &storages {
        let vector = vec![1.0 / (*dim as f32).sqrt(); *dim]; // Normalized vector
        let doc_id = format!("doc_{}", ws_id);
        storage
            .upsert(&[(doc_id.clone(), vector.clone(), serde_json::json!({}))])
            .await
            .unwrap();

        let results = storage.query(&vector, 10, None).await.unwrap();
        assert_eq!(results.len(), 1, "Should find exactly 1 result");
        assert_eq!(results[0].id, doc_id, "Should find the correct document");
        // Cosine similarity of a vector with itself should be 1.0
        assert!(
            (results[0].score - 1.0).abs() < 0.01,
            "Self-similarity should be ~1.0"
        );
    }

    println!(
        "✅ Dimension independence test passed with {} different dimensions!",
        dimensions.len()
    );
}

/// Test that the registry caches and reuses storage instances
#[tokio::test]
async fn test_storage_caching() {
    let registry = create_test_registry(1536);
    let ws_id = Uuid::new_v4();

    let config = WorkspaceVectorConfig {
        workspace_id: ws_id,
        dimension: 1536,
        namespace: "test".to_string(),
    };

    // Create storage first time
    let storage1 = registry.get_or_create(config.clone()).await.unwrap();

    // Store something
    let vector = vec![0.5_f32; 1536];
    storage1
        .upsert(&[(
            "test_doc".to_string(),
            vector.clone(),
            serde_json::json!({}),
        )])
        .await
        .unwrap();

    // Get storage second time - should be the same instance
    let storage2 = registry.get_or_create(config).await.unwrap();

    // Verify it's the same storage (has the same data)
    let results = storage2.query(&vector, 10, None).await.unwrap();
    assert_eq!(
        results.len(),
        1,
        "Should find the document stored in first instance"
    );
    assert_eq!(
        results[0].id, "test_doc",
        "Should find the correct document"
    );

    println!("✅ Storage caching test passed!");
}

/// Test that default_storage returns a valid fallback
#[tokio::test]
async fn test_default_storage() {
    let registry = create_test_registry(1536);

    let default_storage = registry.default_storage();

    // Default should have some sensible dimension (e.g., 1536 for OpenAI)
    let dim = default_storage.dimension();
    assert!(dim > 0, "Default storage should have positive dimension");

    // Should be able to use the default storage
    let vector = vec![0.1_f32; dim];
    default_storage
        .upsert(&[(
            "default_doc".to_string(),
            vector.clone(),
            serde_json::json!({}),
        )])
        .await
        .unwrap();

    let results = default_storage.query(&vector, 10, None).await.unwrap();
    assert_eq!(results.len(), 1, "Should find document in default storage");

    println!("✅ Default storage test passed with dimension {}", dim);
}

/// Test workspace cascade delete clears all storage (SPEC-028)
///
/// This test verifies that when a workspace is deleted:
/// 1. Vector storage is cleared
/// 2. Registry evicts the workspace
/// 3. Subsequent queries return empty results
#[tokio::test]
async fn test_workspace_cascade_delete_clears_vectors() {
    let default_storage = Arc::new(MemoryVectorStorage::new("default", 1536));
    let registry = MemoryWorkspaceVectorRegistry::new(default_storage.clone());

    let workspace_id = Uuid::new_v4();

    let config = WorkspaceVectorConfig {
        workspace_id,
        dimension: 1536,
        namespace: "test".to_string(),
    };

    // Create and populate storage
    let storage = registry.get_or_create(config.clone()).await.unwrap();
    storage.initialize().await.unwrap();

    // Add 10 vectors
    for i in 0..10 {
        let mut vector = vec![0.0_f32; 1536];
        vector[i] = 1.0;
        let doc_id = format!("doc_{}", i);
        storage
            .upsert(&[(
                doc_id,
                vector,
                serde_json::json!({"workspace_id": workspace_id.to_string()}),
            )])
            .await
            .unwrap();
    }

    // Verify vectors exist
    let search_vec = vec![1.0_f32 / 1536.0_f32.sqrt(); 1536];
    let results = storage.query(&search_vec, 100, None).await.unwrap();
    assert_eq!(results.len(), 10, "Should have 10 vectors before delete");

    // SPEC-028: Cascade delete simulation - clear and evict
    storage.clear().await.unwrap();
    registry.evict(&workspace_id).await;

    // Verify vectors are gone
    // Note: After evict, we need to get_or_create again to get fresh storage
    let storage_after = registry.get_or_create(config).await.unwrap();
    storage_after.initialize().await.unwrap();
    let results_after = storage_after.query(&search_vec, 100, None).await.unwrap();
    assert_eq!(
        results_after.len(),
        0,
        "Should have 0 vectors after cascade delete"
    );

    println!("✅ SPEC-028: Workspace cascade delete clears all vectors!");
}

/// Test workspace isolation under concurrent access
#[tokio::test]
async fn test_concurrent_workspace_access() {
    let default_storage = Arc::new(MemoryVectorStorage::new("default", 1536));
    let registry = Arc::new(MemoryWorkspaceVectorRegistry::new(default_storage));
    let ws_count = 10;
    let vectors_per_workspace = 100;

    let mut handles = Vec::new();

    for i in 0..ws_count {
        let registry = Arc::clone(&registry);
        let handle = tokio::spawn(async move {
            let ws_id = Uuid::new_v4();
            let dim = 768 + (i * 128); // Different dimension per workspace

            let config = WorkspaceVectorConfig {
                workspace_id: ws_id,
                dimension: dim,
                namespace: "concurrent".to_string(),
            };

            let storage = registry.get_or_create(config).await.unwrap();
            assert_eq!(storage.dimension(), dim);

            // Store multiple vectors
            for j in 0..vectors_per_workspace {
                let mut vector = vec![0.0_f32; dim];
                vector[j % dim] = 1.0; // Sparse vector
                let doc_id = format!("ws_{}_doc_{}", i, j);
                storage
                    .upsert(&[(doc_id, vector, serde_json::json!({}))])
                    .await
                    .unwrap();
            }

            // Verify we can find vectors
            let search_vec = vec![1.0_f32 / (dim as f32).sqrt(); dim];
            let results = storage
                .query(&search_vec, vectors_per_workspace, None)
                .await
                .unwrap();

            (ws_id, dim, results.len())
        });
        handles.push(handle);
    }

    let results: Vec<(Uuid, usize, usize)> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    for (ws_id, dim, count) in &results {
        assert_eq!(
            *count, vectors_per_workspace,
            "Workspace {} with dim {} should have {} vectors",
            ws_id, dim, vectors_per_workspace
        );
    }

    println!(
        "✅ Concurrent workspace access test passed with {} workspaces!",
        ws_count
    );
}
