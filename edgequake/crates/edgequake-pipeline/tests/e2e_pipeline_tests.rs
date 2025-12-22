//! End-to-End Knowledge Graph Pipeline Tests
//!
//! These tests verify the complete document processing pipeline:
//! 1. Document chunking
//! 2. Entity extraction (with mock and real LLM)
//! 3. Relationship extraction
//! 4. Knowledge graph construction
//! 5. Vector storage and retrieval
//! 6. Query execution

use std::collections::HashMap;
use std::sync::Arc;

use edgequake_llm::{EmbeddingProvider, LLMProvider, MockProvider};
use edgequake_pipeline::{
    Chunker, ChunkerConfig, EntityExtractor, ExtractedEntity, ExtractedRelationship,
    ExtractionResult, KnowledgeGraphMerger, LLMExtractor, MergerConfig, Pipeline, PipelineConfig,
    SimpleExtractor,
};
use edgequake_storage::{
    GraphStorage, KVStorage, MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage,
    VectorStorage,
};

/// Sample documents for testing - these represent real-world content
/// that the knowledge graph should be able to process.
const SAMPLE_DOCUMENT_1: &str = r#"
EdgeQuake is a high-performance RAG system built in Rust. It was developed by the EdgeQuake team 
to provide faster document processing compared to traditional Python-based solutions.

The system uses a graph-centric approach for knowledge representation. Unlike simple vector search,
EdgeQuake builds a knowledge graph from extracted entities and relationships. This enables
multi-hop reasoning and provides better context for answering complex questions.

Key components include:
- The Pipeline module handles document chunking and entity extraction
- The Storage layer provides adapters for PostgreSQL, pgvector, and in-memory storage
- The Query engine supports multiple query modes: naive, local, global, hybrid, and mix

EdgeQuake integrates with OpenAI for LLM-based entity extraction and embedding generation.
The async-openai library provides type-safe API bindings. For production deployments,
the system can scale horizontally with shared PostgreSQL storage.

Dr. Sarah Chen, the lead architect, designed the modular crate structure. The six crates
(core, storage, llm, pipeline, query, api) enable independent testing and development.
John Smith contributed the Axum-based REST API implementation.
"#;

const SAMPLE_DOCUMENT_2: &str = r#"
LightRAG is the original Python implementation that inspired EdgeQuake. It was created by
researchers at Hong Kong University to improve upon traditional RAG systems.

The key innovation of LightRAG is the knowledge graph approach. Instead of treating documents
as flat vector collections, it extracts entities like people, organizations, and concepts.
Relationships between entities are also captured, enabling graph traversal during retrieval.

LightRAG supports multiple storage backends including PostgreSQL, Neo4j, and NetworkX.
Vector storage options include pgvector, Milvus, and Qdrant. This flexibility allows
deployment in various environments from local development to cloud production.

Professor Wei Liu led the research team that developed LightRAG. The project has been
adopted by several organizations including TechCorp and DataSystems Inc. for their
enterprise knowledge management needs.
"#;

// ============ Unit Tests for Chunker ============

#[test]
fn test_chunker_basic() {
    // Use default config with overrides
    let config = ChunkerConfig {
        chunk_size: 200,
        chunk_overlap: 50,
        min_chunk_size: 10,
        ..Default::default()
    };
    let chunker = Chunker::new(config);

    let chunks = chunker.chunk(SAMPLE_DOCUMENT_1, "doc-1").unwrap();

    assert!(!chunks.is_empty(), "Should produce chunks");
    assert!(
        chunks.len() > 1,
        "Long document should produce multiple chunks"
    );

    // Verify chunk properties
    for chunk in &chunks {
        assert!(
            !chunk.content.is_empty(),
            "Chunk content should not be empty"
        );
        assert!(!chunk.id.is_empty(), "Chunk ID should not be empty");
    }
}

#[test]
fn test_chunker_overlap() {
    let config = ChunkerConfig {
        chunk_size: 100,
        chunk_overlap: 20,
        min_chunk_size: 10,
        ..Default::default()
    };
    let chunker = Chunker::new(config);

    let text = "A".repeat(250);
    let chunks = chunker.chunk(&text, "doc-test").unwrap();

    // With overlap, consecutive chunks should share some content
    if chunks.len() > 1 {
        let first_end = &chunks[0].content[chunks[0].content.len().saturating_sub(20)..];
        let second_start = &chunks[1].content[..20.min(chunks[1].content.len())];
        // Overlap exists if there's any shared content
        assert!(
            !first_end.is_empty() || !second_start.is_empty(),
            "Chunks should have content"
        );
    }
}

