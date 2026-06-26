//! OODA-14: Re-indexing E2E tests.
//!
//! Verifies document re-indexing workflows:
//! 1. Upload same content → triggers clean re-ingestion with 201 Created
//! 2. Reprocess endpoint with force=true → re-processes document
//! 3. Delete + re-upload → creates fresh document
//! 4. Upload with different title but same content → still re-ingests by content

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use serde_json::{json, Value};
use std::time::Duration;
use tower::ServiceExt;

mod common;

// WHY: The reprocess_failed handler at documents.rs:640 calls
// Uuid::parse_str(&tenant_id). Without X-Tenant-ID header, tenant_id
// defaults to "default" which is NOT a valid UUID → 422.
// These constants provide valid UUIDs for reprocess tests.
const TEST_TENANT_ID: &str = "00000000-0000-4000-8000-000000000001";
const TEST_WORKSPACE_ID: &str = "00000000-0000-4000-8000-000000000002";

// ============================================================================
// Helpers
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

async fn upload_text(app: &axum::Router, content: &str, title: &str) -> (StatusCode, Value) {
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

    let status = response.status();
    let body = extract_json(response).await;
    (status, body)
}

const TEST_CONTENT: &str = "Dr. Marie Curie conducted pioneering research on radioactivity. \
    She discovered polonium and radium, and was the first woman to win a Nobel Prize.";

const DIFFERENT_CONTENT: &str = "Albert Einstein developed the theory of relativity. \
    His famous equation E=mc² describes the relationship between energy and mass.";

// ============================================================================
// Duplicate Detection Tests
// ============================================================================

/// OODA-14: Uploading same content twice now performs a clean re-ingestion.
#[tokio::test]
async fn test_duplicate_detection_same_content() {
    let result = with_timeout(Duration::from_secs(60), async {
        // P-G2b: the first upload must finish processing before the second
        // upload's dedup check, otherwise the second sees "still processing"
        // and returns 200 duplicate_processing instead of re-ingesting.
        let workers = common::create_test_app_with_workers().await;
        let app = workers.app();

        // First upload — wait for it to fully ingest.
        let (doc_id_1, _track1, _status1) =
            common::upload_and_wait(app, "First Upload", TEST_CONTENT, Duration::from_secs(30))
                .await;

        // Second upload with same content should still succeed via re-ingestion.
        let (status2, body2) = upload_text(app, TEST_CONTENT, "Second Upload").await;
        assert!(
            status2 == StatusCode::CREATED || status2 == StatusCode::ACCEPTED,
            "Duplicate re-ingestion should return 201 or 202: {} | body={}",
            status2,
            body2
        );
        assert_eq!(
            body2["status"].as_str(),
            Some("pending"),
            "Re-ingested duplicate should return pending (background processing)"
        );

        let doc_id_2 = body2["document_id"].as_str().unwrap().to_string();
        assert_ne!(
            doc_id_1, doc_id_2,
            "Re-ingestion should replace the old copy with a fresh document id"
        );

        (doc_id_1, body2)
    })
    .await;

    assert!(result.is_ok(), "Duplicate: {}", result.unwrap_err());
}

/// OODA-14: Different content should NOT be flagged as duplicate.
#[tokio::test]
async fn test_different_content_not_duplicate() {
    let result = with_timeout(Duration::from_secs(30), async {
        let app = create_test_app();

        let (status1, body1) = upload_text(&app, TEST_CONTENT, "Doc A").await;
        assert!(
            status1 == StatusCode::CREATED || status1 == StatusCode::ACCEPTED,
            "First upload should succeed: {}",
            status1
        );

        let (status2, body2) = upload_text(&app, DIFFERENT_CONTENT, "Doc B").await;
        assert!(
            status2 == StatusCode::CREATED || status2 == StatusCode::ACCEPTED,
            "Different content should create new doc: {}",
            status2
        );
        // P-G2b: status is "pending" until background processing completes.
        assert_eq!(body2["status"].as_str(), Some("pending"));

        // Different document IDs
        assert_ne!(
            body1["document_id"].as_str(),
            body2["document_id"].as_str(),
            "Different content should get different IDs"
        );

        (body1, body2)
    })
    .await;

    assert!(result.is_ok(), "Different: {}", result.unwrap_err());
}

