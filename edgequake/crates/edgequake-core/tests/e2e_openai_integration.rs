#![cfg(feature = "pipeline")]

//! OpenAI Integration Tests
//!
//! These tests verify the complete pipeline with real OpenAI LLM provider.
//! They are conditionally compiled and only run when OPENAI_API_KEY is set.
//!
//! Run with:
//! ```bash
//! export OPENAI_API_KEY="sk-your-key"
//! cargo test --package edgequake-core --test e2e_openai_integration
//! ```

use std::collections::HashSet;
use std::env;
use std::sync::Arc;

use edgequake_core::{EdgeQuake, EdgeQuakeConfig, QueryParams, StorageBackend, StorageConfig};
use edgequake_llm::{EmbeddingProvider, LLMProvider, OpenAIProvider};
use edgequake_storage::{GraphStorage, MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage};

// ============================================================================
// Test Configuration
// ============================================================================

/// Check if OpenAI API key is available for real LLM testing.
fn has_openai_key() -> bool {
    env::var("OPENAI_API_KEY")
        .map(|k| !k.is_empty() && k != "test-key")
        .unwrap_or(false)
}

/// Skip test if OpenAI API key is not available.
macro_rules! require_openai {
    () => {
        if !has_openai_key() {
            eprintln!("⏭️  Skipping test: OPENAI_API_KEY not set");
            return;
        }
    };
}

/// Create OpenAI provider with production configuration.
fn create_openai_provider() -> Arc<OpenAIProvider> {
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    Arc::new(
        OpenAIProvider::new(api_key)
            .with_model("gpt-4o-mini") // Cost-effective and fast
            .with_embedding_model("text-embedding-3-small"),
    )
}

/// Create memory storage backends for testing.
fn create_memory_backends(
    namespace: &str,
) -> (
    Arc<MemoryKVStorage>,
    Arc<MemoryVectorStorage>,
    Arc<MemoryGraphStorage>,
) {
    (
        Arc::new(MemoryKVStorage::new(namespace)),
        Arc::new(MemoryVectorStorage::new(namespace, 1536)),
        Arc::new(MemoryGraphStorage::new(namespace)),
    )
}

// ============================================================================
// Sample Documents
// ============================================================================

const DOC_EDGEQUAKE: &str = r#"
EdgeQuake is a high-performance Retrieval-Augmented Generation (RAG) system built in Rust. 
The system was designed by Sarah Chen, who serves as the lead architect. EdgeQuake integrates 
multiple advanced technologies including Apache AGE for graph storage and pgvector for 
similarity search.
"#;

const DOC_ARCHITECTURE: &str = r#"
The architecture follows a modular design with clear separation of concerns. The storage layer 
supports multiple backends including PostgreSQL with AGE extension for production deployments. 
Michael Torres leads the LLM integration team, working closely with Sarah Chen to ensure optimal 
performance.
"#;

const DOC_TEAM: &str = r#"
The core team consists of Sarah Chen (Lead Architect), Michael Torres (LLM Integration Lead),
and several other engineers. They collaborate using modern DevOps practices and continuous
integration. The project is open source and welcomes contributions from the community.
"#;

// ============================================================================
// Entity Extraction Tests
// ============================================================================

#[tokio::test]
async fn test_openai_entity_extraction() {
    require_openai!();

    println!("🔑 Testing with REAL OpenAI provider");

    let provider = create_openai_provider();
    let (kv_storage, vector_storage, graph_storage) = create_memory_backends("openai_test_1");

    let config = EdgeQuakeConfig::new()
        .with_namespace("openai_test_1")
        .with_storage(StorageConfig {
            backend: StorageBackend::Memory,
            ..Default::default()
        });

    let mut edgequake = EdgeQuake::new(config)
        .with_storage_backends(kv_storage, vector_storage, graph_storage.clone())
        .with_providers(
            provider.clone() as Arc<dyn LLMProvider>,
            provider as Arc<dyn EmbeddingProvider>,
        );

    edgequake.initialize().await.expect("Failed to initialize");

    // Insert document
    let result = edgequake
        .insert(DOC_EDGEQUAKE, Some("doc-001"))
        .await
        .expect("Failed to insert document");

    println!("📊 Extraction Results:");
    println!("   Chunks created: {}", result.chunks_created);
    println!("   Entities extracted: {}", result.entities_extracted);
    println!("   Relationships: {}", result.relationships_extracted);

    // Verify entities were extracted
    assert!(
        result.entities_extracted > 0,
        "Should extract at least 1 entity"
    );

    // With real LLM, we expect more entities than mock
    // Real LLM typically extracts 5-10 entities from this document
    println!(
        "✓ Entity extraction passed with {} entities",
        result.entities_extracted
    );
}

