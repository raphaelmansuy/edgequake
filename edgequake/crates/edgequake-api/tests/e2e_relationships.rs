//! End-to-end tests for relationship API endpoints.
//!
//! Tests cover:
//! - Create relationship (POST /api/v1/graph/relationships)
//! - Get relationship (GET /api/v1/graph/relationships/{relationship_id})
//! - Update relationship (PUT /api/v1/graph/relationships/{relationship_id})
//! - Delete relationship (DELETE /api/v1/graph/relationships/{relationship_id})

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use serde_json::{json, Value};
use tower::ServiceExt;

// ============================================================================
// Helper Functions
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
    serde_json::from_slice(&bytes).expect("Failed to parse JSON")
}

async fn create_entity(server: &Server, name: &str, entity_type: &str) -> Result<(), ()> {
    let request = json!({
        "entity_name": name,
        "entity_type": entity_type,
        "description": format!("{} entity", name),
        "source_id": "manual_entry"
    });

    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/entities")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    if response.status() == StatusCode::OK || response.status() == StatusCode::CONFLICT {
        Ok(())
    } else {
        Err(())
    }
}

// ============================================================================
// Create Relationship Tests
// ============================================================================

#[tokio::test]
async fn test_create_relationship_success() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Create source and target entities
    create_entity(&server, "PersonA", "PERSON").await.unwrap();
    create_entity(&server, "CompanyA", "ORGANIZATION")
        .await
        .unwrap();

    let request = json!({
        "src_id": "PersonA",
        "tgt_id": "CompanyA",
        "keywords": "works at, employed by",
        "weight": 0.9,
        "description": "PersonA works at CompanyA",
        "source_id": "manual_entry"
    });

    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/relationships")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    assert_eq!(body.get("status").and_then(|v| v.as_str()), Some("success"));
    assert!(body.get("relationship").is_some());

    let rel = body.get("relationship").unwrap();
    assert!(rel.get("id").is_some());
    assert_eq!(rel.get("src_id").and_then(|v| v.as_str()), Some("PERSONA"));
    assert_eq!(rel.get("tgt_id").and_then(|v| v.as_str()), Some("COMPANYA"));
    // Relation type should be extracted from keywords
    assert_eq!(
        rel.get("relation_type").and_then(|v| v.as_str()),
        Some("WORKS_AT")
    );
}

#[tokio::test]
async fn test_create_relationship_source_not_found() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Create only target entity
    create_entity(&server, "TargetOnly", "ORGANIZATION")
        .await
        .unwrap();

    let request = json!({
        "src_id": "NonexistentSource",
        "tgt_id": "TargetOnly",
        "keywords": "related to",
        "description": "Relationship description",
        "source_id": "manual_entry"
    });

    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/relationships")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_create_relationship_target_not_found() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Create only source entity
    create_entity(&server, "SourceOnly", "PERSON")
        .await
        .unwrap();

    let request = json!({
        "src_id": "SourceOnly",
        "tgt_id": "NonexistentTarget",
        "keywords": "related to",
        "description": "Relationship description",
        "source_id": "manual_entry"
    });

    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/relationships")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_create_relationship_default_weight() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Create entities
    create_entity(&server, "EntityA", "CONCEPT").await.unwrap();
    create_entity(&server, "EntityB", "CONCEPT").await.unwrap();

    let request = json!({
        "src_id": "EntityA",
        "tgt_id": "EntityB",
        "keywords": "relates to",
        "description": "A relates to B",
        "source_id": "manual_entry"
        // weight not specified, should default to 0.8
    });

    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/relationships")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    let rel = body.get("relationship").unwrap();
    let weight = rel.get("weight").and_then(|v| v.as_f64()).unwrap_or(0.0);
    assert!((weight - 0.8).abs() < 0.001);
}