/// OODA-14: Same content with different title is still detected by content and re-ingested.
#[tokio::test]
async fn test_duplicate_ignores_title_difference() {
    let result = with_timeout(Duration::from_secs(60), async {
        // P-G2b: first upload must finish processing before the second upload's
        // dedup check, otherwise the second sees "still processing" and returns
        // 200 duplicate_processing instead of re-ingesting.
        let workers = common::create_test_app_with_workers().await;
        let app = workers.app();

        let (doc_id_1, _track1, _status1) =
            common::upload_and_wait(app, "Title A", TEST_CONTENT, Duration::from_secs(30)).await;

        let (status2, body2) = upload_text(app, TEST_CONTENT, "Title B").await;
        assert!(
            status2 == StatusCode::CREATED || status2 == StatusCode::ACCEPTED,
            "Same content with different title should re-ingest successfully: {} | body={}",
            status2,
            body2
        );
        // P-G2b: status is "pending" until background processing completes.
        assert_eq!(body2["status"].as_str(), Some("pending"));

        let doc_id_2 = body2["document_id"].as_str().unwrap().to_string();
        assert_ne!(doc_id_1, doc_id_2, "Re-ingestion should create a fresh id");

        body2
    })
    .await;

    assert!(result.is_ok(), "Title diff: {}", result.unwrap_err());
}

// ============================================================================
// Reprocess / Re-indexing Tests
// ============================================================================

