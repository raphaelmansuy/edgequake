//! Graph exploration example demonstrating EdgeQuake's knowledge graph capabilities.
//!
//! This example shows how to:
//! 1. Build a knowledge graph with entities and relationships
//! 2. Query the graph structure
//! 3. Traverse relationships
//!
//! Run with: cargo run --example graph_exploration

use std::sync::Arc;

use edgequake_storage::adapters::memory::MemoryGraphStorage;
use edgequake_storage::GraphStorage;
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("EdgeQuake Graph Exploration Example");
    println!("====================================\n");

    // 1. Initialize the graph storage
    println!("1. Initializing graph storage...");
    let graph = Arc::new(MemoryGraphStorage::new("demo"));

    // 2. Add entities (nodes)
    println!("\n2. Adding entities to the knowledge graph...");

    // Programming Languages
    let rust = json!({
        "entity_type": "PROGRAMMING_LANGUAGE",
        "description": "A systems programming language focused on safety, speed, and concurrency",
        "source_id": "chunk-001"
    });
    graph
        .upsert_node(
            "RUST",
            rust.as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
        .await?;
    println!("   ✓ Added RUST (PROGRAMMING_LANGUAGE)");

    let python = json!({
        "entity_type": "PROGRAMMING_LANGUAGE",
        "description": "A high-level, general-purpose programming language emphasizing code readability",
        "source_id": "chunk-002"
    });
    graph
        .upsert_node(
            "PYTHON",
            python
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
        .await?;
    println!("   ✓ Added PYTHON (PROGRAMMING_LANGUAGE)");

    // Organizations
    let mozilla = json!({
        "entity_type": "ORGANIZATION",
        "description": "A free software community that develops the Firefox web browser and other software",
        "source_id": "chunk-001"
    });
    graph
        .upsert_node(
            "MOZILLA",
            mozilla
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
        .await?;
    println!("   ✓ Added MOZILLA (ORGANIZATION)");

    let psf = json!({
        "entity_type": "ORGANIZATION",
        "description": "The Python Software Foundation, a non-profit organization devoted to the Python programming language",
        "source_id": "chunk-002"
    });
    graph
        .upsert_node(
            "PYTHON SOFTWARE FOUNDATION",
            psf.as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
        .await?;
    println!("   ✓ Added PYTHON SOFTWARE FOUNDATION (ORGANIZATION)");

    // People
    let graydon = json!({
        "entity_type": "PERSON",
        "description": "The original designer of the Rust programming language",
        "source_id": "chunk-001"
    });
    graph
        .upsert_node(
            "GRAYDON HOARE",
            graydon
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
        .await?;
    println!("   ✓ Added GRAYDON HOARE (PERSON)");

    let guido = json!({
        "entity_type": "PERSON",
        "description": "The creator of Python and its Benevolent Dictator For Life",
        "source_id": "chunk-002"
    });
    graph
        .upsert_node(
            "GUIDO VAN ROSSUM",
            guido
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
        .await?;
    println!("   ✓ Added GUIDO VAN ROSSUM (PERSON)");

    // Projects
    let edgequake = json!({
        "entity_type": "PROJECT",
        "description": "A high-performance RAG system built in Rust with knowledge graph capabilities",
        "source_id": "chunk-003"
    });
    graph
        .upsert_node(
            "EDGEQUAKE",
            edgequake
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
        .await?;
    println!("   ✓ Added EDGEQUAKE (PROJECT)");

    let lightrag = json!({
        "entity_type": "PROJECT",
        "description": "A Python-based RAG framework with graph-based knowledge representation",
        "source_id": "chunk-003"
    });
    graph
        .upsert_node(
            "LIGHTRAG",
            lightrag
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
        .await?;
    println!("   ✓ Added LIGHTRAG (PROJECT)");

    // 3. Add relationships (edges)
    println!("\n3. Adding relationships...");

    // Creator relationships
    let created_rel = json!({
        "relation_type": "CREATED",
        "description": "Graydon Hoare created Rust while working at Mozilla",
        "weight": 1.0
    });
    graph
        .upsert_edge(
            "GRAYDON HOARE",
            "RUST",
            created_rel
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
        .await?;
    println!("   ✓ GRAYDON HOARE --[CREATED]--> RUST");

    let created_py = json!({
        "relation_type": "CREATED",
        "description": "Guido van Rossum created Python in 1991",
        "weight": 1.0
    });
    graph
        .upsert_edge(
            "GUIDO VAN ROSSUM",
            "PYTHON",
            created_py
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
        .await?;
    println!("   ✓ GUIDO VAN ROSSUM --[CREATED]--> PYTHON");

    // Developed relationships
    let developed = json!({
        "relation_type": "DEVELOPED_BY",
        "description": "Rust was originally developed at Mozilla Research",
        "weight": 1.0
    });
    graph
        .upsert_edge(
            "RUST",
            "MOZILLA",
            developed
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
        .await?;
    println!("   ✓ RUST --[DEVELOPED_BY]--> MOZILLA");

    let governs = json!({
        "relation_type": "GOVERNS",
        "description": "PSF governs the development of Python",
        "weight": 1.0
    });
    graph
        .upsert_edge(
            "PYTHON SOFTWARE FOUNDATION",
            "PYTHON",
            governs
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
        .await?;
    println!("   ✓ PYTHON SOFTWARE FOUNDATION --[GOVERNS]--> PYTHON");

    // Project relationships
    let written_in = json!({
        "relation_type": "WRITTEN_IN",
        "description": "EdgeQuake is written in Rust",
        "weight": 1.0
    });
    graph
        .upsert_edge(
            "EDGEQUAKE",
            "RUST",
            written_in
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
        .await?;
    println!("   ✓ EDGEQUAKE --[WRITTEN_IN]--> RUST");

    let written_in_py = json!({
        "relation_type": "WRITTEN_IN",
        "description": "LightRAG is written in Python",
        "weight": 1.0
    });
    graph
        .upsert_edge(
            "LIGHTRAG",
            "PYTHON",
            written_in_py
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
        .await?;
    println!("   ✓ LIGHTRAG --[WRITTEN_IN]--> PYTHON");

    let rewrite_of = json!({
        "relation_type": "REWRITE_OF",
        "description": "EdgeQuake is a Rust rewrite of LightRAG",
        "weight": 1.0
    });
    graph
        .upsert_edge(
            "EDGEQUAKE",
            "LIGHTRAG",
            rewrite_of
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        )
        .await?;
    println!("   ✓ EDGEQUAKE --[REWRITE_OF]--> LIGHTRAG");

    // 4. Query the graph
    println!("\n4. Querying the knowledge graph...");

    // Get all nodes
    let all_nodes = graph.get_all_nodes().await?;
    println!("\n   Total entities: {}", all_nodes.len());

    // Get all edges
    let all_edges = graph.get_all_edges().await?;
    println!("   Total relationships: {}", all_edges.len());

    // Check specific nodes
    println!("\n5. Exploring specific entities...");

    if let Some(rust_node) = graph.get_node("RUST").await? {
        println!("\n   Entity: RUST");
        if let Some(entity_type) = rust_node.get_property("entity_type") {
            println!("   Type: {}", entity_type);
        }
        if let Some(desc) = rust_node.get_property("description") {
            println!("   Description: {}", desc);
        }
    }

    // Get edges for RUST
    println!("\n6. Exploring relationships for RUST...");
    let rust_edges = graph.get_node_edges("RUST").await?;
    for edge in rust_edges {
        let relation_type = edge
            .properties
            .get("relation_type")
            .and_then(|v| v.as_str())
            .unwrap_or("RELATED_TO");
        println!(
            "   {} --[{}]--> {}",
            edge.source, relation_type, edge.target
        );
    }

    // Get node degree (number of connections)
    let rust_degree = graph.node_degree("RUST").await?;
    println!("\n   RUST has {} connections", rust_degree);

    // 7. Traverse the graph from EdgeQuake
    println!("\n7. Tracing EdgeQuake's lineage...");

    if graph.has_edge("EDGEQUAKE", "RUST").await? {
        println!("   ✓ EdgeQuake is written in Rust");
    }
    if graph.has_edge("EDGEQUAKE", "LIGHTRAG").await? {
        println!("   ✓ EdgeQuake is a rewrite of LightRAG");
    }
    if graph.has_edge("LIGHTRAG", "PYTHON").await? {
        println!("   ✓ LightRAG is written in Python");
    }
    if graph.has_edge("RUST", "MOZILLA").await? {
        println!("   ✓ Rust was developed by Mozilla");
    }

    // 8. Summary
    println!("\n====================================");
    println!("Graph Exploration Summary:");
    println!("  • {} entities in knowledge graph", all_nodes.len());
    println!("  • {} relationships mapped", all_edges.len());
    println!("  • Entity types: PROGRAMMING_LANGUAGE, ORGANIZATION, PERSON, PROJECT");
    println!("  • Relationship types: CREATED, DEVELOPED_BY, GOVERNS, WRITTEN_IN, REWRITE_OF");
    println!("\nExample completed successfully!");

    Ok(())
}
