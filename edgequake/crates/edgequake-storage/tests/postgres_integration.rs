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
    GraphStorage, KVStorage, PgVectorStorage, PostgresAGEGraphStorage, PostgresConfig,
    PostgresKVStorage, VectorStorage,
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
        namespace: format!(
            "test_{}",
            uuid::Uuid::new_v4().to_string().replace("-", "")[..8].to_string()
        ),
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
    kv_storage
        .upsert(&[(
            "doc-1".to_string(),
            serde_json::json!({"title": "Test Document", "content": "Hello World"}),
        )])
        .await
        .expect("Failed to upsert");

    // Get
    let result = kv_storage.get_by_id("doc-1").await.expect("Failed to get");
    assert!(result.is_some());
    let doc = result.unwrap();
    assert_eq!(doc["title"], "Test Document");

    // Update
    kv_storage
        .upsert(&[(
            "doc-1".to_string(),
            serde_json::json!({"title": "Updated Document", "content": "Hello World Updated"}),
        )])
        .await
        .expect("Failed to update");

    let result = kv_storage.get_by_id("doc-1").await.expect("Failed to get");
    assert_eq!(result.unwrap()["title"], "Updated Document");

    // Delete
    kv_storage
        .delete(&["doc-1".to_string()])
        .await
        .expect("Failed to delete");
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
        .map(|i| {
            (
                format!("doc-{}", i),
                serde_json::json!({"index": i, "content": format!("Document {}", i)}),
            )
        })
        .collect();

    kv_storage
        .upsert(&docs)
        .await
        .expect("Failed to bulk upsert");

    // Bulk get
    let ids: Vec<String> = (0..50).map(|i| format!("doc-{}", i)).collect();
    let results = kv_storage
        .get_by_ids(&ids)
        .await
        .expect("Failed to bulk get");
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

    vector_storage
        .initialize()
        .await
        .expect("Failed to initialize");

    // Insert vectors
    let embedding1: Vec<f32> = (0..384).map(|i| (i as f32) / 1000.0).collect();
    let embedding2: Vec<f32> = (0..384).map(|i| ((i + 1) as f32) / 1000.0).collect();

    vector_storage
        .upsert(&[
            (
                "vec-1".to_string(),
                embedding1.clone(),
                serde_json::json!({"name": "Vector 1"}),
            ),
            (
                "vec-2".to_string(),
                embedding2.clone(),
                serde_json::json!({"name": "Vector 2"}),
            ),
        ])
        .await
        .expect("Failed to upsert vectors");

    // Query
    let results = vector_storage
        .query(&embedding1, 5, None)
        .await
        .expect("Failed to query");
    assert!(!results.is_empty());
    assert_eq!(results[0].id, "vec-1"); // Most similar to itself

    // Get by ID
    let vec = vector_storage
        .get_by_id("vec-1")
        .await
        .expect("Failed to get by ID");
    assert!(vec.is_some());
    assert_eq!(vec.unwrap().len(), 384);

    // Delete
    vector_storage
        .delete(&["vec-1".to_string()])
        .await
        .expect("Failed to delete");
    let vec = vector_storage
        .get_by_id("vec-1")
        .await
        .expect("Failed to get by ID");
    assert!(vec.is_none());

    // Cleanup
    vector_storage.clear().await.expect("Failed to clear");
}

#[tokio::test]
async fn test_pgvector_similarity_search() {
    let config = require_postgres!();

    let vector_storage = PgVectorStorage::with_dimension(config, 384);

    vector_storage
        .initialize()
        .await
        .expect("Failed to initialize");

    // Create distinct embedding clusters
    let cluster1_base: Vec<f32> = (0..384).map(|i| (i as f32 * 0.001).sin()).collect();
    let cluster2_base: Vec<f32> = (0..384).map(|i| (i as f32 * 0.001).cos()).collect();

    // Add vectors from cluster 1
    for i in 0..5 {
        let mut embedding = cluster1_base.clone();
        for j in 0..384 {
            embedding[j] += (i as f32) * 0.001;
        }
        vector_storage
            .upsert(&[(
                format!("cluster1-{}", i),
                embedding,
                serde_json::json!({"cluster": 1, "index": i}),
            )])
            .await
            .expect("Failed to upsert");
    }

    // Add vectors from cluster 2
    for i in 0..5 {
        let mut embedding = cluster2_base.clone();
        for j in 0..384 {
            embedding[j] += (i as f32) * 0.001;
        }
        vector_storage
            .upsert(&[(
                format!("cluster2-{}", i),
                embedding,
                serde_json::json!({"cluster": 2, "index": i}),
            )])
            .await
            .expect("Failed to upsert");
    }

    // Query with cluster1 base - should find cluster1 vectors
    let results = vector_storage
        .query(&cluster1_base, 3, None)
        .await
        .expect("Failed to query");
    assert_eq!(results.len(), 3);
    for result in &results {
        assert!(
            result.id.starts_with("cluster1"),
            "Expected cluster1 vectors, got {}",
            result.id
        );
    }

    // Query with cluster2 base - should find cluster2 vectors
    let results = vector_storage
        .query(&cluster2_base, 3, None)
        .await
        .expect("Failed to query");
    assert_eq!(results.len(), 3);
    for result in &results {
        assert!(
            result.id.starts_with("cluster2"),
            "Expected cluster2 vectors, got {}",
            result.id
        );
    }

    // Cleanup
    vector_storage.clear().await.expect("Failed to clear");
}

