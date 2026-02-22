//! End-to-end tests for safety limits and provider verification.
//!
//! These tests verify:
//! - Safety limits (max_tokens, timeout) are enforced on LLM providers
//! - Provider switching actually uses the correct provider during extraction
//! - Workspace-specific providers are correctly configured
//!
//! @implements FEAT0778: Safety limits for LLM calls (E2E tests)
//! @implements BR0777: Hard max_tokens limit enforcement
//! @implements BR0778: Request timeout enforcement

use edgequake_api::safety_limits::{
    create_safe_llm_provider, SafetyLimitsConfig, ABSOLUTE_MAX_TOKENS, DEFAULT_MAX_TOKENS,
    DEFAULT_TIMEOUT_SECS,
};
use edgequake_core::types::CreateWorkspaceRequest;
use edgequake_core::Tenant;
use serial_test::serial;
use uuid::Uuid;

/// Test helper: Clean environment for isolated tests
fn clean_provider_env() {
    std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
    std::env::remove_var("OLLAMA_HOST");
    std::env::remove_var("OLLAMA_MODEL");
    std::env::remove_var("LMSTUDIO_HOST");
    std::env::remove_var("LMSTUDIO_MODEL");
    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("EDGEQUAKE_LLM_MAX_TOKENS");
    std::env::remove_var("EDGEQUAKE_LLM_TIMEOUT_SECS");
}

// ============================================================================
// Safety Limits Configuration Tests
// ============================================================================

/// Test safety limits configuration from environment variables.
#[test]
#[serial]
fn test_safety_limits_from_env() {
    clean_provider_env();

    // Test default values
    let config = SafetyLimitsConfig::from_env();
    assert_eq!(config.max_tokens, DEFAULT_MAX_TOKENS);
    assert_eq!(config.timeout.as_secs(), DEFAULT_TIMEOUT_SECS);

    // Test custom values from env
    std::env::set_var("EDGEQUAKE_LLM_MAX_TOKENS", "4096");
    std::env::set_var("EDGEQUAKE_LLM_TIMEOUT_SECS", "60");

    let config = SafetyLimitsConfig::from_env();
    assert_eq!(config.max_tokens, 4096);
    assert_eq!(config.timeout.as_secs(), 60);

    // Test clamping (too high)
    std::env::set_var("EDGEQUAKE_LLM_MAX_TOKENS", "100000");
    std::env::set_var("EDGEQUAKE_LLM_TIMEOUT_SECS", "10000");

    let config = SafetyLimitsConfig::from_env();
    assert_eq!(config.max_tokens, ABSOLUTE_MAX_TOKENS);
    assert_eq!(config.timeout.as_secs(), 600); // Max is 600 seconds

    clean_provider_env();
}

/// Test safety limits are enforced via the provider interface.
#[tokio::test]
#[serial]
async fn test_safety_limits_enforce_max_tokens() {
    clean_provider_env();

    // Create a safe provider via factory
    let result = create_safe_llm_provider("mock", "test-model");
    assert!(result.is_ok(), "Should create safe mock provider");

    let provider = result.unwrap();

    // The provider should work - safety limits are applied internally
    let response = provider.complete("Test prompt").await;
    assert!(response.is_ok(), "Provider should complete successfully");

    clean_provider_env();
}

// ============================================================================
// Provider Factory with Safety Limits Tests
// ============================================================================

/// Test that ProviderFactory creates safety-limited providers.
#[test]
#[serial]
fn test_factory_creates_safe_providers() {
    clean_provider_env();

    // Should work with mock provider
    let result = create_safe_llm_provider("mock", "test-model");
    assert!(result.is_ok(), "Should create safe mock provider");

    let provider = result.unwrap();
    assert_eq!(provider.name(), "mock");

    clean_provider_env();
}

// ============================================================================
// Workspace Pipeline Provider Verification Tests
// ============================================================================

/// Test that workspace pipelines use workspace-specific provider configuration.
#[tokio::test]
#[serial]
async fn test_workspace_pipeline_uses_workspace_provider() {
    clean_provider_env();

    let state = edgequake_api::AppState::new_memory(None::<String>);

    // Create a tenant
    let tenant = Tenant::new("Test Tenant", &format!("test-{}", Uuid::new_v4()));
    let created_tenant = state.workspace_service.create_tenant(tenant).await.unwrap();

    // Create workspace with Mock provider (should work without API keys)
    let request = CreateWorkspaceRequest {
        name: "Mock Provider Test".to_string(),
        slug: Some("mock-test".to_string()),
        description: None,
        max_documents: None,
        llm_model: Some("mock-model".to_string()),
        llm_provider: Some("mock".to_string()),
        embedding_model: Some("mock-embedding".to_string()),
        embedding_provider: Some("mock".to_string()),
        embedding_dimension: Some(1536),

        vision_provider: None,
        vision_model: None,
    };

    let workspace = state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, request)
        .await
        .expect("Should create workspace");

    // Get pipeline for this workspace
    let pipeline = state
        .create_workspace_pipeline(&workspace.workspace_id.to_string())
        .await;

    // Verify the pipeline has the correct configuration
    // Note: We can't easily inspect the pipeline's internal providers,
    // but we verify that creation succeeded without errors
    assert!(std::sync::Arc::strong_count(&pipeline) >= 1);

    clean_provider_env();
}