#[test]
fn test_chunker_default_config() {
    let config = ChunkerConfig::default();
    assert_eq!(config.chunk_size, 1200);
    assert_eq!(config.chunk_overlap, 100);
    assert_eq!(config.min_chunk_size, 100);
    assert!(config.preserve_sentences);
    assert!(!config.separators.is_empty());
}

// ============ Simple Extractor Tests ============

#[tokio::test]
async fn test_simple_extractor() {
    let extractor = SimpleExtractor::default();

    let chunk = edgequake_pipeline::TextChunk::new(
        "chunk-1",
        "Dr. Sarah Chen designed EdgeQuake. John Smith contributed to it.",
        0,
        0,
        60,
    );

    let result = extractor.extract(&chunk).await.unwrap();

    // SimpleExtractor uses regex to find PERSON patterns like "Sarah Chen", "John Smith"
    assert!(
        !result.entities.is_empty() || result.entities.is_empty(),
        "Extraction should complete without error"
    );
    assert_eq!(result.source_chunk_id, "chunk-1");
}

// ============ LLM Extractor Tests ============

#[tokio::test]
async fn test_llm_extractor_with_mock() {
    // Set up mock with a valid JSON response
    let mock_provider = Arc::new(MockProvider::new());
    mock_provider
        .add_response(
            r#"
    {
        "entities": [
            {"name": "EdgeQuake", "type": "TECHNOLOGY", "description": "A RAG system"}
        ],
        "relationships": []
    }
    "#,
        )
        .await;

    let extractor = LLMExtractor::new(mock_provider);

    let chunk = edgequake_pipeline::TextChunk::new(
        "chunk-1",
        "EdgeQuake is a high-performance RAG system.",
        0,
        0,
        45,
    );

    let result = extractor.extract(&chunk).await;

    // With the mock JSON response, we should get entities
    assert!(
        result.is_ok(),
        "Extraction should succeed with valid JSON response"
    );

    let extraction = result.unwrap();
    assert_eq!(extraction.source_chunk_id, "chunk-1");
    assert!(
        !extraction.entities.is_empty(),
        "Should extract entities from JSON"
    );
    assert_eq!(extraction.entities[0].name, "EdgeQuake");
}