// ============ Graph Storage Tests ============

#[tokio::test]
async fn test_postgres_age_basic_operations() {
    let config = require_postgres!();

    let graph_storage = PostgresAGEGraphStorage::new(config);

    graph_storage
        .initialize()
        .await
        .expect("Failed to initialize");

    // Create nodes
    let mut props1 = HashMap::new();
    props1.insert("label".to_string(), serde_json::json!("EdgeQuake"));
    props1.insert("type".to_string(), serde_json::json!("TECHNOLOGY"));
    graph_storage
        .upsert_node("edgequake", props1)
        .await
        .expect("Failed to upsert node");

    let mut props2 = HashMap::new();
    props2.insert("label".to_string(), serde_json::json!("Rust"));
    props2.insert("type".to_string(), serde_json::json!("TECHNOLOGY"));
    graph_storage
        .upsert_node("rust", props2)
        .await
        .expect("Failed to upsert node");

    // Verify nodes exist
    assert!(graph_storage
        .has_node("edgequake")
        .await
        .expect("Failed to check node"));
    assert!(graph_storage
        .has_node("rust")
        .await
        .expect("Failed to check node"));

    // Create edge
    let mut edge_props = HashMap::new();
    edge_props.insert("relation".to_string(), serde_json::json!("BUILT_WITH"));
    graph_storage
        .upsert_edge("edgequake", "rust", edge_props)
        .await
        .expect("Failed to upsert edge");

    // Verify edge exists
    assert!(graph_storage
        .has_edge("edgequake", "rust")
        .await
        .expect("Failed to check edge"));

    // Get neighbors
    let neighbors = graph_storage
        .get_neighbors("edgequake", 1)
        .await
        .expect("Failed to get neighbors");
    assert!(!neighbors.is_empty());

    // Delete edge
    graph_storage
        .delete_edge("edgequake", "rust")
        .await
        .expect("Failed to delete edge");
    assert!(!graph_storage
        .has_edge("edgequake", "rust")
        .await
        .expect("Failed to check edge"));

    // Delete node
    graph_storage
        .delete_node("edgequake")
        .await
        .expect("Failed to delete node");
    assert!(!graph_storage
        .has_node("edgequake")
        .await
        .expect("Failed to check node"));

    // Cleanup
    graph_storage.clear().await.expect("Failed to clear");
}

#[tokio::test]
async fn test_postgres_age_graph_traversal() {
    let config = require_postgres!();

    let graph_storage = PostgresAGEGraphStorage::new(config);

    graph_storage
        .initialize()
        .await
        .expect("Failed to initialize");

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
        graph_storage
            .upsert_node(id, props)
            .await
            .expect("Failed to upsert node");
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
        graph_storage
            .upsert_edge(src, tgt, props)
            .await
            .expect("Failed to upsert edge");
    }

    // Test traversals
    let node_count = graph_storage
        .node_count()
        .await
        .expect("Failed to count nodes");
    assert_eq!(node_count, 5);

    let edge_count = graph_storage
        .edge_count()
        .await
        .expect("Failed to count edges");
    assert_eq!(edge_count, 4);

    // Get neighbors at depth 1
    let neighbors = graph_storage
        .get_neighbors("edgequake", 1)
        .await
        .expect("Failed to get neighbors");
    assert!(neighbors.len() >= 2); // rust, lightrag, sarah

    // Get knowledge graph
    let kg = graph_storage
        .get_knowledge_graph("edgequake", 2, 100)
        .await
        .expect("Failed to get KG");
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

    kv_storage
        .initialize()
        .await
        .expect("Failed to initialize KV storage");
    vector_storage
        .initialize()
        .await
        .expect("Failed to initialize vector storage");
    graph_storage
        .initialize()
        .await
        .expect("Failed to initialize graph storage");

    // 1. Store a document
    let doc_id = "doc-e2e-1";
    let document = serde_json::json!({
        "title": "EdgeQuake Architecture",
        "content": "EdgeQuake is a high-performance RAG system built in Rust...",
        "metadata": {"source": "integration_test"}
    });
    kv_storage
        .upsert(&[(doc_id.to_string(), document)])
        .await
        .expect("Failed to store document");

    // 2. Store entities in graph
    let entities = [
        (
            "EDGEQUAKE",
            "EdgeQuake",
            "TECHNOLOGY",
            "A high-performance RAG system",
        ),
        (
            "RUST",
            "Rust",
            "TECHNOLOGY",
            "A systems programming language",
        ),
        ("SARAH_CHEN", "Sarah Chen", "PERSON", "Lead architect"),
    ];

    for (id, label, entity_type, description) in entities {
        let mut props = HashMap::new();
        props.insert("label".to_string(), serde_json::json!(label));
        props.insert("type".to_string(), serde_json::json!(entity_type));
        props.insert("description".to_string(), serde_json::json!(description));
        graph_storage
            .upsert_node(id, props)
            .await
            .expect("Failed to create entity");
    }

    // 3. Store relationships
    let relationships = [
        ("EDGEQUAKE", "RUST", "BUILT_WITH"),
        ("SARAH_CHEN", "EDGEQUAKE", "DESIGNED"),
    ];

    for (src, tgt, rel) in relationships {
        let mut props = HashMap::new();
        props.insert("relation".to_string(), serde_json::json!(rel));
        graph_storage
            .upsert_edge(src, tgt, props)
            .await
            .expect("Failed to create relationship");
    }

    // 4. Store entity embeddings
    let create_embedding = |seed: f32| -> Vec<f32> {
        (0..1536)
            .map(|i| ((i as f32 + seed) / 10000.0).sin())
            .collect()
    };

    vector_storage
        .upsert(&[
            (
                "EDGEQUAKE".to_string(),
                create_embedding(0.0),
                serde_json::json!({"label": "EdgeQuake"}),
            ),
            (
                "RUST".to_string(),
                create_embedding(1.0),
                serde_json::json!({"label": "Rust"}),
            ),
            (
                "SARAH_CHEN".to_string(),
                create_embedding(2.0),
                serde_json::json!({"label": "Sarah Chen"}),
            ),
        ])
        .await
        .expect("Failed to store embeddings");

    // 5. Query - verify everything works

    // Document retrieval
    let doc = kv_storage
        .get_by_id(doc_id)
        .await
        .expect("Failed to get document");
    assert!(doc.is_some());
    assert_eq!(doc.unwrap()["title"], "EdgeQuake Architecture");

    // Vector similarity search
    let query_vec = create_embedding(0.0);
    let results = vector_storage
        .query(&query_vec, 3, None)
        .await
        .expect("Failed to query vectors");
    assert!(!results.is_empty());
    assert_eq!(results[0].id, "EDGEQUAKE"); // Most similar to itself

    // Graph traversal
    let neighbors = graph_storage
        .get_neighbors("EDGEQUAKE", 1)
        .await
        .expect("Failed to get neighbors");
    assert!(!neighbors.is_empty());

    // Knowledge graph extraction
    let kg = graph_storage
        .get_knowledge_graph("EDGEQUAKE", 2, 50)
        .await
        .expect("Failed to get KG");
    assert!(kg.node_count() >= 2);
    assert!(kg.edge_count() >= 1);

    // 6. Cleanup
    kv_storage.clear().await.expect("Failed to clear KV");
    vector_storage
        .clear()
        .await
        .expect("Failed to clear vectors");
    graph_storage.clear().await.expect("Failed to clear graph");

    println!("PostgreSQL E2E test completed successfully!");
}

