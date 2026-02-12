#![cfg(feature = "pipeline")]

//! End-to-End Pipeline Tests
//!
//! These tests verify the complete document processing pipeline:
//! Document -> Chunking -> Entity Extraction -> Knowledge Graph Storage
//!
//! Tests cover both Memory and PostgreSQL storage backends.

use std::env;
use std::sync::Arc;

use edgequake_core::{EdgeQuake, EdgeQuakeConfig, StorageBackend, StorageConfig};
use edgequake_llm::{MockProvider, OpenAIProvider};
use edgequake_pipeline::{
    ExtractedEntity, ExtractedRelationship, ExtractionResult, KnowledgeGraphMerger, MergerConfig,
};
use edgequake_storage::{GraphStorage, MemoryGraphStorage, MemoryVectorStorage};

#[cfg(feature = "postgres")]
use edgequake_storage::{KVStorage, VectorStorage};

/// Create LLM provider from environment or use smart mock.
///
/// Checks for OPENAI_API_KEY environment variable:
/// - If present: Creates real OpenAI provider for production testing
/// - If absent: Creates smart mock provider with valid JSON responses
///
/// This allows the same tests to work in:
/// - CI/CD: Uses mock provider (no API keys needed)
/// - Local development: Can test with real LLM if API key is set
/// - Production: Uses real LLM provider
async fn create_llm_provider() -> (
    Arc<dyn edgequake_llm::LLMProvider>,
    Arc<dyn edgequake_llm::EmbeddingProvider>,
) {
    // Check for OpenAI API key
    if let Ok(api_key) = env::var("OPENAI_API_KEY") {
        if !api_key.is_empty() && api_key != "test-key" {
            println!("🔑 Using REAL OpenAI provider (API key found)");
            let provider = Arc::new(
                OpenAIProvider::new(api_key)
                    .with_model("gpt-4o-mini") // Fast and cost-effective
                    .with_embedding_model("text-embedding-3-small"),
            );
            return (
                provider.clone() as Arc<dyn edgequake_llm::LLMProvider>,
                provider as Arc<dyn edgequake_llm::EmbeddingProvider>,
            );
        }
    }

    // Fall back to smart mock provider
    println!("🔧 Using Smart Mock provider (no API key or testing mode)");
    let mock = create_smart_mock_provider().await;
    (
        mock.clone() as Arc<dyn edgequake_llm::LLMProvider>,
        mock as Arc<dyn edgequake_llm::EmbeddingProvider>,
    )
}

/// Create a smart mock provider that returns valid extraction JSON.
async fn create_smart_mock_provider() -> Arc<MockProvider> {
    let provider = Arc::new(MockProvider::new());

    // Add valid extraction JSON response
    let extraction_json = r#"{
  "entities": [
    {"name": "EdgeQuake", "type": "TECHNOLOGY", "description": "A high-performance RAG system built in Rust"},
    {"name": "Sarah Chen", "type": "PERSON", "description": "Lead architect of EdgeQuake"},
    {"name": "Rust", "type": "TECHNOLOGY", "description": "Systems programming language"},
    {"name": "Apache AGE", "type": "TECHNOLOGY", "description": "Graph database extension for PostgreSQL"},
    {"name": "Michael Torres", "type": "PERSON", "description": "LLM integration team lead"}
  ],
  "relationships": [
    {"source": "EdgeQuake", "target": "Rust", "type": "BUILT_WITH", "description": "EdgeQuake is built using Rust programming language"},
    {"source": "Sarah Chen", "target": "EdgeQuake", "type": "DESIGNED", "description": "Sarah Chen designed and architected EdgeQuake"},
    {"source": "EdgeQuake", "target": "Apache AGE", "type": "USES", "description": "EdgeQuake uses Apache AGE for graph storage"},
    {"source": "Michael Torres", "target": "Sarah Chen", "type": "WORKS_WITH", "description": "Michael Torres works with Sarah Chen on the project"}
  ]
}"#;

    provider.add_response(extraction_json).await;
    provider
}

