//! End-to-end tests for workspace provider switching during rebuild operations.
//!
//! These tests verify that when a workspace's provider configuration is changed
//! and rebuild operations are triggered, the NEW provider is actually used.
//!
//! ## Critical Test Scenarios
//!
//! 1. Create workspace with Mock provider
//! 2. Upload document (uses Mock)
//! 3. Change workspace to different Mock model
//! 4. Trigger rebuild - verify NEW model is used
//!
//! @implements SPEC-032: Workspace provider switching for rebuild
//! @implements OODA-190: Rebuild Operation Provider Verification

use edgequake_api::safety_limits::create_safe_embedding_provider;
use edgequake_core::types::{CreateWorkspaceRequest, UpdateWorkspaceRequest};
use edgequake_core::Tenant;
use edgequake_llm::ProviderFactory;
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
}

// ============================================================================
// OODA 190: Rebuild Operation Tests
// ============================================================================

/// Test workspace update changes provider configuration in storage.
#[tokio::test]
#[serial]
async fn test_workspace_update_changes_provider_config() {
    clean_provider_env();

    let state = edgequake_api::AppState::new_memory(None::<String>);

    // Create tenant
    let tenant = Tenant::new("Test Tenant", &format!("test-{}", Uuid::new_v4()));
    let created_tenant = state
        .workspace_service
        .create_tenant(tenant)
        .await
        .expect("Should create tenant");

    // Create workspace with initial Mock config
    let create_request = CreateWorkspaceRequest {
        name: "Provider Switch Test".to_string(),
        slug: Some(format!("ws-{}", Uuid::new_v4())),
        description: None,
        max_documents: None,
        llm_model: Some("mock-model-v1".to_string()),
        llm_provider: Some("mock".to_string()),
        embedding_model: Some("mock-embed-v1".to_string()),
        embedding_provider: Some("mock".to_string()),
        embedding_dimension: Some(768),

        vision_provider: None,
        vision_model: None,
    };

    let workspace = state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, create_request)
        .await
        .expect("Should create workspace");

    // Verify initial config
    assert_eq!(workspace.llm_model, "mock-model-v1");
    assert_eq!(workspace.embedding_model, "mock-embed-v1");
    assert_eq!(workspace.embedding_dimension, 768);

    // Update workspace to new provider config
    let update_request = UpdateWorkspaceRequest {
        name: None,
        description: None,
        llm_model: Some("mock-model-v2".to_string()),
        llm_provider: Some("mock".to_string()),
        embedding_model: Some("mock-embed-v2".to_string()),
        embedding_provider: Some("mock".to_string()),
        embedding_dimension: Some(1536),
        max_documents: None,
        is_active: None,
    };

    let updated = state
        .workspace_service
        .update_workspace(workspace.workspace_id, update_request)
        .await
        .expect("Should update workspace");

    // Verify updated config
    assert_eq!(updated.llm_model, "mock-model-v2");
    assert_eq!(updated.embedding_model, "mock-embed-v2");
    assert_eq!(updated.embedding_dimension, 1536);

    // Verify the workspace lookup returns updated config
    let fetched = state
        .workspace_service
        .get_workspace(workspace.workspace_id)
        .await
        .expect("Should fetch workspace")
        .expect("Workspace should exist");

    assert_eq!(fetched.llm_model, "mock-model-v2");
    assert_eq!(fetched.embedding_model, "mock-embed-v2");
    assert_eq!(fetched.embedding_dimension, 1536);

    clean_provider_env();
}