// ============ AGE-Specific Cypher Tests ============

#[tokio::test]
async fn test_age_cypher_variable_length_paths() {
    let config = require_postgres!();

    let graph_storage = PostgresAGEGraphStorage::new(config);
    graph_storage
        .initialize()
        .await
        .expect("Failed to initialize");

    // Build a chain: A -> B -> C -> D -> E
    let nodes = ["A", "B", "C", "D", "E"];
    for node_id in &nodes {
        let mut props = HashMap::new();
        props.insert("name".to_string(), serde_json::json!(node_id));
        graph_storage
            .upsert_node(node_id, props)
            .await
            .expect("Failed to create node");
    }

    // Create chain edges
    for i in 0..nodes.len() - 1 {
        let mut props = HashMap::new();
        props.insert("relation".to_string(), serde_json::json!("NEXT"));
        graph_storage
            .upsert_edge(nodes[i], nodes[i + 1], props)
            .await
            .expect("Failed to create edge");
    }

    // Test depth 1: A should see B
    let neighbors_1 = graph_storage
        .get_neighbors("A", 1)
        .await
        .expect("Failed to get neighbors at depth 1");
    assert_eq!(neighbors_1.len(), 1, "Depth 1 should find 1 neighbor");
    assert_eq!(neighbors_1[0].id, "B");

    // Test depth 2: A should see B, C
    let neighbors_2 = graph_storage
        .get_neighbors("A", 2)
        .await
        .expect("Failed to get neighbors at depth 2");
    assert_eq!(neighbors_2.len(), 2, "Depth 2 should find 2 neighbors");

    // Test depth 3: A should see B, C, D
    let neighbors_3 = graph_storage
        .get_neighbors("A", 3)
        .await
        .expect("Failed to get neighbors at depth 3");
    assert_eq!(neighbors_3.len(), 3, "Depth 3 should find 3 neighbors");

    // Test full depth: A should see all 4 other nodes
    let neighbors_all = graph_storage
        .get_neighbors("A", 10)
        .await
        .expect("Failed to get all neighbors");
    assert_eq!(neighbors_all.len(), 4, "Should find all 4 neighbors");

    // Cleanup
    graph_storage.clear().await.expect("Failed to clear");
}