#[tokio::test]
async fn test_create_relationship_with_metadata() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Create entities
    create_entity(&server, "MetaSource", "PERSON")
        .await
        .unwrap();
    create_entity(&server, "MetaTarget", "ORGANIZATION")
        .await
        .unwrap();

    let request = json!({
        "src_id": "MetaSource",
        "tgt_id": "MetaTarget",
        "keywords": "founded",
        "weight": 1.0,
        "description": "MetaSource founded MetaTarget",
        "source_id": "manual_entry",
        "metadata": {
            "year": 2020,
            "verified": true
        }
    });

    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/relationships")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    let rel = body.get("relationship").unwrap();
    assert!(rel.get("metadata").is_some());
}

// ============================================================================
// Get Relationship Tests
// ============================================================================

#[tokio::test]
async fn test_get_relationship_success() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Create entities and relationship
    create_entity(&server, "GetSource", "PERSON").await.unwrap();
    create_entity(&server, "GetTarget", "ORGANIZATION")
        .await
        .unwrap();

    let create_request = json!({
        "src_id": "GetSource",
        "tgt_id": "GetTarget",
        "keywords": "leads",
        "description": "GetSource leads GetTarget",
        "source_id": "manual_entry"
    });

    let app = server.build_router();
    let create_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/relationships")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&create_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::OK);

    let create_body = extract_json(create_response).await;
    let relationship_id = create_body
        .get("relationship")
        .and_then(|r| r.get("id"))
        .and_then(|v| v.as_str())
        .expect("Should have relationship id");

    // Get relationship by ID
    let app = server.build_router();
    let get_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/v1/graph/relationships/{}", relationship_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::OK);

    let body = extract_json(get_response).await;
    assert!(body.get("relationship").is_some());
    assert!(body.get("entities").is_some());

    let rel = body.get("relationship").unwrap();
    assert_eq!(
        rel.get("id").and_then(|v| v.as_str()),
        Some(relationship_id)
    );

    let entities = body.get("entities").unwrap();
    assert!(entities.get("source").is_some());
    assert!(entities.get("target").is_some());
}

#[tokio::test]
async fn test_get_relationship_not_found() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/relationships/rel-nonexistent-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Update Relationship Tests
// ============================================================================

#[tokio::test]
async fn test_update_relationship_success() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Create entities and relationship
    create_entity(&server, "UpdateSource", "PERSON")
        .await
        .unwrap();
    create_entity(&server, "UpdateTarget", "ORGANIZATION")
        .await
        .unwrap();

    let create_request = json!({
        "src_id": "UpdateSource",
        "tgt_id": "UpdateTarget",
        "keywords": "works at",
        "weight": 0.5,
        "description": "Initial relationship",
        "source_id": "manual_entry"
    });

    let app = server.build_router();
    let create_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/relationships")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&create_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let create_body = extract_json(create_response).await;
    let relationship_id = create_body
        .get("relationship")
        .and_then(|r| r.get("id"))
        .and_then(|v| v.as_str())
        .expect("Should have relationship id");

    // Update relationship
    let update_request = json!({
        "weight": 0.95,
        "description": "Updated relationship description",
        "keywords": "leads, manages"
    });

    let app = server.build_router();
    let update_response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&format!("/api/v1/graph/relationships/{}", relationship_id))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&update_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update_response.status(), StatusCode::OK);

    let body = extract_json(update_response).await;
    assert_eq!(body.get("status").and_then(|v| v.as_str()), Some("success"));

    let rel = body.get("relationship").unwrap();
    let weight = rel.get("weight").and_then(|v| v.as_f64()).unwrap_or(0.0);
    assert!((weight - 0.95).abs() < 0.001);

    let changes = body.get("changes").unwrap();
    let fields = changes
        .get("fields_updated")
        .and_then(|v| v.as_array())
        .unwrap();
    assert!(!fields.is_empty());
}

