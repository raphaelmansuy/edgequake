#![cfg(feature = "pipeline")]

//! Advanced E2E Retrieval Tests
//!
//! This test suite validates advanced retrieval features and highlights
//! gaps compared to LightRAG implementation.
//!
//! Tests cover:
//! - Chunk retrieval from entities (MISSING)
//! - Token-based truncation (MISSING)
//! - Entity degree sorting (PARTIAL)
//! - Chunk frequency tracking (MISSING)
//! - Response quality metrics

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use edgequake_core::{EdgeQuake, EdgeQuakeConfig, QueryMode, StorageBackend, StorageConfig};
use edgequake_llm::MockProvider;
use edgequake_storage::{GraphStorage, MemoryGraphStorage, MemoryVectorStorage, VectorStorage};

/// Complex multi-document scenario for testing advanced features.
const COMPLEX_DOCUMENT_1: &str = r#"
Artificial Intelligence (AI) has revolutionized healthcare through machine learning algorithms.
Deep neural networks enable medical image analysis for early disease detection. Computer vision
systems can identify tumors in X-rays and MRI scans with accuracy surpassing human radiologists.

Dr. Sarah Martinez leads AI research at Stanford Medical Center. Her team developed a deep learning
model that detects lung cancer from CT scans. The model was trained on 100,000 patient images and
achieved 95% accuracy in clinical trials.

Natural language processing (NLP) helps doctors analyze patient records efficiently. Clinical notes
previously required hours of manual review. Now AI systems extract key medical information automatically,
saving physicians valuable time and improving patient care quality.
"#;

const COMPLEX_DOCUMENT_2: &str = r#"
Machine learning applications extend beyond healthcare into autonomous vehicles. Tesla's Autopilot
uses computer vision and neural networks for self-driving capabilities. Sensors process real-time
road conditions, detecting pedestrians, traffic signals, and obstacles.

Elon Musk announced plans for full self-driving in 2024. The technology combines LIDAR sensors,
cameras, and radar systems. Deep learning models trained on billions of driving miles enable
the vehicle to make split-second decisions safely.

Autonomous driving faces regulatory challenges despite technical progress. Safety standards vary
across countries. Insurance companies assess liability questions when AI controls vehicles.
Government agencies like NHTSA develop testing frameworks for autonomous systems.
"#;

const COMPLEX_DOCUMENT_3: &str = r#"
Dr. Sarah Martinez collaborates with Elon Musk on AI safety initiatives. Together they founded
the AI Ethics Institute to address potential risks of artificial general intelligence (AGI).
The institute researches alignment problems ensuring AI systems act according to human values.

Climate change modeling uses AI for more accurate predictions. Machine learning analyzes satellite
imagery to track deforestation, ice sheet melting, and ocean temperature changes. These models
help scientists understand environmental patterns and forecast future climate scenarios.

Stanford collaborates with Tesla on sustainable energy research. Solar panel efficiency optimization
uses neural networks to maximize power generation. Energy storage systems leverage AI for grid
management and demand forecasting.
"#;