/// Sample document about EdgeQuake for testing.
const SAMPLE_DOCUMENT: &str = r#"
EdgeQuake is a high-performance Retrieval-Augmented Generation (RAG) system built in Rust. 
The system was designed by Sarah Chen, who serves as the lead architect. EdgeQuake integrates 
multiple advanced technologies including Apache AGE for graph storage and pgvector for 
similarity search.

The architecture follows a modular design with clear separation of concerns. The storage layer 
supports multiple backends including PostgreSQL with AGE extension for production deployments. 
For development and testing, an in-memory storage option is provided.

Michael Torres leads the LLM integration team, working closely with Sarah Chen to ensure optimal 
performance. The team has implemented sophisticated caching mechanisms to reduce latency and costs.
EdgeQuake is designed to handle large-scale knowledge graphs with millions of entities and relationships.
"#;

#[tokio::test]
async fn test_memory_e2e_document_to_knowledge_graph() {
    // 1. Initialize memory storage backends
    let kv_storage = Arc::new(edgequake_storage::MemoryKVStorage::new("test_e2e_memory"));
    let vector_storage = Arc::new(MemoryVectorStorage::new("test_e2e_memory", 1536));
    let graph_storage = Arc::new(MemoryGraphStorage::new("test_e2e_memory"));

    // 2. Initialize EdgeQuake with memory storage
    let config = EdgeQuakeConfig::new()
        .with_namespace("test_e2e_memory")
        .with_storage(StorageConfig {
            backend: StorageBackend::Memory,
            ..Default::default()
        });

    // Create LLM provider (real if API key available, mock otherwise)
    let (llm_provider, embedding_provider) = create_llm_provider().await;

    let mut edgequake = EdgeQuake::new(config)
        .with_storage_backends(
            kv_storage.clone(),
            vector_storage.clone(),
            graph_storage.clone(),
        )
        .with_providers(llm_provider, embedding_provider);

    edgequake
        .initialize()
        .await
        .expect("Failed to initialize EdgeQuake");

    // 3. Insert document and process through full pipeline
    println!("\n=== Testing Full E2E Pipeline with Smart Mock ===");
    let result = edgequake
        .insert(SAMPLE_DOCUMENT, Some("doc-edgequake-001"))
        .await
        .expect("Failed to insert document");

    // 4. Verify insertion results
    assert!(result.success, "Insert should succeed");
    assert!(result.chunks_created > 0, "Should create chunks");
    println!("✓ Created {} chunks", result.chunks_created);

    // With SmartMockProvider, extraction should succeed
    assert!(result.entities_extracted > 0, "Should extract entities");
    assert!(
        result.relationships_extracted > 0,
        "Should extract relationships"
    );
    println!("✓ Extracted {} entities", result.entities_extracted);
    println!(
        "✓ Extracted {} relationships",
        result.relationships_extracted
    );

    // 5. Query graph statistics
    let stats = edgequake
        .get_graph_stats()
        .await
        .expect("Failed to get stats");

    println!(
        "✓ Graph stats: {} nodes, {} edges",
        stats.node_count, stats.edge_count
    );
    assert!(stats.node_count > 0, "Should have nodes in graph");
    assert!(stats.edge_count > 0, "Should have edges in graph");

    // 6. Verify specific entities exist in the graph
    assert!(
        graph_storage.has_node("EDGEQUAKE").await.unwrap(),
        "EdgeQuake entity should exist"
    );
    assert!(
        graph_storage.has_node("SARAH_CHEN").await.unwrap(),
        "Sarah Chen entity should exist"
    );
    println!("✓ All expected entities present in graph");

    // 7. Test graph traversal
    let neighbors = graph_storage
        .get_neighbors("SARAH_CHEN", 1)
        .await
        .expect("Failed to get neighbors");
    println!("✓ Sarah Chen has {} neighbors", neighbors.len());
    assert!(!neighbors.is_empty(), "Should have neighbors");

    println!("\n✅ Full E2E Pipeline Test PASSED - Data ingestion working!");
}

