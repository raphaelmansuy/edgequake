//! PostgreSQL Integration Tests
//!
//! These tests require a running PostgreSQL instance with pgvector and AGE extensions.
//! Run with: `cargo test --package edgequake-storage --test postgres_integration --features postgres`
//!
//! Environment variables needed:
//! - POSTGRES_HOST (default: localhost)
//! - POSTGRES_PORT (default: 5432)
//! - POSTGRES_DB (default: edgequake)
//! - POSTGRES_USER (default: edgequake)
//! - POSTGRES_PASSWORD (required)

#![cfg(feature = "postgres")]

use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::Duration;

use edgequake_storage::{
    GraphStorage, KVStorage, VectorStorage,
    PostgresConfig, PostgresKVStorage, PgVectorStorage, PostgresAGEGraphStorage,
};

/// Get PostgreSQL configuration from environment variables.
fn get_test_config() -> Option<PostgresConfig> {
    // Check if password is set (indicates test environment is configured)
    let password = env::var("POSTGRES_PASSWORD").ok()?;
    
    Some(PostgresConfig {
        host: env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string()),
        port: env::var("POSTGRES_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5432),
        database: env::var("POSTGRES_DB").unwrap_or_else(|_| "edgequake".to_string()),
        user: env::var("POSTGRES_USER").unwrap_or_else(|_| "edgequake".to_string()),
        password,
        namespace: format!("test_{}", uuid::Uuid::new_v4().to_string().replace("-", "")[..8].to_string()),
        max_connections: 5,
        min_connections: 1,
        connect_timeout: Duration::from_secs(10),
        idle_timeout: Duration::from_secs(60),
        ..Default::default()
    })
}

/// Skip test if PostgreSQL is not configured.
macro_rules! require_postgres {
    () => {
        match get_test_config() {
            Some(config) => config,
            None => {
                eprintln!("Skipping test: POSTGRES_PASSWORD not set");
                return;
            }
        }
    };
}

// ============ KV Storage Tests ============

#[tokio::test]
async fn test_postgres_kv_basic_operations() {
    let config = require_postgres!();
    
    let kv_storage = PostgresKVStorage::new(config);
    
    kv_storage.initialize().await.expect("Failed to initialize");
    
    // Insert
    kv_storage.upsert(&[(
        "doc-1".to_string(),
        serde_json::json!({"title": "Test Document", "content": "Hello World"}),
    )]).await.expect("Failed to upsert");
    
    // Get
    let result = kv_storage.get_by_id("doc-1").await.expect("Failed to get");
    assert!(result.is_some());
    let doc = result.unwrap();
    assert_eq!(doc["title"], "Test Document");
    
    // Update
    kv_storage.upsert(&[(
        "doc-1".to_string(),
        serde_json::json!({"title": "Updated Document", "content": "Hello World Updated"}),
    )]).await.expect("Failed to update");
    
    let result = kv_storage.get_by_id("doc-1").await.expect("Failed to get");
    assert_eq!(result.unwrap()["title"], "Updated Document");
    
    // Delete
    kv_storage.delete(&["doc-1".to_string()]).await.expect("Failed to delete");
    let result = kv_storage.get_by_id("doc-1").await.expect("Failed to get");
    assert!(result.is_none());
    
    // Cleanup
    kv_storage.clear().await.expect("Failed to clear");
}

#[tokio::test]
async fn test_postgres_kv_bulk_operations() {
    let config = require_postgres!();
    
    let kv_storage = PostgresKVStorage::new(config);
    
    kv_storage.initialize().await.expect("Failed to initialize");
    
    // Bulk insert
    let docs: Vec<(String, serde_json::Value)> = (0..100)
        .map(|i| (
            format!("doc-{}", i),
            serde_json::json!({"index": i, "content": format!("Document {}", i)}),
        ))
        .collect();
    
    kv_storage.upsert(&docs).await.expect("Failed to bulk upsert");
    
    // Bulk get
    let ids: Vec<String> = (0..50).map(|i| format!("doc-{}", i)).collect();
    let results = kv_storage.get_by_ids(&ids).await.expect("Failed to bulk get");
    assert_eq!(results.len(), 50);
    
    // Count
    let count = kv_storage.count().await.expect("Failed to count");
    assert_eq!(count, 100);
    
    // Cleanup
    kv_storage.clear().await.expect("Failed to clear");
}