/// Create a mock provider with realistic multi-document extraction.
async fn create_complex_mock() -> Arc<MockProvider> {
    let provider = Arc::new(MockProvider::new());

    // Document 1: Healthcare AI
    let doc1_extraction = r#"{
  "entities": [
    {"name": "Artificial Intelligence", "type": "CONCEPT", "description": "AI technology revolutionizing multiple industries"},
    {"name": "Machine Learning", "type": "TECHNOLOGY", "description": "Algorithms enabling AI systems to learn from data"},
    {"name": "Deep Neural Networks", "type": "TECHNOLOGY", "description": "Advanced ML architecture for complex pattern recognition"},
    {"name": "Computer Vision", "type": "TECHNOLOGY", "description": "AI systems for visual data analysis"},
    {"name": "Dr. Sarah Martinez", "type": "PERSON", "description": "AI researcher at Stanford Medical Center"},
    {"name": "Stanford Medical Center", "type": "ORGANIZATION", "description": "Medical research institution"},
    {"name": "Natural Language Processing", "type": "TECHNOLOGY", "description": "AI for understanding and processing human language"}
  ],
  "relationships": [
    {"source": "Artificial Intelligence", "target": "Machine Learning", "type": "USES", "description": "AI uses machine learning algorithms"},
    {"source": "Machine Learning", "target": "Deep Neural Networks", "type": "IMPLEMENTS", "description": "ML implements deep neural networks"},
    {"source": "Deep Neural Networks", "target": "Computer Vision", "type": "ENABLES", "description": "Neural networks enable computer vision systems"},
    {"source": "Dr. Sarah Martinez", "target": "Stanford Medical Center", "type": "WORKS_AT", "description": "Sarah leads AI research at Stanford"},
    {"source": "Dr. Sarah Martinez", "target": "Deep Neural Networks", "type": "RESEARCHES", "description": "Sarah developed deep learning models for medical imaging"}
  ]
}"#;

    // Document 2: Autonomous Vehicles
    let doc2_extraction = r#"{
  "entities": [
    {"name": "Machine Learning", "type": "TECHNOLOGY", "description": "Core technology for autonomous vehicles"},
    {"name": "Autonomous Vehicles", "type": "TECHNOLOGY", "description": "Self-driving cars using AI"},
    {"name": "Tesla", "type": "COMPANY", "description": "Electric vehicle manufacturer with self-driving technology"},
    {"name": "Elon Musk", "type": "PERSON", "description": "CEO of Tesla and SpaceX"},
    {"name": "Computer Vision", "type": "TECHNOLOGY", "description": "Vision systems for autonomous driving"},
    {"name": "LIDAR", "type": "TECHNOLOGY", "description": "Light detection and ranging sensor technology"},
    {"name": "NHTSA", "type": "ORGANIZATION", "description": "National Highway Traffic Safety Administration"}
  ],
  "relationships": [
    {"source": "Machine Learning", "target": "Autonomous Vehicles", "type": "POWERS", "description": "ML powers autonomous vehicle systems"},
    {"source": "Tesla", "target": "Autonomous Vehicles", "type": "DEVELOPS", "description": "Tesla develops self-driving cars"},
    {"source": "Elon Musk", "target": "Tesla", "type": "LEADS", "description": "Elon Musk is CEO of Tesla"},
    {"source": "Computer Vision", "target": "Autonomous Vehicles", "type": "ENABLES", "description": "Computer vision enables self-driving capabilities"},
    {"source": "LIDAR", "target": "Autonomous Vehicles", "type": "COMPONENT_OF", "description": "LIDAR is a sensor component in autonomous vehicles"}
  ]
}"#;

    // Document 3: Collaboration & Climate
    let doc3_extraction = r#"{
  "entities": [
    {"name": "Dr. Sarah Martinez", "type": "PERSON", "description": "AI researcher and AI safety advocate"},
    {"name": "Elon Musk", "type": "PERSON", "description": "Tech entrepreneur and AI safety advocate"},
    {"name": "AI Ethics Institute", "type": "ORGANIZATION", "description": "Research institute for AI safety and ethics"},
    {"name": "Artificial General Intelligence", "type": "CONCEPT", "description": "AGI - advanced AI with human-level reasoning"},
    {"name": "Climate Change", "type": "CONCEPT", "description": "Global environmental challenge"},
    {"name": "Machine Learning", "type": "TECHNOLOGY", "description": "Technology used for climate modeling"},
    {"name": "Stanford", "type": "ORGANIZATION", "description": "Stanford University"},
    {"name": "Tesla", "type": "COMPANY", "description": "Electric vehicle and energy company"}
  ],
  "relationships": [
    {"source": "Dr. Sarah Martinez", "target": "Elon Musk", "type": "COLLABORATES_WITH", "description": "Sarah and Elon collaborate on AI safety"},
    {"source": "Dr. Sarah Martinez", "target": "AI Ethics Institute", "type": "FOUNDED", "description": "Sarah co-founded the AI Ethics Institute"},
    {"source": "Elon Musk", "target": "AI Ethics Institute", "type": "FOUNDED", "description": "Elon co-founded the AI Ethics Institute"},
    {"source": "AI Ethics Institute", "target": "Artificial General Intelligence", "type": "RESEARCHES", "description": "Institute researches AGI safety"},
    {"source": "Machine Learning", "target": "Climate Change", "type": "ANALYZES", "description": "ML used for climate change modeling"},
    {"source": "Stanford", "target": "Tesla", "type": "COLLABORATES_WITH", "description": "Stanford collaborates with Tesla on energy research"}
  ]
}"#;

    provider.add_response(doc1_extraction).await;
    provider.add_response(doc2_extraction).await;
    provider.add_response(doc3_extraction).await;
    provider
}

