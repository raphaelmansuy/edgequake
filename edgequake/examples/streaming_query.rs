//! Streaming query example demonstrating EdgeQuake's streaming capabilities.
//!
//! This example shows how to:
//! 1. Set up a query engine with streaming support
//! 2. Execute streaming queries
//! 3. Handle streamed responses
//!
//! Run with: cargo run --example streaming_query

use std::sync::Arc;

use edgequake_llm::MockProvider;
use edgequake_query::{QueryEngine, QueryEngineConfig, QueryMode, QueryRequest};
use edgequake_storage::adapters::memory::{MemoryGraphStorage, MemoryVectorStorage};
use futures::StreamExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("EdgeQuake Streaming Query Example");
    println!("==================================\n");

    // 1. Initialize providers and storage
    println!("1. Initializing components...");
    let mock_provider = Arc::new(MockProvider::new());
    let vector_storage = Arc::new(MemoryVectorStorage::new("demo", 1536));
    let graph_storage = Arc::new(MemoryGraphStorage::new("demo"));

    // 2. Create query engine
    let query_engine = QueryEngine::new(
        QueryEngineConfig {
            default_mode: QueryMode::Naive,
            max_chunks: 5,
            max_entities: 10,
            max_context_tokens: 2000,
            graph_depth: 2,
            min_score: 0.1,
            include_sources: true,
            use_keyword_extraction: false,
            truncation: edgequake_query::TruncationConfig::default(),
        },
        vector_storage.clone(),
        graph_storage.clone(),
        mock_provider.clone(),
        mock_provider.clone(),
    );

    println!("   ✓ Query engine initialized");

    // 3. Add some sample data to the mock provider
    mock_provider
        .add_response(
            "EdgeQuake is a high-performance RAG system built in Rust. \
             It combines vector search with knowledge graph traversal.",
        )
        .await;

    // 4. Execute a streaming query
    println!("\n2. Executing streaming query...");
    let request = QueryRequest::new("What is EdgeQuake?").with_mode(QueryMode::Naive);

    match query_engine.query_stream(request).await {
        Ok(mut stream) => {
            println!("\n3. Receiving streamed response:");
            println!("   ────────────────────────────");

            while let Some(result) = stream.next().await {
                match result {
                    Ok(chunk) => {
                        // In a real application, you'd send this to the client
                        print!("{}", chunk);
                    }
                    Err(e) => {
                        eprintln!("\n   Error in stream: {}", e);
                        break;
                    }
                }
            }

            println!("\n   ────────────────────────────");
        }
        Err(e) => {
            eprintln!("   Error starting stream: {}", e);
        }
    }

    println!("\n4. Stream completed!");
    println!("\n==================================");
    println!("Example completed successfully!");

    Ok(())
}
