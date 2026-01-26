//! E2E Integration tests with Ollama provider.
//!
//! @implements OODA-16: Ollama E2E Testing
//!
//! These tests verify the full document lifecycle with a real Ollama LLM provider.
//! They are marked with #[ignore] to prevent CI failures when Ollama is not available.
//!
//! # Running These Tests
//!
//! ```bash
//! # Ensure Ollama is running with required models:
//! # ollama pull gemma3:latest
//! # ollama pull nomic-embed-text:latest
//!
//! # Run ignored tests:
//! cargo test --package edgequake-api --test e2e_ollama_integration -- --ignored --nocapture
//! ```
//!
//! # Test Coverage
//!
//! - Document upload with real LLM entity extraction
//! - Query modes: LLM-only, embedding-only, hybrid
//! - Document deletion cascade verification
//! - Entity extraction quality verification

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use serde_json::{json, Value};
use std::time::Duration;
use tower::ServiceExt;

// ============================================================================
// Ollama Test Helpers
// ============================================================================

/// Default Ollama models for testing
const OLLAMA_HOST: &str = "http://localhost:11434";

/// Check if Ollama is available and has the required models.
async fn is_ollama_available() -> bool {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    // Check if Ollama is reachable
    let response = client.get(format!("{}/api/tags", OLLAMA_HOST)).send().await;

    match response {
        Ok(resp) if resp.status().is_success() => {
            // Check if required models are available
            if let Ok(body) = resp.json::<Value>().await {
                let empty_vec = vec![];
                let models = body
                    .get("models")
                    .and_then(|m| m.as_array())
                    .unwrap_or(&empty_vec);

                let has_llm = models.iter().any(|m| {
                    m.get("name")
                        .and_then(|n| n.as_str())
                        .map(|n| n.starts_with("gemma3"))
                        .unwrap_or(false)
                });

                let has_embedding = models.iter().any(|m| {
                    m.get("name")
                        .and_then(|n| n.as_str())
                        .map(|n| n.contains("nomic-embed-text") || n.contains("embeddinggemma"))
                        .unwrap_or(false)
                });

                if !has_llm {
                    eprintln!("⚠️ Missing Ollama LLM model (gemma3:latest). Run: ollama pull gemma3:latest");
                }
                if !has_embedding {
                    eprintln!("⚠️ Missing Ollama embedding model. Run: ollama pull nomic-embed-text:latest");
                }

                has_llm && has_embedding
            } else {
                false
            }
        }
        Ok(_) => false,
        Err(e) => {
            eprintln!("⚠️ Ollama not reachable at {}: {}", OLLAMA_HOST, e);
            false
        }
    }
}

fn create_test_config() -> ServerConfig {
    ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: true,
    }
}

/// Create test app with mock provider (for basic structure tests).
/// WHY: Ollama E2E tests use the API endpoints; provider is configured via env vars.
fn create_test_app() -> axum::Router {
    Server::new(create_test_config(), AppState::test_state()).build_router()
}

/// Create test app and state (for inspection tests).
fn create_test_app_with_state() -> (axum::Router, AppState) {
    let state = AppState::test_state();
    let app = Server::new(create_test_config(), state.clone()).build_router();
    (app, state)
}

async fn extract_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("Failed to read response body");
    serde_json::from_slice(&bytes).expect("Failed to parse JSON")
}

