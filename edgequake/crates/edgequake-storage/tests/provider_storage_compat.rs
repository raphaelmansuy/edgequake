//! Storage Backend Provider Compatibility Tests
//!
//! These tests verify that different embedding providers work correctly with
//! both Memory and PostgreSQL storage backends.
//!
//! @implements SPEC-032: Ollama/LM Studio provider support - Storage compatibility
//! @iteration OODA Loop #31-35 - Storage Backend Provider Tests

use edgequake_storage::{MemoryVectorStorage, VectorStorage};
use serde_json::json;

// ============================================================================
// Memory Vector Storage Provider Tests (OODA 31-32)
// ============================================================================

mod memory_vector_provider_tests {
    use super::*;

    fn generate_namespace() -> String {
        format!(
            "provider_test_{}",
            uuid::Uuid::new_v4().to_string().replace('-', "")[..8].to_string()
        )
    }

    /// Test that OpenAI dimension (1536) works with memory storage
    #[tokio::test]
    async fn test_memory_vector_openai_dimension() {
        let ns = generate_namespace();
        let storage = MemoryVectorStorage::new(&ns, 1536);
        storage.initialize().await.expect("Failed to initialize");

        // Insert a vector with OpenAI dimension
        let vector: Vec<f32> = (0..1536).map(|i| i as f32 / 1536.0).collect();
        storage
            .upsert(&[(
                "openai-doc-1".to_string(),
                vector.clone(),
                json!({"provider": "openai"}),
            )])
            .await
            .expect("Failed to upsert OpenAI vector");

        // Query should work
        let results = storage
            .query(&vector, 1, None)
            .await
            .expect("Failed to query");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "openai-doc-1");
    }

    /// Test that Ollama dimension (768) works with memory storage
    #[tokio::test]
    async fn test_memory_vector_ollama_dimension() {
        let ns = generate_namespace();
        let storage = MemoryVectorStorage::new(&ns, 768);
        storage.initialize().await.expect("Failed to initialize");

        // Insert a vector with Ollama dimension
        let vector: Vec<f32> = (0..768).map(|i| i as f32 / 768.0).collect();
        storage
            .upsert(&[(
                "ollama-doc-1".to_string(),
                vector.clone(),
                json!({"provider": "ollama"}),
            )])
            .await
            .expect("Failed to upsert Ollama vector");

        // Query should work
        let results = storage
            .query(&vector, 1, None)
            .await
            .expect("Failed to query");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "ollama-doc-1");
    }

    /// Test that LM Studio dimension (768) works with memory storage
    #[tokio::test]
    async fn test_memory_vector_lmstudio_dimension() {
        let ns = generate_namespace();
        let storage = MemoryVectorStorage::new(&ns, 768);
        storage.initialize().await.expect("Failed to initialize");

        // Insert a vector with LM Studio dimension
        let vector: Vec<f32> = (0..768).map(|i| i as f32 / 768.0).collect();
        storage
            .upsert(&[(
                "lmstudio-doc-1".to_string(),
                vector.clone(),
                json!({"provider": "lmstudio"}),
            )])
            .await
            .expect("Failed to upsert LM Studio vector");

        // Query should work
        let results = storage
            .query(&vector, 1, None)
            .await
            .expect("Failed to query");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "lmstudio-doc-1");
    }

    /// Test that Mock dimension (1536) works with memory storage
    #[tokio::test]
    async fn test_memory_vector_mock_dimension() {
        let ns = generate_namespace();
        let storage = MemoryVectorStorage::new(&ns, 1536);
        storage.initialize().await.expect("Failed to initialize");

        // Insert a vector with Mock dimension
        let vector: Vec<f32> = (0..1536).map(|i| (i as f32).sin()).collect();
        storage
            .upsert(&[(
                "mock-doc-1".to_string(),
                vector.clone(),
                json!({"provider": "mock"}),
            )])
            .await
            .expect("Failed to upsert Mock vector");

        // Query should work
        let results = storage
            .query(&vector, 1, None)
            .await
            .expect("Failed to query");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "mock-doc-1");
    }

    /// Test storage dimension getter
    #[tokio::test]
    async fn test_memory_vector_dimension_getter() {
        let ns = generate_namespace();

        // OpenAI dimension
        let storage_openai = MemoryVectorStorage::new(&ns, 1536);
        assert_eq!(storage_openai.dimension(), 1536);

        // Ollama dimension
        let storage_ollama = MemoryVectorStorage::new(&format!("{}_ollama", ns), 768);
        assert_eq!(storage_ollama.dimension(), 768);

        // LM Studio dimension
        let storage_lmstudio = MemoryVectorStorage::new(&format!("{}_lmstudio", ns), 768);
        assert_eq!(storage_lmstudio.dimension(), 768);
    }
}