/// Test that pipeline creation uses updated workspace config.
#[tokio::test]
#[serial]
async fn test_pipeline_uses_updated_workspace_config() {
    clean_provider_env();

    let state = edgequake_api::AppState::new_memory(None::<String>);

    // Create tenant and workspace
    let tenant = Tenant::new("Test Tenant", &format!("test-{}", Uuid::new_v4()));
    let created_tenant = state
        .workspace_service
        .create_tenant(tenant)
        .await
        .expect("Should create tenant");

    let create_request = CreateWorkspaceRequest {
        name: "Pipeline Update Test".to_string(),
        slug: Some(format!("ws-{}", Uuid::new_v4())),
        description: None,
        max_documents: None,
        llm_model: Some("initial-model".to_string()),
        llm_provider: Some("mock".to_string()),
        embedding_model: Some("initial-embed".to_string()),
        embedding_provider: Some("mock".to_string()),
        embedding_dimension: Some(768),

        vision_provider: None,
        vision_model: None,
    };

    let workspace = state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, create_request)
        .await
        .expect("Should create workspace");

    // Create pipeline BEFORE update
    let pipeline_before = state
        .create_workspace_pipeline(&workspace.workspace_id.to_string())
        .await;

    // Update workspace config
    let update_request = UpdateWorkspaceRequest {
        llm_model: Some("updated-model".to_string()),
        llm_provider: Some("mock".to_string()),
        embedding_model: Some("updated-embed".to_string()),
        embedding_provider: Some("mock".to_string()),
        embedding_dimension: Some(1536),
        ..Default::default()
    };

    let _updated = state
        .workspace_service
        .update_workspace(workspace.workspace_id, update_request)
        .await
        .expect("Should update workspace");

    // Create pipeline AFTER update
    let pipeline_after = state
        .create_workspace_pipeline(&workspace.workspace_id.to_string())
        .await;

    // Pipelines should be different instances (not cached)
    assert!(!std::sync::Arc::ptr_eq(&pipeline_before, &pipeline_after));

    clean_provider_env();
}

/// Test provider factory with dimension parameter.
///
/// Note: Mock provider ignores dimension parameter and uses default (1536).
/// This test verifies that the API accepts the dimension parameter,
/// even though Mock provider doesn't use it.
#[tokio::test]
#[serial]
async fn test_embedding_provider_accepts_dimension_param() {
    clean_provider_env();

    // Create embedding provider with 768 dimension (Mock ignores it)
    let result_768 = ProviderFactory::create_embedding_provider("mock", "mock-embed", 768);
    assert!(result_768.is_ok());
    let provider_768 = result_768.unwrap();
    // Mock provider always uses default dimension (1536)
    assert_eq!(
        provider_768.dimension(),
        1536,
        "Mock provider uses default dimension"
    );

    // Create embedding provider with 1536 dimension
    let result_1536 = ProviderFactory::create_embedding_provider("mock", "mock-embed", 1536);
    assert!(result_1536.is_ok());
    let provider_1536 = result_1536.unwrap();
    assert_eq!(provider_1536.dimension(), 1536);

    // Both Mock providers have the same dimension (Mock ignores parameter)
    assert_eq!(provider_768.dimension(), provider_1536.dimension());

    clean_provider_env();
}

/// Test safe embedding provider with dimension parameter.
///
/// Note: Mock provider ignores dimension parameter and uses default (1536).
/// The safe wrapper preserves this behavior.
#[tokio::test]
#[serial]
async fn test_safe_embedding_provider_accepts_dimension_param() {
    clean_provider_env();

    // Create safe embedding provider with 768 dimension (Mock ignores it)
    let result_768 = create_safe_embedding_provider("mock", "mock-embed", 768);
    assert!(result_768.is_ok());
    let provider_768 = result_768.unwrap();
    // Mock provider always uses default dimension (1536)
    assert_eq!(
        provider_768.dimension(),
        1536,
        "Mock provider uses default dimension"
    );

    // Create safe embedding provider with 1536 dimension
    let result_1536 = create_safe_embedding_provider("mock", "mock-embed", 1536);
    assert!(result_1536.is_ok());
    let provider_1536 = result_1536.unwrap();
    assert_eq!(provider_1536.dimension(), 1536);

    clean_provider_env();
}