#[tokio::test]
async fn test_age_cypher_knowledge_graph_extraction() {
    let config = require_postgres!();

    let graph_storage = PostgresAGEGraphStorage::new(config);
    graph_storage
        .initialize()
        .await
        .expect("Failed to initialize");

    // Build a star graph: Center connected to 5 satellites
    // Also add some satellites connected to each other
    let center = "CENTER";
    let satellites = ["SAT1", "SAT2", "SAT3", "SAT4", "SAT5"];

    let mut props = HashMap::new();
    props.insert("name".to_string(), serde_json::json!("Center Node"));
    props.insert("type".to_string(), serde_json::json!("HUB"));
    graph_storage
        .upsert_node(center, props)
        .await
        .expect("Failed to create center");

    for (i, sat_id) in satellites.iter().enumerate() {
        let mut props = HashMap::new();
        props.insert(
            "name".to_string(),
            serde_json::json!(format!("Satellite {}", i + 1)),
        );
        props.insert("type".to_string(), serde_json::json!("SATELLITE"));
        graph_storage
            .upsert_node(sat_id, props)
            .await
            .expect("Failed to create satellite");

        let mut edge_props = HashMap::new();
        edge_props.insert("relation".to_string(), serde_json::json!("CONNECTED_TO"));
        graph_storage
            .upsert_edge(center, sat_id, edge_props)
            .await
            .expect("Failed to create edge");
    }

    // Add edges between some satellites
    let mut props = HashMap::new();
    props.insert("relation".to_string(), serde_json::json!("PEER"));
    graph_storage
        .upsert_edge("SAT1", "SAT2", props.clone())
        .await
        .expect("Failed");
    graph_storage
        .upsert_edge("SAT3", "SAT4", props)
        .await
        .expect("Failed");

    // Extract knowledge graph from center
    let kg = graph_storage
        .get_knowledge_graph(center, 1, 100)
        .await
        .expect("Failed to get KG");

    assert_eq!(kg.node_count(), 6, "Should have center + 5 satellites");
    assert!(
        kg.edge_count() >= 5,
        "Should have at least 5 edges from center"
    );

    // Extract KG with depth 2 to include satellite-to-satellite edges
    let kg_deep = graph_storage
        .get_knowledge_graph(center, 2, 100)
        .await
        .expect("Failed to get deep KG");

    assert_eq!(kg_deep.node_count(), 6, "Should still have 6 nodes");
    assert_eq!(kg_deep.edge_count(), 7, "Should have 5 + 2 = 7 edges");

    // Test truncation
    let kg_limited = graph_storage
        .get_knowledge_graph(center, 2, 3)
        .await
        .expect("Failed to get limited KG");

    assert!(kg_limited.node_count() <= 3, "Should be limited to 3 nodes");
    assert!(kg_limited.is_truncated, "Should be marked as truncated");

    // Cleanup
    graph_storage.clear().await.expect("Failed to clear");
}

#[tokio::test]
async fn test_age_cypher_node_degree() {
    let config = require_postgres!();

    let graph_storage = PostgresAGEGraphStorage::new(config);
    graph_storage
        .initialize()
        .await
        .expect("Failed to initialize");

    // Create a hub node connected to multiple satellites
    let hub = "HUB";
    let satellites = ["S1", "S2", "S3", "S4", "S5"];

    let mut props = HashMap::new();
    props.insert("type".to_string(), serde_json::json!("hub"));
    graph_storage
        .upsert_node(hub, props)
        .await
        .expect("Failed to create hub");

    for sat in &satellites {
        let mut props = HashMap::new();
        props.insert("type".to_string(), serde_json::json!("satellite"));
        graph_storage
            .upsert_node(sat, props)
            .await
            .expect("Failed to create satellite");

        let mut edge_props = HashMap::new();
        edge_props.insert("weight".to_string(), serde_json::json!(1.0));
        graph_storage
            .upsert_edge(hub, sat, edge_props)
            .await
            .expect("Failed to create edge");
    }

    // Hub should have degree 5 (5 outgoing edges)
    let hub_degree = graph_storage
        .node_degree(hub)
        .await
        .expect("Failed to get degree");
    assert_eq!(hub_degree, 5, "Hub should have degree 5");

    // Satellites should have degree 1 each (1 incoming edge)
    for sat in &satellites {
        let sat_degree = graph_storage
            .node_degree(sat)
            .await
            .expect("Failed to get degree");
        assert_eq!(sat_degree, 1, "Satellite should have degree 1");
    }

    // Cleanup
    graph_storage.clear().await.expect("Failed to clear");
}

#[tokio::test]
async fn test_age_cypher_search_labels() {
    let config = require_postgres!();

    let graph_storage = PostgresAGEGraphStorage::new(config);
    graph_storage
        .initialize()
        .await
        .expect("Failed to initialize");

    // Create nodes with various IDs
    let node_ids = [
        "EDGEQUAKE_MAIN",
        "EDGEQUAKE_API",
        "EDGEQUAKE_STORAGE",
        "LIGHTRAG_CORE",
        "LIGHTRAG_API",
        "RUST_TOKIO",
        "PYTHON_ASYNCIO",
    ];

    for node_id in &node_ids {
        let mut props = HashMap::new();
        props.insert("name".to_string(), serde_json::json!(node_id));
        graph_storage
            .upsert_node(node_id, props)
            .await
            .expect("Failed to create node");
    }

    // Search for "EDGEQUAKE" should find 3 nodes
    let edge_results = graph_storage
        .search_labels("edgequake", 10)
        .await
        .expect("Failed to search");
    assert_eq!(edge_results.len(), 3, "Should find 3 EDGEQUAKE nodes");

    // Search for "API" should find 2 nodes
    let api_results = graph_storage
        .search_labels("api", 10)
        .await
        .expect("Failed to search");
    assert_eq!(api_results.len(), 2, "Should find 2 API nodes");

    // Search for "RUST" should find 1 node
    let rust_results = graph_storage
        .search_labels("rust", 10)
        .await
        .expect("Failed to search");
    assert_eq!(rust_results.len(), 1, "Should find 1 RUST node");

    // Test limit
    let limited = graph_storage
        .search_labels("EDGE", 2)
        .await
        .expect("Failed to search");
    assert_eq!(limited.len(), 2, "Should be limited to 2 results");

    // Cleanup
    graph_storage.clear().await.expect("Failed to clear");
}