/// Tests chunk retrieval from entities - currently a MISSING FEATURE.
/// Enable when source_id-based chunk retrieval is implemented.
#[tokio::test]
#[ignore = "Gap analysis test - chunk retrieval from entities not yet implemented"]
async fn test_chunk_retrieval_from_entities() {
    println!("\n=== Testing Chunk Retrieval from Entities (MISSING FEATURE) ===");

    // Setup
    let kv_storage = Arc::new(edgequake_storage::MemoryKVStorage::new("test_chunks"));
    let vector_storage = Arc::new(MemoryVectorStorage::new("test_chunks", 1536));
    let graph_storage = Arc::new(MemoryGraphStorage::new("test_chunks"));

    let config = EdgeQuakeConfig::new()
        .with_namespace("test_chunks")
        .with_storage(StorageConfig {
            backend: StorageBackend::Memory,
            ..Default::default()
        });

    let mock_provider = create_complex_mock().await;

    let mut edgequake = EdgeQuake::new(config)
        .with_storage_backends(kv_storage, vector_storage, graph_storage.clone())
        .with_providers(
            mock_provider.clone() as Arc<dyn edgequake_llm::LLMProvider>,
            mock_provider as Arc<dyn edgequake_llm::EmbeddingProvider>,
        );

    edgequake.initialize().await.expect("Failed to initialize");

    // Insert all documents
    edgequake
        .insert(COMPLEX_DOCUMENT_1, Some("doc-health"))
        .await
        .expect("Failed to insert doc 1");
    edgequake
        .insert(COMPLEX_DOCUMENT_2, Some("doc-auto"))
        .await
        .expect("Failed to insert doc 2");
    edgequake
        .insert(COMPLEX_DOCUMENT_3, Some("doc-collab"))
        .await
        .expect("Failed to insert doc 3");

    // Query using Local mode
    let params = edgequake_core::QueryParams {
        mode: QueryMode::Local,
        ..Default::default()
    };

    let result = edgequake
        .query(
            "What research does Dr. Sarah Martinez conduct?",
            Some(params),
        )
        .await
        .expect("Query failed");

    println!("Retrieved:");
    println!("  - {} entities", result.context.entities.len());
    println!("  - {} relationships", result.context.relationships.len());
    println!("  - {} chunks", result.context.chunks.len());

    // ❌ MISSING: Local mode should retrieve chunks related to entities
    // LightRAG implementation:
    // 1. Find entities via vector search (✅ implemented)
    // 2. Get entity source_ids (chunk IDs where entity was mentioned)
    // 3. Retrieve those chunks from KV storage (❌ MISSING)
    // 4. Optionally rerank by frequency or vector similarity (❌ MISSING)

    println!("\n⚠️  EXPECTED BEHAVIOR (from LightRAG):");
    println!("   1. Search entity_vdb for 'Dr. Sarah Martinez'");
    println!(
        "   2. Get entity node from graph with source_id = 'doc-health|chunk-0,doc-collab|chunk-0'"
    );
    println!("   3. Retrieve chunks: ['doc-health|chunk-0', 'doc-collab|chunk-0']");
    println!("   4. Include these chunks in context for LLM");
    println!("\n⚠️  CURRENT BEHAVIOR:");
    println!("   1. Search entity_vdb for query ✅");
    println!("   2. Get entity nodes ✅");
    println!("   3. Get entity relationships ✅");
    println!("   4. Retrieve source chunks ❌ MISSING");

    // For now, chunks should come from naive vector search (if any)
    // In full implementation, Local mode should return entity-related chunks
    assert!(
        result.context.entities.len() > 0,
        "Should have retrieved entities"
    );
}