/// Test with simulated real extraction results (without actual LLM calls).
#[tokio::test]
async fn test_memory_e2e_with_simulated_extraction() {
    // 1. Setup storages
    let graph_storage = Arc::new(MemoryGraphStorage::new("test_simulated"));
    let vector_storage = Arc::new(MemoryVectorStorage::new("test_simulated", 1536));

    // 2. Create simulated extraction results (as if from LLM extraction)
    // Note: Entity names will be normalized by merger (e.g., "Sarah Chen" → "SARAH_CHEN")
    let extractions = vec![
        create_test_extraction(
            "chunk-001",
            vec![
                ("EdgeQuake", "TECHNOLOGY", "A RAG system"),
                ("Rust", "TECHNOLOGY", "Programming language"),
            ],
            vec![("EdgeQuake", "Rust", "BUILT_WITH", 1.0)],
        ),
        create_test_extraction(
            "chunk-002",
            vec![
                ("Sarah Chen", "PERSON", "Lead architect"),
                ("EdgeQuake", "TECHNOLOGY", "RAG system"),
            ],
            vec![("Sarah Chen", "EdgeQuake", "DESIGNED", 1.0)],
        ),
        create_test_extraction(
            "chunk-003",
            vec![
                ("Michael Torres", "PERSON", "LLM team lead"),
                ("Sarah Chen", "PERSON", "Lead architect"),
            ],
            vec![("Michael Torres", "Sarah Chen", "WORKS_WITH", 1.0)],
        ),
    ];

    // 3. Merge into knowledge graph
    let merger = KnowledgeGraphMerger::new(
        MergerConfig::default(),
        graph_storage.clone(),
        vector_storage.clone(),
    );

    let merge_stats = merger.merge(extractions).await.expect("Failed to merge");

    println!("Merge stats: {:#?}", merge_stats);

    // 4. Verify knowledge graph
    assert!(merge_stats.entities_created > 0, "Should create entities");
    assert!(
        merge_stats.relationships_created > 0,
        "Should create relationships"
    );

    // 5. Check specific entities exist (using normalized names)
    assert!(
        graph_storage.has_node("EDGEQUAKE").await.unwrap(),
        "EdgeQuake entity should exist"
    );
    assert!(
        graph_storage.has_node("SARAH_CHEN").await.unwrap(),
        "Sarah Chen entity should exist"
    );
    assert!(
        graph_storage.has_node("RUST").await.unwrap(),
        "Rust entity should exist"
    );

    // 6. Check relationships
    assert!(
        graph_storage.has_edge("EDGEQUAKE", "RUST").await.unwrap(),
        "EdgeQuake-Rust relationship should exist"
    );

    assert!(
        graph_storage
            .has_edge("SARAH_CHEN", "EDGEQUAKE")
            .await
            .unwrap(),
        "Sarah Chen-EdgeQuake relationship should exist"
    );

    // 7. Test graph traversal
    let neighbors = graph_storage
        .get_neighbors("SARAH_CHEN", 1)
        .await
        .expect("Failed to get neighbors");
    println!("Sarah Chen neighbors: {}", neighbors.len());
    assert!(!neighbors.is_empty(), "Should have neighbors");

    // 8. Get knowledge subgraph
    let kg = graph_storage
        .get_knowledge_graph("SARAH_CHEN", 2, 50)
        .await
        .expect("Failed to get knowledge graph");

    println!(
        "Knowledge subgraph: {} nodes, {} edges",
        kg.node_count(),
        kg.edge_count()
    );
    assert!(kg.node_count() >= 2, "Should have multiple nodes");
    assert!(kg.edge_count() >= 1, "Should have edges");

    println!("Simulated extraction E2E test completed successfully");
}

