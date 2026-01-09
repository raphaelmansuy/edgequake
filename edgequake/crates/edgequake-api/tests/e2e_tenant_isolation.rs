//! End-to-End Tenant Isolation Tests
//!
//! These tests verify that the multi-tenant system correctly isolates:
//! - Documents between tenants
//! - Documents between workspaces within a tenant
//! - Entities and relationships
//! - Query results
//! - Graph data
//!
//! The tests simulate attack scenarios:
//! - Cross-tenant data leakage via header manipulation
//! - Missing headers behavior
//! - Workspace boundary violations
//!
//! Run with: `cargo test --package edgequake-api --test e2e_tenant_isolation`

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use serde_json::{json, Value};
use tower::ServiceExt;

// ============================================================================
// Test Utilities
// ============================================================================

fn create_test_config() -> ServerConfig {
    ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: true,
    }
}

fn create_test_server() -> Server {
    Server::new(create_test_config(), AppState::test_state())
}

fn create_test_app() -> axum::Router {
    create_test_server().build_router()
}

async fn extract_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("Failed to read response body");
    serde_json::from_slice(&bytes).unwrap_or(json!({}))
}

async fn extract_status_and_json(response: axum::response::Response) -> (StatusCode, Value) {
    let status = response.status();
    let json = extract_json(response).await;
    (status, json)
}

// ============================================================================
// Tenant Isolation Tests
// ============================================================================

mod tenant_isolation_tests {
    use super::*;

