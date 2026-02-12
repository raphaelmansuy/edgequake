#![cfg(feature = "pipeline")]

//! Edge Case and Error Handling Tests
//!
//! These tests verify robust behavior under edge conditions including:
//! - Provider unavailability
//! - Network errors
//! - Concurrent operations
//! - Invalid configurations
//!
//! @implements SPEC-032: Ollama/LM Studio provider support - Edge cases
//! @iteration OODA Loop #36-40 - Edge Case Tests

use edgequake_llm::{EmbeddingProvider, MockProvider, OllamaProvider};

// ============================================================================
// Provider Unavailability Tests (OODA 36)
// ============================================================================

mod provider_unavailability_tests {
    use super::*;

    /// Test Ollama provider returns error when server is not running
    #[tokio::test]
    async fn test_ollama_unavailable_returns_error() {
        // Connect to a non-existent server using builder pattern
        let provider = OllamaProvider::builder()
            .host("http://127.0.0.1:59999")
            .embedding_model("nomic-embed-text")
            .build()
            .expect("Builder should succeed");

        // Attempting to generate embeddings should fail gracefully
        let result = provider.embed(&["test text".to_string()]).await;
        assert!(
            result.is_err(),
            "Should return error when Ollama is unavailable"
        );

        // Error message should be informative
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("connect")
                || err.contains("error")
                || err.contains("failed")
                || err.contains("Error"),
            "Error should indicate connection failure: {}",
            err
        );
    }

    /// Test that mock provider always works (for fallback testing)
    #[tokio::test]
    async fn test_mock_provider_always_available() {
        let provider = MockProvider::new();

        // Mock should always succeed
        let result = provider.embed(&["test text".to_string()]).await;
        assert!(result.is_ok(), "Mock provider should always work");

        // Should return valid embedding
        let embeddings = result.unwrap();
        assert!(!embeddings.is_empty(), "Embeddings should not be empty");
        assert!(
            !embeddings[0].is_empty(),
            "First embedding should not be empty"
        );
    }
}

// ============================================================================
// Invalid Configuration Tests (OODA 37)
// ============================================================================

mod invalid_config_tests {
    use super::*;

    /// Test that empty model name is handled
    #[tokio::test]
    async fn test_empty_model_name_handling() {
        // This should work - the provider just uses empty string as model
        let provider = OllamaProvider::builder()
            .host("http://127.0.0.1:11434")
            .embedding_model("")
            .build()
            .expect("Builder should succeed even with empty model");

        // The provider should be constructable even with empty model
        // Actual error would occur during embedding call
        let result = provider.embed(&["test".to_string()]).await;

        // Should fail because model is empty (or connection fails, either is acceptable)
        assert!(
            result.is_err(),
            "Empty model or unavailable server should cause embedding to fail"
        );
    }

    /// Test that invalid URL is handled gracefully
    #[tokio::test]
    async fn test_invalid_url_handling() {
        // Create provider with invalid URL
        let provider = OllamaProvider::builder()
            .host("not-a-valid-url")
            .embedding_model("nomic-embed-text")
            .build()
            .expect("Builder should succeed");

        // Embedding call should fail gracefully
        let result = provider.embed(&["test".to_string()]).await;
        assert!(result.is_err(), "Invalid URL should cause error");
    }
}

// ============================================================================
// Dimension Mismatch Detection Tests (OODA 38)
// ============================================================================

mod dimension_mismatch_tests {
    use edgequake_storage::{MemoryVectorStorage, VectorStorage};

    fn generate_namespace() -> String {
        format!(
            "dim_mismatch_{}",
            uuid::Uuid::new_v4().to_string().replace('-', "")[..8].to_string()
        )
    }

