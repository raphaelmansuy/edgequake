#!/usr/bin/env rust-script
//! Production example of EdgeQuake with real LLM provider.
//!
//! # Usage
//!
//! ```bash
//! # Set your OpenAI API key
//! export OPENAI_API_KEY="sk-your-key-here"
//!
//! # Run the example
//! cargo run --example production_pipeline
//! ```

use std::env;
use std::sync::Arc;

use edgequake_core::{EdgeQuake, EdgeQuakeConfig, StorageBackend, StorageConfig};
use edgequake_llm::{EmbeddingProvider, LLMProvider, OpenAIProvider};
use edgequake_storage::{GraphStorage, MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🚀 EdgeQuake Production Pipeline Example");
    println!("==========================================\n");

    // 1. Check for API key
    let api_key =
        env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable must be set");

    if api_key.is_empty() || api_key == "test-key" {
        eprintln!("❌ Invalid API key. Please set a real OpenAI API key:");
        eprintln!("   export OPENAI_API_KEY=\"sk-your-key-here\"");
        std::process::exit(1);
    }

    println!("✓ API key found");

    // 2. Create OpenAI provider
    println!("\n📡 Initializing OpenAI provider...");
    let provider = Arc::new(
        OpenAIProvider::new(api_key)
            .with_model("gpt-5-nano") // Cost-effective and fast
            .with_embedding_model("text-embedding-3-small"),
    );

    // Cast to trait objects for provider info
    let llm: Arc<dyn LLMProvider> = provider.clone();
    let embedding: Arc<dyn EmbeddingProvider> = provider.clone();
    println!("✓ LLM Provider: {} (model: {})", llm.name(), llm.model());
    println!(
        "✓ Embedding Provider: {} (model: {})",
        embedding.name(),
        embedding.model()
    );

    // 3. Setup storage backends
    // Note: For production, use PostgreSQL instead of memory storage
    println!("\n💾 Setting up storage backends...");
    let namespace = "production_example";
    let kv_storage = Arc::new(MemoryKVStorage::new(namespace));
    let vector_storage = Arc::new(MemoryVectorStorage::new(namespace, 1536));
    let graph_storage = Arc::new(MemoryGraphStorage::new(namespace));
    println!("✓ Storage backends ready (using memory for demo)");

    // 4. Create EdgeQuake instance
    println!("\n⚙️  Initializing EdgeQuake...");
    let config = EdgeQuakeConfig::new()
        .with_namespace(namespace)
        .with_storage(StorageConfig {
            backend: StorageBackend::Memory,
            ..Default::default()
        });

    let mut edgequake = EdgeQuake::new(config)
        .with_storage_backends(kv_storage, vector_storage, graph_storage.clone())
        .with_providers(
            provider.clone() as Arc<dyn edgequake_llm::LLMProvider>,
            provider as Arc<dyn edgequake_llm::EmbeddingProvider>,
        );

    edgequake.initialize().await?;
    println!("✓ EdgeQuake initialized");

    // 5. Ingest sample documents
    println!("\n📄 Ingesting documents...\n");

    // WHY: Use array instead of vec! since size is known at compile time (clippy::useless_vec)
    let documents = [
        (
            "Introduction to EdgeQuake",
            r#"
EdgeQuake is a high-performance Retrieval-Augmented Generation (RAG) system built in Rust.
The system was designed by Sarah Chen, who serves as the lead architect. EdgeQuake integrates
multiple advanced technologies including Apache AGE for graph storage and pgvector for
similarity search. The architecture follows a modular design with clear separation of concerns.
"#,
        ),
        (
            "Technical Architecture",
            r#"
The storage layer supports multiple backends including PostgreSQL with AGE extension for
production deployments. Michael Torres leads the LLM integration team, working closely with
Sarah Chen to ensure optimal performance. The team has implemented sophisticated caching
mechanisms to reduce latency and costs.
"#,
        ),
        (
            "Team and Development",
            r#"
The core team consists of Sarah Chen (Lead Architect), Michael Torres (LLM Integration Lead),
and several other engineers. They collaborate using modern DevOps practices and continuous
integration. The project is open source and welcomes contributions from the community.
"#,
        ),
    ];

    let mut total_entities = 0;
    let mut total_relationships = 0;

    for (i, (title, content)) in documents.iter().enumerate() {
        let doc_id = format!("doc-{:03}", i + 1);
        println!("→ Processing: {}", title);
        println!("  Document ID: {}", doc_id);

        match edgequake.insert(content, Some(&doc_id)).await {
            Ok(result) => {
                println!("  ✓ Chunks created: {}", result.chunks_created);
                println!("  ✓ Entities extracted: {}", result.entities_extracted);
                println!("  ✓ Relationships: {}", result.relationships_extracted);

                total_entities += result.entities_extracted;
                total_relationships += result.relationships_extracted;
            }
            Err(e) => {
                eprintln!("  ❌ Error: {:?}", e);
                return Err(e.into());
            }
        }
        println!();
    }

    // 6. Display results
    println!("📊 Processing Complete!");
    println!("========================");
    println!("Total documents: {}", documents.len());
    println!("Total entities extracted: {}", total_entities);
    println!("Total relationships: {}", total_relationships);

    // 7. Query the knowledge graph
    println!("\n🔍 Querying Knowledge Graph...\n");

    let stats = edgequake.get_graph_stats().await?;
    println!("Graph Statistics:");
    println!("  • Unique nodes: {}", stats.node_count);
    println!("  • Edges: {}", stats.edge_count);
    println!(
        "  • Entity deduplication: {}% ({}→{} nodes)",
        (100.0 * (1.0 - stats.node_count as f64 / total_entities as f64)) as i32,
        total_entities,
        stats.node_count
    );

    // 8. Test graph operations
    println!("\n🔗 Testing Graph Operations...\n");

    // Check if key entities exist
    let key_entities = vec!["EDGEQUAKE", "SARAH_CHEN", "MICHAEL_TORRES"];

    for entity in &key_entities {
        match graph_storage.has_node(entity).await {
            Ok(true) => {
                println!("✓ Entity found: {}", entity);

                // Get neighbors
                if let Ok(neighbors) = graph_storage.get_neighbors(entity, 1).await {
                    println!("  Connected to {} other entities", neighbors.len());
                }
            }
            Ok(false) => {
                println!("⚠ Entity not found: {}", entity);
            }
            Err(e) => {
                eprintln!("❌ Error checking entity: {:?}", e);
            }
        }
    }

    // 9. Demonstrate graph traversal
    println!("\n🗺️  Graph Traversal Example...\n");

    if let Ok(kg) = graph_storage.get_knowledge_graph("EDGEQUAKE", 2, 50).await {
        println!("EdgeQuake subgraph (2-hop neighborhood):");
        println!("  • Nodes: {}", kg.node_count());
        println!("  • Edges: {}", kg.edge_count());
    }

    // 10. Success!
    println!("\n✅ Production Pipeline Example Complete!");
    println!("\n💡 Next Steps:");
    println!("   1. Replace memory storage with PostgreSQL for persistence");
    println!("   2. Implement rate limiting for API calls");
    println!("   3. Add error recovery and retry logic");
    println!("   4. Enable caching to reduce costs");
    println!("   5. Set up monitoring and metrics");

    println!("\n📚 See docs/production-llm-integration.md for more details");

    Ok(())
}