#[tokio::test]
async fn test_age_cypher_popular_labels() {
    let config = require_postgres!();

    let graph_storage = PostgresAGEGraphStorage::new(config);
    graph_storage
        .initialize()
        .await
        .expect("Failed to initialize");

    // Create nodes with different connectivity
    let nodes = [
        "HIGH_DEGREE",
        "MEDIUM_DEGREE",
        "LOW_DEGREE",
        "LEAF1",
        "LEAF2",
        "LEAF3",
    ];
    for node in &nodes {
        let mut props = HashMap::new();
        props.insert("name".to_string(), serde_json::json!(node));
        graph_storage
            .upsert_node(node, props)
            .await
            .expect("Failed");
    }

    // HIGH_DEGREE connects to all others
    for node in &nodes[1..] {
        let mut props = HashMap::new();
        props.insert("relation".to_string(), serde_json::json!("CONNECTED"));
        graph_storage
            .upsert_edge("HIGH_DEGREE", node, props)
            .await
            .expect("Failed");
    }

    // MEDIUM_DEGREE connects to leafs
    for node in &nodes[3..] {
        let mut props = HashMap::new();
        props.insert("relation".to_string(), serde_json::json!("CONNECTED"));
        graph_storage
            .upsert_edge("MEDIUM_DEGREE", node, props)
            .await
            .expect("Failed");
    }

    // LOW_DEGREE connects to one leaf
    let mut props = HashMap::new();
    props.insert("relation".to_string(), serde_json::json!("CONNECTED"));
    graph_storage
        .upsert_edge("LOW_DEGREE", "LEAF1", props)
        .await
        .expect("Failed");

    // Get popular labels (by degree)
    let popular = graph_storage
        .get_popular_labels(3)
        .await
        .expect("Failed to get popular");

    assert!(!popular.is_empty(), "Should have popular labels");
    // HIGH_DEGREE should be first (degree 5)
    assert_eq!(
        popular[0], "HIGH_DEGREE",
        "HIGH_DEGREE should be most popular"
    );

    // Cleanup
    graph_storage.clear().await.expect("Failed to clear");
}

#[tokio::test]
async fn test_age_cypher_edge_properties() {
    let config = require_postgres!();

    let graph_storage = PostgresAGEGraphStorage::new(config);
    graph_storage
        .initialize()
        .await
        .expect("Failed to initialize");

    // Create nodes
    graph_storage
        .upsert_node("A", HashMap::new())
        .await
        .expect("Failed");
    graph_storage
        .upsert_node("B", HashMap::new())
        .await
        .expect("Failed");

    // Create edge with rich properties
    let mut edge_props = HashMap::new();
    edge_props.insert("relation".to_string(), serde_json::json!("KNOWS"));
    edge_props.insert("weight".to_string(), serde_json::json!(0.95));
    edge_props.insert("since".to_string(), serde_json::json!("2024-01-01"));
    edge_props.insert("count".to_string(), serde_json::json!(42));

    graph_storage
        .upsert_edge("A", "B", edge_props)
        .await
        .expect("Failed to create edge");

    // Retrieve and verify edge
    let edge = graph_storage
        .get_edge("A", "B")
        .await
        .expect("Failed to get edge");

    assert!(edge.is_some(), "Edge should exist");
    let edge = edge.unwrap();
    assert_eq!(edge.source, "A");
    assert_eq!(edge.target, "B");
    assert_eq!(edge.properties.get("relation").unwrap(), "KNOWS");
    assert_eq!(edge.properties.get("weight").unwrap(), 0.95);
    assert_eq!(edge.properties.get("count").unwrap(), 42);

    // Update edge properties
    let mut updated_props = HashMap::new();
    updated_props.insert("relation".to_string(), serde_json::json!("CLOSE_FRIENDS"));
    updated_props.insert("weight".to_string(), serde_json::json!(0.99));

    graph_storage
        .upsert_edge("A", "B", updated_props)
        .await
        .expect("Failed to update edge");

    let updated_edge = graph_storage
        .get_edge("A", "B")
        .await
        .expect("Failed to get updated edge");

    assert!(updated_edge.is_some());
    let updated_edge = updated_edge.unwrap();
    assert_eq!(
        updated_edge.properties.get("relation").unwrap(),
        "CLOSE_FRIENDS"
    );
    assert_eq!(updated_edge.properties.get("weight").unwrap(), 0.99);

    // Cleanup
    graph_storage.clear().await.expect("Failed to clear");
}