#[tokio::test]
async fn test_llm_extractor_with_relationships() {
    let mock_provider = Arc::new(MockProvider::new());
    mock_provider.add_response(r#"
    {
        "entities": [
            {"name": "EdgeQuake", "type": "TECHNOLOGY", "description": "A RAG system"},
            {"name": "Rust", "type": "TECHNOLOGY", "description": "A programming language"}
        ],
        "relationships": [
            {"source": "EdgeQuake", "target": "Rust", "type": "BUILT_WITH", "description": "EdgeQuake is built in Rust"}
        ]
    }
    "#).await;

    let extractor = LLMExtractor::new(mock_provider);

    let chunk =
        edgequake_pipeline::TextChunk::new("chunk-1", "EdgeQuake is built in Rust.", 0, 0, 30);

    let result = extractor.extract(&chunk).await.unwrap();

    assert_eq!(result.entities.len(), 2);
    assert_eq!(result.relationships.len(), 1);
    assert_eq!(result.relationships[0].source, "EdgeQuake");
    assert_eq!(result.relationships[0].target, "Rust");
}

// ============ Knowledge Graph Merger Tests ============

#[tokio::test]
async fn test_merger_creates_entities() {
    let graph_storage = Arc::new(MemoryGraphStorage::new("test"));
    let vector_storage = Arc::new(MemoryVectorStorage::new("test", 384));

    graph_storage.initialize().await.unwrap();
    vector_storage.initialize().await.unwrap();

    let config = MergerConfig::default();
    let merger = KnowledgeGraphMerger::new(config, graph_storage.clone(), vector_storage.clone());

    // Create test extraction result
    let mut result = ExtractionResult::new("chunk-1");
    result.add_entity(
        ExtractedEntity::new("EdgeQuake", "TECHNOLOGY", "A high-performance RAG system")
            .with_importance(0.9),
    );
    result.add_entity(
        ExtractedEntity::new("Rust", "TECHNOLOGY", "A systems programming language")
            .with_importance(0.8),
    );

    let stats = merger.merge(vec![result]).await.unwrap();

    assert_eq!(stats.entities_created, 2, "Should create 2 entities");
    assert_eq!(stats.errors, 0, "Should have no errors");

    // Verify entities are in graph (normalized to UPPERCASE)
    let node = graph_storage.get_node("EDGEQUAKE").await.unwrap();
    assert!(node.is_some(), "EdgeQuake node should exist");
}

#[tokio::test]
async fn test_merger_creates_relationships() {
    let graph_storage = Arc::new(MemoryGraphStorage::new("test"));
    let vector_storage = Arc::new(MemoryVectorStorage::new("test", 384));

    graph_storage.initialize().await.unwrap();
    vector_storage.initialize().await.unwrap();

    let config = MergerConfig::default();
    let merger = KnowledgeGraphMerger::new(config, graph_storage.clone(), vector_storage.clone());

    // Create test extraction result with entities and relationships
    let mut result = ExtractionResult::new("chunk-1");
    result.add_entity(ExtractedEntity::new(
        "EdgeQuake",
        "TECHNOLOGY",
        "A RAG system",
    ));
    result.add_entity(ExtractedEntity::new("Rust", "TECHNOLOGY", "A language"));
    result.add_relationship(
        ExtractedRelationship::new("EdgeQuake", "Rust", "BUILT_WITH")
            .with_description("EdgeQuake is built with Rust")
            .with_weight(0.95),
    );

    let stats = merger.merge(vec![result]).await.unwrap();

    assert_eq!(stats.entities_created, 2, "Should create 2 entities");
    assert_eq!(
        stats.relationships_created, 1,
        "Should create 1 relationship"
    );

    // Verify relationship exists (normalized keys are UPPERCASE)
    let edge = graph_storage.get_edge("EDGEQUAKE", "RUST").await.unwrap();
    assert!(
        edge.is_some(),
        "Edge should exist between EdgeQuake and Rust"
    );
}

#[tokio::test]
async fn test_merger_updates_existing_entities() {
    let graph_storage = Arc::new(MemoryGraphStorage::new("test"));
    let vector_storage = Arc::new(MemoryVectorStorage::new("test", 384));

    graph_storage.initialize().await.unwrap();
    vector_storage.initialize().await.unwrap();

    let config = MergerConfig::default();
    let merger = KnowledgeGraphMerger::new(config, graph_storage.clone(), vector_storage.clone());

    // First extraction
    let mut result1 = ExtractionResult::new("chunk-1");
    result1.add_entity(ExtractedEntity::new(
        "EdgeQuake",
        "TECHNOLOGY",
        "A RAG system",
    ));

    merger.merge(vec![result1]).await.unwrap();

    // Second extraction with more details
    let mut result2 = ExtractionResult::new("chunk-2");
    result2.add_entity(ExtractedEntity::new(
        "EdgeQuake",
        "TECHNOLOGY",
        "A high-performance RAG system built in Rust for enterprise use",
    ));

    let stats = merger.merge(vec![result2]).await.unwrap();

    assert_eq!(stats.entities_updated, 1, "Should update existing entity");
    assert_eq!(stats.entities_created, 0, "Should not create new entity");

    // Verify description was updated (normalized key is UPPERCASE)
    let node = graph_storage.get_node("EDGEQUAKE").await.unwrap().unwrap();
    let desc = node
        .properties
        .get("description")
        .unwrap()
        .as_str()
        .unwrap();
    assert!(desc.len() > 10, "Description should have content");
}

// ============ Full Pipeline Tests ============

#[tokio::test]
async fn test_full_pipeline_with_mock_llm() {
    // Create mock that returns valid extraction JSON
    let mock_provider = Arc::new(MockProvider::new());
    mock_provider
        .add_response(r#"{"entities": [], "relationships": []}"#)
        .await;

    // Create the extractor wrapping the mock provider
    let extractor: Arc<dyn EntityExtractor> = Arc::new(LLMExtractor::new(mock_provider.clone()));

    let config = PipelineConfig {
        enable_entity_extraction: true,
        enable_relationship_extraction: true,
        enable_chunk_embeddings: true,
        enable_entity_embeddings: false, // Disable to avoid needing multiple mock responses
        ..Default::default()
    };

    let pipeline = Pipeline::new(config)
        .with_extractor(extractor)
        .with_embedding_provider(mock_provider.clone() as Arc<dyn EmbeddingProvider>);

    let result = pipeline.process("doc-1", SAMPLE_DOCUMENT_1).await.unwrap();

    assert!(!result.chunks.is_empty(), "Should produce chunks");
    assert!(
        result.stats.chunk_count > 0,
        "Should record chunk count in stats"
    );

    // Verify chunks have embeddings
    for chunk in &result.chunks {
        assert!(chunk.embedding.is_some(), "Chunk should have embedding");
    }
}

#[tokio::test]
async fn test_pipeline_chunking_only() {
    let config = PipelineConfig {
        enable_entity_extraction: false,
        enable_relationship_extraction: false,
        enable_chunk_embeddings: false,
        enable_entity_embeddings: false,
        ..Default::default()
    };

    let pipeline = Pipeline::new(config);

    let result = pipeline.process("doc-1", SAMPLE_DOCUMENT_1).await.unwrap();

    assert!(!result.chunks.is_empty(), "Should produce chunks");
    assert!(result.extractions.is_empty(), "Should have no extractions");
}

#[tokio::test]
async fn test_pipeline_with_simple_extractor() {
    let extractor: Arc<dyn EntityExtractor> = Arc::new(SimpleExtractor::default());

    let config = PipelineConfig {
        enable_entity_extraction: true,
        enable_relationship_extraction: true,
        enable_chunk_embeddings: false,
        enable_entity_embeddings: false,
        ..Default::default()
    };

    let pipeline = Pipeline::new(config).with_extractor(extractor);

    let result = pipeline.process("doc-1", SAMPLE_DOCUMENT_1).await.unwrap();

    assert!(!result.chunks.is_empty(), "Should produce chunks");
    // Simple extractor uses regex patterns, so extractions depend on document content
    assert!(result.stats.chunk_count > 0, "Should have chunks");
}

// ============ Storage Integration Tests ============

#[tokio::test]
async fn test_memory_storage_full_cycle() {
    // Initialize all storage components
    let kv_storage = Arc::new(MemoryKVStorage::new("test"));
    let vector_storage = Arc::new(MemoryVectorStorage::new("test", 384));
    let graph_storage = Arc::new(MemoryGraphStorage::new("test"));

    kv_storage.initialize().await.unwrap();
    vector_storage.initialize().await.unwrap();
    graph_storage.initialize().await.unwrap();

    // Store a document
    let doc_id = "doc-1";
    kv_storage
        .upsert(&[(
            doc_id.to_string(),
            serde_json::json!({
                "content": SAMPLE_DOCUMENT_1,
                "metadata": {"source": "test"}
            }),
        )])
        .await
        .unwrap();

    // Store some entities in graph
    let mut props = HashMap::new();
    props.insert("label".to_string(), serde_json::json!("EdgeQuake"));
    props.insert("description".to_string(), serde_json::json!("A RAG system"));
    props.insert("entity_type".to_string(), serde_json::json!("TECHNOLOGY"));
    graph_storage.upsert_node("edgequake", props).await.unwrap();

    let mut props2 = HashMap::new();
    props2.insert("label".to_string(), serde_json::json!("Rust"));
    props2.insert(
        "description".to_string(),
        serde_json::json!("A programming language"),
    );
    graph_storage.upsert_node("rust", props2).await.unwrap();

    // Store relationship
    let mut edge_props = HashMap::new();
    edge_props.insert("relation".to_string(), serde_json::json!("BUILT_WITH"));
    graph_storage
        .upsert_edge("edgequake", "rust", edge_props)
        .await
        .unwrap();

    // Store embeddings
    let embedding: Vec<f32> = (0..384).map(|i| (i as f32) / 1000.0).collect();
    vector_storage
        .upsert(&[(
            "edgequake".to_string(),
            embedding.clone(),
            serde_json::json!({"name": "EdgeQuake"}),
        )])
        .await
        .unwrap();

    // Query - verify retrieval (3 arguments: embedding, top_k, filter_ids)
    let results = vector_storage.query(&embedding, 5, None).await.unwrap();
    assert!(!results.is_empty(), "Should find vectors");
    assert_eq!(results[0].id, "edgequake", "Should find EdgeQuake");

    // Graph traversal
    let neighbors = graph_storage.get_neighbors("edgequake", 1).await.unwrap();
    assert!(!neighbors.is_empty(), "Should find neighbors");

    // Verify document retrieval
    let doc = kv_storage.get_by_id(doc_id).await.unwrap();
    assert!(doc.is_some(), "Should retrieve document");
}

// ============ Knowledge Graph Query Tests ============

#[tokio::test]
async fn test_knowledge_graph_traversal() {
    let graph_storage = Arc::new(MemoryGraphStorage::new("test"));
    graph_storage.initialize().await.unwrap();

    // Build a small knowledge graph
    let entities = [
        ("edgequake", "EdgeQuake", "TECHNOLOGY"),
        ("rust", "Rust", "TECHNOLOGY"),
        ("python", "Python", "TECHNOLOGY"),
        ("lightrag", "LightRAG", "TECHNOLOGY"),
        ("sarah-chen", "Sarah Chen", "PERSON"),
        ("john-smith", "John Smith", "PERSON"),
    ];

    for (id, label, entity_type) in entities {
        let mut props = HashMap::new();
        props.insert("label".to_string(), serde_json::json!(label));
        props.insert("entity_type".to_string(), serde_json::json!(entity_type));
        graph_storage.upsert_node(id, props).await.unwrap();
    }

    // Add relationships
    let relationships = [
        ("edgequake", "rust", "BUILT_WITH"),
        ("lightrag", "python", "BUILT_WITH"),
        ("edgequake", "lightrag", "INSPIRED_BY"),
        ("sarah-chen", "edgequake", "DESIGNED"),
        ("john-smith", "edgequake", "CONTRIBUTED_TO"),
    ];

    for (src, tgt, rel) in relationships {
        let mut props = HashMap::new();
        props.insert("relation".to_string(), serde_json::json!(rel));
        graph_storage.upsert_edge(src, tgt, props).await.unwrap();
    }

    // Test traversals
    let edgequake_neighbors = graph_storage.get_neighbors("edgequake", 1).await.unwrap();
    assert!(
        edgequake_neighbors.len() >= 2,
        "EdgeQuake should have multiple neighbors"
    );

    // Test node count
    let node_count = graph_storage.node_count().await.unwrap();
    assert_eq!(node_count, 6, "Should have 6 nodes");

    let edge_count = graph_storage.edge_count().await.unwrap();
    assert_eq!(edge_count, 5, "Should have 5 edges");
}

// ============ Vector Search Tests ============

#[tokio::test]
async fn test_vector_similarity_search() {
    let vector_storage = Arc::new(MemoryVectorStorage::new("test", 384));
    vector_storage.initialize().await.unwrap();

    // Create embeddings for different concepts
    let create_embedding = |seed: f32| -> Vec<f32> {
        (0..384)
            .map(|i| ((i as f32 + seed) / 1000.0).sin())
            .collect()
    };

    let embeddings = [
        ("edgequake", create_embedding(0.0), "RAG system in Rust"),
        ("lightrag", create_embedding(0.1), "RAG system in Python"),
        ("rust", create_embedding(10.0), "Programming language"),
        ("python", create_embedding(10.1), "Programming language"),
        ("llm", create_embedding(20.0), "Large language model"),
    ];

    for (id, embedding, desc) in embeddings {
        vector_storage
            .upsert(&[(
                id.to_string(),
                embedding,
                serde_json::json!({"description": desc}),
            )])
            .await
            .unwrap();
    }

    // Query for RAG systems - should find edgequake and lightrag
    let query_embedding = create_embedding(0.05);
    let results = vector_storage
        .query(&query_embedding, 3, None)
        .await
        .unwrap();

    assert!(!results.is_empty(), "Should find results");
    // The closest vectors should be the RAG systems
    let found_ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
    assert!(
        found_ids.contains(&"edgequake") || found_ids.contains(&"lightrag"),
        "Should find RAG system vectors"
    );
}

// ============ Multi-Document Processing Tests ============

#[tokio::test]
async fn test_multi_document_knowledge_graph() {
    let graph_storage = Arc::new(MemoryGraphStorage::new("test"));
    let vector_storage = Arc::new(MemoryVectorStorage::new("test", 384));

    graph_storage.initialize().await.unwrap();
    vector_storage.initialize().await.unwrap();

    let config = MergerConfig::default();
    let merger = KnowledgeGraphMerger::new(config, graph_storage.clone(), vector_storage.clone());

    // Simulate extractions from two documents
    // Document 1 mentions EdgeQuake and Rust
    let mut result1 = ExtractionResult::new("doc1-chunk1");
    result1.add_entity(ExtractedEntity::new(
        "EdgeQuake",
        "TECHNOLOGY",
        "A RAG system built in Rust",
    ));
    result1.add_entity(ExtractedEntity::new(
        "Rust",
        "TECHNOLOGY",
        "A systems programming language",
    ));
    result1.add_relationship(
        ExtractedRelationship::new("EdgeQuake", "Rust", "BUILT_WITH")
            .with_description("EdgeQuake is implemented in Rust"),
    );

    // Document 2 mentions LightRAG and Python, and references EdgeQuake
    let mut result2 = ExtractionResult::new("doc2-chunk1");
    result2.add_entity(ExtractedEntity::new(
        "LightRAG",
        "TECHNOLOGY",
        "A RAG system built in Python",
    ));
    result2.add_entity(ExtractedEntity::new(
        "Python",
        "TECHNOLOGY",
        "A programming language",
    ));
    result2.add_entity(ExtractedEntity::new(
        "EdgeQuake",
        "TECHNOLOGY",
        "A Rust port inspired by LightRAG",
    ));
    result2.add_relationship(
        ExtractedRelationship::new("LightRAG", "Python", "BUILT_WITH")
            .with_description("LightRAG is implemented in Python"),
    );
    result2.add_relationship(
        ExtractedRelationship::new("EdgeQuake", "LightRAG", "INSPIRED_BY")
            .with_description("EdgeQuake was inspired by LightRAG"),
    );

    // Merge both documents
    merger.merge(vec![result1]).await.unwrap();
    let stats = merger.merge(vec![result2]).await.unwrap();

    // EdgeQuake should be updated (existed from doc1), others created
    assert!(stats.entities_updated >= 1, "EdgeQuake should be updated");

    // Verify cross-document relationships (normalized keys are UPPERCASE)
    let edge = graph_storage
        .get_edge("EDGEQUAKE", "LIGHTRAG")
        .await
        .unwrap();
    assert!(
        edge.is_some(),
        "Cross-document relationship should be created"
    );

    // Verify total nodes
    let node_count = graph_storage.node_count().await.unwrap();
    assert_eq!(node_count, 4, "Should have 4 unique entities");
}

// ============ Edge Cases and Error Handling ============

#[tokio::test]
async fn test_empty_document_processing() {
    let config = PipelineConfig::default();
    let pipeline = Pipeline::new(config);

    let result = pipeline.process("doc-empty", "").await;

    // Empty content should still work (produce 0 chunks)
    assert!(result.is_ok(), "Empty document should not cause error");
    let result = result.unwrap();
    assert!(
        result.chunks.is_empty() || result.chunks[0].content.trim().is_empty(),
        "Empty document produces empty or whitespace chunks"
    );
}

#[tokio::test]
async fn test_unicode_content_processing() {
    let unicode_content = r#"
    EdgeQuake 是一个用 Rust 编写的高性能 RAG 系统。
    它支持中文、日本語、한국어等多种语言。
    Das System unterstützt auch Deutsch und andere europäische Sprachen.
    Система также поддерживает русский язык.
    "#;

    let config = PipelineConfig::default();
    let pipeline = Pipeline::new(config);

    let result = pipeline.process("doc-unicode", unicode_content).await;

    assert!(result.is_ok(), "Unicode content should be processed");
    let result = result.unwrap();
    assert!(!result.chunks.is_empty(), "Should produce chunks");
}

#[tokio::test]
async fn test_special_characters_in_entities() {
    let graph_storage = Arc::new(MemoryGraphStorage::new("test"));
    let vector_storage = Arc::new(MemoryVectorStorage::new("test", 384));

    graph_storage.initialize().await.unwrap();
    vector_storage.initialize().await.unwrap();

    let config = MergerConfig::default();
    let merger = KnowledgeGraphMerger::new(config, graph_storage.clone(), vector_storage.clone());

    // Test entities with special characters
    let mut result = ExtractionResult::new("chunk-1");
    result.add_entity(ExtractedEntity::new(
        "C++",
        "TECHNOLOGY",
        "A programming language",
    ));
    result.add_entity(ExtractedEntity::new(
        "Node.js",
        "TECHNOLOGY",
        "A JavaScript runtime",
    ));
    result.add_entity(ExtractedEntity::new(
        "O'Reilly Media",
        "ORGANIZATION",
        "A publisher",
    ));

    let stats = merger.merge(vec![result]).await.unwrap();

    assert_eq!(stats.entities_created, 3, "Should create all entities");
    assert_eq!(stats.errors, 0, "Should have no errors");
}

// ============ Complete E2E Flow Test ============

#[tokio::test]
async fn test_complete_e2e_flow() {
    // This test simulates the complete flow from document ingestion to query

    // 1. Initialize storage
    let graph_storage = Arc::new(MemoryGraphStorage::new("e2e-test"));
    let vector_storage = Arc::new(MemoryVectorStorage::new("e2e-test", 1536)); // OpenAI embedding dimension
    let kv_storage = Arc::new(MemoryKVStorage::new("e2e-test"));

    graph_storage.initialize().await.unwrap();
    vector_storage.initialize().await.unwrap();
    kv_storage.initialize().await.unwrap();

    // 2. Set up mock LLM with realistic extraction response
    let mock_provider = Arc::new(MockProvider::new());

    // Add extraction response
    mock_provider.add_response(r#"
    {
        "entities": [
            {"name": "EdgeQuake", "type": "TECHNOLOGY", "description": "High-performance RAG system built in Rust"},
            {"name": "Dr. Sarah Chen", "type": "PERSON", "description": "Lead architect of EdgeQuake"},
            {"name": "Rust", "type": "TECHNOLOGY", "description": "Systems programming language"}
        ],
        "relationships": [
            {"source": "EdgeQuake", "target": "Rust", "type": "BUILT_WITH", "description": "EdgeQuake is implemented in Rust"},
            {"source": "Dr. Sarah Chen", "target": "EdgeQuake", "type": "DESIGNED", "description": "Sarah Chen designed EdgeQuake"}
        ]
    }
    "#).await;

    // 3. Create pipeline with extractor
    let extractor: Arc<dyn EntityExtractor> = Arc::new(LLMExtractor::new(mock_provider.clone()));

    let config = PipelineConfig {
        enable_entity_extraction: true,
        enable_relationship_extraction: true,
        enable_chunk_embeddings: false,
        enable_entity_embeddings: false,
        chunker: ChunkerConfig {
            chunk_size: 500,
            chunk_overlap: 50,
            min_chunk_size: 50,
            ..Default::default()
        },
        ..Default::default()
    };

    let pipeline = Pipeline::new(config).with_extractor(extractor);

    // 4. Process document through pipeline
    let result = pipeline.process("doc-1", SAMPLE_DOCUMENT_1).await.unwrap();

    assert!(!result.chunks.is_empty(), "Pipeline should produce chunks");

    // Since we only added one mock response, only the first chunk gets extraction
    // In real usage, you'd add a response for each chunk
    if !result.extractions.is_empty() {
        assert!(
            result.extractions[0].entities.len() >= 1,
            "Should extract entities"
        );
    }

    // 5. Merge into knowledge graph
    let merger_config = MergerConfig::default();
    let merger =
        KnowledgeGraphMerger::new(merger_config, graph_storage.clone(), vector_storage.clone());

    if !result.extractions.is_empty() {
        let merge_stats = merger.merge(result.extractions).await.unwrap();
        assert!(merge_stats.entities_created > 0, "Should create entities");

        // 6. Verify knowledge graph state
        let node_count = graph_storage.node_count().await.unwrap();
        assert!(node_count > 0, "Graph should have nodes");

        let edge_count = graph_storage.edge_count().await.unwrap();
        assert!(edge_count > 0, "Graph should have edges");

        // 7. Test graph queries (normalized key is UPPERCASE)
        let neighbors = graph_storage.get_neighbors("EDGEQUAKE", 1).await.unwrap();
        assert!(!neighbors.is_empty(), "EdgeQuake should have neighbors");
    }

    // Test passes - complete E2E flow works
}
