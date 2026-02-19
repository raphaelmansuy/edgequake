//! E2E tests for workspace pipeline provider integration.
//!
//! These tests verify that `create_workspace_pipeline()` correctly integrates
//! workspace configuration with ProviderFactory to create workspace-specific
//! pipeline instances.
//!
//! @implements SPEC-032: Workspace pipeline provider integration
//! @implements OODA-221: Pipeline creation with workspace providers

use edgequake_api::AppState;
use edgequake_core::types::{CreateWorkspaceRequest, UpdateWorkspaceRequest};
use edgequake_core::Tenant;
use uuid::Uuid;

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a workspace with specified provider configuration.
async fn create_workspace_with_providers(
    state: &AppState,
    name: &str,
    llm_provider: &str,
    llm_model: &str,
    embedding_provider: &str,
    embedding_model: &str,
    embedding_dimension: usize,
) -> edgequake_core::Workspace {
    // Create tenant
    let tenant = Tenant::new(
        &format!("Tenant {}", name),
        &format!("tenant-{}", Uuid::new_v4()),
    );
    let created_tenant = state
        .workspace_service
        .create_tenant(tenant)
        .await
        .expect("Should create tenant");

    // Create workspace
    let request = CreateWorkspaceRequest {
        name: name.to_string(),
        slug: Some(format!("ws-{}", Uuid::new_v4())),
        description: Some(format!("Test workspace with {} providers", llm_provider)),
        max_documents: None,
        llm_model: Some(llm_model.to_string()),
        llm_provider: Some(llm_provider.to_string()),
        embedding_model: Some(embedding_model.to_string()),
        embedding_provider: Some(embedding_provider.to_string()),
        embedding_dimension: Some(embedding_dimension),

        vision_provider: None,
        vision_model: None,
    };

    state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, request)
        .await
        .expect("Should create workspace")
}

// ============================================================================
// Pipeline Creation Tests
// ============================================================================

/// Test: create_workspace_pipeline with valid Ollama workspace.
///
/// Verifies that a workspace with Ollama config results in
/// a pipeline that uses Ollama providers.
#[tokio::test]
async fn test_workspace_pipeline_with_ollama() {
    let state = AppState::test_state();

    // Create workspace with Ollama config
    let workspace = create_workspace_with_providers(
        &state,
        "Ollama Pipeline Test",
        "ollama",
        "gemma3:12b",
        "ollama",
        "nomic-embed-text",
        768,
    )
    .await;

    // Create workspace pipeline
    let pipeline = state
        .create_workspace_pipeline(&workspace.workspace_id.to_string())
        .await;

    // Pipeline should be created (it's Arc<Pipeline>)
    // The fact that we get a pipeline back means the workspace config was found
    assert!(
        !std::ptr::eq(
            pipeline.as_ref() as *const _,
            state.pipeline.as_ref() as *const _
        ),
        "Should create workspace-specific pipeline, not global"
    );
}

/// Test: create_workspace_pipeline with invalid UUID.
///
/// Invalid workspace ID should fall back to global pipeline.
#[tokio::test]
async fn test_workspace_pipeline_invalid_uuid() {
    let state = AppState::test_state();

    // Use invalid UUID format
    let pipeline = state.create_workspace_pipeline("not-a-valid-uuid").await;

    // Should return global pipeline
    assert!(
        std::ptr::eq(
            pipeline.as_ref() as *const _,
            state.pipeline.as_ref() as *const _
        ),
        "Invalid UUID should return global pipeline"
    );
}

/// Test: create_workspace_pipeline with non-existent workspace.
///
/// Valid UUID but workspace doesn't exist should fall back to global.
#[tokio::test]
async fn test_workspace_pipeline_nonexistent_workspace() {
    let state = AppState::test_state();

    // Use valid UUID that doesn't exist
    let fake_uuid = Uuid::new_v4();
    let pipeline = state
        .create_workspace_pipeline(&fake_uuid.to_string())
        .await;

    // Should return global pipeline
    assert!(
        std::ptr::eq(
            pipeline.as_ref() as *const _,
            state.pipeline.as_ref() as *const _
        ),
        "Non-existent workspace should return global pipeline"
    );
}

/// Test: create_workspace_pipeline with LMStudio config.
///
/// Verifies LMStudio provider configuration works.
#[tokio::test]
async fn test_workspace_pipeline_with_lmstudio() {
    let state = AppState::test_state();

    // Create workspace with LMStudio config
    let workspace = create_workspace_with_providers(
        &state,
        "LMStudio Pipeline Test",
        "lmstudio",
        "qwen2.5-coder",
        "lmstudio",
        "text-embedding-nomic-embed",
        768,
    )
    .await;

    // Create workspace pipeline
    let pipeline = state
        .create_workspace_pipeline(&workspace.workspace_id.to_string())
        .await;

    // Pipeline should be workspace-specific
    assert!(
        !std::ptr::eq(
            pipeline.as_ref() as *const _,
            state.pipeline.as_ref() as *const _
        ),
        "Should create workspace-specific pipeline for LMStudio"
    );
}

/// Test: create_workspace_pipeline with mock provider.
///
/// Mock provider should always work without external dependencies.
#[tokio::test]
async fn test_workspace_pipeline_with_mock() {
    let state = AppState::test_state();

    // Create workspace with mock config
    let workspace = create_workspace_with_providers(
        &state,
        "Mock Pipeline Test",
        "mock",
        "mock-model",
        "mock",
        "mock-embedding",
        768,
    )
    .await;

    // Create workspace pipeline
    let pipeline = state
        .create_workspace_pipeline(&workspace.workspace_id.to_string())
        .await;

    // Mock provider should always work
    assert!(
        !std::ptr::eq(
            pipeline.as_ref() as *const _,
            state.pipeline.as_ref() as *const _
        ),
        "Should create workspace-specific pipeline for mock provider"
    );
}