#[tokio::test]
async fn test_age_cypher_node_update() {
    let config = require_postgres!();

    let graph_storage = PostgresAGEGraphStorage::new(config);
    graph_storage
        .initialize()
        .await
        .expect("Failed to initialize");

    // Create node with initial properties
    let mut initial_props = HashMap::new();
    initial_props.insert("name".to_string(), serde_json::json!("Alice"));
    initial_props.insert("age".to_string(), serde_json::json!(30));

    graph_storage
        .upsert_node("user-1", initial_props)
        .await
        .expect("Failed to create node");

    // Verify initial state
    let node = graph_storage
        .get_node("user-1")
        .await
        .expect("Failed to get node");

    assert!(node.is_some());
    let node = node.unwrap();
    assert_eq!(node.id, "user-1");
    assert_eq!(node.properties.get("name").unwrap(), "Alice");
    assert_eq!(node.properties.get("age").unwrap(), 30);

    // Update node properties
    let mut updated_props = HashMap::new();
    updated_props.insert("name".to_string(), serde_json::json!("Alice Smith"));
    updated_props.insert("age".to_string(), serde_json::json!(31));
    updated_props.insert("city".to_string(), serde_json::json!("New York"));

    graph_storage
        .upsert_node("user-1", updated_props)
        .await
        .expect("Failed to update node");

    // Verify updated state
    let updated = graph_storage
        .get_node("user-1")
        .await
        .expect("Failed to get updated node");

    assert!(updated.is_some());
    let updated = updated.unwrap();
    assert_eq!(updated.properties.get("name").unwrap(), "Alice Smith");
    assert_eq!(updated.properties.get("age").unwrap(), 31);
    assert_eq!(updated.properties.get("city").unwrap(), "New York");

    // Cleanup
    graph_storage.clear().await.expect("Failed to clear");
}

#[tokio::test]
async fn test_age_cypher_detach_delete() {
    let config = require_postgres!();

    let graph_storage = PostgresAGEGraphStorage::new(config);
    graph_storage
        .initialize()
        .await
        .expect("Failed to initialize");

    // Create a hub with edges
    graph_storage
        .upsert_node("HUB", HashMap::new())
        .await
        .expect("Failed");
    graph_storage
        .upsert_node("SAT1", HashMap::new())
        .await
        .expect("Failed");
    graph_storage
        .upsert_node("SAT2", HashMap::new())
        .await
        .expect("Failed");

    let mut props = HashMap::new();
    props.insert("relation".to_string(), serde_json::json!("CONNECTS"));
    graph_storage
        .upsert_edge("HUB", "SAT1", props.clone())
        .await
        .expect("Failed");
    graph_storage
        .upsert_edge("HUB", "SAT2", props)
        .await
        .expect("Failed");

    // Verify initial state
    assert_eq!(graph_storage.node_count().await.expect("Failed"), 3);
    assert_eq!(graph_storage.edge_count().await.expect("Failed"), 2);

    // Delete HUB (should also delete connected edges via DETACH DELETE)
    graph_storage
        .delete_node("HUB")
        .await
        .expect("Failed to delete hub");

    // Verify HUB is gone but satellites remain
    assert!(!graph_storage.has_node("HUB").await.expect("Failed"));
    assert!(graph_storage.has_node("SAT1").await.expect("Failed"));
    assert!(graph_storage.has_node("SAT2").await.expect("Failed"));

    // Edges should be gone too
    assert_eq!(graph_storage.edge_count().await.expect("Failed"), 0);
    assert_eq!(graph_storage.node_count().await.expect("Failed"), 2);

    // Cleanup
    graph_storage.clear().await.expect("Failed to clear");
}

// ============ Source Tracking Tests ============

/// Test that source tracking fields are properly stored and retrieved from graph nodes
#[tokio::test]
async fn test_postgres_source_tracking_in_entities() {
    let config = require_postgres!();
    let graph_storage = PostgresAGEGraphStorage::new(config);

    graph_storage
        .initialize()
        .await
        .expect("Failed to initialize");

    // Create entity node with source tracking properties
    let mut props = HashMap::new();
    props.insert("label".to_string(), serde_json::json!("Sarah Chen"));
    props.insert("type".to_string(), serde_json::json!("PERSON"));
    props.insert(
        "description".to_string(),
        serde_json::json!("Lead researcher"),
    );
    props.insert(
        "source_chunk_ids".to_string(),
        serde_json::json!(["chunk-001", "chunk-002", "chunk-003"]),
    );
    props.insert(
        "source_document_id".to_string(),
        serde_json::json!("doc-abc123"),
    );
    props.insert(
        "source_file_path".to_string(),
        serde_json::json!("/documents/research.pdf"),
    );

    graph_storage
        .upsert_node("SARAH_CHEN", props)
        .await
        .expect("Failed to upsert node");

    // Retrieve the node and verify source tracking
    let node = graph_storage
        .get_node("SARAH_CHEN")
        .await
        .expect("Failed to get node");
    assert!(node.is_some());
    let node = node.unwrap();

    // Verify source_chunk_ids is an array
    let source_chunk_ids = node
        .properties
        .get("source_chunk_ids")
        .and_then(|v| v.as_array())
        .expect("source_chunk_ids should be an array");
    assert_eq!(source_chunk_ids.len(), 3);
    assert!(source_chunk_ids
        .iter()
        .any(|v| v.as_str() == Some("chunk-001")));
    assert!(source_chunk_ids
        .iter()
        .any(|v| v.as_str() == Some("chunk-002")));

    // Verify source_document_id
    let source_doc_id = node
        .properties
        .get("source_document_id")
        .and_then(|v| v.as_str());
    assert_eq!(source_doc_id, Some("doc-abc123"));

    // Verify source_file_path
    let source_file = node
        .properties
        .get("source_file_path")
        .and_then(|v| v.as_str());
    assert_eq!(source_file, Some("/documents/research.pdf"));

    // Cleanup
    graph_storage.clear().await.expect("Failed to clear");
}

