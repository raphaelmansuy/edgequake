//! End-to-end tests for entity API endpoints.
//!
//! Tests cover:
//! - Create entity (POST /api/v1/graph/entities)
//! - Get entity (GET /api/v1/graph/entities/{entity_name})
//! - Update entity (PUT /api/v1/graph/entities/{entity_name})
//! - Delete entity (DELETE /api/v1/graph/entities/{entity_name})
//! - Entity exists (GET /api/v1/graph/entities/exists)
//! - Merge entities (POST /api/v1/graph/entities/merge)

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

// ============================================================================
// Create Entity Tests
// ============================================================================

#[tokio::test]
async fn test_create_entity_success() {
    let app = create_test_app();

    let request = json!({
        "entity_name": "quantum computing",
        "entity_type": "TECHNOLOGY",
        "description": "A type of computation that harnesses quantum mechanics",
        "source_id": "manual_entry",
        "metadata": {"created_by": "test"}
    });

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

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    assert_eq!(body.get("status").and_then(|v| v.as_str()), Some("success"));
    assert!(body.get("entity").is_some());

    let entity = body.get("entity").unwrap();
    // Entity name should be normalized to QUANTUM_COMPUTING
    assert_eq!(
        entity.get("entity_name").and_then(|v| v.as_str()),
        Some("QUANTUM_COMPUTING")
    );
    assert_eq!(
        entity.get("entity_type").and_then(|v| v.as_str()),
        Some("TECHNOLOGY")
    );
}

#[tokio::test]
async fn test_create_entity_duplicate() {
    let server = Server::new(create_test_config(), AppState::test_state());

    let request = json!({
        "entity_name": "apple inc",
        "entity_type": "ORGANIZATION",
        "description": "Technology company",
        "source_id": "manual_entry"
    });

    // Create first entity
    let app = server.build_router();
    let response1 = app
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

    assert_eq!(response1.status(), StatusCode::OK);

    // Try to create duplicate
    let app = server.build_router();
    let response2 = app
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

    assert_eq!(response2.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_create_entity_name_normalization() {
    let app = create_test_app();

    let request = json!({
        "entity_name": "Machine Learning",
        "entity_type": "CONCEPT",
        "description": "A field of AI",
        "source_id": "manual_entry"
    });

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

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    let entity = body.get("entity").unwrap();
    // Should be normalized to MACHINE_LEARNING
    assert_eq!(
        entity.get("entity_name").and_then(|v| v.as_str()),
        Some("MACHINE_LEARNING")
    );
}

// ============================================================================
// Get Entity Tests
// ============================================================================

#[tokio::test]
async fn test_get_entity_success() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Create entity first
    let create_request = json!({
        "entity_name": "Tesla",
        "entity_type": "ORGANIZATION",
        "description": "Electric vehicle company",
        "source_id": "manual_entry"
    });

    let app = server.build_router();
    let create_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/entities")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&create_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::OK);

    // Get entity
    let app = server.build_router();
    let get_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/entities/TESLA")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::OK);

    let body = extract_json(get_response).await;
    assert!(body.get("entity").is_some());
    assert!(body.get("relationships").is_some());
    assert!(body.get("statistics").is_some());

    let entity = body.get("entity").unwrap();
    assert_eq!(
        entity.get("entity_name").and_then(|v| v.as_str()),
        Some("TESLA")
    );
}

#[tokio::test]
async fn test_get_entity_not_found() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/entities/NONEXISTENT_ENTITY")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_entity_name_normalization() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Create entity
    let create_request = json!({
        "entity_name": "OpenAI",
        "entity_type": "ORGANIZATION",
        "description": "AI research company",
        "source_id": "manual_entry"
    });

    let app = server.build_router();
    let _create_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/entities")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&create_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Get with lowercase (should be normalized)
    let app = server.build_router();
    let get_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/entities/openai")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::OK);
}

// ============================================================================
// Update Entity Tests
// ============================================================================

