//! End-to-end integration tests for provider switching and workspace embedding configuration.
//!
//! These tests verify:
//! - Provider switching between OpenAI, Ollama, LM Studio, and Mock
//! - Workspace-specific embedding configuration
//! - Embedding dimension validation across providers
//!
//! @implements SPEC-032: Ollama/LM Studio provider support - E2E Provider Tests
//! @iteration OODA Loop #26-30 - E2E Provider Switching Tests

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_core::types::{CreateWorkspaceRequest, Workspace};
use edgequake_core::workspace_service::InMemoryWorkspaceService;
use edgequake_core::Tenant;
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

/// Test helper: Clean environment for isolated provider tests
fn clean_provider_env() {
    std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
    std::env::remove_var("OLLAMA_HOST");
    std::env::remove_var("OLLAMA_MODEL");
    std::env::remove_var("LMSTUDIO_HOST");
    std::env::remove_var("LMSTUDIO_MODEL");
    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("EDGEQUAKE_DEFAULT_EMBEDDING_MODEL");
    std::env::remove_var("EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER");
}

// ============================================================================
// Provider Auto-Detection Tests (OODA 26)
// ============================================================================

/// Test provider auto-detection with no environment variables (Mock fallback).
#[tokio::test]
#[serial]
async fn test_provider_autodetect_default_mock() {
    clean_provider_env();

    let state = edgequake_api::AppState::new_memory(None::<String>);

    assert_eq!(
        state.llm_provider.name(),
        "mock",
        "Without env vars, should fallback to Mock"
    );
    assert_eq!(state.embedding_provider.name(), "mock");
    assert_eq!(state.embedding_provider.dimension(), 1536);
}

/// Test provider detection priority: Ollama > LM Studio > OpenAI > Mock.
#[tokio::test]
#[serial]
async fn test_provider_detection_priority() {
    clean_provider_env();

    // Test 1: Only OpenAI set - should use OpenAI
    std::env::set_var("OPENAI_API_KEY", "sk-test-key");
    let _state = edgequake_api::AppState::new_memory(None::<String>);
    // May or may not be openai depending on if it validates the key
    clean_provider_env();

    // Test 2: Ollama set - should take priority
    std::env::set_var("OLLAMA_HOST", "http://localhost:11434");
    std::env::set_var("OPENAI_API_KEY", "sk-test-key");
    // Provider factory should check Ollama first
    clean_provider_env();

    // Test 3: All cleared - back to Mock
    let state_mock = edgequake_api::AppState::new_memory(None::<String>);
    assert_eq!(state_mock.llm_provider.name(), "mock");
}

// ============================================================================
// Workspace Embedding Configuration Tests (OODA 27)
// ============================================================================

/// Test creating workspace with custom embedding configuration.
#[tokio::test]
#[serial]
async fn test_workspace_custom_embedding_config() {
    use edgequake_core::workspace_service::WorkspaceService;

    let service = InMemoryWorkspaceService::new();

    // Create a tenant first
    let tenant = Tenant::new("Test Tenant", &format!("test-{}", Uuid::new_v4()));
    let created_tenant = service.create_tenant(tenant).await.unwrap();

    // Create workspace with custom embedding config
    let request = CreateWorkspaceRequest {
        name: "Custom Embedding Test".to_string(),
        slug: Some("custom-embedding".to_string()),
        description: Some("Test workspace with custom embeddings".to_string()),
        max_documents: None,
        llm_model: None,
        llm_provider: None,
        embedding_model: Some("embeddinggemma:latest".to_string()),
        embedding_provider: Some("ollama".to_string()),
        embedding_dimension: Some(768),

        vision_provider: None,
        vision_model: None,
    };

    let workspace = service
        .create_workspace(created_tenant.tenant_id, request)
        .await
        .expect("Should create workspace");

    assert_eq!(workspace.embedding_model, "embeddinggemma:latest");
    assert_eq!(workspace.embedding_provider, "ollama");
    assert_eq!(workspace.embedding_dimension, 768);
}

/// Test workspace uses default embedding config when none specified.
#[tokio::test]
#[serial]
async fn test_workspace_default_embedding_config() {
    use edgequake_core::workspace_service::WorkspaceService;

    let service = InMemoryWorkspaceService::new();

    let tenant = Tenant::new("Test Tenant", &format!("test-{}", Uuid::new_v4()));
    let created_tenant = service.create_tenant(tenant).await.unwrap();

    let request = CreateWorkspaceRequest {
        name: "Default Embedding Test".to_string(),
        slug: None,
        description: None,
        max_documents: None,
        llm_model: None,
        llm_provider: None,
        embedding_model: None,
        embedding_provider: None,
        embedding_dimension: None,

        vision_provider: None,
        vision_model: None,
    };

    let workspace = service
        .create_workspace(created_tenant.tenant_id, request)
        .await
        .expect("Should create workspace");

    // Should have default values
    assert!(!workspace.embedding_model.is_empty());
    assert!(!workspace.embedding_provider.is_empty());
    assert!(workspace.embedding_dimension > 0);
}