#[tokio::test]
async fn test_openai_multi_document_extraction() {
    require_openai!();

    println!("🔑 Testing multi-document extraction with REAL OpenAI");

    let provider = create_openai_provider();
    let (kv_storage, vector_storage, graph_storage) = create_memory_backends("openai_test_2");

    let config = EdgeQuakeConfig::new()
        .with_namespace("openai_test_2")
        .with_storage(StorageConfig {
            backend: StorageBackend::Memory,
            ..Default::default()
        });

    let mut edgequake = EdgeQuake::new(config)
        .with_storage_backends(kv_storage, vector_storage, graph_storage.clone())
        .with_providers(
            provider.clone() as Arc<dyn LLMProvider>,
            provider as Arc<dyn EmbeddingProvider>,
        );

    edgequake.initialize().await.expect("Failed to initialize");

    let documents = [
        ("doc-001", DOC_EDGEQUAKE),
        ("doc-002", DOC_ARCHITECTURE),
        ("doc-003", DOC_TEAM),
    ];

    let mut total_entities = 0;
    let mut total_relationships = 0;

    for (doc_id, content) in documents {
        let result = edgequake
            .insert(content, Some(doc_id))
            .await
            .expect("Failed to insert document");

        println!(
            "📄 {}: {} entities, {} relationships",
            doc_id, result.entities_extracted, result.relationships_extracted
        );

        total_entities += result.entities_extracted;
        total_relationships += result.relationships_extracted;
    }

    println!("\n📊 Total Results:");
    println!("   Documents: {}", documents.len());
    println!("   Total Entities: {}", total_entities);
    println!("   Total Relationships: {}", total_relationships);

    // Real LLM should extract many entities across documents
    assert!(
        total_entities >= 5,
        "Should extract at least 5 entities across documents"
    );

    // Verify graph has nodes
    let graph_node_count = graph_storage
        .node_count()
        .await
        .expect("Failed to get node count");
    println!("   Graph Nodes (deduplicated): {}", graph_node_count);

    // Graph should have extracted entities - exact count may vary
    // Node count includes all unique entities across documents
    assert!(
        graph_node_count >= 3,
        "Graph should have at least 3 unique entities"
    );
    println!("✓ Multi-document extraction passed");
}

// ============================================================================
// Query Tests
// ============================================================================

#[tokio::test]
async fn test_openai_query_response() {
    require_openai!();

    println!("🔑 Testing query with REAL OpenAI");

    let provider = create_openai_provider();
    let (kv_storage, vector_storage, graph_storage) = create_memory_backends("openai_test_3");

    let config = EdgeQuakeConfig::new()
        .with_namespace("openai_test_3")
        .with_storage(StorageConfig {
            backend: StorageBackend::Memory,
            ..Default::default()
        });

    let mut edgequake = EdgeQuake::new(config)
        .with_storage_backends(kv_storage, vector_storage, graph_storage.clone())
        .with_providers(
            provider.clone() as Arc<dyn LLMProvider>,
            provider as Arc<dyn EmbeddingProvider>,
        );

    edgequake.initialize().await.expect("Failed to initialize");

    // Insert documents
    for (doc_id, content) in [
        ("doc-001", DOC_EDGEQUAKE),
        ("doc-002", DOC_ARCHITECTURE),
        ("doc-003", DOC_TEAM),
    ] {
        edgequake
            .insert(content, Some(doc_id))
            .await
            .expect("Failed to insert");
    }

    // Query
    let response = edgequake
        .query("Who is the lead architect of EdgeQuake?", None)
        .await
        .expect("Failed to query");

    println!("📝 Query Response:");
    println!("   Answer: {}", response.response);
    println!("   Context entities: {}", response.context.entities.len());
    println!("   Context chunks: {}", response.context.chunks.len());

    // Real LLM should give a meaningful answer
    let answer_lower = response.response.to_lowercase();
    assert!(
        answer_lower.contains("sarah") || answer_lower.contains("chen"),
        "Answer should mention Sarah Chen"
    );

    println!("✓ Query response passed");
}