/// Test: Pipeline changes after provider switch.
///
/// After updating workspace providers, create_workspace_pipeline
/// should return a new pipeline with updated config.
#[tokio::test]
async fn test_pipeline_changes_after_provider_switch() {
    let state = AppState::test_state();

    // Create workspace with Ollama
    let workspace = create_workspace_with_providers(
        &state,
        "Provider Switch Pipeline",
        "ollama",
        "gemma3:12b",
        "ollama",
        "nomic-embed-text",
        768,
    )
    .await;

    // Get initial pipeline
    let pipeline1 = state
        .create_workspace_pipeline(&workspace.workspace_id.to_string())
        .await;

    // Verify initial pipeline is workspace-specific
    assert!(!std::ptr::eq(
        pipeline1.as_ref() as *const _,
        state.pipeline.as_ref() as *const _
    ));

    // Switch to LMStudio
    let update = UpdateWorkspaceRequest {
        name: None,
        description: None,
        max_documents: None,
        is_active: None,
        llm_model: Some("qwen2.5-coder".to_string()),
        llm_provider: Some("lmstudio".to_string()),
        embedding_model: Some("text-embedding-nomic".to_string()),
        embedding_provider: Some("lmstudio".to_string()),
        embedding_dimension: Some(768),

        vision_provider: None,
        vision_model: None,
    };

    state
        .workspace_service
        .update_workspace(workspace.workspace_id, update)
        .await
        .expect("Update should succeed");

    // Get new pipeline after switch
    let pipeline2 = state
        .create_workspace_pipeline(&workspace.workspace_id.to_string())
        .await;

    // New pipeline should also be workspace-specific
    assert!(!std::ptr::eq(
        pipeline2.as_ref() as *const _,
        state.pipeline.as_ref() as *const _
    ));

    // The two pipelines should be different instances
    // (new pipeline created with new config)
    assert!(
        !std::ptr::eq(
            pipeline1.as_ref() as *const _,
            pipeline2.as_ref() as *const _
        ),
        "Pipeline should be recreated after provider switch"
    );
}

/// Test: Multiple workspaces get isolated pipelines.
///
/// Different workspaces with different configs should get
/// different pipeline instances.
#[tokio::test]
async fn test_isolated_pipelines_per_workspace() {
    let state = AppState::test_state();

    // Create workspace 1 with Ollama
    let ws1 = create_workspace_with_providers(
        &state,
        "Workspace 1",
        "ollama",
        "gemma3:12b",
        "ollama",
        "nomic-embed-text",
        768,
    )
    .await;

    // Create workspace 2 with LMStudio
    let ws2 = create_workspace_with_providers(
        &state,
        "Workspace 2",
        "lmstudio",
        "qwen2.5-coder",
        "lmstudio",
        "text-embedding-nomic",
        768,
    )
    .await;

    // Create workspace 3 with mock
    let ws3 = create_workspace_with_providers(
        &state,
        "Workspace 3",
        "mock",
        "mock-model",
        "mock",
        "mock-embedding",
        512,
    )
    .await;

    // Get pipelines
    let pipeline1 = state
        .create_workspace_pipeline(&ws1.workspace_id.to_string())
        .await;
    let pipeline2 = state
        .create_workspace_pipeline(&ws2.workspace_id.to_string())
        .await;
    let pipeline3 = state
        .create_workspace_pipeline(&ws3.workspace_id.to_string())
        .await;

    // All should be workspace-specific (not global)
    assert!(!std::ptr::eq(
        pipeline1.as_ref() as *const _,
        state.pipeline.as_ref() as *const _
    ));
    assert!(!std::ptr::eq(
        pipeline2.as_ref() as *const _,
        state.pipeline.as_ref() as *const _
    ));
    assert!(!std::ptr::eq(
        pipeline3.as_ref() as *const _,
        state.pipeline.as_ref() as *const _
    ));

    // Each should be a distinct instance
    assert!(!std::ptr::eq(
        pipeline1.as_ref() as *const _,
        pipeline2.as_ref() as *const _
    ));
    assert!(!std::ptr::eq(
        pipeline2.as_ref() as *const _,
        pipeline3.as_ref() as *const _
    ));
    assert!(!std::ptr::eq(
        pipeline1.as_ref() as *const _,
        pipeline3.as_ref() as *const _
    ));
}

/// Test: OpenAI workspace without API key falls back.
///
/// When OpenAI is configured but no API key is set,
/// should fall back to global pipeline.
#[tokio::test]
async fn test_openai_workspace_without_key_fallback() {
    let state = AppState::test_state();

    // Ensure no OpenAI key
    std::env::remove_var("OPENAI_API_KEY");

    // Create workspace with OpenAI config (will fail on provider creation)
    let workspace = create_workspace_with_providers(
        &state,
        "OpenAI No Key",
        "openai",
        "gpt-4o-mini",
        "openai",
        "text-embedding-3-small",
        1536,
    )
    .await;

    // Get pipeline - should fall back to global
    let pipeline = state
        .create_workspace_pipeline(&workspace.workspace_id.to_string())
        .await;

    // Should return global pipeline since OpenAI creation fails
    assert!(
        std::ptr::eq(
            pipeline.as_ref() as *const _,
            state.pipeline.as_ref() as *const _
        ),
        "OpenAI without key should fall back to global pipeline"
    );
}
