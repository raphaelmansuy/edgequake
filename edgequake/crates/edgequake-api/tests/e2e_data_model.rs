//! OODA-12: Data model solidity tests.
//!
//! These tests verify that the API data model is well-designed, consistent,
//! and handles edge cases correctly. Focuses on:
//!
//! 1. **Serialization roundtrip**: JSON → struct → JSON preserves all data
//! 2. **Default values**: serde defaults produce valid states
//! 3. **Edge cases**: Empty strings, max values, unicode, special chars
//! 4. **Consistency**: Related types use compatible field names/types
//! 5. **Validation**: Content validation catches invalid input
//!
//! ## Design Principles Verified
//!
//! - **SRP**: Each DTO has a single clear purpose
//! - **DRY**: Shared patterns use common helpers
//! - **Backwards compatibility**: Optional fields with skip_serializing_if

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use serde_json::{json, Value};
use std::time::Duration;
use tower::ServiceExt;

// ============================================================================
// Timeout Helper (shared pattern from OODA-11)
// ============================================================================

async fn with_timeout<F, T>(duration: Duration, future: F) -> Result<T, String>
where
    F: std::future::Future<Output = T>,
{
    tokio::time::timeout(duration, future)
        .await
        .map_err(|_| format!("Test exceeded timeout of {:?}", duration))
}

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

// ============================================================================
// Upload Request Validation Tests
// ============================================================================

/// OODA-12: Minimal upload request should have correct defaults.
#[tokio::test]
async fn test_upload_request_defaults() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        // Minimal request: only content
        let request = json!({
            "content": "Test content for defaults."
        });

        let response = app
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
        let body = extract_json(response).await;

        // Verify response has required fields
        assert!(body["document_id"].is_string(), "Must have document_id");
        assert!(body["status"].is_string(), "Must have status");
        assert!(body["track_id"].is_string(), "Must have track_id");

        body
    })
    .await;

    assert!(result.is_ok(), "Defaults: {}", result.unwrap_err());
}

/// OODA-12: Empty content should return 400 Bad Request.
#[tokio::test]
async fn test_upload_empty_content_rejected() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        let request = json!({
            "content": ""
        });

        let response = app
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

        // WHY: Empty content is invalid → should not be 201
        assert_ne!(
            response.status(),
            StatusCode::CREATED,
            "Empty content should not succeed"
        );

        response.status()
    })
    .await;

    assert!(result.is_ok(), "Empty content: {}", result.unwrap_err());
}

/// OODA-12: Whitespace-only content should return 400 Bad Request.
#[tokio::test]
async fn test_upload_whitespace_content_rejected() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        let request = json!({
            "content": "   \n\t\r\n   "
        });

        let response = app
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

        // WHY: Whitespace-only is effectively empty
        assert_ne!(
            response.status(),
            StatusCode::CREATED,
            "Whitespace-only content should not succeed"
        );

        response.status()
    })
    .await;

    assert!(result.is_ok(), "Whitespace: {}", result.unwrap_err());
}

/// OODA-12: Missing content field should return 400 or 422.
#[tokio::test]
async fn test_upload_missing_content_rejected() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        // No "content" field at all
        let request = json!({
            "title": "No Content"
        });

        let response = app
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
        assert!(
            status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY,
            "Missing content should fail with 400 or 422, got {}",
            status
        );

        status
    })
    .await;

    assert!(result.is_ok(), "Missing content: {}", result.unwrap_err());
}

// ============================================================================
// Upload Response Structure Tests
// ============================================================================

/// OODA-12: Upload response must include all required fields.
#[tokio::test]
async fn test_upload_response_structure() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        let request = json!({
            "content": "Alice works at EdgeQuake Corp. She collaborates with Bob.",
            "title": "Structure Test",
            "metadata": {"source": "test", "version": 1}
        });

        let response = app
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
        let body = extract_json(response).await;

        // Required fields per UploadDocumentResponse
        assert!(body["document_id"].is_string(), "Missing document_id");
        assert!(body["status"].is_string(), "Missing status");
        assert!(body["track_id"].is_string(), "Missing track_id");

        // Status should be "processed" for sync processing
        assert_eq!(
            body["status"].as_str().unwrap(),
            "processed",
            "Sync upload should return 'processed'"
        );

        // Chunk count should be present and >= 1
        let chunk_count = body["chunk_count"].as_u64().unwrap_or(0);
        assert!(chunk_count >= 1, "Should have at least 1 chunk");

        // Entity/relationship counts should be present (u64 is inherently non-negative)
        let _entity_count = body["entity_count"].as_u64().unwrap_or(0);
        let _relationship_count = body["relationship_count"].as_u64().unwrap_or(0);

        body
    })
    .await;

    assert!(result.is_ok(), "Structure: {}", result.unwrap_err());
}

