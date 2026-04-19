//! E2E tests for provider status endpoint.
//!
//! @implements SPEC-032: Ollama/LM Studio provider support - Status API tests
//! @iteration OODA Loop #5 - Phase 5E.8

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::get,
    Router,
};
use common::clear_provider_detection_env;
use serial_test::serial;
use tower::ServiceExt;

#[tokio::test]
#[serial]
async fn test_provider_status_mock() {
    clear_provider_detection_env();

    let app_state = edgequake_api::AppState::new_memory(None::<String>);
    let app = edgequake_api::create_router(app_state);

    // Act: GET /api/v1/settings/provider/status
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/settings/provider/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert: 200 OK
    assert_eq!(response.status(), StatusCode::OK);

    // Assert: Response structure
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let status: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(status["provider"]["name"], "mock");
    assert_eq!(status["provider"]["type"], "llm");
    assert_eq!(status["provider"]["status"], "connected");
    assert_eq!(status["embedding"]["dimension"], 1536);
    assert_eq!(status["storage"]["type"], "memory");
    assert_eq!(status["storage"]["dimension"], 1536);
    assert_eq!(status["storage"]["dimension_mismatch"], false);

    // Assert: Metadata exists
    assert!(status["metadata"]["checked_at"].is_string());
    assert!(status["metadata"]["uptime_seconds"].is_number());
}

#[tokio::test]
#[serial]
async fn test_provider_status_ollama() {
    clear_provider_detection_env();
    std::env::set_var("OLLAMA_HOST", "http://localhost:11434");

    let app_state = edgequake_api::AppState::new_memory(None::<String>);
    let app = edgequake_api::create_router(app_state);

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/settings/provider/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let status: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(status["provider"]["name"], "ollama");
    assert_eq!(status["provider"]["type"], "llm");
    assert_eq!(status["embedding"]["dimension"], 768);
    assert_eq!(status["storage"]["dimension"], 768);

    // Cleanup
    clear_provider_detection_env();
}

#[tokio::test]
#[serial]
async fn test_models_health_ollama_http_endpoint() {
    clear_provider_detection_env();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let probe_app = Router::new().route(
        "/api/version",
        get(|| async { axum::Json(serde_json::json!({ "version": "test" })) }),
    );

    tokio::spawn(async move {
        axum::serve(listener, probe_app).await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    std::env::set_var("OLLAMA_HOST", format!("http://{}", addr));

    let app_state = edgequake_api::AppState::new_memory(None::<String>);
    let app = edgequake_api::create_router(app_state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/models/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let providers: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let ollama = providers
        .as_array()
        .unwrap()
        .iter()
        .find(|provider| provider["name"] == "ollama")
        .expect("ollama provider present");

    assert_eq!(ollama["health"]["available"], true);
    clear_provider_detection_env();
}

#[tokio::test]
#[serial]
async fn test_provider_status_uptime() {
    clear_provider_detection_env();

    let app_state = edgequake_api::AppState::new_memory(None::<String>);
    let app = edgequake_api::create_router(app_state);

    // Wait a bit to accumulate uptime
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/settings/provider/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let status: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let uptime = status["metadata"]["uptime_seconds"].as_u64().unwrap();
    assert!(
        uptime >= 1,
        "Uptime should be at least 1 second, got {}",
        uptime
    );

    let checked_at = status["metadata"]["checked_at"].as_str().unwrap();
    assert!(!checked_at.is_empty(), "checked_at should not be empty");

    // Verify ISO 8601 format (basic check)
    assert!(
        checked_at.contains("T"),
        "checked_at should be ISO 8601 format"
    );
}

#[tokio::test]
#[serial]
async fn test_provider_status_dimension_mismatch() {
    // Setup: Create state with mismatched dimensions (if possible in future)
    // For now, just verify the field exists
    clear_provider_detection_env();

    let app_state = edgequake_api::AppState::new_memory(None::<String>);
    let app = edgequake_api::create_router(app_state);

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/settings/provider/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let status: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Verify dimension_mismatch field exists and is boolean
    assert!(status["storage"]["dimension_mismatch"].is_boolean());
}