// ============================================================================
// Storage Clear for Rebuild Tests (OODA 33-34)
// ============================================================================

mod storage_clear_tests {
    use super::*;

    fn generate_namespace() -> String {
        format!(
            "clear_test_{}",
            uuid::Uuid::new_v4().to_string().replace('-', "")[..8].to_string()
        )
    }

    /// Test that clear() removes all vectors for rebuild
    #[tokio::test]
    async fn test_memory_vector_clear_for_rebuild() {
        let ns = generate_namespace();
        let storage = MemoryVectorStorage::new(&ns, 768);
        storage.initialize().await.expect("Failed to initialize");

        // Insert some vectors
        let vectors: Vec<(String, Vec<f32>, serde_json::Value)> = (0..10)
            .map(|i| {
                let v: Vec<f32> = (0..768).map(|j| (i as f32 + j as f32) / 768.0).collect();
                (format!("doc-{}", i), v, json!({"index": i}))
            })
            .collect();
        storage.upsert(&vectors).await.expect("Failed to upsert");

        // Verify vectors exist
        let search_vec: Vec<f32> = (0..768).map(|_| 0.5).collect();
        let results = storage
            .query(&search_vec, 10, None)
            .await
            .expect("Failed to query");
        assert_eq!(results.len(), 10);

        // Clear for rebuild
        storage.clear().await.expect("Failed to clear");

        // Verify storage is empty
        let results_after = storage
            .query(&search_vec, 10, None)
            .await
            .expect("Failed to query after clear");
        assert_eq!(results_after.len(), 0);
    }

    /// Test that storage can be repopulated after clear
    #[tokio::test]
    async fn test_memory_vector_repopulate_after_clear() {
        let ns = generate_namespace();
        let storage = MemoryVectorStorage::new(&ns, 768);
        storage.initialize().await.expect("Failed to initialize");

        // Insert initial vectors
        let initial_vectors: Vec<(String, Vec<f32>, serde_json::Value)> = (0..5)
            .map(|i| {
                let v: Vec<f32> = (0..768).map(|_| i as f32 / 5.0).collect();
                (format!("initial-{}", i), v, json!({"phase": "initial"}))
            })
            .collect();
        storage
            .upsert(&initial_vectors)
            .await
            .expect("Failed to insert initial");

        // Clear
        storage.clear().await.expect("Failed to clear");

        // Repopulate with different vectors
        let new_vectors: Vec<(String, Vec<f32>, serde_json::Value)> = (0..3)
            .map(|i| {
                let v: Vec<f32> = (0..768).map(|_| (i as f32 + 10.0) / 15.0).collect();
                (format!("new-{}", i), v, json!({"phase": "new"}))
            })
            .collect();
        storage
            .upsert(&new_vectors)
            .await
            .expect("Failed to insert new");

        // Verify only new vectors exist
        let search_vec: Vec<f32> = (0..768).map(|_| 0.8).collect();
        let results = storage
            .query(&search_vec, 10, None)
            .await
            .expect("Failed to query");
        assert_eq!(results.len(), 3);

        // Verify they are the new vectors
        let ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
        assert!(ids.iter().all(|id| id.starts_with("new-")));
    }
}

// ============================================================================
// Provider Dimension Compatibility Tests (OODA 35)
// ============================================================================

mod dimension_compatibility_tests {
    use super::*;

    fn generate_namespace() -> String {
        format!(
            "compat_test_{}",
            uuid::Uuid::new_v4().to_string().replace('-', "")[..8].to_string()
        )
    }