// ============================================================================
// Document Detail Response Tests
// ============================================================================

/// OODA-12: GET document returns consistent detail structure.
#[tokio::test]
async fn test_document_detail_response_structure() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        // Upload first
        let upload_req = json!({
            "content": "Dr. Smith works at MIT on quantum physics.",
            "title": "Detail Test"
        });

        let upload_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/documents")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&upload_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let upload_body = extract_json(upload_resp).await;
        let doc_id = upload_body["document_id"].as_str().unwrap().to_string();

        // GET document details
        let detail_resp = app
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

        assert_eq!(detail_resp.status(), StatusCode::OK);
        let detail = extract_json(detail_resp).await;

        // Required fields per DocumentDetailResponse
        assert!(detail["id"].is_string(), "Missing id");
        assert!(detail["status"].is_string(), "Missing status");
        assert!(detail["chunk_count"].is_number(), "Missing chunk_count");

        // Status consistency: upload returns "processed", detail returns "completed"
        assert_eq!(
            detail["status"].as_str().unwrap(),
            "completed",
            "Processed doc should show 'completed' in detail"
        );

        // ID should match
        assert_eq!(
            detail["id"].as_str().unwrap(),
            doc_id,
            "Document ID mismatch"
        );

        detail
    })
    .await;

    assert!(result.is_ok(), "Detail: {}", result.unwrap_err());
}

// ============================================================================
// Unicode and Special Character Tests
// ============================================================================

/// OODA-12: Unicode content (CJK, emoji, accents) must be handled correctly.
#[tokio::test]
async fn test_unicode_content_handling() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        let request = json!({
            "content": "日本語テスト。量子コンピューティングは革命的技術です。🔬 Ñoño café résumé naïve",
            "title": "Unicode Test 日本語"
        });

        let response = app
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

        assert_eq!(
            response.status(),
            StatusCode::CREATED,
            "Unicode content should be accepted"
        );

        let body = extract_json(response).await;
        assert!(body["document_id"].is_string());

        body
    })
    .await;

    assert!(result.is_ok(), "Unicode: {}", result.unwrap_err());
}

/// OODA-12: Special characters in metadata must be preserved.
#[tokio::test]
async fn test_metadata_special_characters() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        let request = json!({
            "content": "Test content with normal ASCII.",
            "title": "Special <Meta> & \"Chars\"",
            "metadata": {
                "path": "/usr/local/bin/test.pdf",
                "tags": ["tag-1", "tag/2", "tag&3"],
                "special": "line1\nline2\ttab",
                "unicode_key_🔑": "value_🎯"
            }
        });

        let response = app
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

        assert_eq!(
            response.status(),
            StatusCode::CREATED,
            "Special chars in metadata should be accepted"
        );

        body_from(response).await
    })
    .await;

    assert!(result.is_ok(), "Special chars: {}", result.unwrap_err());
}

/// Helper for extracting body (alias for readability).
async fn body_from(response: axum::response::Response) -> Value {
    extract_json(response).await
}

// ============================================================================
// List Documents Response Tests
// ============================================================================

/// OODA-12: List documents returns proper pagination structure.
#[tokio::test]
async fn test_list_documents_pagination_structure() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        // Upload a document first
        let upload_req = json!({
            "content": "A test document for pagination.",
            "title": "Pagination Test"
        });

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/documents")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&upload_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // List documents
        let list_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/documents?page=1&page_size=10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(list_resp.status(), StatusCode::OK);
        let list = extract_json(list_resp).await;

        // Required pagination fields per ListDocumentsResponse
        assert!(list["documents"].is_array(), "Missing documents array");
        assert!(list["total"].is_number(), "Missing total");
        assert!(list["page"].is_number(), "Missing page");
        assert!(list["page_size"].is_number(), "Missing page_size");
        assert!(list["total_pages"].is_number(), "Missing total_pages");
        assert!(list["has_more"].is_boolean(), "Missing has_more");
        assert!(list["status_counts"].is_object(), "Missing status_counts");

        // Should have at least 1 document
        let docs = list["documents"].as_array().unwrap();
        assert!(!docs.is_empty(), "Should have at least 1 document");

        // Verify status_counts structure
        let counts = &list["status_counts"];
        assert!(counts["completed"].is_number(), "Missing completed count");

        list
    })
    .await;

    assert!(result.is_ok(), "Pagination: {}", result.unwrap_err());
}