    /// Test that storage correctly reports its dimension
    #[tokio::test]
    async fn test_storage_dimension_reporting() {
        let ns = generate_namespace();

        // Create storages with different dimensions
        let storage_768 = MemoryVectorStorage::new(&ns, 768);
        let storage_1536 = MemoryVectorStorage::new(&format!("{}_1536", ns), 1536);

        assert_eq!(storage_768.dimension(), 768);
        assert_eq!(storage_1536.dimension(), 1536);
    }

    /// Test that embedding dimension can be validated before storage
    #[tokio::test]
    async fn test_embedding_dimension_validation() {
        let ns = generate_namespace();
        let expected_dim = 768;
        let storage = MemoryVectorStorage::new(&ns, expected_dim);
        storage.initialize().await.expect("Failed to initialize");

        // Correct dimension embedding
        let correct_vec: Vec<f32> = (0..768).map(|i| i as f32 / 768.0).collect();
        assert_eq!(correct_vec.len(), expected_dim);

        // The storage dimension matches
        assert_eq!(storage.dimension(), correct_vec.len());
    }

    /// Test dimension validation helper
    #[tokio::test]
    async fn test_validate_embedding_dimension() {
        fn validate_dimension(embedding: &[f32], expected: usize) -> Result<(), String> {
            if embedding.len() != expected {
                return Err(format!(
                    "Dimension mismatch: expected {}, got {}",
                    expected,
                    embedding.len()
                ));
            }
            Ok(())
        }

        // Valid cases
        let vec_768: Vec<f32> = (0..768).map(|_| 0.5).collect();
        let vec_1536: Vec<f32> = (0..1536).map(|_| 0.5).collect();

        assert!(validate_dimension(&vec_768, 768).is_ok());
        assert!(validate_dimension(&vec_1536, 1536).is_ok());

        // Invalid case
        assert!(validate_dimension(&vec_768, 1536).is_err());
        assert!(validate_dimension(&vec_1536, 768).is_err());
    }
}

// ============================================================================
// Concurrent Access Tests (OODA 39)
// ============================================================================

mod concurrent_access_tests {
    use edgequake_storage::{MemoryVectorStorage, VectorStorage};
    use serde_json::json;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn generate_namespace() -> String {
        format!(
            "concurrent_{}",
            uuid::Uuid::new_v4().to_string().replace('-', "")[..8].to_string()
        )
    }

    /// Test concurrent reads from storage
    #[tokio::test]
    async fn test_concurrent_reads() {
        let ns = generate_namespace();
        let storage = Arc::new(MemoryVectorStorage::new(&ns, 768));
        storage.initialize().await.expect("Failed to initialize");

        // Insert some vectors
        let vectors: Vec<(String, Vec<f32>, serde_json::Value)> = (0..10)
            .map(|i| {
                let v: Vec<f32> = (0..768).map(|j| (i as f32 + j as f32) / 1000.0).collect();
                (format!("doc-{}", i), v, json!({"index": i}))
            })
            .collect();
        storage.upsert(&vectors).await.expect("Failed to insert");

        // Spawn multiple concurrent read tasks
        let mut handles = vec![];
        for _ in 0..5 {
            let storage_clone = Arc::clone(&storage);
            let handle = tokio::spawn(async move {
                let query: Vec<f32> = (0..768).map(|_| 0.5).collect();
                storage_clone.query(&query, 5, None).await
            });
            handles.push(handle);
        }

        // All reads should succeed
        for handle in handles {
            let result = handle.await.expect("Task panicked");
            assert!(result.is_ok(), "Concurrent read should succeed");
            assert!(!result.unwrap().is_empty());
        }
    }