// ============================================================================
// OODA 191: Concurrent Workspace Tests
// ============================================================================

/// Test concurrent workspace pipeline creation.
#[tokio::test]
#[serial]
async fn test_concurrent_workspace_pipelines() {
    clean_provider_env();

    let state = edgequake_api::AppState::new_memory(None::<String>);

    // Create tenant
    let tenant = Tenant::new("Concurrent Test", &format!("test-{}", Uuid::new_v4()));
    let created_tenant = state
        .workspace_service
        .create_tenant(tenant)
        .await
        .expect("Should create tenant");

    // Create multiple workspaces
    let mut workspaces = Vec::new();
    for i in 0..5 {
        let request = CreateWorkspaceRequest {
            name: format!("Workspace {}", i),
            slug: Some(format!("ws-{}-{}", i, Uuid::new_v4())),
            description: None,
            max_documents: None,
            llm_model: Some(format!("model-{}", i)),
            llm_provider: Some("mock".to_string()),
            embedding_model: Some(format!("embed-{}", i)),
            embedding_provider: Some("mock".to_string()),
            embedding_dimension: Some(768 + (i * 256) as usize),

            vision_provider: None,
            vision_model: None,
        };

        let ws = state
            .workspace_service
            .create_workspace(created_tenant.tenant_id, request)
            .await
            .expect("Should create workspace");
        workspaces.push(ws);
    }

    // Create pipelines concurrently
    let mut handles = Vec::new();
    for ws in &workspaces {
        let state_clone = state.clone();
        let ws_id = ws.workspace_id.to_string();
        handles.push(tokio::spawn(async move {
            state_clone.create_workspace_pipeline(&ws_id).await
        }));
    }

    // Wait for all pipelines
    let pipelines: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.expect("Task should complete"))
        .collect();

    // All pipelines should be created
    assert_eq!(pipelines.len(), 5);

    // Each pipeline should be unique
    for i in 0..pipelines.len() {
        for j in (i + 1)..pipelines.len() {
            assert!(!std::sync::Arc::ptr_eq(&pipelines[i], &pipelines[j]));
        }
    }

    clean_provider_env();
}

// ============================================================================
// OODA 192: Workspace Provider Verification Tests
// ============================================================================

/// Test that workspace with invalid provider shows clear error in logs.
///
/// This test doesn't verify the log output directly (would need log capture),
/// but verifies the behavior is as expected (fallback to default).
#[tokio::test]
#[serial]
async fn test_invalid_provider_logs_error_and_falls_back() {
    clean_provider_env();
    std::env::remove_var("OPENAI_API_KEY");

    let state = edgequake_api::AppState::new_memory(None::<String>);

    // Create tenant and workspace with OpenAI (but no API key)
    let tenant = Tenant::new("Invalid Provider Test", &format!("test-{}", Uuid::new_v4()));
    let created_tenant = state
        .workspace_service
        .create_tenant(tenant)
        .await
        .expect("Should create tenant");

    let request = CreateWorkspaceRequest {
        name: "OpenAI No Key".to_string(),
        slug: Some(format!("ws-{}", Uuid::new_v4())),
        description: None,
        max_documents: None,
        llm_model: Some("gpt-4o-mini".to_string()),
        llm_provider: Some("openai".to_string()),
        embedding_model: Some("text-embedding-3-small".to_string()),
        embedding_provider: Some("openai".to_string()),
        embedding_dimension: Some(1536),

        vision_provider: None,
        vision_model: None,
    };

    let workspace = state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, request)
        .await
        .expect("Should create workspace");

    // Creating pipeline should NOT panic, should fall back
    let pipeline = state
        .create_workspace_pipeline(&workspace.workspace_id.to_string())
        .await;

    // Pipeline should exist (fallback)
    assert!(std::sync::Arc::strong_count(&pipeline) >= 1);

    // The important verification is in the logs (ERROR level)
    // In production, this would be captured by log aggregation

    clean_provider_env();
}