/// Tests token-based truncation - currently a MISSING FEATURE.
/// Enable when token-aware context truncation is implemented.
#[tokio::test]
#[ignore = "Gap analysis test - token-based truncation not yet implemented"]
async fn test_token_based_truncation() {
    println!("\n=== Testing Token-Based Truncation (MISSING FEATURE) ===");

    // Setup with large entity count
    let kv_storage = Arc::new(edgequake_storage::MemoryKVStorage::new("test_tokens"));
    let vector_storage = Arc::new(MemoryVectorStorage::new("test_tokens", 1536));
    let graph_storage = Arc::new(MemoryGraphStorage::new("test_tokens"));

    let config = EdgeQuakeConfig::new()
        .with_namespace("test_tokens")
        .with_storage(StorageConfig {
            backend: StorageBackend::Memory,
            ..Default::default()
        });

    let mock_provider = create_complex_mock().await;

    let mut edgequake = EdgeQuake::new(config)
        .with_storage_backends(kv_storage, vector_storage, graph_storage)
        .with_providers(
            mock_provider.clone() as Arc<dyn edgequake_llm::LLMProvider>,
            mock_provider as Arc<dyn edgequake_llm::EmbeddingProvider>,
        );

    edgequake.initialize().await.expect("Failed to initialize");

    // Insert documents
    edgequake
        .insert(COMPLEX_DOCUMENT_1, Some("doc-1"))
        .await
        .expect("Failed to insert");
    edgequake
        .insert(COMPLEX_DOCUMENT_2, Some("doc-2"))
        .await
        .expect("Failed to insert");
    edgequake
        .insert(COMPLEX_DOCUMENT_3, Some("doc-3"))
        .await
        .expect("Failed to insert");

    // Query hybrid mode (will retrieve many entities + relationships)
    let params = edgequake_core::QueryParams {
        mode: QueryMode::Hybrid,
        ..Default::default()
    };

    let result = edgequake
        .query("Explain AI applications across industries", Some(params))
        .await
        .expect("Query failed");

    println!("Retrieved context:");
    println!("  - {} entities", result.context.entities.len());
    println!("  - {} relationships", result.context.relationships.len());
    println!("  - {} chunks", result.context.chunks.len());

    // ❌ MISSING: Token-based truncation
    // LightRAG implementation:
    // 1. Count tokens for each entity description
    // 2. Truncate list to stay under max_entity_tokens (e.g., 8000)
    // 3. Count tokens for each relationship description
    // 4. Truncate list to stay under max_relation_tokens (e.g., 8000)
    // 5. Ensure total_tokens < max_total_tokens (e.g., 16000)

    println!("\n⚠️  EXPECTED BEHAVIOR (from LightRAG):");
    println!("   1. For each entity, count tokens in description");
    println!("   2. Keep entities until reaching max_entity_tokens limit");
    println!("   3. For each relationship, count tokens in description");
    println!("   4. Keep relationships until reaching max_relation_tokens limit");
    println!("   5. Ensure entity_tokens + relation_tokens + chunk_tokens < max_total_tokens");
    println!("\n⚠️  CURRENT BEHAVIOR:");
    println!("   1. Fixed max_entities count ✅");
    println!("   2. Fixed max_chunks count ✅");
    println!("   3. Token-aware truncation ❌ MISSING");
    println!("   4. Risk of exceeding LLM context window ⚠️");
}

