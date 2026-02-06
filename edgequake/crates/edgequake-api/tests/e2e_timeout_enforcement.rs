//! OODA-11: Timeout enforcement tests for critical E2E paths.
//!
//! These tests wrap the most critical document processing operations
//! with explicit timeouts to prevent indefinite hangs in CI/CD.
//!
//! ## Timeout Strategy
//!
//! | Test Category    | Timeout | Rationale                                |
//! |-----------------|---------|------------------------------------------|
//! | Unit tests      | 10s     | In-memory only, should be fast           |
//! | Document upload | 30s     | Mock pipeline, no external calls         |
//! | Full pipeline   | 30s     | Mock extraction + embedding              |
//! | E2E with LLM   | 120s    | Real API calls, network latency          |
//! | Query           | 30s     | Graph traversal + mock LLM              |
//!
//! ## Usage Pattern
//!
//! ```ignore
//! #[tokio::test]
//! async fn test_with_timeout() {
//!     let result = with_timeout(Duration::from_secs(30), async {
//!         // test body
//!     }).await;
//!     assert!(result.is_ok(), "Test should complete within 30s");
//! }
//! ```

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use serde_json::{json, Value};
use std::time::Duration;
use tower::ServiceExt;

// ============================================================================
// Timeout Helper
// ============================================================================

/// Run an async operation with a timeout, returning Err if it exceeds the limit.
///
/// OODA-11: All critical E2E paths must complete within a defined time budget.
async fn with_timeout<F, T>(duration: Duration, future: F) -> Result<T, String>
where
    F: std::future::Future<Output = T>,
{
    tokio::time::timeout(duration, future)
        .await
        .map_err(|_| format!("Test exceeded timeout of {:?}", duration))
}

// ============================================================================
// Test Infrastructure (shared with other E2E tests)
// ============================================================================

fn create_test_app() -> axum::Router {
    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: true,
    };
    let server = Server::new(config, AppState::test_state());
    server.build_router()
}

async fn extract_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("Failed to read response body");
    serde_json::from_slice(&bytes).expect("Failed to parse JSON")
}

async fn upload_document(app: &axum::Router, content: &str, title: &str) -> Value {
    let request = json!({
        "content": content,
        "title": title,
        "metadata": {"test": true}
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

    assert_eq!(response.status(), StatusCode::CREATED);
    extract_json(response).await
}

async fn query_rag(app: &axum::Router, query: &str) -> Value {
    let request = json!({ "query": query });

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

    assert_eq!(response.status(), StatusCode::OK);
    extract_json(response).await
}

// ============================================================================
// Test Documents
// ============================================================================

const SMALL_DOC: &str = "Sarah Chen is a senior AI researcher at TechCorp Labs. \
    She leads the NLP team and collaborates with Dr. James Wilson.";

const MEDIUM_DOC: &str = r#"
EdgeQuake Corporation is headquartered in San Francisco. Founded by Michael Roberts
and Lisa Chang in 2020, the company specializes in knowledge graph technologies.

Dr. Emily Watson serves as the CTO, leading 150 engineers. She previously worked
at Google Brain. The partnership with Stanford University involves Professor David Kim.

The company raised $100 million in Series C from Sequoia Capital and Andreessen Horowitz.
"#;

const LARGE_DOC: &str = r#"
# Quantum Computing Overview

Quantum computing represents a paradigm shift in computational technology. Unlike
classical computers that use bits (0 or 1), quantum computers use qubits that exist
in superposition.

## Key Concepts

Dr. Richard Feynman proposed quantum computing in 1982 at Caltech. IBM's team, led
by Dr. Sarah Williams, demonstrated practical entanglement. Google's Sycamore processor
uses tunable couplers developed by Dr. John Martinis.

## Major Players

IBM operates quantum computers via cloud. Dr. Jay Gambetta leads quantum research.
Google achieved quantum supremacy in 2019 with Sycamore, led by Hartmut Neven.
Microsoft's Dr. Krysta Svore leads quantum software with Q# language.
IonQ uses trapped ion technology, co-founded by Christopher Monroe and Jungsang Kim.
Rigetti builds hybrid systems, founded by Chad Rigetti.

## Applications

Peter Shor's algorithm threatens encryption. Dr. Matthias Troyer at Microsoft leads
drug discovery simulations. Goldman Sachs and JPMorgan invest in quantum finance.
Dr. Marco Pistoia leads JPMorgan's quantum research. BMW and Daimler explore batteries.

## Challenges

Dr. Michel Devoret at Yale pioneered coherence extension. Dr. Austin Fowler at Google
developed surface codes. Dr. Mikhail Lukin at Harvard develops neutral atom architectures.
"#;

// ============================================================================
// Timeout Tests: Document Upload (30s budget)
// ============================================================================

/// OODA-11: Small document upload must complete within 10s.
#[tokio::test]
async fn test_timeout_small_document_upload_10s() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();
        upload_document(&app, SMALL_DOC, "Timeout Small").await
    })
    .await;

    assert!(result.is_ok(), "Small doc upload: {}", result.unwrap_err());
    let body = result.unwrap();
    assert!(body["document_id"].is_string());
    assert_eq!(body["status"], "processed");
}