#[tokio::test]
async fn test_update_entity_success() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Create entity
    let create_request = json!({
        "entity_name": "Google",
        "entity_type": "ORGANIZATION",
        "description": "Search engine company",
        "source_id": "manual_entry"
    });

    let app = server.build_router();
    let _create_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/entities")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&create_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Update entity
    let update_request = json!({
        "description": "Technology conglomerate specializing in Internet services",
        "entity_type": "TECH_COMPANY"
    });

    let app = server.build_router();
    let update_response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/graph/entities/GOOGLE")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&update_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update_response.status(), StatusCode::OK);

    let body = extract_json(update_response).await;
    assert_eq!(body.get("status").and_then(|v| v.as_str()), Some("success"));

    let entity = body.get("entity").unwrap();
    assert!(entity
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap()
        .contains("Technology conglomerate"));

    let changes = body.get("changes").unwrap();
    let fields = changes.get("fields_updated").and_then(|v| v.as_array());
    assert!(fields.is_some());
}

#[tokio::test]
async fn test_update_entity_not_found() {
    let app = create_test_app();

    let update_request = json!({
        "description": "New description"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/graph/entities/NONEXISTENT")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&update_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_entity_metadata() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Create entity
    let create_request = json!({
        "entity_name": "Amazon",
        "entity_type": "ORGANIZATION",
        "description": "E-commerce company",
        "source_id": "manual_entry"
    });

    let app = server.build_router();
    let _create_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/entities")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&create_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Update with metadata
    let update_request = json!({
        "metadata": {
            "stock_symbol": "AMZN",
            "founded": 1994
        }
    });

    let app = server.build_router();
    let update_response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/graph/entities/AMAZON")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&update_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update_response.status(), StatusCode::OK);
}

// ============================================================================
// Delete Entity Tests
// ============================================================================

#[tokio::test]
async fn test_delete_entity_success() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Create entity
    let create_request = json!({
        "entity_name": "Temporary Entity",
        "entity_type": "TEST",
        "description": "Entity to be deleted",
        "source_id": "manual_entry"
    });

    let app = server.build_router();
    let _create_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/entities")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&create_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Delete entity (with confirmation)
    let app = server.build_router();
    let delete_response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/graph/entities/TEMPORARY_ENTITY?confirm=true")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(delete_response.status(), StatusCode::OK);

    let body = extract_json(delete_response).await;
    assert_eq!(body.get("status").and_then(|v| v.as_str()), Some("success"));
    assert_eq!(
        body.get("deleted_entity_id").and_then(|v| v.as_str()),
        Some("TEMPORARY_ENTITY")
    );

    // Verify entity is gone
    let app = server.build_router();
    let get_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/entities/TEMPORARY_ENTITY")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_entity_no_confirmation() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Create entity
    let create_request = json!({
        "entity_name": "Protected Entity",
        "entity_type": "TEST",
        "description": "Entity that requires confirmation",
        "source_id": "manual_entry"
    });

    let app = server.build_router();
    let _create_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/entities")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&create_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Try to delete without confirmation
    let app = server.build_router();
    let delete_response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/graph/entities/PROTECTED_ENTITY?confirm=false")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(delete_response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_delete_entity_not_found() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/graph/entities/NONEXISTENT?confirm=true")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Entity Exists Tests
// ============================================================================

#[tokio::test]
async fn test_entity_exists_true() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Create entity
    let create_request = json!({
        "entity_name": "Existing Entity",
        "entity_type": "TEST",
        "description": "Entity that exists",
        "source_id": "manual_entry"
    });

    let app = server.build_router();
    let _create_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/entities")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&create_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Check exists
    let app = server.build_router();
    let exists_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/entities/exists?entity_name=Existing%20Entity")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(exists_response.status(), StatusCode::OK);

    let body = extract_json(exists_response).await;
    assert_eq!(body.get("exists").and_then(|v| v.as_bool()), Some(true));
    assert_eq!(
        body.get("entity_id").and_then(|v| v.as_str()),
        Some("EXISTING_ENTITY")
    );
}