/// Test multi-document ingestion with full pipeline.
#[tokio::test]
async fn test_multi_document_ingestion_pipeline() {
    println!("\n=== Testing Multi-Document Ingestion Pipeline ===");

    // 1. Setup storage backends
    let kv_storage = Arc::new(edgequake_storage::MemoryKVStorage::new("test_multi_doc"));
    let vector_storage = Arc::new(MemoryVectorStorage::new("test_multi_doc", 1536));
    let graph_storage = Arc::new(MemoryGraphStorage::new("test_multi_doc"));

    // 2. Create EdgeQuake instance
    let config = EdgeQuakeConfig::new()
        .with_namespace("test_multi_doc")
        .with_storage(StorageConfig {
            backend: StorageBackend::Memory,
            ..Default::default()
        });

    // 3. Setup LLM provider (real or mock based on environment)
    // For mock provider, pre-configure responses for 3 documents
    let (llm_provider, embedding_provider) = if env::var("OPENAI_API_KEY").is_ok()
        && env::var("OPENAI_API_KEY").unwrap() != "test-key"
    {
        // Real provider - will make actual API calls
        create_llm_provider().await
    } else {
        // Mock provider - pre-configure responses
        let mock_provider = Arc::new(MockProvider::new());

        // Document 1 extraction response
        mock_provider.add_response(r#"{
  "entities": [
    {"name": "EdgeQuake", "type": "TECHNOLOGY", "description": "RAG system in Rust"},
    {"name": "Sarah Chen", "type": "PERSON", "description": "Lead architect"},
    {"name": "Rust", "type": "TECHNOLOGY", "description": "Programming language"}
  ],
  "relationships": [
    {"source": "EdgeQuake", "target": "Rust", "type": "BUILT_WITH", "description": "Built with Rust"},
    {"source": "Sarah Chen", "target": "EdgeQuake", "type": "DESIGNED", "description": "Sarah designed EdgeQuake"}
  ]
}"#).await;

        // Document 2 extraction response
        mock_provider.add_response(r#"{
  "entities": [
    {"name": "EdgeQuake", "type": "TECHNOLOGY", "description": "High-performance knowledge graph system"},
    {"name": "Michael Torres", "type": "PERSON", "description": "LLM integration lead"},
    {"name": "Apache AGE", "type": "TECHNOLOGY", "description": "Graph database for PostgreSQL"}
  ],
  "relationships": [
    {"source": "EdgeQuake", "target": "Apache AGE", "type": "USES", "description": "Uses Apache AGE for storage"},
    {"source": "Michael Torres", "target": "EdgeQuake", "type": "WORKS_ON", "description": "Works on EdgeQuake LLM integration"}
  ]
}"#).await;

        // Document 3 extraction response
        mock_provider.add_response(r#"{
  "entities": [
    {"name": "Sarah Chen", "type": "PERSON", "description": "Senior architect with graph expertise"},
    {"name": "Michael Torres", "type": "PERSON", "description": "LLM specialist"},
    {"name": "PostgreSQL", "type": "TECHNOLOGY", "description": "Database system"}
  ],
  "relationships": [
    {"source": "Sarah Chen", "target": "Michael Torres", "type": "COLLABORATES_WITH", "description": "Works together on the project"},
    {"source": "Apache AGE", "target": "PostgreSQL", "type": "EXTENDS", "description": "AGE is an extension of PostgreSQL"}
  ]
}"#).await;

        (
            mock_provider.clone() as Arc<dyn edgequake_llm::LLMProvider>,
            mock_provider as Arc<dyn edgequake_llm::EmbeddingProvider>,
        )
    };

    let mut edgequake = EdgeQuake::new(config)
        .with_storage_backends(kv_storage, vector_storage.clone(), graph_storage.clone())
        .with_providers(llm_provider, embedding_provider);

    edgequake.initialize().await.expect("Failed to initialize");

    // 4. Ingest multiple documents
    let documents = vec![
        ("doc-001", SAMPLE_DOCUMENT),
        (
            "doc-002",
            r#"
Michael Torres leads the LLM integration efforts for EdgeQuake, working closely with the architecture team.
EdgeQuake leverages Apache AGE, a powerful graph database extension for PostgreSQL, to store and query
complex knowledge graphs efficiently. The system can handle millions of entities and relationships.
"#,
        ),
        (
            "doc-003",
            r#"
The development team consists of Sarah Chen, who brings extensive experience in graph databases and
distributed systems, and Michael Torres, a specialist in large language models and embeddings.
Together, they've built a system that combines the best of both worlds. Apache AGE extends PostgreSQL
with graph capabilities, providing ACID guarantees and efficient graph traversal.
"#,
        ),
    ];

    let mut total_entities = 0;
    let mut total_relationships = 0;

    for (doc_id, content) in &documents {
        println!("\n→ Ingesting document: {}", doc_id);
        let result = edgequake
            .insert(content, Some(doc_id))
            .await
            .expect("Failed to insert document");

        assert!(result.success, "Insert should succeed");
        total_entities += result.entities_extracted;
        total_relationships += result.relationships_extracted;

        println!("  ✓ Created {} chunks", result.chunks_created);
        println!("  ✓ Extracted {} entities", result.entities_extracted);
        println!(
            "  ✓ Extracted {} relationships",
            result.relationships_extracted
        );
    }

    println!("\n=== Final Results ===");
    println!("Total entities extracted: {}", total_entities);
    println!("Total relationships extracted: {}", total_relationships);

    // 5. Verify knowledge graph
    let stats = edgequake
        .get_graph_stats()
        .await
        .expect("Failed to get stats");
    println!(
        "Graph contains: {} unique nodes, {} edges",
        stats.node_count, stats.edge_count
    );

    // Should have fewer unique entities than total extracted (due to merging)
    assert!(stats.node_count > 0, "Should have nodes");
    assert!(stats.edge_count > 0, "Should have edges");
    assert!(
        stats.node_count <= total_entities,
        "Should merge duplicate entities"
    );

    // 6. Verify entity merging worked
    assert!(
        graph_storage.has_node("EDGEQUAKE").await.unwrap(),
        "EdgeQuake should exist"
    );
    assert!(
        graph_storage.has_node("SARAH_CHEN").await.unwrap(),
        "Sarah Chen should exist"
    );
    assert!(
        graph_storage.has_node("MICHAEL_TORRES").await.unwrap(),
        "Michael Torres should exist"
    );

    // 7. Check relationships between key entities
    let sarah_neighbors = graph_storage
        .get_neighbors("SARAH_CHEN", 1)
        .await
        .expect("Failed to get neighbors");
    println!(
        "Sarah Chen is connected to {} entities",
        sarah_neighbors.len()
    );
    assert!(
        sarah_neighbors.len() >= 2,
        "Sarah should be connected to multiple entities"
    );

    // 8. Test knowledge graph traversal
    let kg = graph_storage
        .get_knowledge_graph("EDGEQUAKE", 2, 50)
        .await
        .expect("Failed to get knowledge graph");
    println!(
        "EdgeQuake subgraph: {} nodes, {} edges",
        kg.node_count(),
        kg.edge_count()
    );
    assert!(kg.node_count() >= 3, "Should have substantial subgraph");

    println!("\n✅ Multi-Document Ingestion Pipeline Test PASSED!");
    println!(
        "   Successfully ingested {} documents into unified knowledge graph",
        documents.len()
    );
}