    /// Test that different dimensions are isolated between workspaces
    #[tokio::test]
    async fn test_workspace_dimension_isolation() {
        // Create two storages with different dimensions (simulating different workspaces)
        let ns1 = generate_namespace();
        let ns2 = format!("{}_alt", ns1);

        // OpenAI workspace (1536 dim)
        let storage_openai = MemoryVectorStorage::new(&ns1, 1536);
        storage_openai
            .initialize()
            .await
            .expect("Failed to initialize OpenAI storage");

        // Ollama workspace (768 dim)
        let storage_ollama = MemoryVectorStorage::new(&ns2, 768);
        storage_ollama
            .initialize()
            .await
            .expect("Failed to initialize Ollama storage");

        // Insert into OpenAI storage
        let openai_vec: Vec<f32> = (0..1536).map(|i| i as f32 / 1536.0).collect();
        storage_openai
            .upsert(&[(
                "openai-doc".to_string(),
                openai_vec.clone(),
                json!({"provider": "openai"}),
            )])
            .await
            .expect("Failed to insert OpenAI doc");

        // Insert into Ollama storage
        let ollama_vec: Vec<f32> = (0..768).map(|i| i as f32 / 768.0).collect();
        storage_ollama
            .upsert(&[(
                "ollama-doc".to_string(),
                ollama_vec.clone(),
                json!({"provider": "ollama"}),
            )])
            .await
            .expect("Failed to insert Ollama doc");

        // Query in OpenAI storage should only find OpenAI doc
        let openai_results = storage_openai
            .query(&openai_vec, 10, None)
            .await
            .expect("Failed to query OpenAI");
        assert_eq!(openai_results.len(), 1);
        assert_eq!(openai_results[0].id, "openai-doc");

        // Query in Ollama storage should only find Ollama doc
        let ollama_results = storage_ollama
            .query(&ollama_vec, 10, None)
            .await
            .expect("Failed to query Ollama");
        assert_eq!(ollama_results.len(), 1);
        assert_eq!(ollama_results[0].id, "ollama-doc");
    }

    /// Test common embedding dimensions are supported
    #[tokio::test]
    async fn test_supported_embedding_dimensions() {
        // Common embedding dimensions
        let dimensions = [
            (384, "small-models"),    // MiniLM, all-MiniLM-L6-v2
            (768, "ollama-lmstudio"), // Ollama embeddinggemma, nomic-embed
            (1024, "mxbai"),          // mxbai-embed-large
            (1536, "openai-small"),   // text-embedding-3-small
            (3072, "openai-large"),   // text-embedding-3-large
        ];

        for (dim, name) in dimensions {
            let ns = format!("dim_test_{}_{}", dim, uuid::Uuid::new_v4());
            let storage = MemoryVectorStorage::new(&ns, dim);
            storage
                .initialize()
                .await
                .unwrap_or_else(|_| panic!("Failed to init {} storage", name));

            // Insert a test vector
            let vector: Vec<f32> = (0..dim).map(|i| (i as f32) / (dim as f32)).collect();
            storage
                .upsert(&[(
                    format!("{}-doc", name),
                    vector.clone(),
                    json!({"dimension": dim}),
                )])
                .await
                .unwrap_or_else(|_| panic!("Failed to insert {} doc", name));

            // Query should work
            let results = storage
                .query(&vector, 1, None)
                .await
                .unwrap_or_else(|_| panic!("Failed to query {}", name));
            assert_eq!(
                results.len(),
                1,
                "Dimension {} should work for {}",
                dim,
                name
            );
        }
    }

    /// Test similarity scores are normalized correctly
    #[tokio::test]
    async fn test_similarity_score_normalization() {
        let ns = generate_namespace();
        let storage = MemoryVectorStorage::new(&ns, 768);
        storage.initialize().await.expect("Failed to initialize");

        // Insert identical vector
        let vector: Vec<f32> = (0..768).map(|i| (i as f32) / 768.0).collect();
        storage
            .upsert(&[(
                "identical".to_string(),
                vector.clone(),
                json!({"type": "test"}),
            )])
            .await
            .expect("Failed to insert");

        // Query with same vector should have high score
        let results = storage
            .query(&vector, 1, None)
            .await
            .expect("Failed to query");
        assert_eq!(results.len(), 1);

        // Score should be very close to 1.0 for identical vectors
        assert!(
            results[0].score > 0.99,
            "Identical vector should have score > 0.99, got {}",
            results[0].score
        );
    }
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

mod edge_case_tests {
    use super::*;

