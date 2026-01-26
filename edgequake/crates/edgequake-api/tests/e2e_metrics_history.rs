//! E2E tests for workspace metrics history API.
//!
//! OODA-23: Tests for metrics recording and history endpoint.
//!
//! ## Test Coverage
//!
//! - Metrics history endpoint returns correct structure
//! - Pagination works as expected
//! - Empty history for new workspace
//! - API responds correctly with in-memory storage

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

use edgequake_api::{AppState, Server, ServerConfig};

/// Create a test server configuration.
fn create_test_config() -> ServerConfig {
    ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: true,
    }
}

/// Create a test server with default AppState.
fn create_test_server() -> (Router, String) {
    let state = AppState::test_state();
    let server = Server::new(create_test_config(), state);
    let router = server.build_router();
    
    // Create a test workspace ID (default workspace)
    let workspace_id = Uuid::nil().to_string();
    
    (router, workspace_id)
}

/// Test: GET /api/v1/workspaces/{id}/metrics-history returns empty list for new workspace
#[tokio::test]
async fn test_metrics_history_empty_for_new_workspace() {
    let (router, workspace_id) = create_test_server();
    
    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/workspaces/{}/metrics-history", workspace_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&body_bytes).unwrap();
    
    // Verify response structure
    assert!(body.get("workspace_id").is_some());
    assert!(body.get("snapshots").is_some());
    assert!(body.get("count").is_some());
    assert!(body.get("limit").is_some());
    assert!(body.get("offset").is_some());
    
    // In-memory storage returns empty history
    let snapshots = body["snapshots"].as_array().unwrap();
    assert_eq!(snapshots.len(), 0);
    assert_eq!(body["count"].as_u64().unwrap(), 0);
}

/// Test: Metrics history endpoint respects limit parameter
#[tokio::test]
async fn test_metrics_history_limit_parameter() {
    let (router, workspace_id) = create_test_server();
    
    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/workspaces/{}/metrics-history?limit=50",
                    workspace_id
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&body_bytes).unwrap();
    
    // Verify limit is applied
    assert_eq!(body["limit"].as_u64().unwrap(), 50);
}

/// Test: Metrics history endpoint respects offset parameter
#[tokio::test]
async fn test_metrics_history_offset_parameter() {
    let (router, workspace_id) = create_test_server();
    
    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/workspaces/{}/metrics-history?offset=10",
                    workspace_id
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&body_bytes).unwrap();
    
    // Verify offset is applied
    assert_eq!(body["offset"].as_u64().unwrap(), 10);
}

/// Test: Metrics history endpoint limits maximum results
#[tokio::test]
async fn test_metrics_history_max_limit_enforced() {
    let (router, workspace_id) = create_test_server();
    
    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/workspaces/{}/metrics-history?limit=5000",
                    workspace_id
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&body_bytes).unwrap();
    
    // Verify limit is capped at 1000
    assert_eq!(body["limit"].as_u64().unwrap(), 1000);
}

/// Test: Metrics history endpoint with both limit and offset
#[tokio::test]
async fn test_metrics_history_pagination_combined() {
    let (router, workspace_id) = create_test_server();
    
    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/v1/workspaces/{}/metrics-history?limit=25&offset=50",
                    workspace_id
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&body_bytes).unwrap();
    
    // Verify both params applied
    assert_eq!(body["limit"].as_u64().unwrap(), 25);
    assert_eq!(body["offset"].as_u64().unwrap(), 50);
}
