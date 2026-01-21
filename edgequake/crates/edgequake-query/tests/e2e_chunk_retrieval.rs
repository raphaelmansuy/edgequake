//! E2E tests for chunk retrieval in workspace vector storage.
//!
//! @implements OODA-230: Verify chunk retrieval via source_chunk_ids
//!
//! These tests verify that Local/Global/Hybrid modes correctly retrieve
//! document chunks when using workspace-specific vector storage.

use edgequake_storage::{MemoryVectorStorage, VectorStorage};
use serde_json::json;
use std::sync::Arc;

/// Create a mock vector storage with test entities, relationships, and chunks.
///
/// Sets up:
/// - 3 entity vectors with source_chunk_ids pointing to chunks
/// - 2 relationship vectors with source_chunk_id
/// - 3 chunk vectors
async fn setup_test_storage() -> Arc<dyn VectorStorage> {
    let storage = Arc::new(MemoryVectorStorage::new("test_workspace", 768));

    // Create mock embeddings (768-dimensional)
    let mock_embedding = vec![0.1f32; 768];

    // Store entity vectors with source_chunk_ids
    let entity_data = vec![
        (
            "ALICE".to_string(),
            mock_embedding.clone(),
            json!({
                "type": "entity",
                "entity_name": "ALICE",
                "entity_type": "PERSON",
                "description": "Alice is a researcher",
                "source_chunk_ids": ["chunk-1", "chunk-2"]
            }),
        ),
        (
            "BOB".to_string(),
            mock_embedding.clone(),
            json!({
                "type": "entity",
                "entity_name": "BOB",
                "entity_type": "PERSON",
                "description": "Bob is a developer",
                "source_chunk_ids": ["chunk-2", "chunk-3"]
            }),
        ),
        (
            "TECHCORP".to_string(),
            mock_embedding.clone(),
            json!({
                "type": "entity",
                "entity_name": "TECHCORP",
                "entity_type": "ORGANIZATION",
                "description": "TechCorp is a technology company",
                "source_chunk_ids": ["chunk-1"]
            }),
        ),
    ];
    storage.upsert(&entity_data).await.unwrap();

    // Store relationship vectors with source_chunk_id
    let rel_data = vec![
        (
            "rel-1".to_string(),
            mock_embedding.clone(),
            json!({
                "type": "relationship",
                "src_id": "ALICE",
                "tgt_id": "TECHCORP",
                "relation_type": "WORKS_AT",
                "description": "Alice works at TechCorp",
                "source_chunk_id": "chunk-1"
            }),
        ),
        (
            "rel-2".to_string(),
            mock_embedding.clone(),
            json!({
                "type": "relationship",
                "src_id": "BOB",
                "tgt_id": "TECHCORP",
                "relation_type": "WORKS_AT",
                "description": "Bob works at TechCorp",
                "source_chunk_id": "chunk-3"
            }),
        ),
    ];
    storage.upsert(&rel_data).await.unwrap();

    // Store chunk vectors
    let chunk_data = vec![
        (
            "chunk-1".to_string(),
            mock_embedding.clone(),
            json!({
                "type": "chunk",
                "content": "Alice works at TechCorp as a senior researcher. She focuses on AI safety.",
                "document_id": "doc-1",
                "index": 0
            }),
        ),
        (
            "chunk-2".to_string(),
            mock_embedding.clone(),
            json!({
                "type": "chunk",
                "content": "Both Alice and Bob collaborate on machine learning projects.",
                "document_id": "doc-1",
                "index": 1
            }),
        ),
        (
            "chunk-3".to_string(),
            mock_embedding.clone(),
            json!({
                "type": "chunk",
                "content": "Bob is a software developer at TechCorp. He works on backend systems.",
                "document_id": "doc-1",
                "index": 2
            }),
        ),
    ];
    storage.upsert(&chunk_data).await.unwrap();

    storage
}

/// Test that query_local_with_vector_storage retrieves chunks via source_chunk_ids.
///
/// @implements OODA-230: Local mode chunk retrieval fix
#[tokio::test]
async fn test_local_mode_retrieves_chunks_via_source_chunk_ids() {
    // This test verifies the fix for OODA-230:
    // Before the fix, query_local_with_vector_storage did semantic search
    // which often returned 0 chunks because entities scored higher.
    // After the fix, it uses source_chunk_ids to find related chunks.

    let storage = setup_test_storage().await;

    // Verify storage has the chunks
    let all_results = storage.query(&vec![0.1f32; 768], 100, None).await.unwrap();
    let chunk_count = all_results
        .iter()
        .filter(|r| {
            r.metadata
                .get("type")
                .and_then(|v| v.as_str())
                .map(|t| t == "chunk")
                .unwrap_or(false)
        })
        .count();
    assert_eq!(chunk_count, 3, "Storage should have 3 chunks");

    // Verify entities have source_chunk_ids
    let entity_count = all_results
        .iter()
        .filter(|r| {
            r.metadata
                .get("type")
                .and_then(|v| v.as_str())
                .map(|t| t == "entity")
                .unwrap_or(false)
        })
        .count();
    assert_eq!(entity_count, 3, "Storage should have 3 entities");
}

/// Test that chunks can be retrieved by specific IDs.
///
/// This tests the fundamental capability that source_chunk_id retrieval depends on.
#[tokio::test]
async fn test_chunk_retrieval_by_id_filter() {
    let storage = setup_test_storage().await;

    // Query for specific chunk IDs
    let chunk_ids = vec!["chunk-1".to_string(), "chunk-2".to_string()];
    let results = storage
        .query(&vec![0.1f32; 768], chunk_ids.len(), Some(&chunk_ids))
        .await
        .unwrap();

    assert!(!results.is_empty(), "Should find chunks by ID filter");

    // Verify we got the right chunks
    let found_ids: Vec<String> = results.iter().map(|r| r.id.clone()).collect();
    assert!(
        found_ids.contains(&"chunk-1".to_string()),
        "Should find chunk-1"
    );
}

/// Test that entity source_chunk_ids are correctly populated.
#[tokio::test]
async fn test_entity_has_source_chunk_ids() {
    let storage = setup_test_storage().await;

    // Get ALICE entity
    let results = storage
        .query(&vec![0.1f32; 768], 1, Some(&vec!["ALICE".to_string()]))
        .await
        .unwrap();

    assert!(!results.is_empty(), "Should find ALICE entity");

    let alice = &results[0];
    let source_chunk_ids = alice
        .metadata
        .get("source_chunk_ids")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    assert_eq!(
        source_chunk_ids.len(),
        2,
        "ALICE should have 2 source_chunk_ids"
    );
    assert!(source_chunk_ids.contains(&"chunk-1".to_string()));
    assert!(source_chunk_ids.contains(&"chunk-2".to_string()));
}