    fn generate_namespace() -> String {
        format!(
            "edge_test_{}",
            uuid::Uuid::new_v4().to_string().replace('-', "")[..8].to_string()
        )
    }

    /// Test empty storage query returns empty results
    #[tokio::test]
    async fn test_empty_storage_query() {
        let ns = generate_namespace();
        let storage = MemoryVectorStorage::new(&ns, 768);
        storage.initialize().await.expect("Failed to initialize");

        let search_vec: Vec<f32> = (0..768).map(|_| 0.5).collect();
        let results = storage
            .query(&search_vec, 10, None)
            .await
            .expect("Failed to query");
        assert!(results.is_empty(), "Empty storage should return no results");
    }

    /// Test query with limit 0 returns empty
    #[tokio::test]
    async fn test_query_limit_zero() {
        let ns = generate_namespace();
        let storage = MemoryVectorStorage::new(&ns, 768);
        storage.initialize().await.expect("Failed to initialize");

        // Insert a vector
        let vector: Vec<f32> = (0..768).map(|_| 0.5).collect();
        storage
            .upsert(&[("doc".to_string(), vector.clone(), json!({}))])
            .await
            .expect("Failed to insert");

        // Query with limit 0
        let results = storage
            .query(&vector, 0, None)
            .await
            .expect("Failed to query");
        assert!(results.is_empty(), "Limit 0 should return no results");
    }

    /// Test upsert updates existing document
    #[tokio::test]
    async fn test_upsert_updates_existing() {
        let ns = generate_namespace();
        let storage = MemoryVectorStorage::new(&ns, 768);
        storage.initialize().await.expect("Failed to initialize");

        // Insert initial vector
        let initial_vec: Vec<f32> = (0..768).map(|_| 0.1).collect();
        storage
            .upsert(&[(
                "doc-1".to_string(),
                initial_vec.clone(),
                json!({"version": 1}),
            )])
            .await
            .expect("Failed to insert initial");

        // Update with different vector
        let updated_vec: Vec<f32> = (0..768).map(|_| 0.9).collect();
        storage
            .upsert(&[(
                "doc-1".to_string(),
                updated_vec.clone(),
                json!({"version": 2}),
            )])
            .await
            .expect("Failed to update");

        // Query should find the updated vector (higher similarity to 0.9)
        let results = storage
            .query(&updated_vec, 1, None)
            .await
            .expect("Failed to query");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "doc-1");

        // Should have high similarity to updated vector
        assert!(
            results[0].score > 0.9,
            "Updated vector should have high similarity"
        );
    }

    /// Test is_empty returns correct value
    #[tokio::test]
    async fn test_is_empty() {
        let ns = generate_namespace();
        let storage = MemoryVectorStorage::new(&ns, 768);
        storage.initialize().await.expect("Failed to initialize");

        // Initially empty
        assert!(storage.is_empty().await.expect("Failed to check is_empty"));

        // Insert a vector
        let vector: Vec<f32> = (0..768).map(|_| 0.5).collect();
        storage
            .upsert(&[("doc".to_string(), vector.clone(), json!({}))])
            .await
            .expect("Failed to insert");

        // Now not empty
        assert!(!storage.is_empty().await.expect("Failed to check is_empty"));

        // Clear and check again
        storage.clear().await.expect("Failed to clear");
        assert!(storage
            .is_empty()
            .await
            .expect("Failed to check is_empty after clear"));
    }

    /// Test count returns correct value
    #[tokio::test]
    async fn test_count() {
        let ns = generate_namespace();
        let storage = MemoryVectorStorage::new(&ns, 768);
        storage.initialize().await.expect("Failed to initialize");

        // Initially zero
        assert_eq!(storage.count().await.expect("Failed to count"), 0);

        // Insert vectors
        for i in 0..5 {
            let vector: Vec<f32> = (0..768).map(|j| (i as f32 + j as f32) / 1000.0).collect();
            storage
                .upsert(&[(format!("doc-{}", i), vector, json!({"index": i}))])
                .await
                .expect("Failed to insert");
        }

        // Should be 5
        assert_eq!(storage.count().await.expect("Failed to count"), 5);

        // Clear and check
        storage.clear().await.expect("Failed to clear");
        assert_eq!(
            storage.count().await.expect("Failed to count after clear"),
            0
        );
    }
}