// ============================================================================
// Embedding Tests
// ============================================================================

#[tokio::test]
async fn test_openai_embeddings() {
    require_openai!();

    println!("🔑 Testing embeddings with REAL OpenAI");

    let provider = create_openai_provider();

    // Generate embeddings
    let texts = vec![
        "EdgeQuake is a RAG system".to_string(),
        "Sarah Chen is the lead architect".to_string(),
        "The system uses Rust programming language".to_string(),
    ];

    let embeddings = provider.embed(&texts).await.expect("Failed to embed");

    println!("📊 Embedding Results:");
    println!("   Text count: {}", texts.len());
    println!("   Embedding count: {}", embeddings.len());
    println!("   Embedding dimensions: {}", embeddings[0].len());

    assert_eq!(embeddings.len(), texts.len());
    assert_eq!(
        embeddings[0].len(),
        1536,
        "text-embedding-3-small has 1536 dimensions"
    );

    // Verify embeddings are normalized (L2 norm ≈ 1)
    let norm: f32 = embeddings[0].iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 0.1, "Embeddings should be normalized");

    println!("✓ Embedding test passed");
}

#[tokio::test]
async fn test_openai_semantic_similarity() {
    require_openai!();

    println!("🔑 Testing semantic similarity with REAL OpenAI");

    let provider = create_openai_provider();

    let texts = vec![
        "EdgeQuake is a graph-based RAG system".to_string(),
        "EdgeQuake uses knowledge graphs for retrieval".to_string(),
        "The weather is sunny today".to_string(),
    ];

    let embeddings = provider.embed(&texts).await.expect("Failed to embed");

    // Calculate cosine similarity
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        dot / (norm_a * norm_b)
    }

    let sim_0_1 = cosine_similarity(&embeddings[0], &embeddings[1]);
    let sim_0_2 = cosine_similarity(&embeddings[0], &embeddings[2]);

    println!("📊 Similarity Scores:");
    println!(
        "   '{}...' vs '{}...'",
        &texts[0].chars().take(30).collect::<String>(),
        &texts[1].chars().take(30).collect::<String>()
    );
    println!("   Similarity: {:.4}", sim_0_1);
    println!(
        "   '{}...' vs '{}'",
        &texts[0].chars().take(30).collect::<String>(),
        &texts[2]
    );
    println!("   Similarity: {:.4}", sim_0_2);

    // Similar texts should have higher similarity
    assert!(
        sim_0_1 > sim_0_2,
        "Similar texts should have higher similarity score"
    );

    println!("✓ Semantic similarity test passed");
}

// ============================================================================
// Entity Deduplication Tests
// ============================================================================

