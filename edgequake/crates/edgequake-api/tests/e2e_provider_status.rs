//! E2E tests for provider status endpoint.
//!
//! @implements SPEC-032: Ollama/LM Studio provider support - Status API tests
//! @iteration OODA Loop #5 - Phase 5E.8

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serial_test::serial;
use tower::ServiceExt;

#[tokio::test]
#[serial]
async fn test_provider_status_mock() {
    // Setup: Use Mock provider
    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("OLLAMA_HOST");
    std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");

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
    // Setup: Use Ollama provider
    std::env::set_var("OLLAMA_HOST", "http://localhost:11434");
    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");

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
    std::env::remove_var("OLLAMA_HOST");
}

#[tokio::test]
#[serial]
async fn test_provider_status_uptime() {
    // Setup
    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("OLLAMA_HOST");
    std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");

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
    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("OLLAMA_HOST");
    std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");

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