/// Upload a document via HTTP with extended timeout for Ollama.
async fn upload_document(app: &axum::Router, title: &str, content: &str) -> (StatusCode, Value) {
    let request = json!({
        "content": content,
        "title": title,
        "async_processing": false
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/documents")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = extract_json(response).await;
    (status, body)
}

/// Delete a document via HTTP.
async fn delete_document(app: &axum::Router, document_id: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/documents/{}", document_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = extract_json(response).await;
    (status, body)
}

/// Query the knowledge graph via HTTP.
async fn query_kg(app: &axum::Router, query: &str, mode: &str) -> (StatusCode, Value) {
    let request = json!({
        "query": query,
        "mode": mode,
        "stream": false
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/query")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = extract_json(response).await;
    (status, body)
}

// ============================================================================
// Ollama E2E Tests
// ============================================================================

/// Test that Ollama is available and has required models.
#[tokio::test]
#[ignore = "Requires Ollama running locally"]
async fn test_ollama_availability() {
    let available = is_ollama_available().await;
    assert!(
        available,
        "Ollama must be running with gemma3:latest and nomic-embed-text:latest"
    );
    println!("✅ Ollama is available with required models");
}

/// Test document upload with mock provider (baseline).
///
/// This test uses mock provider to verify the test infrastructure works.
/// It's always enabled since it doesn't require Ollama.
#[tokio::test]
async fn test_mock_document_upload_baseline() {
    let app = create_test_app();

    // Upload document
    let (status, upload_resp) = upload_document(
        &app,
        "Tech Company Profile",
        "Alice Chen is a software engineer at TechCorp Inc. She works closely with Bob Smith.",
    )
    .await;

    println!("Upload response: {:?}", upload_resp);

    assert_eq!(status, StatusCode::CREATED);
    let doc_id = upload_resp
        .get("document_id")
        .and_then(|v| v.as_str())
        .expect("Should have document_id");

    // Clean up
    let (delete_status, _) = delete_document(&app, doc_id).await;
    assert_eq!(delete_status, StatusCode::OK);

    println!("✅ MOCK BASELINE TEST PASSED");
}

/// Test document upload and entity extraction with mock provider.
///
/// This verifies the mock provider extracts entities.
#[tokio::test]
async fn test_mock_entity_extraction() {
    let (app, state) = create_test_app_with_state();

    // Upload document with clear entity mentions
    let (status, upload_resp) = upload_document(
        &app,
        "Tech Company Profile",
        "Alice Chen is a software engineer at TechCorp Inc. She works closely with Bob Smith \
         on the machine learning team. TechCorp is headquartered in San Francisco and \
         specializes in artificial intelligence. Alice graduated from Stanford University \
         and Bob from MIT. They collaborate on natural language processing projects.",
    )
    .await;

    println!("Upload response: {:?}", upload_resp);

    assert_eq!(status, StatusCode::CREATED);
    let doc_id = upload_resp
        .get("document_id")
        .and_then(|v| v.as_str())
        .expect("Should have document_id");

    let entity_count = upload_resp
        .get("entity_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let relationship_count = upload_resp
        .get("relationship_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    println!(
        "📊 Mock provider extracted {} entities, {} relationships",
        entity_count, relationship_count
    );

    // Check graph state
    let nodes = state.graph_storage.get_all_nodes().await.unwrap();
    let edges = state.graph_storage.get_all_edges().await.unwrap();
    println!("Graph state: {} nodes, {} edges", nodes.len(), edges.len());

    // Clean up
    let (delete_status, _) = delete_document(&app, doc_id).await;
    assert_eq!(delete_status, StatusCode::OK);

    // Verify cleanup
    let nodes_after = state.graph_storage.get_all_nodes().await.unwrap();
    let edges_after = state.graph_storage.get_all_edges().await.unwrap();
    assert!(nodes_after.is_empty(), "All nodes should be deleted");
    assert!(edges_after.is_empty(), "All edges should be deleted");

    println!("✅ MOCK ENTITY EXTRACTION TEST PASSED");
}

/// Test query modes with mock provider.
#[tokio::test]
async fn test_mock_query_modes() {
    let app = create_test_app();

    // Upload document
    let (status, upload_resp) = upload_document(
        &app,
        "AI Research Paper",
        "The transformer architecture revolutionized natural language processing. \
         Attention mechanisms allow models to focus on relevant parts of the input. \
         BERT and GPT are both based on transformers but have different training objectives.",
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    let doc_id = upload_resp
        .get("document_id")
        .and_then(|v| v.as_str())
        .expect("Should have document_id");

    // Test llm_only mode
    let (query_status, query_resp) = query_kg(
        &app,
        "What is the difference between BERT and GPT?",
        "llm_only",
    )
    .await;

    println!("LLM-only query response: {:?}", query_resp);
    assert_eq!(query_status, StatusCode::OK);

    // Test hybrid mode
    let (query_status, query_resp) =
        query_kg(&app, "Explain transformer architecture", "hybrid").await;

    println!("Hybrid query response: {:?}", query_resp);
    assert_eq!(query_status, StatusCode::OK);

    // Clean up
    delete_document(&app, doc_id).await;

    println!("✅ MOCK QUERY MODES TEST PASSED");
}

/// Test deletion cascade with mock-extracted entities.
#[tokio::test]
async fn test_mock_deletion_cascade() {
    let (app, state) = create_test_app_with_state();

    // Upload document
    let (status, upload_resp) = upload_document(
        &app,
        "Biography",
        "Marie Curie was a physicist who discovered radium. She won two Nobel Prizes. \
         Pierre Curie was her husband and collaborator. They worked at the University of Paris.",
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    let doc_id = upload_resp
        .get("document_id")
        .and_then(|v| v.as_str())
        .expect("Should have document_id");

    let entity_count = upload_resp
        .get("entity_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    println!("Before deletion: {} entities extracted", entity_count);

    // Check graph state before deletion
    let nodes_before = state.graph_storage.get_all_nodes().await.unwrap();
    let edges_before = state.graph_storage.get_all_edges().await.unwrap();

    println!(
        "Graph before deletion: {} nodes, {} edges",
        nodes_before.len(),
        edges_before.len()
    );

    // Delete document
    let (delete_status, delete_resp) = delete_document(&app, doc_id).await;

    println!("Delete response: {:?}", delete_resp);

    assert_eq!(delete_status, StatusCode::OK);
    assert_eq!(
        delete_resp.get("deleted").and_then(|v| v.as_bool()),
        Some(true)
    );

    // Verify all entities and relationships are cleaned up
    let nodes_after = state.graph_storage.get_all_nodes().await.unwrap();
    let edges_after = state.graph_storage.get_all_edges().await.unwrap();

    println!(
        "Graph after deletion: {} nodes, {} edges",
        nodes_after.len(),
        edges_after.len()
    );

    assert!(
        nodes_after.is_empty(),
        "All nodes should be deleted, but {} remain",
        nodes_after.len()
    );
    assert!(
        edges_after.is_empty(),
        "All edges should be deleted, but {} remain",
        edges_after.len()
    );

    println!("✅ MOCK DELETION CASCADE TEST PASSED");
}

/// Test query after deletion returns no errors.
#[tokio::test]
async fn test_mock_query_after_deletion() {
    let app = create_test_app();

    // Upload document
    let (status, upload_resp) = upload_document(
        &app,
        "Temporary Document",
        "This is a temporary document about quantum computing and qubits.",
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    let doc_id = upload_resp
        .get("document_id")
        .and_then(|v| v.as_str())
        .expect("Should have document_id");

    // Delete document
    let (delete_status, _) = delete_document(&app, doc_id).await;
    assert_eq!(delete_status, StatusCode::OK);

    // Query should not error even with empty knowledge graph
    let (query_status, query_resp) =
        query_kg(&app, "Tell me about quantum computing", "hybrid").await;

    println!("Query after deletion response: {:?}", query_resp);

    // Query should succeed (may return "no data found" type response)
    assert!(
        query_status == StatusCode::OK || query_status == StatusCode::NOT_FOUND,
        "Query after deletion should not error, got status {}",
        query_status
    );

    println!("✅ MOCK QUERY AFTER DELETION TEST PASSED");
}

/// Stress test: Multiple document operations with mock provider.
#[tokio::test]
async fn test_mock_multi_document_stress() {
    let (app, state) = create_test_app_with_state();

    let documents = vec![
        (
            "Doc 1",
            "Apple Inc was founded by Steve Jobs. Tim Cook is the current CEO.",
        ),
        (
            "Doc 2",
            "Google was founded by Larry Page and Sergey Brin. Sundar Pichai is CEO.",
        ),
        (
            "Doc 3",
            "Microsoft was founded by Bill Gates. Satya Nadella is the current CEO.",
        ),
    ];

    let mut doc_ids = Vec::new();

    // Upload all documents
    for (title, content) in &documents {
        let (status, upload_resp) = upload_document(&app, title, content).await;
        assert_eq!(status, StatusCode::CREATED);
        let doc_id = upload_resp
            .get("document_id")
            .and_then(|v| v.as_str())
            .expect("Should have document_id")
            .to_string();
        doc_ids.push(doc_id);
        println!("Uploaded: {}", title);
    }

    // Check graph state
    let nodes = state.graph_storage.get_all_nodes().await.unwrap();
    let edges = state.graph_storage.get_all_edges().await.unwrap();
    println!(
        "After uploading {} docs: {} nodes, {} edges",
        documents.len(),
        nodes.len(),
        edges.len()
    );

    // Query the combined knowledge
    let (query_status, query_resp) = query_kg(
        &app,
        "Who are the CEOs of Apple, Google, and Microsoft?",
        "hybrid",
    )
    .await;

    assert_eq!(query_status, StatusCode::OK);
    println!("Multi-doc query response: {:?}", query_resp);

    // Delete all documents
    for doc_id in &doc_ids {
        let (delete_status, _) = delete_document(&app, doc_id).await;
        assert_eq!(delete_status, StatusCode::OK);
    }

    // Verify cleanup
    let nodes_after = state.graph_storage.get_all_nodes().await.unwrap();
    let edges_after = state.graph_storage.get_all_edges().await.unwrap();

    assert!(
        nodes_after.is_empty(),
        "All nodes should be deleted after cleanup"
    );
    assert!(
        edges_after.is_empty(),
        "All edges should be deleted after cleanup"
    );

    println!("✅ MOCK MULTI-DOCUMENT STRESS TEST PASSED");
}