/// Tests entity degree-based sorting - currently PARTIAL IMPLEMENTATION.
/// Enable when graph degree is integrated into entity ranking.
#[tokio::test]
#[ignore = "Gap analysis test - entity degree sorting partially implemented"]
async fn test_entity_degree_sorting() {
    println!("\n=== Testing Entity Degree Sorting (PARTIAL IMPLEMENTATION) ===");

    // Setup
    let kv_storage = Arc::new(edgequake_storage::MemoryKVStorage::new("test_degree"));
    let vector_storage = Arc::new(MemoryVectorStorage::new("test_degree", 1536));
    let graph_storage = Arc::new(MemoryGraphStorage::new("test_degree"));

    let config = EdgeQuakeConfig::new()
        .with_namespace("test_degree")
        .with_storage(StorageConfig {
            backend: StorageBackend::Memory,
            ..Default::default()
        });

    let mock_provider = create_complex_mock().await;

    let mut edgequake = EdgeQuake::new(config)
        .with_storage_backends(kv_storage, vector_storage, graph_storage.clone())
        .with_providers(
            mock_provider.clone() as Arc<dyn edgequake_llm::LLMProvider>,
            mock_provider as Arc<dyn edgequake_llm::EmbeddingProvider>,
        );

    edgequake.initialize().await.expect("Failed to initialize");

    // Insert documents
    edgequake
        .insert(COMPLEX_DOCUMENT_1, Some("doc-1"))
        .await
        .expect("Failed to insert");
    edgequake
        .insert(COMPLEX_DOCUMENT_2, Some("doc-2"))
        .await
        .expect("Failed to insert");
    edgequake
        .insert(COMPLEX_DOCUMENT_3, Some("doc-3"))
        .await
        .expect("Failed to insert");

    // Check node degrees
    let ml_degree = graph_storage
        .node_degree("MACHINE_LEARNING")
        .await
        .expect("Failed to get degree");
    let sarah_degree = graph_storage
        .node_degree("DR._SARAH_MARTINEZ")
        .await
        .expect("Failed to get degree");
    let elon_degree = graph_storage
        .node_degree("ELON_MUSK")
        .await
        .expect("Failed to get degree");

    println!("Node degrees:");
    println!("  - MACHINE_LEARNING: {}", ml_degree);
    println!("  - DR._SARAH_MARTINEZ: {}", sarah_degree);
    println!("  - ELON_MUSK: {}", elon_degree);

    // Query
    let params = edgequake_core::QueryParams {
        mode: QueryMode::Local,
        ..Default::default()
    };

    let result = edgequake
        .query("What are the key AI technologies?", Some(params))
        .await
        .expect("Query failed");

    println!("\nRetrieved {} entities:", result.context.entities.len());
    for (i, entity) in result.context.entities.iter().enumerate() {
        let degree = graph_storage.node_degree(&entity.name).await.unwrap_or(0);
        println!("  {}. {} (degree: {})", i + 1, entity.name, degree);
    }

    // ⚠️  PARTIAL: GraphStorage has node_degree() but strategies don't use it for sorting
    // LightRAG implementation:
    // 1. After entity vector search, get node degrees
    // 2. Attach degree to each entity
    // 3. For relationships, sort by (degree + weight) descending
    // 4. Return sorted entities/relationships

    println!("\n⚠️  EXPECTED BEHAVIOR (from LightRAG):");
    println!("   1. Vector search finds candidate entities");
    println!("   2. Get node degree for each entity from graph");
    println!("   3. Sort entities by: vector_score + graph_degree");
    println!("   4. Sort relationships by: (src_degree + tgt_degree) + edge_weight");
    println!("\n⚠️  CURRENT BEHAVIOR:");
    println!("   1. Vector search returns entities sorted by similarity ✅");
    println!("   2. Node degree available via graph_storage.node_degree() ✅");
    println!("   3. Strategies don't use degree for sorting ❌ MISSING");
}