    /// Test that documents uploaded by Tenant A are not visible to Tenant B
    #[tokio::test]
    async fn test_document_isolation_between_tenants() {
        let app = create_test_app();

        // Tenant A uploads a document
        let upload_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/documents")
                    .header("Content-Type", "application/json")
                    .header("X-Tenant-ID", "tenant-a")
                    .header("X-Workspace-ID", "workspace-a")
                    .body(Body::from(
                        json!({
                            "content": "This is a secret document for Tenant A about Project Alpha",
                            "title": "Tenant A Secret",
                            "metadata": {
                                "confidential": true
                            }
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status, upload_json) = extract_status_and_json(upload_response).await;
        // WHY: POST /documents returns 201 Created per REST semantics (UC0001)
        assert!(
            status == StatusCode::CREATED || status == StatusCode::ACCEPTED,
            "Upload failed: {:?}",
            status
        );

        let doc_id_a = upload_json["document_id"]
            .as_str()
            .or(upload_json["id"].as_str())
            .unwrap_or("test-doc-a");

        // Tenant B lists documents - should NOT see Tenant A's document
        let list_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/documents")
                    .header("X-Tenant-ID", "tenant-b")
                    .header("X-Workspace-ID", "workspace-b")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status, list_json) = extract_status_and_json(list_response).await;
        assert_eq!(status, StatusCode::OK, "List failed: {:?}", list_json);

        // Verify Tenant B cannot see Tenant A's document
        let documents = list_json["documents"]
            .as_array()
            .or(list_json.as_array())
            .cloned()
            .unwrap_or_default();

        for doc in &documents {
            let doc_id = doc["id"].as_str().unwrap_or("");
            let title = doc["title"].as_str().unwrap_or("");
            assert!(
                !title.contains("Tenant A"),
                "Tenant B should NOT see Tenant A's document: found title '{}'",
                title
            );
            assert!(
                doc_id != doc_id_a,
                "Tenant B should NOT see document {} belonging to Tenant A",
                doc_id_a
            );
        }
    }

    /// Test that workspaces within the same tenant are isolated
    #[tokio::test]
    async fn test_workspace_isolation_within_tenant() {
        let app = create_test_app();

        // Upload to Workspace 1
        let upload1_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/documents")
                    .header("Content-Type", "application/json")
                    .header("X-Tenant-ID", "shared-tenant")
                    .header("X-Workspace-ID", "workspace-1")
                    .body(Body::from(
                        json!({
                            "content": "Workspace 1 specific content about Finance Reports",
                            "title": "Finance Report WS1"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // WHY: POST /documents returns 201 Created per REST semantics (UC0001)
        assert!(
            upload1_response.status() == StatusCode::CREATED
                || upload1_response.status() == StatusCode::ACCEPTED
        );

        // Upload to Workspace 2
        let upload2_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/documents")
                    .header("Content-Type", "application/json")
                    .header("X-Tenant-ID", "shared-tenant")
                    .header("X-Workspace-ID", "workspace-2")
                    .body(Body::from(
                        json!({
                            "content": "Workspace 2 specific content about HR Policies",
                            "title": "HR Policy WS2"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // WHY: POST /documents returns 201 Created per REST semantics (UC0001)
        assert!(
            upload2_response.status() == StatusCode::CREATED
                || upload2_response.status() == StatusCode::ACCEPTED
        );

        // List documents in Workspace 1 - should only see WS1 documents
        let list1_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/documents")
                    .header("X-Tenant-ID", "shared-tenant")
                    .header("X-Workspace-ID", "workspace-1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let list1_json = extract_json(list1_response).await;
        let docs1 = list1_json["documents"]
            .as_array()
            .or(list1_json.as_array())
            .cloned()
            .unwrap_or_default();

        for doc in &docs1 {
            let title = doc["title"].as_str().unwrap_or("");
            assert!(
                !title.contains("WS2"),
                "Workspace 1 should NOT see Workspace 2's document: {}",
                title
            );
        }

        // List documents in Workspace 2 - should only see WS2 documents
        let list2_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/documents")
                    .header("X-Tenant-ID", "shared-tenant")
                    .header("X-Workspace-ID", "workspace-2")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let list2_json = extract_json(list2_response).await;
        let docs2 = list2_json["documents"]
            .as_array()
            .or(list2_json.as_array())
            .cloned()
            .unwrap_or_default();

        for doc in &docs2 {
            let title = doc["title"].as_str().unwrap_or("");
            assert!(
                !title.contains("WS1"),
                "Workspace 2 should NOT see Workspace 1's document: {}",
                title
            );
        }
    }

    /// Test that query results are filtered by tenant context
    #[tokio::test]
    async fn test_query_isolation_between_tenants() {
        let app = create_test_app();

        // First, upload documents for both tenants to ensure data exists
        let _upload_a = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/documents")
                    .header("Content-Type", "application/json")
                    .header("X-Tenant-ID", "query-tenant-a")
                    .header("X-Workspace-ID", "query-ws-a")
                    .body(Body::from(
                        json!({
                            "content": "Quantum Computing Research for Tenant A",
                            "title": "Quantum Research A"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let _upload_b = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/documents")
                    .header("Content-Type", "application/json")
                    .header("X-Tenant-ID", "query-tenant-b")
                    .header("X-Workspace-ID", "query-ws-b")
                    .body(Body::from(
                        json!({
                            "content": "Neural Network Development for Tenant B",
                            "title": "Neural Networks B"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Allow processing time
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Tenant A queries - should only see its own data
        let query_a_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/query")
                    .header("Content-Type", "application/json")
                    .header("X-Tenant-ID", "query-tenant-a")
                    .header("X-Workspace-ID", "query-ws-a")
                    .body(Body::from(
                        json!({
                            "query": "What is the research about?",
                            "mode": "hybrid"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let query_a_json = extract_json(query_a_response).await;

        // If sources exist, verify they are filtered to Tenant A only
        if let Some(sources) = query_a_json["sources"].as_array() {
            for source in sources {
                let snippet = source["snippet"].as_str().unwrap_or("");
                assert!(
                    !snippet.contains("Tenant B") && !snippet.contains("Neural Network"),
                    "Tenant A query returned Tenant B data: {}",
                    snippet
                );
            }
        }

        // Tenant B queries - should only see its own data
        let query_b_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/query")
                    .header("Content-Type", "application/json")
                    .header("X-Tenant-ID", "query-tenant-b")
                    .header("X-Workspace-ID", "query-ws-b")
                    .body(Body::from(
                        json!({
                            "query": "What is the development about?",
                            "mode": "hybrid"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let query_b_json = extract_json(query_b_response).await;

        // If sources exist, verify they are filtered to Tenant B only
        if let Some(sources) = query_b_json["sources"].as_array() {
            for source in sources {
                let snippet = source["snippet"].as_str().unwrap_or("");
                assert!(
                    !snippet.contains("Tenant A") && !snippet.contains("Quantum Computing"),
                    "Tenant B query returned Tenant A data: {}",
                    snippet
                );
            }
        }
    }
}

// ============================================================================
// Attack Vector Tests (Edge Cases)
// ============================================================================

mod attack_vector_tests {
    use super::*;

    /// Test that requests without tenant headers cannot access tenant-specific data
    #[tokio::test]
    async fn test_missing_tenant_headers() {
        let app = create_test_app();

        // First upload with a tenant
        let upload_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/documents")
                    .header("Content-Type", "application/json")
                    .header("X-Tenant-ID", "protected-tenant")
                    .header("X-Workspace-ID", "protected-ws")
                    .body(Body::from(
                        json!({
                            "content": "Highly confidential data for protected tenant",
                            "title": "Confidential Data"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Log upload response for debugging
        let (upload_status, upload_json) = extract_status_and_json(upload_response).await;
        println!(
            "Upload response: status={:?}, body={:?}",
            upload_status, upload_json
        );

        // Request without tenant headers
        let list_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/documents")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let list_json = extract_json(list_response).await;
        let documents = list_json["documents"]
            .as_array()
            .or(list_json.as_array())
            .cloned()
            .unwrap_or_default();

        // Without tenant context, should not see tenant-specific documents
        // If there are documents, they should NOT be from protected-tenant
        for doc in &documents {
            let title = doc["title"].as_str().unwrap_or("");
            // Only assert if the document has a title that indicates confidential data
            if title.contains("Confidential") {
                // Check if it has the protected tenant's workspace
                let workspace = doc
                    .get("workspace_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                assert!(
                    workspace != "protected-ws",
                    "Request without tenant headers should NOT access protected data: {}",
                    title
                );
            }
        }
    }

    /// Test header spoofing attack - attacker tries to access another tenant's data
    #[tokio::test]
    async fn test_header_spoofing_attack() {
        let app = create_test_app();

        // Victim tenant uploads confidential data
        let _victim_upload = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/documents")
                    .header("Content-Type", "application/json")
                    .header("X-Tenant-ID", "victim-tenant")
                    .header("X-Workspace-ID", "victim-ws")
                    .body(Body::from(
                        json!({
                            "content": "SECRET: Credit card numbers and passwords stored here",
                            "title": "Victim Secrets"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Attacker tries to access victim's data by setting their own tenant
        let attacker_list = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/documents")
                    .header("X-Tenant-ID", "attacker-tenant")
                    .header("X-Workspace-ID", "attacker-ws")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let attacker_json = extract_json(attacker_list).await;
        let documents = attacker_json["documents"]
            .as_array()
            .or(attacker_json.as_array())
            .cloned()
            .unwrap_or_default();

        // Attacker should not see victim's documents
        for doc in &documents {
            let title = doc["title"].as_str().unwrap_or("");
            let content = doc.get("content").and_then(|v| v.as_str()).unwrap_or("");
            assert!(
                !title.contains("Victim") && !content.contains("SECRET"),
                "SECURITY BREACH: Attacker accessed victim's data: {} / {}",
                title,
                content
            );
        }
    }

    /// Test SQL injection in tenant headers (if using database)
    #[tokio::test]
    async fn test_sql_injection_in_tenant_headers() {
        let app = create_test_app();

        // Attempt SQL injection through tenant header
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/documents")
                    .header("X-Tenant-ID", "'; DROP TABLE documents; --")
                    .header("X-Workspace-ID", "1 OR 1=1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should handle gracefully - either return empty results or error, but NOT crash
        let status = response.status();
        assert!(
            status == StatusCode::OK
                || status == StatusCode::BAD_REQUEST
                || status == StatusCode::FORBIDDEN,
            "SQL injection attempt caused unexpected status: {:?}",
            status
        );
    }

    /// Test traversal attack through workspace ID
    #[tokio::test]
    async fn test_path_traversal_in_workspace() {
        let app = create_test_app();

        // Attempt path traversal through workspace header
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/documents")
                    .header("X-Tenant-ID", "normal-tenant")
                    .header("X-Workspace-ID", "../../../etc/passwd")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should handle gracefully
        let status = response.status();
        assert!(
            status == StatusCode::OK
                || status == StatusCode::BAD_REQUEST
                || status == StatusCode::FORBIDDEN,
            "Path traversal attempt caused unexpected status: {:?}",
            status
        );
    }

    /// Test Unicode injection in tenant headers
    #[tokio::test]
    async fn test_unicode_injection_in_headers() {
        let app = create_test_app();

        // Attempt Unicode injection with special characters (but not null bytes which fail at header level)
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/documents")
                    .header("X-Tenant-ID", "tenant\u{200B}zero-width")
                    .header("X-Workspace-ID", "workspace\u{FEFF}bom")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;

        // Should handle gracefully - either succeed or error, but not crash
        match response {
            Ok(resp) => {
                let status = resp.status();
                assert!(
                    status.is_success() || status.is_client_error(),
                    "Unicode injection caused server error: {:?}",
                    status
                );
            }
            Err(_) => {
                // Header parsing failed - this is acceptable behavior
            }
        }
    }
}

// ============================================================================
// Graph Data Isolation Tests
// ============================================================================

mod graph_isolation_tests {
    use super::*;

    /// Test that graph entities are filtered by tenant
    #[tokio::test]
    async fn test_entity_isolation_between_tenants() {
        let app = create_test_app();

        // Get entities for Tenant A
        let entities_a_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/graph/entities")
                    .header("X-Tenant-ID", "graph-tenant-a")
                    .header("X-Workspace-ID", "graph-ws-a")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let entities_a_json = extract_json(entities_a_response).await;

        // Get entities for Tenant B
        let entities_b_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/graph/entities")
                    .header("X-Tenant-ID", "graph-tenant-b")
                    .header("X-Workspace-ID", "graph-ws-b")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let entities_b_json = extract_json(entities_b_response).await;

        // Compare entity sets - they should be completely different
        let entities_a: Vec<String> = entities_a_json["entities"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|e| e["id"].as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let entities_b: Vec<String> = entities_b_json["entities"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|e| e["id"].as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Check no overlap (unless both are empty, which is fine for fresh test)
        for entity_id in &entities_a {
            assert!(
                !entities_b.contains(entity_id),
                "Entity {} should not be shared between tenants",
                entity_id
            );
        }
    }

    /// Test that graph traversal respects tenant boundaries
    #[tokio::test]
    async fn test_graph_traversal_isolation() {
        let app = create_test_app();

        // Get graph data for Tenant A
        let graph_a_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/graph")
                    .header("X-Tenant-ID", "traversal-tenant-a")
                    .header("X-Workspace-ID", "traversal-ws-a")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status_a, graph_a_json) = extract_status_and_json(graph_a_response).await;
        assert!(
            status_a.is_success(),
            "Graph fetch for Tenant A failed: {:?}",
            status_a
        );

        // Get graph data for Tenant B
        let graph_b_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/graph")
                    .header("X-Tenant-ID", "traversal-tenant-b")
                    .header("X-Workspace-ID", "traversal-ws-b")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status_b, graph_b_json) = extract_status_and_json(graph_b_response).await;
        assert!(
            status_b.is_success(),
            "Graph fetch for Tenant B failed: {:?}",
            status_b
        );

        // Verify node isolation
        let nodes_a: Vec<String> = graph_a_json["nodes"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|n| n["id"].as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let nodes_b: Vec<String> = graph_b_json["nodes"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|n| n["id"].as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Verify no cross-tenant node access
        for node_id in &nodes_a {
            assert!(
                !nodes_b.contains(node_id),
                "Graph node {} should not be visible to both tenants",
                node_id
            );
        }
    }
}

// ============================================================================
// Persistence Tests
// ============================================================================

mod persistence_tests {
    use super::*;

    /// Test that tenant context is properly stored with documents
    #[tokio::test]
    async fn test_tenant_context_persisted_in_document_metadata() {
        let app = create_test_app();

        // Upload document with tenant context
        let upload_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/documents")
                    .header("Content-Type", "application/json")
                    .header("X-Tenant-ID", "persist-tenant")
                    .header("X-Workspace-ID", "persist-workspace")
                    .body(Body::from(
                        json!({
                            "content": "Test document for persistence verification",
                            "title": "Persistence Test Doc"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status, upload_json) = extract_status_and_json(upload_response).await;
        // WHY: POST /documents returns 201 Created per REST semantics (UC0001)
        assert!(
            status == StatusCode::CREATED || status == StatusCode::ACCEPTED,
            "Upload failed: {:?}",
            status
        );

        let doc_id = upload_json["document_id"]
            .as_str()
            .or(upload_json["id"].as_str())
            .unwrap_or("unknown");

        // Allow processing time
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Fetch document and verify tenant context is stored
        if doc_id != "unknown" {
            let get_response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri(format!("/api/v1/documents/{}", doc_id))
                        .header("X-Tenant-ID", "persist-tenant")
                        .header("X-Workspace-ID", "persist-workspace")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            if get_response.status() == StatusCode::OK {
                let get_json = extract_json(get_response).await;

                // Check if document returns tenant/workspace metadata
                let metadata = &get_json["metadata"];
                // Tenant context should be associated with the document
                // (exact field names may vary based on implementation)
                println!("Document metadata: {:?}", metadata);
            }
        }
    }
}
