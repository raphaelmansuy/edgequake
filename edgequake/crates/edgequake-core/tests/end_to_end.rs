#![cfg(feature = "pipeline")]

use edgequake_core::orchestrator::{EdgeQuake, EdgeQuakeConfig};
use edgequake_llm::MockProvider;
use edgequake_storage::adapters::memory::{
    MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage,
};
use std::sync::Arc;

#[tokio::test]
async fn test_end_to_end_flow() {
    // 1. Setup storage
    let kv = Arc::new(MemoryKVStorage::new("test"));
    let vector = Arc::new(MemoryVectorStorage::new("test", 1536));
    let graph = Arc::new(MemoryGraphStorage::new("test"));

    // 2. Setup providers
    let mock_provider = Arc::new(MockProvider::new());

    // Add a valid JSON response for entity extraction
    mock_provider.add_response(r#"{
        "entities": [
            {"name": "EdgeQuake", "type": "TECHNOLOGY", "description": "A high-performance RAG system built in Rust"},
            {"name": "Rust", "type": "TECHNOLOGY", "description": "A systems programming language"}
        ],
        "relationships": [
            {"source": "EdgeQuake", "target": "Rust", "type": "BUILT_IN", "description": "EdgeQuake is built in Rust"}
        ]
    }"#).await;

    // Add a response for the query
    mock_provider
        .add_response("EdgeQuake is a high-performance RAG system built in Rust.")
        .await;
    let config = EdgeQuakeConfig::default();
    let mut eq = EdgeQuake::new(config)
        .with_storage_backends(kv, vector, graph)
        .with_providers(mock_provider.clone(), mock_provider.clone());

    eq.initialize()
        .await
        .expect("Failed to initialize EdgeQuake");

    // 4. Insert a document
    let content = "EdgeQuake is a high-performance RAG system built in Rust. It uses knowledge graphs to improve retrieval accuracy.";
    let insert_result = eq
        .insert(content, Some("doc-1"))
        .await
        .expect("Failed to insert document");

    assert!(insert_result.success);
    assert!(insert_result.chunks_created > 0);

    // 5. Query the system
    let query = "What is EdgeQuake?";
    let query_result = eq.query(query, None).await.expect("Failed to query");

    assert!(!query_result.response.is_empty());
    // Default mode is now Hybrid (combines Local + Global)
    assert_eq!(query_result.mode, edgequake_core::types::QueryMode::Hybrid);

    // 6. Check graph stats
    let stats = eq.get_graph_stats().await.expect("Failed to get stats");
    // Mock provider might not extract anything unless configured, but let's check
    println!("Graph stats: {:?}", stats);
}