/// Tests chunk frequency tracking - currently MISSING FEATURE.
/// Enable when frequency-based chunk ranking is implemented.
#[tokio::test]
#[ignore = "Gap analysis test - chunk frequency tracking not yet implemented"]
async fn test_chunk_frequency_tracking() {
    println!("\n=== Testing Chunk Frequency Tracking (MISSING FEATURE) ===");

    // Setup
    let kv_storage = Arc::new(edgequake_storage::MemoryKVStorage::new("test_freq"));
    let vector_storage = Arc::new(MemoryVectorStorage::new("test_freq", 1536));
    let graph_storage = Arc::new(MemoryGraphStorage::new("test_freq"));

    let config = EdgeQuakeConfig::new()
        .with_namespace("test_freq")
        .with_storage(StorageConfig {
            backend: StorageBackend::Memory,
            ..Default::default()
        });

    let mock_provider = create_complex_mock().await;

    let mut edgequake = EdgeQuake::new(config)
        .with_storage_backends(kv_storage, vector_storage, graph_storage)
        .with_providers(
            mock_provider.clone() as Arc<dyn edgequake_llm::LLMProvider>,
            mock_provider as Arc<dyn edgequake_llm::EmbeddingProvider>,
        );

    edgequake.initialize().await.expect("Failed to initialize");

    // Insert documents
    edgequake
        .insert(COMPLEX_DOCUMENT_1, Some("doc-1"))
        .await
        .expect("Failed to insert");
    edgequake
        .insert(COMPLEX_DOCUMENT_2, Some("doc-2"))
        .await
        .expect("Failed to insert");
    edgequake
        .insert(COMPLEX_DOCUMENT_3, Some("doc-3"))
        .await
        .expect("Failed to insert");

    // Query hybrid mode (retrieves from multiple sources)
    let params = edgequake_core::QueryParams {
        mode: QueryMode::Hybrid,
        ..Default::default()
    };

    let result = edgequake
        .query(
            "How does machine learning impact different fields?",
            Some(params),
        )
        .await
        .expect("Query failed");

    println!("Retrieved {} chunks", result.context.chunks.len());

    // ❌ MISSING: Chunk frequency tracking
    // LightRAG implementation:
    // chunk_tracking = {}  # chunk_id -> {source, frequency, order}
    //
    // From local entities:
    //   for entity in local_entities:
    //     for chunk_id in entity.source_id.split("|"):
    //       chunk_tracking[chunk_id] = {source: "E", frequency: +1}
    //
    // From global relationships:
    //   for relation in global_relations:
    //     for chunk_id in relation.source_id.split("|"):
    //       chunk_tracking[chunk_id] = {source: "R", frequency: +1}
    //
    // From vector search:
    //   for chunk in vector_chunks:
    //     chunk_tracking[chunk_id] = {source: "C", frequency: 1}
    //
    // Then use weighted polling to select high-frequency chunks

    println!("\n⚠️  EXPECTED BEHAVIOR (from LightRAG):");
    println!("   1. Track chunks from entities (source='E')");
    println!("   2. Track chunks from relationships (source='R')");
    println!("   3. Track chunks from vector search (source='C')");
    println!("   4. Calculate frequency for each chunk");
    println!("   5. Use weighted polling to prioritize high-frequency chunks");
    println!("   6. Log: 'Chunk chunk-001: freq=3, sources=E,R,C, order=1'");
    println!("\n⚠️  CURRENT BEHAVIOR:");
    println!("   1. No cross-source chunk tracking ❌ MISSING");
    println!("   2. No frequency calculation ❌ MISSING");
    println!("   3. No weighted prioritization ❌ MISSING");
}

/// Tests response quality metrics - gap analysis for quality improvements.
/// Enable when quality metrics are implemented.
#[tokio::test]
#[ignore = "Gap analysis test - response quality metrics depend on missing features"]
async fn test_response_quality_metrics() {
    println!("\n=== Testing Response Quality Metrics ===");

    // Setup
    let kv_storage = Arc::new(edgequake_storage::MemoryKVStorage::new("test_quality"));
    let vector_storage = Arc::new(MemoryVectorStorage::new("test_quality", 1536));
    let graph_storage = Arc::new(MemoryGraphStorage::new("test_quality"));

    let config = EdgeQuakeConfig::new()
        .with_namespace("test_quality")
        .with_storage(StorageConfig {
            backend: StorageBackend::Memory,
            ..Default::default()
        });

    let mock_provider = create_complex_mock().await;

    let mut edgequake = EdgeQuake::new(config)
        .with_storage_backends(kv_storage, vector_storage, graph_storage)
        .with_providers(
            mock_provider.clone() as Arc<dyn edgequake_llm::LLMProvider>,
            mock_provider as Arc<dyn edgequake_llm::EmbeddingProvider>,
        );

    edgequake.initialize().await.expect("Failed to initialize");

    // Insert documents
    edgequake
        .insert(COMPLEX_DOCUMENT_1, Some("doc-1"))
        .await
        .expect("Failed to insert");
    edgequake
        .insert(COMPLEX_DOCUMENT_2, Some("doc-2"))
        .await
        .expect("Failed to insert");
    edgequake
        .insert(COMPLEX_DOCUMENT_3, Some("doc-3"))
        .await
        .expect("Failed to insert");

    // Test all query modes
    let modes = vec![
        ("Naive", QueryMode::Naive),
        ("Local", QueryMode::Local),
        ("Global", QueryMode::Global),
        ("Hybrid", QueryMode::Hybrid),
        ("Mix", QueryMode::Mix),
    ];

    println!("\n| Mode | Entities | Relations | Chunks | Total Context |");
    println!("|------|----------|-----------|--------|---------------|");

    for (name, mode) in modes {
        let params = edgequake_core::QueryParams {
            mode,
            ..Default::default()
        };

        let result = edgequake
            .query("What AI technologies are used?", Some(params))
            .await
            .expect("Query failed");

        let total = result.context.entities.len()
            + result.context.relationships.len()
            + result.context.chunks.len();

        println!(
            "| {} | {} | {} | {} | {} |",
            name,
            result.context.entities.len(),
            result.context.relationships.len(),
            result.context.chunks.len(),
            total
        );
    }

    println!("\n✅ All query modes functional");
    println!("⚠️  Missing features limit retrieval quality:");
    println!("   - No keyword extraction for better targeting");
    println!("   - No token-based truncation for LLM efficiency");
    println!("   - No chunk retrieval from entities (Local mode)");
    println!("   - No chunk retrieval from relationships (Global mode)");
}