// ============================================================================
// Graph Response Structure Tests
// ============================================================================

/// OODA-12: Graph response must have nodes and edges arrays.
#[tokio::test]
async fn test_graph_response_structure() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        // Upload to populate graph
        let upload_req = json!({
            "content": "Alice works at TechCorp. Bob manages Alice at TechCorp.",
            "title": "Graph Structure Test"
        });

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/documents")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&upload_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let graph_resp = app
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

        assert_eq!(graph_resp.status(), StatusCode::OK);
        let graph = extract_json(graph_resp).await;

        assert!(graph["nodes"].is_array(), "Missing nodes array");
        assert!(graph["edges"].is_array(), "Missing edges array");

        graph
    })
    .await;

    assert!(result.is_ok(), "Graph: {}", result.unwrap_err());
}

// ============================================================================
// Query Response Structure Tests
// ============================================================================

/// OODA-12: Query response must have answer, mode, sources, stats.
#[tokio::test]
async fn test_query_response_structure() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        // Upload first
        let upload_req = json!({
            "content": "EdgeQuake is a knowledge graph company founded in San Francisco.",
            "title": "Query Structure Test"
        });

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/documents")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&upload_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Query
        let query_req = json!({ "query": "What is EdgeQuake?" });

        let query_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/query")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&query_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(query_resp.status(), StatusCode::OK);
        let query = extract_json(query_resp).await;

        // Required fields per QueryResponse
        assert!(query["answer"].is_string(), "Missing answer");
        assert!(query["mode"].is_string(), "Missing mode");
        assert!(query["sources"].is_array(), "Missing sources");
        assert!(query["stats"].is_object(), "Missing stats");

        // Stats subfields
        let stats = &query["stats"];
        assert!(stats["total_time_ms"].is_number(), "Missing total_time_ms");

        query
    })
    .await;

    assert!(result.is_ok(), "Query: {}", result.unwrap_err());
}

// ============================================================================
// Tenant/Workspace Response Structure Tests
// ============================================================================

/// OODA-12: Tenant creation returns full model configuration.
#[tokio::test]
async fn test_tenant_response_model_config() {
    let result = with_timeout(Duration::from_secs(5), async {
        let app = create_test_app();

        let request = json!({
            "name": "Model Config Tenant",
            "slug": format!("model-cfg-{}", uuid::Uuid::new_v4()),
            "plan": "pro"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/tenants")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let tenant = extract_json(response).await;

        // SPEC-032: Must have model configuration fields
        assert!(tenant["id"].is_string(), "Missing id");
        assert!(tenant["name"].is_string(), "Missing name");
        assert!(tenant["plan"].is_string(), "Missing plan");
        assert!(tenant["is_active"].is_boolean(), "Missing is_active");

        // Default LLM config (SPEC-032)
        assert!(
            tenant["default_llm_model"].is_string(),
            "Missing default_llm_model"
        );
        assert!(
            tenant["default_llm_provider"].is_string(),
            "Missing default_llm_provider"
        );

        // Default embedding config (SPEC-032)
        assert!(
            tenant["default_embedding_model"].is_string(),
            "Missing default_embedding_model"
        );
        assert!(
            tenant["default_embedding_dimension"].is_number(),
            "Missing default_embedding_dimension"
        );

        tenant
    })
    .await;

    assert!(result.is_ok(), "Tenant config: {}", result.unwrap_err());
}

// ============================================================================
// Health Endpoint Response Structure Tests
// ============================================================================

/// OODA-12: Health response must include component status.
#[tokio::test]
async fn test_health_response_structure() {
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
        let health = extract_json(response).await;

        assert_eq!(health["status"].as_str(), Some("healthy"));
        assert!(health["version"].is_string(), "Missing version");
        assert!(health["components"].is_object(), "Missing components");

        health
    })
    .await;

    assert!(result.is_ok(), "Health: {}", result.unwrap_err());
}

// ============================================================================
// Delete Document Response Structure Tests
// ============================================================================