// ============ Vector Storage Tests ============

#[tokio::test]
async fn test_pgvector_basic_operations() {
    let config = require_postgres!();
    
    let vector_storage = PgVectorStorage::with_dimension(config, 384);
    
    vector_storage.initialize().await.expect("Failed to initialize");
    
    // Insert vectors
    let embedding1: Vec<f32> = (0..384).map(|i| (i as f32) / 1000.0).collect();
    let embedding2: Vec<f32> = (0..384).map(|i| ((i + 1) as f32) / 1000.0).collect();
    
    vector_storage.upsert(&[
        ("vec-1".to_string(), embedding1.clone(), serde_json::json!({"name": "Vector 1"})),
        ("vec-2".to_string(), embedding2.clone(), serde_json::json!({"name": "Vector 2"})),
    ]).await.expect("Failed to upsert vectors");
    
    // Query
    let results = vector_storage.query(&embedding1, 5, None).await.expect("Failed to query");
    assert!(!results.is_empty());
    assert_eq!(results[0].id, "vec-1"); // Most similar to itself
    
    // Get by ID
    let vec = vector_storage.get_by_id("vec-1").await.expect("Failed to get by ID");
    assert!(vec.is_some());
    assert_eq!(vec.unwrap().len(), 384);
    
    // Delete
    vector_storage.delete(&["vec-1".to_string()]).await.expect("Failed to delete");
    let vec = vector_storage.get_by_id("vec-1").await.expect("Failed to get by ID");
    assert!(vec.is_none());
    
    // Cleanup
    vector_storage.clear().await.expect("Failed to clear");
}

#[tokio::test]
async fn test_pgvector_similarity_search() {
    let config = require_postgres!();
    
    let vector_storage = PgVectorStorage::with_dimension(config, 384);
    
    vector_storage.initialize().await.expect("Failed to initialize");
    
    // Create distinct embedding clusters
    let cluster1_base: Vec<f32> = (0..384).map(|i| (i as f32 * 0.001).sin()).collect();
    let cluster2_base: Vec<f32> = (0..384).map(|i| (i as f32 * 0.001).cos()).collect();
    
    // Add vectors from cluster 1
    for i in 0..5 {
        let mut embedding = cluster1_base.clone();
        for j in 0..384 {
            embedding[j] += (i as f32) * 0.001;
        }
        vector_storage.upsert(&[(
            format!("cluster1-{}", i),
            embedding,
            serde_json::json!({"cluster": 1, "index": i}),
        )]).await.expect("Failed to upsert");
    }
    
    // Add vectors from cluster 2
    for i in 0..5 {
        let mut embedding = cluster2_base.clone();
        for j in 0..384 {
            embedding[j] += (i as f32) * 0.001;
        }
        vector_storage.upsert(&[(
            format!("cluster2-{}", i),
            embedding,
            serde_json::json!({"cluster": 2, "index": i}),
        )]).await.expect("Failed to upsert");
    }
    
    // Query with cluster1 base - should find cluster1 vectors
    let results = vector_storage.query(&cluster1_base, 3, None).await.expect("Failed to query");
    assert_eq!(results.len(), 3);
    for result in &results {
        assert!(result.id.starts_with("cluster1"), "Expected cluster1 vectors, got {}", result.id);
    }
    
    // Query with cluster2 base - should find cluster2 vectors
    let results = vector_storage.query(&cluster2_base, 3, None).await.expect("Failed to query");
    assert_eq!(results.len(), 3);
    for result in &results {
        assert!(result.id.starts_with("cluster2"), "Expected cluster2 vectors, got {}", result.id);
    }
    
    // Cleanup
    vector_storage.clear().await.expect("Failed to clear");
}

// ============ Graph Storage Tests ============

