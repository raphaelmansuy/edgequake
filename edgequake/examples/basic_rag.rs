//! Basic RAG example demonstrating EdgeQuake usage.
//!
//! This example shows how to:
//! 1. Set up storage adapters
//! 2. Configure the pipeline
//! 3. Ingest documents
//! 4. Query the knowledge base
//!
//! Run with: cargo run --example basic_rag

use std::sync::Arc;

use edgequake_pipeline::{Chunker, ChunkerConfig, TextChunk};
use edgequake_query::QueryMode;
use edgequake_storage::adapters::memory::{
    MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage,
};
use edgequake_storage::GraphStorage;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("EdgeQuake Basic RAG Example");
    println!("===========================\n");

    // 1. Initialize storage backends
    println!("1. Initializing storage backends...");
    let _kv_storage = Arc::new(MemoryKVStorage::new("demo"));
    let _vector_storage = Arc::new(MemoryVectorStorage::new("demo", 384)); // 384-dim embeddings
    let graph_storage = Arc::new(MemoryGraphStorage::new("demo"));

    println!("   ✓ KV Storage: in-memory");
    println!("   ✓ Vector Storage: in-memory (384 dimensions)");
    println!("   ✓ Graph Storage: in-memory");

    // 2. Configure the chunker
    println!("\n2. Configuring text chunker...");
    let chunker_config = ChunkerConfig {
        chunk_size: 512,
        chunk_overlap: 50,
        ..Default::default()
    };
    let chunker = Chunker::new(chunker_config);
    println!("   ✓ Chunker: token-based, 512 tokens max, 50 token overlap");

    // 3. Sample document
    let sample_text = r#"
    Rust is a systems programming language focused on safety, speed, and concurrency.
    It was originally designed by Graydon Hoare at Mozilla Research.
    
    The language achieves memory safety without garbage collection through its ownership system.
    Rust's ownership model ensures that memory is managed at compile time.
    
    Key features of Rust include:
    - Zero-cost abstractions
    - Move semantics
    - Guaranteed memory safety
    - Threads without data races
    - Trait-based generics
    - Pattern matching
    
    Rust has been used to build Firefox's Servo browser engine, the Deno runtime,
    and many other high-performance applications.
    "#;

    let doc_id = "sample-rust-intro";
    println!("\n3. Processing sample document...");
    println!("   Document ID: {}", doc_id);
    println!("   Content length: {} chars", sample_text.len());

    // 4. Chunk the document
    let chunks: Vec<TextChunk> = chunker.chunk(sample_text, doc_id)?;
    println!("\n4. Chunking document...");
    println!("   Generated {} chunks", chunks.len());
    for (i, chunk) in chunks.iter().enumerate() {
        println!(
            "   Chunk {}: {} chars, ~{} tokens",
            i + 1,
            chunk.content.len(),
            chunk.token_count
        );
    }

    // 5. Store in graph (simplified)
    println!("\n5. Storing in knowledge graph...");

    // Add some sample entities manually for demonstration
    let rust_props = serde_json::json!({
        "entity_type": "PROGRAMMING_LANGUAGE",
        "description": "A systems programming language focused on safety, speed, and concurrency"
    });
    graph_storage
        .upsert_node(
            "RUST",
            rust_props
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
        .await?;

    let mozilla_props = serde_json::json!({
        "entity_type": "ORGANIZATION",
        "description": "Research organization that originally developed Rust"
    });
    graph_storage
        .upsert_node(
            "MOZILLA RESEARCH",
            mozilla_props
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
        .await?;

    // Add relationship
    let rel_props = serde_json::json!({
        "relation_type": "DEVELOPED_BY",
        "description": "Rust was originally developed by Mozilla Research"
    });
    graph_storage
        .upsert_edge(
            "RUST",
            "MOZILLA RESEARCH",
            rel_props
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
        .await?;

    println!("   ✓ Added entity: RUST (PROGRAMMING_LANGUAGE)");
    println!("   ✓ Added entity: MOZILLA RESEARCH (ORGANIZATION)");
    println!("   ✓ Added relationship: RUST -> DEVELOPED_BY -> MOZILLA RESEARCH");

    // 6. Display graph stats
    println!("\n6. Knowledge Graph Statistics:");
    let nodes = graph_storage.get_all_nodes().await?;
    let edges = graph_storage.get_all_edges().await?;
    println!("   Total nodes: {}", nodes.len());
    println!("   Total edges: {}", edges.len());

    // 7. Query modes explanation
    println!("\n7. Available Query Modes:");
    for mode in QueryMode::all() {
        let vector = if mode.uses_vector_search() {
            "✓"
        } else {
            "✗"
        };
        let graph = if mode.uses_graph() { "✓" } else { "✗" };
        println!(
            "   - {:8} | Vector: {} | Graph: {}",
            mode.as_str(),
            vector,
            graph
        );
    }

    println!("\n===========================");
    println!("Example completed successfully!");
    println!("\nNext steps:");
    println!("  1. Add an LLM provider for intelligent extraction");
    println!("  2. Add an embedding provider for vector search");
    println!("  3. Use the full QueryEngine for RAG queries");

    Ok(())
}