// Helper functions

fn create_test_extraction(
    chunk_id: &str,
    entities: Vec<(&str, &str, &str)>, // (name, type, description)
    relationships: Vec<(&str, &str, &str, f32)>, // (src, tgt, rel_type, weight)
) -> ExtractionResult {
    let entities = entities
        .into_iter()
        .map(|(name, entity_type, desc)| {
            ExtractedEntity::new(name, entity_type, desc).with_importance(0.8)
        })
        .collect();

    let relationships = relationships
        .into_iter()
        .map(|(src, tgt, rel_type, weight)| {
            ExtractedRelationship::new(src, tgt, rel_type)
                .with_description(format!("{} {} {}", src, rel_type, tgt))
                .with_weight(weight)
        })
        .collect();

    let mut result = ExtractionResult::new(chunk_id);
    result.entities = entities;
    result.relationships = relationships;
    result
}

/// Test PostgreSQL-based full E2E pipeline.
#[cfg(feature = "postgres")]
#[tokio::test]
async fn test_postgres_e2e_document_to_knowledge_graph() {
    use edgequake_storage::{
        PgVectorStorage, PostgresAGEGraphStorage, PostgresConfig, PostgresKVStorage,
    };
    use std::time::Duration;

    // Get PostgreSQL configuration from environment
    let password = match std::env::var("POSTGRES_PASSWORD") {
        Ok(pwd) if !pwd.is_empty() => pwd,
        _ => {
            println!("Skipping test: POSTGRES_PASSWORD not set");
            return;
        }
    };

    let config = PostgresConfig {
        host: std::env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string()),
        port: std::env::var("POSTGRES_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5432),
        database: std::env::var("POSTGRES_DB").unwrap_or_else(|_| "edgequake".to_string()),
        user: std::env::var("POSTGRES_USER").unwrap_or_else(|_| "edgequake".to_string()),
        password,
        namespace: format!(
            "test_e2e_{}",
            uuid::Uuid::new_v4().to_string().replace("-", "")[..8].to_string()
        ),
        max_connections: 5,
        min_connections: 1,
        connect_timeout: Duration::from_secs(10),
        idle_timeout: Duration::from_secs(60),
        ..Default::default()
    };

    // Initialize PostgreSQL storage backends
    let kv_storage: Arc<dyn KVStorage> = Arc::new(PostgresKVStorage::new(config.clone()));
    let vector_storage: Arc<dyn VectorStorage> =
        Arc::new(PgVectorStorage::with_dimension(config.clone(), 1536));
    let graph_storage: Arc<dyn GraphStorage> =
        Arc::new(PostgresAGEGraphStorage::new(config.clone()));

    // Initialize all storage backends
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
        .expect("Failed to initialize graph");

    // Create mock provider with valid extraction JSON
    let mock_provider = Arc::new(MockProvider::new());
    mock_provider.add_response(r#"{
  "entities": [
    {"name": "EdgeQuake", "type": "TECHNOLOGY", "description": "A high-performance RAG system built in Rust"},
    {"name": "Sarah Chen", "type": "PERSON", "description": "Lead architect of EdgeQuake"},
    {"name": "Rust", "type": "TECHNOLOGY", "description": "Systems programming language"}
  ],
  "relationships": [
    {"source": "EdgeQuake", "target": "Rust", "type": "BUILT_WITH", "description": "EdgeQuake is built using Rust"},
    {"source": "Sarah Chen", "target": "EdgeQuake", "type": "DESIGNED", "description": "Sarah Chen designed EdgeQuake"}
  ]
}"#).await;

    let edgequake_config = EdgeQuakeConfig::new()
        .with_namespace(&config.namespace)
        .with_storage(StorageConfig {
            backend: StorageBackend::Postgres,
            ..Default::default()
        });

    let mut edgequake = EdgeQuake::new(edgequake_config)
        .with_storage_backends(kv_storage, vector_storage, graph_storage)
        .with_providers(
            mock_provider.clone() as Arc<dyn edgequake_llm::LLMProvider>,
            mock_provider as Arc<dyn edgequake_llm::EmbeddingProvider>,
        );

    edgequake
        .initialize()
        .await
        .expect("Failed to initialize EdgeQuake");

    // Insert document
    let result = edgequake
        .insert(SAMPLE_DOCUMENT, Some("doc-postgres-001"))
        .await
        .expect("Insert should succeed");

    // Verify results
    assert!(result.success, "Insert should succeed");
    assert!(result.chunks_created > 0, "Should create chunks");
    assert!(result.entities_extracted > 0, "Should extract entities");
    assert!(
        result.relationships_extracted > 0,
        "Should extract relationships"
    );

    println!("✓ PostgreSQL E2E test completed successfully");
    println!("  - Chunks: {}", result.chunks_created);
    println!("  - Entities: {}", result.entities_extracted);
    println!("  - Relationships: {}", result.relationships_extracted);
}