#[tokio::test]
async fn test_postgres_age_basic_operations() {
    let config = require_postgres!();
    
    let graph_storage = PostgresAGEGraphStorage::new(config);
    
    graph_storage.initialize().await.expect("Failed to initialize");
    
    // Create nodes
    let mut props1 = HashMap::new();
    props1.insert("label".to_string(), serde_json::json!("EdgeQuake"));
    props1.insert("type".to_string(), serde_json::json!("TECHNOLOGY"));
    graph_storage.upsert_node("edgequake", props1).await.expect("Failed to upsert node");
    
    let mut props2 = HashMap::new();
    props2.insert("label".to_string(), serde_json::json!("Rust"));
    props2.insert("type".to_string(), serde_json::json!("TECHNOLOGY"));
    graph_storage.upsert_node("rust", props2).await.expect("Failed to upsert node");
    
    // Verify nodes exist
    assert!(graph_storage.has_node("edgequake").await.expect("Failed to check node"));
    assert!(graph_storage.has_node("rust").await.expect("Failed to check node"));
    
    // Create edge
    let mut edge_props = HashMap::new();
    edge_props.insert("relation".to_string(), serde_json::json!("BUILT_WITH"));
    graph_storage.upsert_edge("edgequake", "rust", edge_props).await.expect("Failed to upsert edge");
    
    // Verify edge exists
    assert!(graph_storage.has_edge("edgequake", "rust").await.expect("Failed to check edge"));
    
    // Get neighbors
    let neighbors = graph_storage.get_neighbors("edgequake", 1).await.expect("Failed to get neighbors");
    assert!(!neighbors.is_empty());
    
    // Delete edge
    graph_storage.delete_edge("edgequake", "rust").await.expect("Failed to delete edge");
    assert!(!graph_storage.has_edge("edgequake", "rust").await.expect("Failed to check edge"));
    
    // Delete node
    graph_storage.delete_node("edgequake").await.expect("Failed to delete node");
    assert!(!graph_storage.has_node("edgequake").await.expect("Failed to check node"));
    
    // Cleanup
    graph_storage.clear().await.expect("Failed to clear");
}

#[tokio::test]
async fn test_postgres_age_graph_traversal() {
    let config = require_postgres!();
    
    let graph_storage = PostgresAGEGraphStorage::new(config);
    
    graph_storage.initialize().await.expect("Failed to initialize");
    
    // Build a small knowledge graph
    let entities = [
        ("edgequake", "EdgeQuake", "TECHNOLOGY"),
        ("rust", "Rust", "TECHNOLOGY"),
        ("python", "Python", "TECHNOLOGY"),
        ("lightrag", "LightRAG", "TECHNOLOGY"),
        ("sarah", "Sarah Chen", "PERSON"),
    ];
    
    for (id, label, entity_type) in entities {
        let mut props = HashMap::new();
        props.insert("label".to_string(), serde_json::json!(label));
        props.insert("type".to_string(), serde_json::json!(entity_type));
        graph_storage.upsert_node(id, props).await.expect("Failed to upsert node");
    }
    
    let relationships = [
        ("edgequake", "rust", "BUILT_WITH"),
        ("lightrag", "python", "BUILT_WITH"),
        ("edgequake", "lightrag", "INSPIRED_BY"),
        ("sarah", "edgequake", "DESIGNED"),
    ];
    
    for (src, tgt, rel) in relationships {
        let mut props = HashMap::new();
        props.insert("relation".to_string(), serde_json::json!(rel));
        graph_storage.upsert_edge(src, tgt, props).await.expect("Failed to upsert edge");
    }
    
    // Test traversals
    let node_count = graph_storage.node_count().await.expect("Failed to count nodes");
    assert_eq!(node_count, 5);
    
    let edge_count = graph_storage.edge_count().await.expect("Failed to count edges");
    assert_eq!(edge_count, 4);
    
    // Get neighbors at depth 1
    let neighbors = graph_storage.get_neighbors("edgequake", 1).await.expect("Failed to get neighbors");
    assert!(neighbors.len() >= 2); // rust, lightrag, sarah
    
    // Get knowledge graph
    let kg = graph_storage.get_knowledge_graph("edgequake", 2, 100).await.expect("Failed to get KG");
    assert!(!kg.nodes.is_empty());
    assert!(!kg.edges.is_empty());
    
    // Cleanup
    graph_storage.clear().await.expect("Failed to clear");
}

// ============ Full E2E with PostgreSQL ============