/// Test that source tracking works for relationships/edges
#[tokio::test]
async fn test_postgres_source_tracking_in_relationships() {
    let config = require_postgres!();
    let graph_storage = PostgresAGEGraphStorage::new(config);

    graph_storage
        .initialize()
        .await
        .expect("Failed to initialize");

    // Create source and target nodes
    let mut props = HashMap::new();
    props.insert("label".to_string(), serde_json::json!("Alice"));
    props.insert("type".to_string(), serde_json::json!("PERSON"));
    graph_storage
        .upsert_node("ALICE", props.clone())
        .await
        .expect("Failed to create source node");

    props.insert("label".to_string(), serde_json::json!("Bob"));
    graph_storage
        .upsert_node("BOB", props)
        .await
        .expect("Failed to create target node");

    // Create edge with source tracking
    let mut edge_props = HashMap::new();
    edge_props.insert("relation".to_string(), serde_json::json!("KNOWS"));
    edge_props.insert(
        "description".to_string(),
        serde_json::json!("Alice knows Bob from work"),
    );
    edge_props.insert(
        "source_chunk_id".to_string(),
        serde_json::json!("chunk-005"),
    );
    edge_props.insert(
        "source_document_id".to_string(),
        serde_json::json!("doc-xyz789"),
    );
    edge_props.insert(
        "source_file_path".to_string(),
        serde_json::json!("/documents/team.md"),
    );

    graph_storage
        .upsert_edge("ALICE", "BOB", edge_props)
        .await
        .expect("Failed to create edge");

    // Retrieve edge and verify source tracking
    let edge = graph_storage
        .get_edge("ALICE", "BOB")
        .await
        .expect("Failed to get edge");
    assert!(edge.is_some());
    let edge = edge.unwrap();

    // Verify source_chunk_id (singular for relationships)
    let source_chunk = edge
        .properties
        .get("source_chunk_id")
        .and_then(|v: &serde_json::Value| v.as_str());
    assert_eq!(source_chunk, Some("chunk-005"));

    // Verify source_document_id
    let source_doc = edge
        .properties
        .get("source_document_id")
        .and_then(|v: &serde_json::Value| v.as_str());
    assert_eq!(source_doc, Some("doc-xyz789"));

    // Verify source_file_path
    let source_file = edge
        .properties
        .get("source_file_path")
        .and_then(|v: &serde_json::Value| v.as_str());
    assert_eq!(source_file, Some("/documents/team.md"));

    // Cleanup
    graph_storage.clear().await.expect("Failed to clear");
}

/// Test source tracking roundtrip through full E2E pipeline
#[tokio::test]
async fn test_postgres_source_tracking_e2e() {
    let config = require_postgres!();

    let kv_storage = Arc::new(PostgresKVStorage::new(config.clone()));
    let vector_storage = Arc::new(PgVectorStorage::with_dimension(config.clone(), 1536));
    let graph_storage = Arc::new(PostgresAGEGraphStorage::new(config));

    kv_storage
        .initialize()
        .await
        .expect("Failed to initialize KV storage");
    vector_storage
        .initialize()
        .await
        .expect("Failed to initialize vector storage");
    graph_storage
        .initialize()
        .await
        .expect("Failed to initialize graph storage");

    // 1. Store document
    let doc_id = "doc-source-tracking-test";
    let file_path = "/test/documents/source-tracking.txt";
    let document = serde_json::json!({
        "title": "Source Tracking Test",
        "content": "This is a test document for source tracking...",
        "file_path": file_path,
    });
    kv_storage
        .upsert(&[(doc_id.to_string(), document)])
        .await
        .expect("Failed to store document");

    // 2. Store chunks with document reference
    let chunk_id = "chunk-source-test-001";
    let chunk_embedding: Vec<f32> = (0..1536).map(|i| (i as f32) / 1536.0).collect();
    let chunk_metadata = serde_json::json!({
        "document_id": doc_id,
        "file_path": file_path,
    });
    vector_storage
        .upsert(&[(chunk_id.to_string(), chunk_embedding, chunk_metadata)])
        .await
        .expect("Failed to store chunk");

    // 3. Store entity with source tracking in graph
    let entity_id = "TEST_ENTITY";
    let mut entity_props = HashMap::new();
    entity_props.insert("label".to_string(), serde_json::json!("Test Entity"));
    entity_props.insert("type".to_string(), serde_json::json!("CONCEPT"));
    entity_props.insert(
        "source_chunk_ids".to_string(),
        serde_json::json!([chunk_id]),
    );
    entity_props.insert("source_document_id".to_string(), serde_json::json!(doc_id));
    entity_props.insert("source_file_path".to_string(), serde_json::json!(file_path));
    graph_storage
        .upsert_node(entity_id, entity_props)
        .await
        .expect("Failed to store entity");

    // 4. Verify we can trace from entity back to document
    let retrieved = graph_storage
        .get_node(entity_id)
        .await
        .expect("Failed to get node");
    assert!(retrieved.is_some());
    let node = retrieved.unwrap();

    // Extract source info
    let source_doc = node
        .properties
        .get("source_document_id")
        .and_then(|v: &serde_json::Value| v.as_str())
        .unwrap();
    assert_eq!(source_doc, doc_id);

    // Verify we can fetch the original document using source_document_id
    let original_doc = kv_storage
        .get_by_id(source_doc)
        .await
        .expect("Failed to get document");
    assert!(original_doc.is_some());
    let doc = original_doc.unwrap();
    assert_eq!(
        doc.get("title")
            .and_then(|v: &serde_json::Value| v.as_str()),
        Some("Source Tracking Test")
    );

    // Cleanup
    kv_storage.clear().await.expect("Failed to clear KV");
    vector_storage
        .clear()
        .await
        .expect("Failed to clear vector");
    graph_storage.clear().await.expect("Failed to clear graph");
}