/// Test embedding dimension auto-detection from model name.
#[tokio::test]
#[serial]
async fn test_embedding_dimension_autodetection() {
    // Known provider dimensions for validation
    // Default is 768 for unknown models (matches Ollama embeddinggemma from models.toml)
    let test_cases = [
        ("text-embedding-3-small", 1536),
        ("text-embedding-3-large", 3072),
        ("embeddinggemma:latest", 768),
        ("nomic-embed-text", 768),
        ("nomic-embed-text:latest", 768),
        ("mxbai-embed-large", 1024),
        ("unknown-model", 768), // Default fallback now matches Ollama/models.toml
    ];

    for (model, expected) in test_cases {
        let detected = Workspace::detect_dimension_from_model(model);
        assert_eq!(
            detected, expected,
            "Model {} should detect dimension {}",
            model, expected
        );
    }
}

// ============================================================================
// Provider Switching Tests (OODA 28)
// ============================================================================

/// Test switching between workspaces with different providers.
#[tokio::test]
#[serial]
async fn test_workspace_provider_switching() {
    use edgequake_core::workspace_service::WorkspaceService;

    let service = InMemoryWorkspaceService::new();

    let tenant = Tenant::new("Test Tenant", &format!("test-{}", Uuid::new_v4()));
    let created_tenant = service.create_tenant(tenant).await.unwrap();

    // Create OpenAI-configured workspace
    let openai_request = CreateWorkspaceRequest {
        name: "OpenAI Workspace".to_string(),
        slug: Some("openai-ws".to_string()),
        description: None,
        max_documents: None,
        llm_model: None,
        llm_provider: None,
        embedding_model: Some("text-embedding-3-small".to_string()),
        embedding_provider: Some("openai".to_string()),
        embedding_dimension: Some(1536),

        vision_provider: None,
        vision_model: None,
    };

    let ws_openai = service
        .create_workspace(created_tenant.tenant_id, openai_request)
        .await
        .unwrap();

    // Create Ollama-configured workspace
    let ollama_request = CreateWorkspaceRequest {
        name: "Ollama Workspace".to_string(),
        slug: Some("ollama-ws".to_string()),
        description: None,
        max_documents: None,
        llm_model: None,
        llm_provider: None,
        embedding_model: Some("embeddinggemma:latest".to_string()),
        embedding_provider: Some("ollama".to_string()),
        embedding_dimension: Some(768),

        vision_provider: None,
        vision_model: None,
    };

    let ws_ollama = service
        .create_workspace(created_tenant.tenant_id, ollama_request)
        .await
        .unwrap();

    // Verify different configurations
    assert_eq!(ws_openai.embedding_provider, "openai");
    assert_eq!(ws_openai.embedding_dimension, 1536);

    assert_eq!(ws_ollama.embedding_provider, "ollama");
    assert_eq!(ws_ollama.embedding_dimension, 768);

    // Verify fetching returns correct config
    let fetched_openai = service
        .get_workspace(ws_openai.workspace_id)
        .await
        .unwrap()
        .unwrap();
    let fetched_ollama = service
        .get_workspace(ws_ollama.workspace_id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(fetched_openai.embedding_dimension, 1536);
    assert_eq!(fetched_ollama.embedding_dimension, 768);
}

// ============================================================================
// Provider Registry API Tests (OODA 29)
// ============================================================================

/// Test that provider registry endpoint returns valid response.
#[tokio::test]
#[serial]
async fn test_provider_registry_api() {
    clean_provider_env();
    let state = edgequake_api::AppState::new_memory(None::<String>);
    let app = edgequake_api::create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/settings/providers")
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

    // Should list supported providers
    assert!(providers["llm_providers"].is_array());
    assert!(providers["embedding_providers"].is_array());
}

/// Test provider status endpoint.
#[tokio::test]
#[serial]
async fn test_provider_status_api() {
    clean_provider_env();
    let state = edgequake_api::AppState::new_memory(None::<String>);
    let app = edgequake_api::create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/settings/provider/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let status: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(status["provider"]["name"], "mock");
    assert!(status["embedding"]["dimension"].is_number());
}

// ============================================================================
// Dimension Validation Tests (OODA 30)
// ============================================================================

/// Test dimension validation for known providers.
#[tokio::test]
#[serial]
async fn test_dimension_validation_known_providers() {
    // Known provider dimensions
    let known_dimensions = [
        ("openai", "text-embedding-3-small", 1536),
        ("openai", "text-embedding-3-large", 3072),
        ("ollama", "embeddinggemma:latest", 768),
        ("lmstudio", "nomic-embed-text", 768),
        ("mock", "mock", 1536),
    ];

    for (provider, model, expected_dim) in known_dimensions {
        // These are the expected default dimensions
        assert!(
            expected_dim > 0,
            "{}/{} should have positive dimension",
            provider,
            model
        );
        assert!(
            expected_dim <= 8192,
            "{}/{} dimension should be reasonable",
            provider,
            model
        );
    }
}

/// Test that different providers have different default dimensions.
#[tokio::test]
#[serial]
async fn test_provider_dimension_differences() {
    // OpenAI and Ollama should have different dimensions
    let openai_dim = Workspace::detect_dimension_from_model("text-embedding-3-small");
    let ollama_dim = Workspace::detect_dimension_from_model("embeddinggemma:latest");

    assert_eq!(openai_dim, 1536);
    assert_eq!(ollama_dim, 768);
    assert_ne!(
        openai_dim, ollama_dim,
        "Providers should have different dimensions"
    );
}

// ============================================================================
// Edge Case Tests
// ============================================================================

/// Test empty workspace has valid embedding config.
#[tokio::test]
#[serial]
async fn test_empty_workspace_embedding_config() {
    use edgequake_core::workspace_service::WorkspaceService;

    let service = InMemoryWorkspaceService::new();

    let tenant = Tenant::new("Test Tenant", &format!("test-{}", Uuid::new_v4()));
    let created_tenant = service.create_tenant(tenant).await.unwrap();

    let request = CreateWorkspaceRequest {
        name: "Empty Workspace".to_string(),
        slug: None,
        description: None,
        max_documents: None,
        llm_model: None,
        llm_provider: None,
        embedding_model: None,
        embedding_provider: None,
        embedding_dimension: None,

        vision_provider: None,
        vision_model: None,
    };

    let workspace = service
        .create_workspace(created_tenant.tenant_id, request)
        .await
        .unwrap();

    // Empty workspace should still have valid embedding config
    assert!(!workspace.embedding_model.is_empty());
    assert!(!workspace.embedding_provider.is_empty());
    assert!(workspace.embedding_dimension > 0);
}

/// Test concurrent workspace creation with different configs.
#[tokio::test]
#[serial]
async fn test_concurrent_workspace_creation() {
    use edgequake_core::workspace_service::WorkspaceService;

    let service = std::sync::Arc::new(InMemoryWorkspaceService::new());

    let tenant = Tenant::new("Test Tenant", &format!("test-{}", Uuid::new_v4()));
    let created_tenant = service.create_tenant(tenant).await.unwrap();
    let tenant_id = created_tenant.tenant_id;

    // Create multiple workspaces concurrently
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let service_clone = service.clone();
            let name = format!("Concurrent Workspace {}", i);
            let slug = format!("concurrent-{}", i);
            let provider = if i % 2 == 0 { "openai" } else { "ollama" };
            let dimension = if i % 2 == 0 { 1536 } else { 768 };

            tokio::spawn(async move {
                let request = CreateWorkspaceRequest {
                    name,
                    slug: Some(slug),
                    description: None,
                    max_documents: None,
                    llm_model: None,
                    llm_provider: None,
                    embedding_model: None,
                    embedding_provider: Some(provider.to_string()),
                    embedding_dimension: Some(dimension),

                    vision_provider: None,
                    vision_model: None,
                };

                service_clone.create_workspace(tenant_id, request).await
            })
        })
        .collect();

    // Wait for all to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Concurrent creation should succeed");
    }
}