    /// Test concurrent writes to different documents
    #[tokio::test]
    async fn test_concurrent_writes_different_docs() {
        let ns = generate_namespace();
        let storage = Arc::new(MemoryVectorStorage::new(&ns, 768));
        storage.initialize().await.expect("Failed to initialize");

        // Spawn multiple concurrent write tasks
        let mut handles = vec![];
        for i in 0..5 {
            let storage_clone = Arc::clone(&storage);
            let handle = tokio::spawn(async move {
                let vector: Vec<f32> = (0..768).map(|j| (i as f32 + j as f32) / 1000.0).collect();
                storage_clone
                    .upsert(&[(format!("concurrent-doc-{}", i), vector, json!({"task": i}))])
                    .await
            });
            handles.push(handle);
        }

        // All writes should succeed
        for handle in handles {
            let result = handle.await.expect("Task panicked");
            assert!(result.is_ok(), "Concurrent write should succeed");
        }

        // Verify all documents were inserted
        let query: Vec<f32> = (0..768).map(|_| 0.5).collect();
        let results = storage
            .query(&query, 10, None)
            .await
            .expect("Failed to query");
        assert_eq!(results.len(), 5, "All 5 concurrent writes should succeed");
    }

    /// Test read during rebuild (clear + repopulate)
    #[tokio::test]
    async fn test_read_during_rebuild() {
        let ns = generate_namespace();
        let storage = Arc::new(RwLock::new(MemoryVectorStorage::new(&ns, 768)));

        // Initialize
        {
            let s = storage.read().await;
            s.initialize().await.expect("Failed to initialize");
        }

        // Insert initial data
        {
            let s = storage.read().await;
            let vectors: Vec<(String, Vec<f32>, serde_json::Value)> = (0..5)
                .map(|i| {
                    let v: Vec<f32> = (0..768).map(|_| i as f32 / 5.0).collect();
                    (format!("initial-{}", i), v, json!({}))
                })
                .collect();
            s.upsert(&vectors).await.expect("Failed to insert initial");
        }

        // Simulate rebuild operation
        let storage_clone = Arc::clone(&storage);
        let rebuild_handle = tokio::spawn(async move {
            // Clear
            {
                let s = storage_clone.read().await;
                s.clear().await.expect("Failed to clear");
            }

            // Small delay to simulate rebuild time
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;

            // Repopulate
            {
                let s = storage_clone.read().await;
                let vectors: Vec<(String, Vec<f32>, serde_json::Value)> = (0..3)
                    .map(|i| {
                        let v: Vec<f32> = (0..768).map(|_| (i as f32 + 10.0) / 15.0).collect();
                        (format!("new-{}", i), v, json!({}))
                    })
                    .collect();
                s.upsert(&vectors).await.expect("Failed to insert new");
            }
        });

        // Wait for rebuild to complete
        rebuild_handle.await.expect("Rebuild task panicked");

        // Verify new data is accessible
        let query: Vec<f32> = (0..768).map(|_| 0.8).collect();
        let s = storage.read().await;
        let results = s
            .query(&query, 10, None)
            .await
            .expect("Failed to query after rebuild");

        // Should only have new documents
        assert_eq!(
            results.len(),
            3,
            "Should have 3 new documents after rebuild"
        );
        assert!(
            results.iter().all(|r| r.id.starts_with("new-")),
            "All results should be new documents"
        );
    }
}

// ============================================================================
// Empty and Edge Value Tests (OODA 40)
// ============================================================================

mod empty_value_tests {
    use super::*;
    use edgequake_storage::{MemoryVectorStorage, VectorStorage};
    use serde_json::json;

    fn generate_namespace() -> String {
        format!(
            "empty_test_{}",
            uuid::Uuid::new_v4().to_string().replace('-', "")[..8].to_string()
        )
    }

    /// Test empty text embedding behavior
    #[tokio::test]
    async fn test_mock_empty_text_embedding() {
        let provider = MockProvider::new();

        // Empty text should still produce an embedding
        let result = provider.embed(&["".to_string()]).await;
        assert!(result.is_ok(), "Empty text should produce embedding");

        let embeddings = result.unwrap();
        assert!(!embeddings.is_empty(), "Embeddings should have values");
        assert!(
            !embeddings[0].is_empty(),
            "First embedding should have values"
        );
    }

