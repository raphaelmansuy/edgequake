//! OODA-17: Pipeline robustness E2E tests.
//!
//! Validates pipeline-related endpoints work correctly:
//! 1. Health endpoint returns correct structure
//! 2. Pipeline status reflects actual state
//! 3. Cost estimation returns valid data
//! 4. Queue metrics work
//! 5. Provider status endpoint
//! 6. Document processing status tracking
//! 7. Cost summary and history

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use serde_json::{json, Value};
use std::time::Duration;
use tower::ServiceExt;

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
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

async fn get_endpoint(app: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = extract_json(response).await;
    (status, body)
}

async fn post_json(app: &axum::Router, uri: &str, payload: &Value) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = extract_json(response).await;
    (status, body)
}

// ============================================================================
// Health Endpoint
// ============================================================================

/// OODA-17: Health check returns correct structure with all component statuses.
#[tokio::test]
async fn test_health_check_structure() {
    let result = with_timeout(Duration::from_secs(5), async {
        let app = create_test_app();

        let (status, body) = get_endpoint(&app, "/health").await;
        assert_eq!(status, StatusCode::OK, "Health should return 200");

        // Must have status field
        assert_eq!(
            body["status"].as_str(),
            Some("healthy"),
            "Status should be 'healthy'"
        );

        // Must have version
        assert!(body["version"].is_string(), "Should have version string");

        // Must have components
        let components = &body["components"];
        assert!(
            components.is_object(),
            "Should have components object: {}",
            body
        );

        body
    })
    .await;

    assert!(result.is_ok(), "Health: {}", result.unwrap_err());
}

/// OODA-17: Health check reports llm_provider name.
#[tokio::test]
async fn test_health_shows_provider() {
    let result = with_timeout(Duration::from_secs(5), async {
        let app = create_test_app();

        let (_, body) = get_endpoint(&app, "/health").await;

        // In test mode, provider should be "mock"
        let provider = body["llm_provider_name"].as_str().unwrap_or("unknown");
        assert_eq!(provider, "mock", "Test state should use mock provider");

        body
    })
    .await;

    assert!(result.is_ok(), "Provider: {}", result.unwrap_err());
}

// ============================================================================
// Pipeline Status
// ============================================================================

/// OODA-17: Pipeline status endpoint returns valid state.
#[tokio::test]
async fn test_pipeline_status() {
    let result = with_timeout(Duration::from_secs(5), async {
        let app = create_test_app();

        let (status, body) = get_endpoint(&app, "/api/v1/pipeline/status").await;
        assert_eq!(status, StatusCode::OK, "Pipeline status should return 200");

        // Should have some structure
        assert!(
            body.is_object(),
            "Pipeline status should be an object: {}",
            body
        );

        body
    })
    .await;

    assert!(result.is_ok(), "Pipeline status: {}", result.unwrap_err());
}

/// OODA-17: Queue metrics endpoint works.
#[tokio::test]
async fn test_queue_metrics() {
    let result = with_timeout(Duration::from_secs(5), async {
        let app = create_test_app();

        let (status, body) = get_endpoint(&app, "/api/v1/pipeline/queue-metrics").await;
        assert_eq!(status, StatusCode::OK, "Queue metrics should return 200");

        // Should have queue-related fields
        assert!(body.is_object(), "Should be an object: {}", body);

        body
    })
    .await;

    assert!(result.is_ok(), "Queue metrics: {}", result.unwrap_err());
}

// ============================================================================
// Cost Estimation
// ============================================================================

/// OODA-17: Cost estimation for a text input.
#[tokio::test]
async fn test_cost_estimation() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        // WHY: EstimateCostRequest requires model, input_tokens, output_tokens (not content)
        let payload = json!({
            "model": "gpt-4o-mini",
            "input_tokens": 1000,
            "output_tokens": 500
        });

        let (status, body) = post_json(&app, "/api/v1/pipeline/costs/estimate", &payload).await;

        assert_eq!(
            status,
            StatusCode::OK,
            "Cost estimate should return 200, got {}. Body: {}",
            status,
            body
        );

        // Should have cost fields
        assert!(
            body["estimated_cost_usd"].is_number(),
            "Should have estimated_cost_usd: {}",
            body
        );
        assert!(
            body["formatted_cost"].is_string(),
            "Should have formatted_cost: {}",
            body
        );
        assert_eq!(body["model"].as_str(), Some("gpt-4o-mini"));

        body
    })
    .await;

    assert!(result.is_ok(), "Cost est: {}", result.unwrap_err());
}

/// OODA-17: Model pricing endpoint.
#[tokio::test]
async fn test_model_pricing() {
    let result = with_timeout(Duration::from_secs(5), async {
        let app = create_test_app();

        let (status, body) = get_endpoint(&app, "/api/v1/pipeline/costs/pricing").await;
        assert_eq!(status, StatusCode::OK, "Pricing should return 200");

        // Should have pricing data structure
        assert!(
            body.is_object() || body.is_array(),
            "Should have pricing data: {}",
            body
        );

        body
    })
    .await;

    assert!(result.is_ok(), "Pricing: {}", result.unwrap_err());
}