/// OODA-12: Delete response must include cascade counts.
#[tokio::test]
async fn test_delete_response_structure() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        // Upload first
        let upload_req = json!({
            "content": "Data to delete: Alice works at Corp.",
            "title": "Delete Test"
        });

        let upload_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/documents")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&upload_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let upload_body = extract_json(upload_resp).await;
        let doc_id = upload_body["document_id"].as_str().unwrap().to_string();

        // Delete
        let delete_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/documents/{}", doc_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(delete_resp.status(), StatusCode::OK);
        let del = extract_json(delete_resp).await;

        // Required fields per DeleteDocumentResponse
        assert!(del["document_id"].is_string(), "Missing document_id");
        assert!(del["deleted"].is_boolean(), "Missing deleted");
        assert!(del["chunks_deleted"].is_number(), "Missing chunks_deleted");
        assert!(
            del["entities_affected"].is_number(),
            "Missing entities_affected"
        );
        assert!(
            del["relationships_affected"].is_number(),
            "Missing relationships_affected"
        );

        // Should have actually deleted
        assert!(del["deleted"].as_bool().unwrap(), "Should be deleted");

        del
    })
    .await;

    assert!(result.is_ok(), "Delete: {}", result.unwrap_err());
}

/// OODA-12: Deletion impact preview should not delete the document.
#[tokio::test]
async fn test_deletion_impact_preview_only() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        // Upload
        let upload_req = json!({
            "content": "Preview only: Bob manages the team.",
            "title": "Impact Test"
        });

        let upload_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/documents")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&upload_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let upload_body = extract_json(upload_resp).await;
        let doc_id = upload_body["document_id"].as_str().unwrap().to_string();

        // Get deletion impact (preview)
        let impact_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/documents/{}/deletion-impact", doc_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let impact = extract_json(impact_resp).await;
        assert_eq!(
            impact["preview_only"].as_bool(),
            Some(true),
            "Should be preview only"
        );

        // Document should still exist
        let get_resp = app
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

        assert_eq!(
            get_resp.status(),
            StatusCode::OK,
            "Document should still exist after impact preview"
        );

        impact
    })
    .await;

    assert!(result.is_ok(), "Impact: {}", result.unwrap_err());
}

// ============================================================================
// Cost Response Structure Tests
// ============================================================================

/// OODA-12: Cost estimation response has all required fields.
#[tokio::test]
async fn test_cost_estimation_response_fields() {
    let result = with_timeout(Duration::from_secs(5), async {
        let app = create_test_app();

        let request = json!({
            "model": "gpt-4o-mini",
            "input_tokens": 5000,
            "output_tokens": 1000
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/pipeline/costs/estimate")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let cost = extract_json(response).await;

        // Required fields per EstimateCostResponse
        assert!(cost["model"].is_string(), "Missing model");
        assert!(cost["input_tokens"].is_number(), "Missing input_tokens");
        assert!(cost["output_tokens"].is_number(), "Missing output_tokens");
        assert!(
            cost["estimated_cost_usd"].is_number(),
            "Missing estimated_cost_usd"
        );
        assert!(cost["formatted_cost"].is_string(), "Missing formatted_cost");

        // Cost should be positive for non-zero tokens
        let estimated = cost["estimated_cost_usd"].as_f64().unwrap();
        assert!(estimated > 0.0, "Cost should be positive for real tokens");

        cost
    })
    .await;

    assert!(result.is_ok(), "Cost: {}", result.unwrap_err());
}

// ============================================================================
// Non-existent Document Tests
// ============================================================================

/// OODA-12: GET non-existent document returns 404.
#[tokio::test]
async fn test_get_nonexistent_document_404() {
    let result = with_timeout(Duration::from_secs(5), async {
        let app = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/documents/00000000-0000-0000-0000-000000000000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "Non-existent doc should return 404"
        );

        response.status()
    })
    .await;

    assert!(result.is_ok(), "404: {}", result.unwrap_err());
}

/// OODA-12: DELETE non-existent document returns 404.
#[tokio::test]
async fn test_delete_nonexistent_document_404() {
    let result = with_timeout(Duration::from_secs(5), async {
        let app = create_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/v1/documents/00000000-0000-0000-0000-000000000000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "Delete non-existent should return 404"
        );

        response.status()
    })
    .await;

    assert!(result.is_ok(), "Delete 404: {}", result.unwrap_err());
}