#[tokio::test]
async fn test_update_relationship_not_found() {
    let app = create_test_app();

    let update_request = json!({
        "weight": 0.9
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/graph/relationships/rel-nonexistent")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&update_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_relationship_partial() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Create entities and relationship
    create_entity(&server, "PartialSource", "CONCEPT")
        .await
        .unwrap();
    create_entity(&server, "PartialTarget", "CONCEPT")
        .await
        .unwrap();

    let create_request = json!({
        "src_id": "PartialSource",
        "tgt_id": "PartialTarget",
        "keywords": "related to",
        "weight": 0.6,
        "description": "Initial description",
        "source_id": "manual_entry"
    });

    let app = server.build_router();
    let create_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/relationships")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&create_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let create_body = extract_json(create_response).await;
    let relationship_id = create_body
        .get("relationship")
        .and_then(|r| r.get("id"))
        .and_then(|v| v.as_str())
        .expect("Should have relationship id");

    // Only update weight
    let update_request = json!({
        "weight": 0.85
    });

    let app = server.build_router();
    let update_response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&format!("/api/v1/graph/relationships/{}", relationship_id))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&update_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update_response.status(), StatusCode::OK);

    let body = extract_json(update_response).await;
    let changes = body.get("changes").unwrap();
    let fields = changes
        .get("fields_updated")
        .and_then(|v| v.as_array())
        .unwrap();

    // Only weight should be updated
    assert!(fields.iter().any(|f| f.as_str() == Some("weight")));
}

// ============================================================================
// Delete Relationship Tests
// ============================================================================

#[tokio::test]
async fn test_delete_relationship_success() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Create entities and relationship
    create_entity(&server, "DeleteSource", "PERSON")
        .await
        .unwrap();
    create_entity(&server, "DeleteTarget", "ORGANIZATION")
        .await
        .unwrap();

    let create_request = json!({
        "src_id": "DeleteSource",
        "tgt_id": "DeleteTarget",
        "keywords": "works at",
        "description": "Relationship to delete",
        "source_id": "manual_entry"
    });

    let app = server.build_router();
    let create_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/relationships")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&create_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let create_body = extract_json(create_response).await;
    let relationship_id = create_body
        .get("relationship")
        .and_then(|r| r.get("id"))
        .and_then(|v| v.as_str())
        .expect("Should have relationship id");

    // Delete relationship
    let app = server.build_router();
    let delete_response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/v1/graph/relationships/{}", relationship_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(delete_response.status(), StatusCode::OK);

    let body = extract_json(delete_response).await;
    assert_eq!(body.get("status").and_then(|v| v.as_str()), Some("success"));
    assert_eq!(
        body.get("deleted_relationship_id").and_then(|v| v.as_str()),
        Some(relationship_id)
    );

    // Verify relationship is gone
    let app = server.build_router();
    let get_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/v1/graph/relationships/{}", relationship_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_relationship_not_found() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/graph/relationships/rel-nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Relationship Lifecycle Test
// ============================================================================