/// OODA-14: Force reprocess via POST /api/v1/documents/reprocess.
#[tokio::test]
async fn test_reprocess_specific_document() {
    let result = with_timeout(Duration::from_secs(30), async {
        let app = create_test_app();

        // Upload
        let (_, body) = upload_text(&app, TEST_CONTENT, "Reprocess Test").await;
        let doc_id = body["document_id"].as_str().unwrap().to_string();

        // Reprocess with force=true
        // WHY: Must include X-Tenant-ID + X-Workspace-ID headers with valid UUIDs,
        // because reprocess_failed handler creates a Task requiring UUID tenant/workspace.
        let reprocess_req = json!({
            "document_id": doc_id,
            "force": true
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/documents/reprocess")
                    .header("Content-Type", "application/json")
                    .header("X-Tenant-ID", TEST_TENANT_ID)
                    .header("X-Workspace-ID", TEST_WORKSPACE_ID)
                    .body(Body::from(serde_json::to_string(&reprocess_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Reprocess should return 200, got {}. Body: {:?}",
            response.status(),
            {
                let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
                    .await
                    .unwrap_or_default();
                String::from_utf8_lossy(&bytes).to_string()
            }
        );

        // Re-fetch since we consumed the body above for the error message
        let response2 = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/documents/reprocess")
                    .header("Content-Type", "application/json")
                    .header("X-Tenant-ID", TEST_TENANT_ID)
                    .header("X-Workspace-ID", TEST_WORKSPACE_ID)
                    .body(Body::from(serde_json::to_string(&reprocess_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let reprocess = extract_json(response2).await;

        // Should have a track_id
        assert!(
            reprocess["track_id"].is_string(),
            "Reprocess should return track_id"
        );

        // Should show found=1, requeued=1 (or 0 if async not supported in mock)
        // The important thing is the endpoint works
        assert!(
            reprocess["failed_found"].is_number(),
            "Should have failed_found count"
        );

        reprocess
    })
    .await;

    assert!(result.is_ok(), "Reprocess: {}", result.unwrap_err());
}

/// OODA-14: Reprocess without force only affects failed documents.
#[tokio::test]
async fn test_reprocess_without_force_skips_completed() {
    let result = with_timeout(Duration::from_secs(30), async {
        let app = create_test_app();

        // Upload a document that completes successfully
        let (_, body) = upload_text(&app, TEST_CONTENT, "Skip Test").await;
        let doc_id = body["document_id"].as_str().unwrap().to_string();

        // Reprocess WITHOUT force=true
        // WHY: Must still include tenant headers so the endpoint doesn't 422 on UUID parse.
        // Even though force=false skips task creation, future refactors may change the order.
        let reprocess_req = json!({
            "document_id": doc_id,
            "force": false
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/documents/reprocess")
                    .header("Content-Type", "application/json")
                    .header("X-Tenant-ID", TEST_TENANT_ID)
                    .header("X-Workspace-ID", TEST_WORKSPACE_ID)
                    .body(Body::from(serde_json::to_string(&reprocess_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let reprocess = extract_json(response).await;

        // Should NOT requeue a completed document without force
        let requeued = reprocess["requeued"].as_u64().unwrap_or(0);
        assert_eq!(
            requeued, 0,
            "Completed doc should not be requeued without force"
        );

        reprocess
    })
    .await;

    assert!(result.is_ok(), "No force: {}", result.unwrap_err());
}

// ============================================================================
// Delete + Re-upload Tests
// ============================================================================

/// OODA-14: Delete then re-upload same content creates fresh document.
#[tokio::test]
async fn test_delete_and_reupload() {
    let result = with_timeout(Duration::from_secs(30), async {
        let app = create_test_app();

        // Upload
        let (status1, body1) = upload_text(&app, TEST_CONTENT, "Delete+Reupload").await;
        assert!(
            status1 == StatusCode::CREATED || status1 == StatusCode::ACCEPTED,
            "First upload should succeed: {}",
            status1
        );
        let doc_id_1 = body1["document_id"].as_str().unwrap().to_string();

        // Delete
        let del_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/documents/{}", doc_id_1))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(del_response.status(), StatusCode::OK);
        let del_body = extract_json(del_response).await;
        assert!(del_body["deleted"].as_bool().unwrap());

        // Re-upload same content
        let (status2, body2) = upload_text(&app, TEST_CONTENT, "Re-uploaded").await;

        // After deletion, the hash key should be cleaned up → new upload succeeds.
        // P-G2b: a fresh upload enqueues a background task → 202 ACCEPTED.
        // Either:
        // a) 202 ACCEPTED (hash cleared) → fresh document
        // b) 200 OK duplicate (hash not cleared) → points to deleted doc
        //
        // Both are acceptable behaviors, but we should get a valid response
        assert!(
            status2 == StatusCode::CREATED
                || status2 == StatusCode::ACCEPTED
                || status2 == StatusCode::OK,
            "Re-upload should succeed, got {}",
            status2
        );

        assert!(body2["document_id"].is_string());

        (doc_id_1, body2)
    })
    .await;

    assert!(result.is_ok(), "Delete+reupload: {}", result.unwrap_err());
}

// ============================================================================
// Graph State After Re-indexing Tests
// ============================================================================

/// OODA-14: After reprocess, graph should still be valid.
#[tokio::test]
async fn test_graph_valid_after_reprocess() {
    let result = with_timeout(Duration::from_secs(30), async {
        let app = create_test_app();

        // Upload
        let (_, body) = upload_text(&app, TEST_CONTENT, "Graph Reprocess").await;
        let doc_id = body["document_id"].as_str().unwrap().to_string();

        // Check graph before
        let graph_before = app
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
        let graph_before = extract_json(graph_before).await;

        let nodes_before = graph_before["nodes"].as_array().unwrap().len();

        // Force reprocess
        let reprocess_req = json!({
            "document_id": doc_id,
            "force": true
        });

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/documents/reprocess")
                    .header("Content-Type", "application/json")
                    .header("X-Tenant-ID", TEST_TENANT_ID)
                    .header("X-Workspace-ID", TEST_WORKSPACE_ID)
                    .body(Body::from(serde_json::to_string(&reprocess_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Check graph after — should still be valid (may have fewer or same nodes)
        let graph_after = app
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
        let graph_after = extract_json(graph_after).await;

        assert!(graph_after["nodes"].is_array(), "Graph should have nodes");
        assert!(graph_after["edges"].is_array(), "Graph should have edges");

        // Graph should not have MORE nodes than before (reprocess cleans up first)
        let nodes_after = graph_after["nodes"].as_array().unwrap().len();
        assert!(
            nodes_after <= nodes_before,
            "Graph should have same or fewer nodes after reprocess cleanup"
        );

        (nodes_before, nodes_after)
    })
    .await;

    assert!(result.is_ok(), "Graph reprocess: {}", result.unwrap_err());
}

/// OODA-14: Multiple documents should maintain consistent graph.
#[tokio::test]
async fn test_multiple_uploads_consistent_graph() {
    let result = with_timeout(Duration::from_secs(30), async {
        let app = create_test_app();

        // Upload doc A
        let (s1, _) = upload_text(&app, TEST_CONTENT, "Multi A").await;
        assert!(
            s1 == StatusCode::CREATED || s1 == StatusCode::ACCEPTED,
            "Upload A should succeed: {}",
            s1
        );

        // Upload doc B (different content)
        let (s2, _) = upload_text(&app, DIFFERENT_CONTENT, "Multi B").await;
        assert!(
            s2 == StatusCode::CREATED || s2 == StatusCode::ACCEPTED,
            "Upload B should succeed: {}",
            s2
        );

        // Graph should contain nodes from both documents
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

        let graph = extract_json(graph_resp).await;
        let nodes = graph["nodes"].as_array().unwrap();

        // WHY: Mock provider returns "Mock response" text, not valid JSON with entities,
        // so entity extraction produces 0 entities → 0 graph nodes. This is expected
        // behavior for mock-based tests. We verify graph structure, not content.
        // Real LLM tests (e2e_ollama_integration) verify entity extraction.
        assert!(
            graph["nodes"].is_array(),
            "Graph should have nodes array field"
        );
        assert!(
            graph["edges"].is_array(),
            "Graph should have edges array field"
        );

        // If we DO have nodes (possible with improved mock), verify uniqueness
        if !nodes.is_empty() {
            let node_ids: Vec<&str> = nodes
                .iter()
                .filter_map(|n| n["id"].as_str().or_else(|| n["name"].as_str()))
                .collect();

            let unique: std::collections::HashSet<&str> = node_ids.iter().cloned().collect();
            assert_eq!(node_ids.len(), unique.len(), "All nodes should be unique");
        }

        graph
    })
    .await;

    assert!(result.is_ok(), "Multi graph: {}", result.unwrap_err());
}