#[tokio::test]
async fn test_openai_entity_deduplication() {
    require_openai!();

    println!("🔑 Testing entity deduplication with REAL OpenAI");

    let provider = create_openai_provider();
    let (kv_storage, vector_storage, graph_storage) = create_memory_backends("openai_test_5");

    let config = EdgeQuakeConfig::new()
        .with_namespace("openai_test_5")
        .with_storage(StorageConfig {
            backend: StorageBackend::Memory,
            ..Default::default()
        });

    let mut edgequake = EdgeQuake::new(config)
        .with_storage_backends(kv_storage, vector_storage, graph_storage.clone())
        .with_providers(
            provider.clone() as Arc<dyn LLMProvider>,
            provider as Arc<dyn EmbeddingProvider>,
        );

    edgequake.initialize().await.expect("Failed to initialize");

    // Documents with overlapping entities
    let doc1 = "Sarah Chen leads the EdgeQuake project. She designed the core architecture.";
    let doc2 = "The lead architect Sarah Chen works with Michael Torres on LLM integration.";
    let doc3 = "Sarah Chen and Michael Torres are key contributors to EdgeQuake.";

    let mut total_entities = 0;

    for (i, doc) in [doc1, doc2, doc3].iter().enumerate() {
        let result = edgequake
            .insert(doc, Some(&format!("doc-{}", i + 1)))
            .await
            .expect("Failed to insert");
        total_entities += result.entities_extracted;
    }

    let graph_nodes = graph_storage.node_count().await.expect("Failed to count");

    println!("📊 Deduplication Results:");
    println!("   Total entities extracted: {}", total_entities);
    println!("   Unique nodes in graph: {}", graph_nodes);
    println!(
        "   Deduplication ratio: {:.1}%",
        (1.0 - graph_nodes as f64 / total_entities as f64) * 100.0
    );

    // Should have deduplicated some entities (Sarah Chen, EdgeQuake mentioned multiple times)
    assert!(
        graph_nodes < total_entities,
        "Should have fewer nodes than total extracted entities"
    );

    println!("✓ Entity deduplication test passed");
}

// ============================================================================
// Quality Metrics Tests
// ============================================================================

#[tokio::test]
async fn test_openai_extraction_quality() {
    require_openai!();

    println!("🔑 Testing extraction quality with REAL OpenAI");

    let provider = create_openai_provider();
    let (kv_storage, vector_storage, graph_storage) = create_memory_backends("openai_test_6");

    let config = EdgeQuakeConfig::new()
        .with_namespace("openai_test_6")
        .with_storage(StorageConfig {
            backend: StorageBackend::Memory,
            ..Default::default()
        });

    let mut edgequake = EdgeQuake::new(config)
        .with_storage_backends(kv_storage, vector_storage, graph_storage.clone())
        .with_providers(
            provider.clone() as Arc<dyn LLMProvider>,
            provider as Arc<dyn EmbeddingProvider>,
        );

    edgequake.initialize().await.expect("Failed to initialize");

    // Insert all documents
    for (doc_id, content) in [
        ("doc-001", DOC_EDGEQUAKE),
        ("doc-002", DOC_ARCHITECTURE),
        ("doc-003", DOC_TEAM),
    ] {
        edgequake
            .insert(content, Some(doc_id))
            .await
            .expect("Failed to insert");
    }

    // Get all nodes and check expected entities are present
    let nodes = graph_storage
        .get_all_nodes()
        .await
        .expect("Failed to get nodes");
    let node_names: HashSet<String> = nodes.iter().map(|n| n.id.to_uppercase()).collect();

    println!("📊 Extracted Entities:");
    for name in &node_names {
        println!("   - {}", name);
    }

    // Check for expected key entities (normalized to uppercase)
    let expected_entities = [
        "SARAH_CHEN",
        "MICHAEL_TORRES",
        "EDGEQUAKE",
        "RUST",
        "POSTGRESQL",
    ];
    let mut found_count = 0;

    for expected in expected_entities {
        // Check if entity exists (may be variations like SARAH CHEN vs SARAH_CHEN)
        let found = node_names
            .iter()
            .any(|n: &String| n.contains(&expected.replace("_", " ")) || n.contains(expected));
        if found {
            found_count += 1;
            println!("   ✓ Found: {}", expected);
        } else {
            println!("   ⚠ Not found: {} (may have different name)", expected);
        }
    }

    println!("\n📊 Quality Metrics:");
    println!("   Expected entities: {}", expected_entities.len());
    println!("   Found: {}/{}", found_count, expected_entities.len());
    println!("   Total extracted: {}", nodes.len());

    // Should find at least some expected entities
    assert!(
        found_count >= 2,
        "Should find at least 2 of the expected key entities"
    );

    println!("✓ Extraction quality test passed");
}