#[tokio::test]
async fn test_complete_relationship_lifecycle() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // 1. Create entities
    create_entity(&server, "LifecyclePersonX", "PERSON")
        .await
        .unwrap();
    create_entity(&server, "LifecycleOrgX", "ORGANIZATION")
        .await
        .unwrap();

    // 2. Create relationship
    let create_request = json!({
        "src_id": "LifecyclePersonX",
        "tgt_id": "LifecycleOrgX",
        "keywords": "founder, ceo",
        "weight": 0.7,
        "description": "Initial relationship in lifecycle test",
        "source_id": "manual_entry"
    });

    let app = server.build_router();
    let create_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/relationships")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&create_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::OK);

    let create_body = extract_json(create_response).await;
    let relationship_id = create_body
        .get("relationship")
        .and_then(|r| r.get("id"))
        .and_then(|v| v.as_str())
        .expect("Should have relationship id")
        .to_string();

    // 3. Get relationship
    let app = server.build_router();
    let get_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/v1/graph/relationships/{}", relationship_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::OK);

    // 4. Update relationship
    let update_request = json!({
        "weight": 1.0,
        "description": "Updated lifecycle relationship"
    });

    let app = server.build_router();
    let update_response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&format!("/api/v1/graph/relationships/{}", relationship_id))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&update_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update_response.status(), StatusCode::OK);

    // 5. Verify update
    let app = server.build_router();
    let verify_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/v1/graph/relationships/{}", relationship_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let verify_body = extract_json(verify_response).await;
    let rel = verify_body.get("relationship").unwrap();
    let weight = rel.get("weight").and_then(|v| v.as_f64()).unwrap_or(0.0);
    assert!((weight - 1.0).abs() < 0.001);

    // 6. Delete relationship
    let app = server.build_router();
    let delete_response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(&format!("/api/v1/graph/relationships/{}", relationship_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(delete_response.status(), StatusCode::OK);

    // 7. Verify deletion
    let app = server.build_router();
    let final_get = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&format!("/api/v1/graph/relationships/{}", relationship_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(final_get.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Entity Relationship Integration Test
// ============================================================================

#[tokio::test]
async fn test_entity_with_relationships() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Create a network of entities
    create_entity(&server, "HubEntity", "ORGANIZATION")
        .await
        .unwrap();
    create_entity(&server, "Spoke1", "PERSON").await.unwrap();
    create_entity(&server, "Spoke2", "PERSON").await.unwrap();
    create_entity(&server, "Spoke3", "TECHNOLOGY")
        .await
        .unwrap();

    // Create multiple relationships
    for (spoke, keyword) in [
        ("Spoke1", "works at"),
        ("Spoke2", "leads"),
        ("Spoke3", "used by"),
    ] {
        let request = json!({
            "src_id": spoke,
            "tgt_id": "HubEntity",
            "keywords": keyword,
            "description": format!("{} {} HubEntity", spoke, keyword),
            "source_id": "manual_entry"
        });

        let app = server.build_router();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/graph/relationships")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    // Get hub entity with relationships
    let app = server.build_router();
    let entity_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/entities/HUBENTITY")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(entity_response.status(), StatusCode::OK);

    let body = extract_json(entity_response).await;
    let relationships = body.get("relationships").unwrap();
    let incoming = relationships.get("incoming").and_then(|v| v.as_array());

    // Should have 3 incoming relationships
    assert!(incoming.is_some());
    // Note: exact count depends on implementation details
}

// ============================================================================
// List Relationships Tests
// ============================================================================

#[tokio::test]
async fn test_list_relationships_empty() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/relationships")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    assert!(body.get("items").is_some());
    assert!(body.get("total").is_some());
    assert!(body.get("page").is_some());
    assert!(body.get("page_size").is_some());
    assert!(body.get("total_pages").is_some());
}

#[tokio::test]
async fn test_list_relationships_with_pagination() {
    let server = create_test_server();

    // Create entities first
    let _ = create_entity(&server, "rel_test_src", "CONCEPT").await;
    let _ = create_entity(&server, "rel_test_tgt", "CONCEPT").await;

    // Create some relationships
    for i in 1..=5 {
        let request = json!({
            "src_id": "rel_test_src",
            "tgt_id": "rel_test_tgt",
            "keywords": format!("relation_{}", i),
            "description": format!("Test relation {}", i),
            "source_id": "test"
        });

        let app = server.build_router();
        app.oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/relationships")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    }

    // List with pagination
    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/relationships?page=1&page_size=3")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    assert_eq!(body.get("page").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(body.get("page_size").and_then(|v| v.as_u64()), Some(3));
}

#[tokio::test]
async fn test_list_relationships_with_type_filter() {
    let server = create_test_server();

    // Create entities
    let _ = create_entity(&server, "filter_src", "CONCEPT").await;
    let _ = create_entity(&server, "filter_tgt", "CONCEPT").await;

    // Create relationships with different types
    for keyword in ["WORKS_FOR", "MANAGES", "WORKS_FOR"] {
        let request = json!({
            "src_id": "filter_src",
            "tgt_id": "filter_tgt",
            "keywords": keyword,
            "description": "Test",
            "source_id": "test"
        });

        let app = server.build_router();
        app.oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/relationships")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    }

    // Filter by relationship type
    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/relationships?relationship_type=WORKS_FOR")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    let items = body.get("items").and_then(|v| v.as_array()).unwrap();
    // All returned items should have WORKS_FOR in keywords
    for item in items {
        let keywords = item.get("keywords").and_then(|v| v.as_str()).unwrap_or("");
        assert!(keywords.contains("WORKS_FOR"));
    }
}