#[tokio::test]
async fn test_entity_exists_false() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/entities/exists?entity_name=Nonexistent%20Entity")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    assert_eq!(body.get("exists").and_then(|v| v.as_bool()), Some(false));
    assert!(body.get("entity_id").is_none() || body.get("entity_id").unwrap().is_null());
}

// ============================================================================
// Merge Entities Tests
// ============================================================================

#[tokio::test]
async fn test_merge_entities_success() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Create source entity
    let source_request = json!({
        "entity_name": "Source Entity",
        "entity_type": "TEST",
        "description": "Source description",
        "source_id": "manual_entry"
    });

    let app = server.build_router();
    let _source_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/entities")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&source_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Create target entity
    let target_request = json!({
        "entity_name": "Target Entity",
        "entity_type": "TEST",
        "description": "Target description",
        "source_id": "manual_entry"
    });

    let app = server.build_router();
    let _target_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/entities")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&target_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Merge entities
    let merge_request = json!({
        "source_entity": "Source Entity",
        "target_entity": "Target Entity",
        "merge_strategy": "prefer_target"
    });

    let app = server.build_router();
    let merge_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/entities/merge")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&merge_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(merge_response.status(), StatusCode::OK);

    let body = extract_json(merge_response).await;
    assert_eq!(body.get("status").and_then(|v| v.as_str()), Some("success"));
    assert!(body.get("merged_entity").is_some());
    assert!(body.get("merge_details").is_some());

    // Verify source entity is gone
    let app = server.build_router();
    let source_check = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/entities/SOURCE_ENTITY")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(source_check.status(), StatusCode::NOT_FOUND);

    // Verify target entity still exists
    let app = server.build_router();
    let target_check = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/entities/TARGET_ENTITY")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(target_check.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_merge_entities_source_not_found() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Create only target entity
    let target_request = json!({
        "entity_name": "Only Target",
        "entity_type": "TEST",
        "description": "Target description",
        "source_id": "manual_entry"
    });

    let app = server.build_router();
    let _target_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/entities")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&target_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Try to merge with nonexistent source
    let merge_request = json!({
        "source_entity": "Nonexistent",
        "target_entity": "Only Target"
    });

    let app = server.build_router();
    let merge_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/entities/merge")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&merge_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(merge_response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_merge_entities_with_strategy() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // Create source with rich description
    let source_request = json!({
        "entity_name": "Detailed Source",
        "entity_type": "TEST",
        "description": "This is a very detailed source description with lots of information",
        "source_id": "manual_entry"
    });

    let app = server.build_router();
    let _source_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/entities")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&source_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Create target
    let target_request = json!({
        "entity_name": "Brief Target",
        "entity_type": "TEST",
        "description": "Brief description",
        "source_id": "manual_entry"
    });

    let app = server.build_router();
    let _target_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/entities")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&target_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Merge with prefer_source strategy
    let merge_request = json!({
        "source_entity": "Detailed Source",
        "target_entity": "Brief Target",
        "merge_strategy": "prefer_source"
    });

    let app = server.build_router();
    let merge_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/entities/merge")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&merge_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(merge_response.status(), StatusCode::OK);

    let body = extract_json(merge_response).await;
    let merged = body.get("merged_entity").unwrap();

    // With prefer_source, description should contain the detailed source description
    let description = merged
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(description.contains("detailed") || description.contains("very"));
}

// ============================================================================
// Entity Lifecycle Test
// ============================================================================