#[tokio::test]
async fn test_postgres_full_e2e_pipeline() {
    let config = require_postgres!();
    
    // Initialize all PostgreSQL storage components
    let kv_storage = Arc::new(PostgresKVStorage::new(config.clone()));
    let vector_storage = Arc::new(PgVectorStorage::with_dimension(config.clone(), 1536));
    let graph_storage = Arc::new(PostgresAGEGraphStorage::new(config));
    
    kv_storage.initialize().await.expect("Failed to initialize KV storage");
    vector_storage.initialize().await.expect("Failed to initialize vector storage");
    graph_storage.initialize().await.expect("Failed to initialize graph storage");
    
    // 1. Store a document
    let doc_id = "doc-e2e-1";
    let document = serde_json::json!({
        "title": "EdgeQuake Architecture",
        "content": "EdgeQuake is a high-performance RAG system built in Rust...",
        "metadata": {"source": "integration_test"}
    });
    kv_storage.upsert(&[(doc_id.to_string(), document)]).await.expect("Failed to store document");
    
    // 2. Store entities in graph
    let entities = [
        ("EDGEQUAKE", "EdgeQuake", "TECHNOLOGY", "A high-performance RAG system"),
        ("RUST", "Rust", "TECHNOLOGY", "A systems programming language"),
        ("SARAH_CHEN", "Sarah Chen", "PERSON", "Lead architect"),
    ];
    
    for (id, label, entity_type, description) in entities {
        let mut props = HashMap::new();
        props.insert("label".to_string(), serde_json::json!(label));
        props.insert("type".to_string(), serde_json::json!(entity_type));
        props.insert("description".to_string(), serde_json::json!(description));
        graph_storage.upsert_node(id, props).await.expect("Failed to create entity");
    }
    
    // 3. Store relationships
    let relationships = [
        ("EDGEQUAKE", "RUST", "BUILT_WITH"),
        ("SARAH_CHEN", "EDGEQUAKE", "DESIGNED"),
    ];
    
    for (src, tgt, rel) in relationships {
        let mut props = HashMap::new();
        props.insert("relation".to_string(), serde_json::json!(rel));
        graph_storage.upsert_edge(src, tgt, props).await.expect("Failed to create relationship");
    }
    
    // 4. Store entity embeddings
    let create_embedding = |seed: f32| -> Vec<f32> {
        (0..1536).map(|i| ((i as f32 + seed) / 10000.0).sin()).collect()
    };
    
    vector_storage.upsert(&[
        ("EDGEQUAKE".to_string(), create_embedding(0.0), serde_json::json!({"label": "EdgeQuake"})),
        ("RUST".to_string(), create_embedding(1.0), serde_json::json!({"label": "Rust"})),
        ("SARAH_CHEN".to_string(), create_embedding(2.0), serde_json::json!({"label": "Sarah Chen"})),
    ]).await.expect("Failed to store embeddings");
    
    // 5. Query - verify everything works
    
    // Document retrieval
    let doc = kv_storage.get_by_id(doc_id).await.expect("Failed to get document");
    assert!(doc.is_some());
    assert_eq!(doc.unwrap()["title"], "EdgeQuake Architecture");
    
    // Vector similarity search
    let query_vec = create_embedding(0.0);
    let results = vector_storage.query(&query_vec, 3, None).await.expect("Failed to query vectors");
    assert!(!results.is_empty());
    assert_eq!(results[0].id, "EDGEQUAKE"); // Most similar to itself
    
    // Graph traversal
    let neighbors = graph_storage.get_neighbors("EDGEQUAKE", 1).await.expect("Failed to get neighbors");
    assert!(!neighbors.is_empty());
    
    // Knowledge graph extraction
    let kg = graph_storage.get_knowledge_graph("EDGEQUAKE", 2, 50).await.expect("Failed to get KG");
    assert!(kg.node_count() >= 2);
    assert!(kg.edge_count() >= 1);
    
    // 6. Cleanup
    kv_storage.clear().await.expect("Failed to clear KV");
    vector_storage.clear().await.expect("Failed to clear vectors");
    graph_storage.clear().await.expect("Failed to clear graph");
    
    println!("PostgreSQL E2E test completed successfully!");
}