/// Test that source tracking fields are properly populated through the extraction pipeline.
/// This verifies that:
/// 1. ExtractedEntity and ExtractedRelationship have source fields populated
/// 2. Merger stores source tracking in graph nodes
/// 3. Source info can be retrieved from stored entities
#[tokio::test]
async fn test_source_tracking_in_extraction_pipeline() {
    // Create memory storage backends
    let graph_storage: Arc<dyn GraphStorage> = Arc::new(MemoryGraphStorage::new("test"));
    let vector_storage = Arc::new(MemoryVectorStorage::new("test", 1536));

    // Create extracted entities with source tracking
    let chunk_id = "chunk-001";
    let document_id = "doc-source-test";
    let file_path = "/documents/test-file.pdf";

    let entity1 = ExtractedEntity::new("EdgeQuake", "TECHNOLOGY", "A RAG system")
        .with_source_chunk_id(chunk_id)
        .with_source_document_id(document_id)
        .with_source_file_path(file_path)
        .with_importance(0.9);

    let entity2 = ExtractedEntity::new("Sarah Chen", "PERSON", "Lead architect")
        .with_source_chunk_id(chunk_id)
        .with_source_document_id(document_id)
        .with_source_file_path(file_path)
        .with_importance(0.8);

    let relationship = ExtractedRelationship::new("Sarah Chen", "EdgeQuake", "DESIGNED")
        .with_description("Sarah Chen designed EdgeQuake")
        .with_source_chunk_id(chunk_id)
        .with_source_document_id(document_id)
        .with_source_file_path(file_path)
        .with_weight(0.9);

    // Verify source tracking is set on extracted items
    assert!(!entity1.source_chunk_ids.is_empty());
    assert_eq!(entity1.source_chunk_ids[0], chunk_id);
    assert_eq!(entity1.source_document_id, Some(document_id.to_string()));
    assert_eq!(entity1.source_file_path, Some(file_path.to_string()));

    assert_eq!(relationship.source_chunk_id, Some(chunk_id.to_string()));
    assert_eq!(
        relationship.source_document_id,
        Some(document_id.to_string())
    );

    // Create extraction result
    let mut result = ExtractionResult::new(chunk_id);
    result.add_entity(entity1);
    result.add_entity(entity2);
    result.add_relationship(relationship);

    // Use merger to store in graph
    let merger_config = MergerConfig::default();
    let merger = KnowledgeGraphMerger::new(merger_config, graph_storage.clone(), vector_storage);

    let stats = merger
        .merge(vec![result])
        .await
        .expect("Merge should succeed");

    assert_eq!(stats.entities_created, 2);
    assert_eq!(stats.relationships_created, 1);

    // Verify source tracking was stored in graph nodes
    let node = graph_storage
        .get_node("EDGEQUAKE")
        .await
        .expect("Should get node");
    assert!(node.is_some(), "Node should exist");

    let node = node.unwrap();

    // Verify source_chunk_ids was stored
    let source_chunks = node
        .properties
        .get("source_chunk_ids")
        .and_then(|v| v.as_array());
    assert!(source_chunks.is_some(), "source_chunk_ids should exist");
    let source_chunks = source_chunks.unwrap();
    assert!(
        source_chunks
            .iter()
            .any(|v| v.as_str() == Some("chunk-001")),
        "chunk-001 should be in source_chunk_ids"
    );

    // Verify source_document_id was stored
    let source_doc = node
        .properties
        .get("source_document_id")
        .and_then(|v| v.as_str());
    assert_eq!(source_doc, Some(document_id));

    // Verify source_file_path was stored
    let source_file = node
        .properties
        .get("source_file_path")
        .and_then(|v| v.as_str());
    assert_eq!(source_file, Some(file_path));

    println!("✅ Source tracking E2E test passed!");
    println!("   - Entities created: {}", stats.entities_created);
    println!(
        "   - Relationships created: {}",
        stats.relationships_created
    );
    println!("   - Source tracking fields verified in graph storage");
}