#[tokio::test]
async fn test_complete_entity_lifecycle() {
    let server = Server::new(create_test_config(), AppState::test_state());

    // 1. Create entity
    let create_request = json!({
        "entity_name": "Lifecycle Entity",
        "entity_type": "TEST",
        "description": "Initial description",
        "source_id": "manual_entry"
    });

    let app = server.build_router();
    let create_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/entities")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&create_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::OK);

    // 2. Verify it exists
    let app = server.build_router();
    let exists_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/entities/exists?entity_name=Lifecycle%20Entity")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let exists_body = extract_json(exists_response).await;
    assert_eq!(
        exists_body.get("exists").and_then(|v| v.as_bool()),
        Some(true)
    );

    // 3. Get full entity details
    let app = server.build_router();
    let get_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/entities/LIFECYCLE_ENTITY")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::OK);

    // 4. Update entity
    let update_request = json!({
        "description": "Updated description with more details"
    });

    let app = server.build_router();
    let update_response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/graph/entities/LIFECYCLE_ENTITY")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&update_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update_response.status(), StatusCode::OK);

    // 5. Delete entity
    let app = server.build_router();
    let delete_response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/graph/entities/LIFECYCLE_ENTITY?confirm=true")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(delete_response.status(), StatusCode::OK);

    // 6. Verify it's gone
    let app = server.build_router();
    let final_check = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/entities/exists?entity_name=Lifecycle%20Entity")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let final_body = extract_json(final_check).await;
    assert_eq!(
        final_body.get("exists").and_then(|v| v.as_bool()),
        Some(false)
    );
}

// ============================================================================
// List Entities Tests
// ============================================================================

#[tokio::test]
async fn test_list_entities_empty() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/entities")
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
async fn test_list_entities_with_pagination() {
    let server = create_test_server();

    // Create some entities first
    for i in 1..=5 {
        let app = server.build_router();
        let request = json!({
            "entity_name": format!("list_test_entity_{}", i),
            "entity_type": "CONCEPT",
            "description": format!("Test entity {}", i),
            "source_id": "test"
        });

        app.oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/entities")
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
                .uri("/api/v1/graph/entities?page=1&page_size=3")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    assert_eq!(body.get("page").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(body.get("page_size").and_then(|v| v.as_u64()), Some(3));
    // Should have at least 5 total (may have more from other tests)
    assert!(body.get("total").and_then(|v| v.as_u64()).unwrap_or(0) >= 5);
}

#[tokio::test]
async fn test_list_entities_with_type_filter() {
    let server = create_test_server();

    // Create entities with different types
    for (name, entity_type) in [
        ("filter_person_1", "PERSON"),
        ("filter_org_1", "ORGANIZATION"),
        ("filter_person_2", "PERSON"),
    ] {
        let app = server.build_router();
        let request = json!({
            "entity_name": name,
            "entity_type": entity_type,
            "description": "Test",
            "source_id": "test"
        });

        app.oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graph/entities")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    }

    // Filter by PERSON type
    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/entities?entity_type=PERSON")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    let items = body.get("items").and_then(|v| v.as_array()).unwrap();
    // All returned items should be PERSON type
    for item in items {
        assert_eq!(
            item.get("entity_type").and_then(|v| v.as_str()),
            Some("PERSON")
        );
    }
}

// ============================================================================
// Entity Neighborhood Tests
// ============================================================================

#[tokio::test]
async fn test_entity_neighborhood_not_found() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/entities/NONEXISTENT_ENTITY/neighborhood")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_entity_neighborhood_basic() {
    let server = create_test_server();

    // Create a central entity
    let app = server.build_router();
    let request = json!({
        "entity_name": "neighborhood_center",
        "entity_type": "CONCEPT",
        "description": "Central node",
        "source_id": "test"
    });

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

    assert_eq!(response.status(), StatusCode::OK);

    // Get neighborhood (should return just the node with no edges)
    let app = server.build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/entities/NEIGHBORHOOD_CENTER/neighborhood")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response).await;
    assert!(body.get("nodes").is_some());
    assert!(body.get("edges").is_some());

    let nodes = body.get("nodes").and_then(|v| v.as_array()).unwrap();
    assert!(!nodes.is_empty());
    // The center node should be in the response
    assert!(nodes
        .iter()
        .any(|n| n.get("id").and_then(|v| v.as_str()) == Some("NEIGHBORHOOD_CENTER")));
}

#[tokio::test]
async fn test_entity_neighborhood_with_depth() {
    let app = create_test_app();

    // Test depth parameter parsing
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/entities/TEST/neighborhood?depth=2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Will be 404 since entity doesn't exist, but validates route parsing
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