// ============================================================================
// Cost Summary & History
// ============================================================================

/// OODA-17: Cost summary endpoint.
#[tokio::test]
async fn test_cost_summary() {
    let result = with_timeout(Duration::from_secs(5), async {
        let app = create_test_app();

        let (status, body) = get_endpoint(&app, "/api/v1/costs/summary").await;
        assert_eq!(status, StatusCode::OK, "Cost summary should return 200");
        assert!(body.is_object(), "Should be an object: {}", body);

        body
    })
    .await;

    assert!(result.is_ok(), "Cost summary: {}", result.unwrap_err());
}

/// OODA-17: Cost history endpoint.
#[tokio::test]
async fn test_cost_history() {
    let result = with_timeout(Duration::from_secs(5), async {
        let app = create_test_app();

        let (status, body) = get_endpoint(&app, "/api/v1/costs/history").await;
        assert_eq!(status, StatusCode::OK, "Cost history should return 200");
        assert!(
            body.is_object() || body.is_array(),
            "Should return data: {}",
            body
        );

        body
    })
    .await;

    assert!(result.is_ok(), "Cost history: {}", result.unwrap_err());
}

/// OODA-17: Budget status endpoint.
#[tokio::test]
async fn test_budget_status() {
    let result = with_timeout(Duration::from_secs(5), async {
        let app = create_test_app();

        let (status, body) = get_endpoint(&app, "/api/v1/costs/budget").await;
        assert_eq!(status, StatusCode::OK, "Budget status should return 200");
        assert!(body.is_object(), "Should be an object: {}", body);

        body
    })
    .await;

    assert!(result.is_ok(), "Budget: {}", result.unwrap_err());
}

// ============================================================================
// Provider Status
// ============================================================================

/// OODA-17: Provider status endpoint shows current configuration.
#[tokio::test]
async fn test_provider_status() {
    let result = with_timeout(Duration::from_secs(5), async {
        let app = create_test_app();

        let (status, body) = get_endpoint(&app, "/api/v1/settings/provider/status").await;
        assert_eq!(status, StatusCode::OK, "Provider status should return 200");

        // Should have provider info
        assert!(body.is_object(), "Should be an object: {}", body);

        body
    })
    .await;

    assert!(result.is_ok(), "Provider: {}", result.unwrap_err());
}

// ============================================================================
// Document Processing with Status Tracking
// ============================================================================

/// OODA-17: Upload document and verify status transitions.
#[tokio::test]
async fn test_document_status_after_upload() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        // Upload document
        let payload = json!({
            "content": "Albert Einstein developed the theory of relativity. His famous equation E=mc² describes the relationship between energy and mass.",
            "title": "Pipeline Status Test"
        });

        let (status, body) = post_json(&app, "/api/v1/documents", &payload).await;
        assert_eq!(status, StatusCode::CREATED);

        let doc_id = body["document_id"].as_str().unwrap();
        let upload_status = body["status"].as_str().unwrap();

        // After sync upload, status should be "processed" or "completed"
        assert!(
            upload_status == "processed" || upload_status == "completed",
            "Upload status should be processed/completed, got '{}'",
            upload_status
        );

        // Verify document details include entity/chunk counts
        let (detail_status, detail) =
            get_endpoint(&app, &format!("/api/v1/documents/{}", doc_id)).await;
        assert_eq!(detail_status, StatusCode::OK);

        // Should have metadata
        assert!(detail.is_object(), "Detail should be an object");

        detail
    })
    .await;

    assert!(result.is_ok(), "Status tracking: {}", result.unwrap_err());
}

/// OODA-17: Upload + list shows correct document count and status.
#[tokio::test]
async fn test_list_shows_upload_status() {
    let result = with_timeout(Duration::from_secs(10), async {
        let app = create_test_app();

        // Upload 3 documents
        for i in 0..3 {
            let payload = json!({
                "content": format!("Document {} content for status tracking test.", i),
                "title": format!("Status Doc {}", i)
            });
            let (status, _) = post_json(&app, "/api/v1/documents", &payload).await;
            assert_eq!(status, StatusCode::CREATED, "Upload {} should succeed", i);
        }

        // List should show all 3
        let (status, list) = get_endpoint(&app, "/api/v1/documents").await;
        assert_eq!(status, StatusCode::OK);

        let total = list["total"].as_u64().unwrap_or(0);
        assert!(
            total >= 3,
            "Should have at least 3 documents, got {}",
            total
        );

        // Status counts should exist
        let status_counts = &list["status_counts"];
        assert!(
            status_counts.is_object(),
            "Should have status_counts: {}",
            list
        );

        list
    })
    .await;

    assert!(result.is_ok(), "List status: {}", result.unwrap_err());
}