/// OODA-11: Medium document upload must complete within 30s.
#[tokio::test]
async fn test_timeout_medium_document_upload_30s() {
    let result = with_timeout(Duration::from_secs(30), async {
        let app = create_test_app();
        upload_document(&app, MEDIUM_DOC, "Timeout Medium").await
    })
    .await;

    assert!(result.is_ok(), "Medium doc upload: {}", result.unwrap_err());
    let body = result.unwrap();
    assert!(body["document_id"].is_string());
}

/// OODA-11: Large document upload must complete within 30s.
#[tokio::test]
async fn test_timeout_large_document_upload_30s() {
    let result = with_timeout(Duration::from_secs(30), async {
        let app = create_test_app();
        upload_document(&app, LARGE_DOC, "Timeout Large").await
    })
    .await;

    assert!(result.is_ok(), "Large doc upload: {}", result.unwrap_err());
    let body = result.unwrap();
    assert!(body["document_id"].is_string());
}

// ============================================================================
// Timeout Tests: Full Pipeline (30s budget)
// ============================================================================

/// OODA-11: Full pipeline (upload + graph check) must complete within 30s.
#[tokio::test]
async fn test_timeout_full_pipeline_30s() {
    let result = with_timeout(Duration::from_secs(30), async {
        let app = create_test_app();

        // Upload
        let upload = upload_document(&app, MEDIUM_DOC, "Pipeline Timeout").await;
        let doc_id = upload["document_id"].as_str().unwrap().to_string();

        // Get document details
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/documents/{}", doc_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Get graph
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/graph")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        doc_id
    })
    .await;

    assert!(result.is_ok(), "Full pipeline: {}", result.unwrap_err());
}

// ============================================================================
// Timeout Tests: Query (30s budget)
// ============================================================================

/// OODA-11: Query after ingestion must complete within 30s.
#[tokio::test]
async fn test_timeout_query_after_ingestion_30s() {
    let result = with_timeout(Duration::from_secs(30), async {
        let app = create_test_app();

        // Upload first
        upload_document(&app, MEDIUM_DOC, "Query Timeout Test").await;

        // Query
        query_rag(&app, "What is EdgeQuake Corporation?").await
    })
    .await;

    assert!(
        result.is_ok(),
        "Query after ingestion: {}",
        result.unwrap_err()
    );
    let body = result.unwrap();
    assert!(body["answer"].is_string());
}

// ============================================================================
// Timeout Tests: Multiple Operations (30s budget)
// ============================================================================

/// OODA-11: Multiple sequential uploads must complete within 30s.
#[tokio::test]
async fn test_timeout_sequential_uploads_30s() {
    let result = with_timeout(Duration::from_secs(30), async {
        let app = create_test_app();

        let doc1 = upload_document(&app, SMALL_DOC, "Seq 1").await;
        let doc2 = upload_document(&app, MEDIUM_DOC, "Seq 2").await;
        let doc3 = upload_document(&app, LARGE_DOC, "Seq 3").await;

        (doc1, doc2, doc3)
    })
    .await;

    assert!(
        result.is_ok(),
        "Sequential uploads: {}",
        result.unwrap_err()
    );
    let (d1, d2, d3) = result.unwrap();
    assert!(d1["document_id"].is_string());
    assert!(d2["document_id"].is_string());
    assert!(d3["document_id"].is_string());
}

/// OODA-11: Health check must complete within 5s.
#[tokio::test]
async fn test_timeout_health_check_5s() {
    let result = with_timeout(Duration::from_secs(5), async {
        let app = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        extract_json(response).await
    })
    .await;

    assert!(result.is_ok(), "Health check: {}", result.unwrap_err());
    let body = result.unwrap();
    assert_eq!(body["status"], "healthy");
}

/// OODA-11: Tenant creation must complete within 5s.
#[tokio::test]
async fn test_timeout_tenant_creation_5s() {
    let result = with_timeout(Duration::from_secs(5), async {
        let app = create_test_app();

        let create_req = json!({
            "name": "Timeout Test Tenant",
            "slug": format!("timeout-test-{}", uuid::Uuid::new_v4()),
            "plan": "free"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/tenants")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&create_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        extract_json(response).await
    })
    .await;

    assert!(result.is_ok(), "Tenant creation: {}", result.unwrap_err());
}