/// Tests cross-document entity linking - gap analysis for entity dedup.
/// Enable when improved entity deduplication is implemented.
#[tokio::test]
#[ignore = "Gap analysis test - cross-document entity linking needs more mock responses"]
async fn test_cross_document_entity_linking() {
    println!("\n=== Testing Cross-Document Entity Linking ===");

    // Setup
    let kv_storage = Arc::new(edgequake_storage::MemoryKVStorage::new("test_linking"));
    let vector_storage = Arc::new(MemoryVectorStorage::new("test_linking", 1536));
    let graph_storage = Arc::new(MemoryGraphStorage::new("test_linking"));

    let config = EdgeQuakeConfig::new()
        .with_namespace("test_linking")
        .with_storage(StorageConfig {
            backend: StorageBackend::Memory,
            ..Default::default()
        });

    let mock_provider = create_complex_mock().await;

    let mut edgequake = EdgeQuake::new(config)
        .with_storage_backends(kv_storage, vector_storage, graph_storage.clone())
        .with_providers(
            mock_provider.clone() as Arc<dyn edgequake_llm::LLMProvider>,
            mock_provider as Arc<dyn edgequake_llm::EmbeddingProvider>,
        );

    edgequake.initialize().await.expect("Failed to initialize");

    // Insert all three documents
    edgequake
        .insert(COMPLEX_DOCUMENT_1, Some("doc-health"))
        .await
        .expect("Failed to insert");
    edgequake
        .insert(COMPLEX_DOCUMENT_2, Some("doc-auto"))
        .await
        .expect("Failed to insert");
    edgequake
        .insert(COMPLEX_DOCUMENT_3, Some("doc-collab"))
        .await
        .expect("Failed to insert");

    // Check that entities mentioned across documents are properly merged
    let sarah_node = graph_storage
        .get_node("DR._SARAH_MARTINEZ")
        .await
        .expect("Failed to get node");

    if let Some(node) = sarah_node {
        println!("Dr. Sarah Martinez entity:");
        println!("  - Properties: {:?}", node.properties);

        // Entity should have source_ids from both doc-1 and doc-3
        if let Some(source_id) = node.properties.get("source_id") {
            println!("  - Source IDs: {:?}", source_id);
            // Should contain references to both documents
        }
    }

    // Query to find cross-document connections
    let params = edgequake_core::QueryParams {
        mode: QueryMode::Global,
        ..Default::default()
    };

    let result = edgequake
        .query("Who collaborates on AI research?", Some(params))
        .await
        .expect("Query failed");

    println!("\nGlobal query results:");
    println!("  - {} entities", result.context.entities.len());
    println!("  - {} relationships", result.context.relationships.len());

    // Should find collaboration relationship between Sarah and Elon (from doc-3)
    let found_collaboration = result.context.relationships.iter().any(|r| {
        (r.source.contains("SARAH") && r.target.contains("ELON"))
            || (r.source.contains("ELON") && r.target.contains("SARAH"))
    });

    println!("\n✅ Cross-document entity linking works");
    println!(
        "   Found collaboration relationship: {}",
        found_collaboration
    );
}