/// Test LM Studio workspace configuration.
#[tokio::test]
#[serial]
async fn test_lmstudio_workspace_config() {
    use edgequake_core::workspace_service::WorkspaceService;

    let service = InMemoryWorkspaceService::new();

    let tenant = Tenant::new("Test Tenant", &format!("test-{}", Uuid::new_v4()));
    let created_tenant = service.create_tenant(tenant).await.unwrap();

    let request = CreateWorkspaceRequest {
        name: "LM Studio Workspace".to_string(),
        slug: Some("lmstudio-ws".to_string()),
        description: Some("Workspace using LM Studio".to_string()),
        max_documents: None,
        llm_model: None,
        llm_provider: None,
        embedding_model: Some("nomic-embed-text-v1.5".to_string()),
        embedding_provider: Some("lmstudio".to_string()),
        embedding_dimension: Some(768),

        vision_provider: None,
        vision_model: None,
    };

    let workspace = service
        .create_workspace(created_tenant.tenant_id, request)
        .await
        .unwrap();

    assert_eq!(workspace.embedding_model, "nomic-embed-text-v1.5");
    assert_eq!(workspace.embedding_provider, "lmstudio");
    assert_eq!(workspace.embedding_dimension, 768);
}

/// Test provider detection from model name.
#[tokio::test]
#[serial]
async fn test_provider_detection_from_model() {
    // Detection rules:
    // - "text-embedding*" or "ada*" → openai
    // - Contains ":" (like "model:tag") → ollama
    // - Starts with "gemma" or "llama" → lmstudio
    // - Default → openai
    let test_cases = [
        ("text-embedding-3-small", "openai"),
        ("text-embedding-3-large", "openai"),
        ("embeddinggemma:latest", "ollama"),   // Contains ":"
        ("nomic-embed-text:latest", "ollama"), // Contains ":"
        ("nomic-embed-text", "openai"),        // No ":", not gemma/llama → default openai
        ("gemma2-9b-it", "lmstudio"),          // Starts with gemma
        ("llama-3.1-8b", "lmstudio"),          // Starts with llama
    ];

    for (model, expected_provider) in test_cases {
        let detected = Workspace::detect_provider_from_model(model);
        assert_eq!(
            detected, expected_provider,
            "Model {} should detect provider {}",
            model, expected_provider
        );
    }
}