/// Test that nested arrays and objects are properly serialized in Cypher
/// This validates the recursive `value_to_cypher` function
#[tokio::test]
async fn test_postgres_nested_array_and_object_properties() {
    let config = require_postgres!();
    let graph_storage = PostgresAGEGraphStorage::new(config);

    graph_storage
        .initialize()
        .await
        .expect("Failed to initialize");

    // Create node with nested structures
    let mut props = HashMap::new();
    props.insert("name".to_string(), serde_json::json!("TestEntity"));

    // Simple array (already tested, but good to have)
    props.insert(
        "tags".to_string(),
        serde_json::json!(["alpha", "beta", "gamma"]),
    );

    // Array of numbers
    props.insert("scores".to_string(), serde_json::json!([95, 87, 92, 78]));

    // Mixed type array
    props.insert(
        "mixed".to_string(),
        serde_json::json!(["text", 42, true, null]),
    );

    // Nested object
    props.insert(
        "metadata".to_string(),
        serde_json::json!({
            "version": "1.0",
            "count": 5,
            "active": true
        }),
    );

    // Array of objects
    props.insert(
        "references".to_string(),
        serde_json::json!([
            {"id": "ref-001", "type": "citation"},
            {"id": "ref-002", "type": "source"}
        ]),
    );

    graph_storage
        .upsert_node("TEST_NESTED", props)
        .await
        .expect("Failed to upsert node with nested properties");

    // Retrieve and verify
    let node = graph_storage
        .get_node("TEST_NESTED")
        .await
        .expect("Failed to get node")
        .expect("Node should exist");

    // Verify simple array
    let tags = node.properties.get("tags").and_then(|v| v.as_array());
    assert!(tags.is_some(), "tags should be an array");
    let tags = tags.unwrap();
    assert_eq!(tags.len(), 3);
    assert!(tags.contains(&serde_json::json!("alpha")));
    assert!(tags.contains(&serde_json::json!("beta")));
    assert!(tags.contains(&serde_json::json!("gamma")));

    // Verify number array
    let scores = node.properties.get("scores").and_then(|v| v.as_array());
    assert!(scores.is_some(), "scores should be an array");
    let scores = scores.unwrap();
    assert_eq!(scores.len(), 4);
    assert!(scores.contains(&serde_json::json!(95)));

    // Verify mixed type array
    let mixed = node.properties.get("mixed").and_then(|v| v.as_array());
    assert!(mixed.is_some(), "mixed should be an array");
    let mixed = mixed.unwrap();
    assert_eq!(mixed.len(), 4);
    assert!(mixed.contains(&serde_json::json!("text")));
    assert!(mixed.contains(&serde_json::json!(42)));
    assert!(mixed.contains(&serde_json::json!(true)));

    // Verify nested object
    let metadata = node.properties.get("metadata").and_then(|v| v.as_object());
    assert!(metadata.is_some(), "metadata should be an object");
    let metadata = metadata.unwrap();
    assert_eq!(
        metadata.get("version").and_then(|v| v.as_str()),
        Some("1.0")
    );
    assert_eq!(metadata.get("count").and_then(|v| v.as_i64()), Some(5));
    assert_eq!(metadata.get("active").and_then(|v| v.as_bool()), Some(true));

    // Verify array of objects
    let refs = node.properties.get("references").and_then(|v| v.as_array());
    assert!(refs.is_some(), "references should be an array");
    let refs = refs.unwrap();
    assert_eq!(refs.len(), 2);
    let ref0 = refs[0].as_object().expect("should be object");
    assert_eq!(ref0.get("id").and_then(|v| v.as_str()), Some("ref-001"));
    assert_eq!(ref0.get("type").and_then(|v| v.as_str()), Some("citation"));

    // Cleanup
    graph_storage.clear().await.expect("Failed to clear");
}