    /// Test whitespace-only text embedding
    #[tokio::test]
    async fn test_mock_whitespace_text_embedding() {
        let provider = MockProvider::new();

        // Whitespace-only text should still produce an embedding
        let result = provider.embed(&["   \n\t  ".to_string()]).await;
        assert!(result.is_ok(), "Whitespace text should produce embedding");
    }

    /// Test very long text embedding
    #[tokio::test]
    async fn test_mock_long_text_embedding() {
        let provider = MockProvider::new();

        // Very long text
        let long_text: String = "word ".repeat(10000);
        let result = provider.embed(&[long_text]).await;
        assert!(result.is_ok(), "Long text should produce embedding");

        let embeddings = result.unwrap();
        assert!(!embeddings.is_empty(), "Embeddings should have values");
        assert!(
            !embeddings[0].is_empty(),
            "First embedding should have values"
        );
    }

    /// Test special characters in text
    #[tokio::test]
    async fn test_mock_special_characters_embedding() {
        let provider = MockProvider::new();

        // Text with special characters
        let special_text =
            "Hello 世界! 🎉 <script>alert('xss')</script> SELECT * FROM users;".to_string();
        let result = provider.embed(&[special_text]).await;
        assert!(
            result.is_ok(),
            "Special characters should produce embedding"
        );
    }

    /// Test storage with minimal documents
    #[tokio::test]
    async fn test_storage_single_document() {
        let ns = generate_namespace();
        let storage = MemoryVectorStorage::new(&ns, 768);
        storage.initialize().await.expect("Failed to initialize");

        // Insert single document
        let vector: Vec<f32> = (0..768).map(|_| 0.5).collect();
        storage
            .upsert(&[("only-doc".to_string(), vector.clone(), json!({}))])
            .await
            .expect("Failed to insert");

        // Query should find it
        let results = storage
            .query(&vector, 10, None)
            .await
            .expect("Failed to query");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "only-doc");
    }

    /// Test query with top_k larger than document count
    #[tokio::test]
    async fn test_query_topk_larger_than_count() {
        let ns = generate_namespace();
        let storage = MemoryVectorStorage::new(&ns, 768);
        storage.initialize().await.expect("Failed to initialize");

        // Insert 3 documents
        let vectors: Vec<(String, Vec<f32>, serde_json::Value)> = (0..3)
            .map(|i| {
                let v: Vec<f32> = (0..768).map(|_| i as f32 / 3.0).collect();
                (format!("doc-{}", i), v, json!({}))
            })
            .collect();
        storage.upsert(&vectors).await.expect("Failed to insert");

        // Query with top_k=100 (larger than document count)
        let query: Vec<f32> = (0..768).map(|_| 0.5).collect();
        let results = storage
            .query(&query, 100, None)
            .await
            .expect("Failed to query");

        // Should return all 3 documents
        assert_eq!(results.len(), 3, "Should return all available documents");
    }

    /// Test metadata is preserved correctly
    #[tokio::test]
    async fn test_metadata_preservation() {
        let ns = generate_namespace();
        let storage = MemoryVectorStorage::new(&ns, 768);
        storage.initialize().await.expect("Failed to initialize");

        // Insert with complex metadata
        let vector: Vec<f32> = (0..768).map(|_| 0.5).collect();
        let metadata = json!({
            "title": "Test Document",
            "author": "Test Author",
            "tags": ["rust", "testing", "embedding"],
            "nested": {
                "key": "value",
                "count": 42
            }
        });

        storage
            .upsert(&[("meta-doc".to_string(), vector.clone(), metadata.clone())])
            .await
            .expect("Failed to insert");

        // Query and verify metadata
        let results = storage
            .query(&vector, 1, None)
            .await
            .expect("Failed to query");
        assert_eq!(results.len(), 1);

        // Metadata should be preserved
        let result_meta = &results[0].metadata;
        assert_eq!(result_meta["title"], "Test Document");
        assert_eq!(result_meta["author"], "Test Author");
        assert_eq!(result_meta["nested"]["count"], 42);
    }
}