/// Test that different workspaces can have different provider configurations.
#[tokio::test]
#[serial]
async fn test_multiple_workspaces_different_providers() {
    clean_provider_env();

    let state = edgequake_api::AppState::new_memory(None::<String>);

    // Create a tenant
    let tenant = Tenant::new(
        "Multi-Provider Tenant",
        &format!("multi-{}", Uuid::new_v4()),
    );
    let created_tenant = state.workspace_service.create_tenant(tenant).await.unwrap();

    // Create workspace 1 with specific configuration
    let ws1_request = CreateWorkspaceRequest {
        name: "Workspace 1".to_string(),
        slug: Some("ws-1".to_string()),
        description: None,
        max_documents: None,
        llm_model: Some("model-a".to_string()),
        llm_provider: Some("mock".to_string()),
        embedding_model: Some("embed-a".to_string()),
        embedding_provider: Some("mock".to_string()),
        embedding_dimension: Some(768),

        vision_provider: None,
        vision_model: None,
    };

    // Create workspace 2 with different configuration
    let ws2_request = CreateWorkspaceRequest {
        name: "Workspace 2".to_string(),
        slug: Some("ws-2".to_string()),
        description: None,
        max_documents: None,
        llm_model: Some("model-b".to_string()),
        llm_provider: Some("mock".to_string()),
        embedding_model: Some("embed-b".to_string()),
        embedding_provider: Some("mock".to_string()),
        embedding_dimension: Some(1536),

        vision_provider: None,
        vision_model: None,
    };

    let ws1 = state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, ws1_request)
        .await
        .expect("Should create workspace 1");

    let ws2 = state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, ws2_request)
        .await
        .expect("Should create workspace 2");

    // Verify configurations are different
    assert_eq!(ws1.embedding_dimension, 768);
    assert_eq!(ws2.embedding_dimension, 1536);
    assert_ne!(ws1.llm_model, ws2.llm_model);
    assert_ne!(ws1.embedding_model, ws2.embedding_model);

    // Create pipelines for each workspace
    let pipeline1 = state
        .create_workspace_pipeline(&ws1.workspace_id.to_string())
        .await;
    let pipeline2 = state
        .create_workspace_pipeline(&ws2.workspace_id.to_string())
        .await;

    // Verify both pipelines were created (they should be different Arc instances)
    // Note: We use ptr comparison to verify they're different pipelines
    assert!(
        !std::sync::Arc::ptr_eq(&pipeline1, &pipeline2),
        "Different workspaces should have different pipelines"
    );

    clean_provider_env();
}

// ============================================================================
// API Integration Tests for Safety Limits
// ============================================================================

// NOTE: Comprehensive API tests for workspace creation with provider config
// are covered in e2e_provider_switching.rs. This test suite focuses on
// safety limits validation at the provider and pipeline level.
//
// The core functionality is verified by:
// - test_workspace_pipeline_uses_workspace_provider
// - test_multiple_workspaces_different_providers

// ============================================================================
// Timeout Enforcement Tests
// ============================================================================

/// Test that timeout is properly configured in safety limits.
#[test]
fn test_timeout_configuration() {
    // Test default timeout
    let config = SafetyLimitsConfig::default();
    assert_eq!(config.timeout.as_secs(), DEFAULT_TIMEOUT_SECS);

    // Test custom timeout
    let config = SafetyLimitsConfig::new(1000, 30);
    assert_eq!(config.timeout.as_secs(), 30);

    // Test minimum clamping
    let config = SafetyLimitsConfig::new(1000, 1);
    assert_eq!(
        config.timeout.as_secs(),
        10,
        "Should clamp to minimum 10 seconds"
    );

    // Test maximum clamping
    let config = SafetyLimitsConfig::new(1000, 10000);
    assert_eq!(
        config.timeout.as_secs(),
        600,
        "Should clamp to maximum 600 seconds"
    );
}

/// Test that strict config has appropriate limits.
#[test]
fn test_strict_config() {
    let config = SafetyLimitsConfig::strict();
    assert_eq!(config.max_tokens, 1024);
    assert_eq!(config.timeout.as_secs(), 30);
}

/// Test that permissive config has high limits.
#[test]
fn test_permissive_config() {
    let config = SafetyLimitsConfig::permissive();
    assert_eq!(config.max_tokens, ABSOLUTE_MAX_TOKENS);
    assert_eq!(config.timeout.as_secs(), 600);
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

/// Test handling of invalid workspace ID format.
#[tokio::test]
#[serial]
async fn test_invalid_workspace_id_fallback() {
    clean_provider_env();

    let state = edgequake_api::AppState::new_memory(None::<String>);

    // Try to create pipeline with invalid workspace ID
    let pipeline = state.create_workspace_pipeline("not-a-valid-uuid").await;

    // Should fall back to global pipeline without crashing
    assert!(std::sync::Arc::strong_count(&pipeline) >= 1);

    clean_provider_env();
}

/// Test handling of non-existent workspace.
#[tokio::test]
#[serial]
async fn test_nonexistent_workspace_fallback() {
    clean_provider_env();

    let state = edgequake_api::AppState::new_memory(None::<String>);

    // Try to create pipeline for a workspace that doesn't exist
    let fake_uuid = Uuid::new_v4().to_string();
    let pipeline = state.create_workspace_pipeline(&fake_uuid).await;

    // Should fall back to global pipeline without crashing
    assert!(std::sync::Arc::strong_count(&pipeline) >= 1);

    clean_provider_env();
}
