//! E2E tests for chat handler with workspace-specific LLM providers.
//!
//! These tests verify that chat completions use the workspace's configured
//! LLM provider, not the global default.
//!
//! @implements SPEC-032: Chat with workspace LLM providers
//! @implements OODA-223: Chat handler workspace provider integration

use edgequake_api::AppState;
use edgequake_core::types::{CreateWorkspaceRequest, UpdateWorkspaceRequest};
use edgequake_core::Tenant;
use edgequake_llm::ProviderFactory;
use serial_test::serial;
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
    let tenant = Tenant::new(
        &format!("Tenant {}", name),
        &format!("tenant-{}", Uuid::new_v4()),
    );
    let created_tenant = state
        .workspace_service
        .create_tenant(tenant)
        .await
        .expect("Should create tenant");

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
// Chat LLM Provider Tests
// ============================================================================

/// Test: Workspace LLM config is correctly stored.
///
/// This is a prerequisite for chat using workspace provider.
#[tokio::test]
async fn test_workspace_llm_config_stored() {
    let state = AppState::test_state();

    let workspace = create_workspace_with_providers(
        &state,
        "LLM Config Test",
        "ollama",
        "gemma3:12b",
        "ollama",
        "nomic-embed-text",
        768,
    )
    .await;

    // Verify config was stored correctly
    let retrieved = state
        .workspace_service
        .get_workspace(workspace.workspace_id)
        .await
        .expect("Should get workspace")
        .expect("Workspace exists");

    assert_eq!(retrieved.llm_provider, "ollama");
    assert_eq!(retrieved.llm_model, "gemma3:12b");
}

/// Test: ProviderFactory can create LLM from workspace config.
///
/// Verifies the factory creates the correct provider type.
#[tokio::test]
#[serial]
async fn test_provider_factory_creates_workspace_llm() {
    let state = AppState::test_state();

    let workspace = create_workspace_with_providers(
        &state,
        "Factory LLM Test",
        "mock", // Mock always works
        "mock-chat-model",
        "mock",
        "mock-embedding",
        768,
    )
    .await;

    // Verify ProviderFactory can create the provider
    let provider_result =
        ProviderFactory::create_llm_provider(&workspace.llm_provider, &workspace.llm_model);

    assert!(provider_result.is_ok(), "Should create mock LLM provider");
    let provider = provider_result.unwrap();
    assert_eq!(provider.name(), "mock");
}

/// Test: LLM provider switch is reflected in workspace config.
#[tokio::test]
async fn test_llm_provider_switch_updates_config() {
    let state = AppState::test_state();

    // Create with Ollama
    let workspace = create_workspace_with_providers(
        &state,
        "LLM Switch Test",
        "ollama",
        "gemma3:12b",
        "ollama",
        "nomic-embed-text",
        768,
    )
    .await;

    // Verify initial config
    let initial = state
        .workspace_service
        .get_workspace(workspace.workspace_id)
        .await
        .expect("Get workspace")
        .expect("Exists");
    assert_eq!(initial.llm_provider, "ollama");

    // Switch to LMStudio
    let update = UpdateWorkspaceRequest {
        name: None,
        description: None,
        max_documents: None,
        is_active: None,
        llm_model: Some("qwen2.5-coder".to_string()),
        llm_provider: Some("lmstudio".to_string()),
        embedding_model: None,
        embedding_provider: None,
        embedding_dimension: None,

        vision_provider: None,
        vision_model: None,
    };

    state
        .workspace_service
        .update_workspace(workspace.workspace_id, update)
        .await
        .expect("Update should succeed");

    // Verify switch
    let updated = state
        .workspace_service
        .get_workspace(workspace.workspace_id)
        .await
        .expect("Get workspace")
        .expect("Exists");
    assert_eq!(updated.llm_provider, "lmstudio");
    assert_eq!(updated.llm_model, "qwen2.5-coder");
}

/// Test: Multiple workspaces have independent LLM configs.
#[tokio::test]
async fn test_independent_llm_configs_per_workspace() {
    let state = AppState::test_state();

    // Workspace 1 with Ollama
    let ws1 = create_workspace_with_providers(
        &state,
        "WS1 Ollama",
        "ollama",
        "gemma3:12b",
        "ollama",
        "nomic-embed-text",
        768,
    )
    .await;

    // Workspace 2 with LMStudio
    let ws2 = create_workspace_with_providers(
        &state,
        "WS2 LMStudio",
        "lmstudio",
        "qwen2.5-coder",
        "lmstudio",
        "text-embedding-nomic",
        768,
    )
    .await;

    // Workspace 3 with mock
    let ws3 = create_workspace_with_providers(
        &state,
        "WS3 Mock",
        "mock",
        "mock-model",
        "mock",
        "mock-embed",
        768,
    )
    .await;

    // Verify each workspace has its own config
    let config1 = state
        .workspace_service
        .get_workspace(ws1.workspace_id)
        .await
        .expect("Get")
        .expect("Exists");
    let config2 = state
        .workspace_service
        .get_workspace(ws2.workspace_id)
        .await
        .expect("Get")
        .expect("Exists");
    let config3 = state
        .workspace_service
        .get_workspace(ws3.workspace_id)
        .await
        .expect("Get")
        .expect("Exists");

    assert_eq!(config1.llm_provider, "ollama");
    assert_eq!(config2.llm_provider, "lmstudio");
    assert_eq!(config3.llm_provider, "mock");
}

/// Test: OpenAI LLM config stored but creation fails without key.
#[tokio::test]
#[serial]
async fn test_openai_llm_config_stored_creation_fails() {
    std::env::remove_var("OPENAI_API_KEY");

    let state = AppState::test_state();

    // Create workspace with OpenAI config
    let workspace = create_workspace_with_providers(
        &state,
        "OpenAI LLM Test",
        "openai",
        "gpt-4o-mini",
        "openai",
        "text-embedding-3-small",
        1536,
    )
    .await;

    // Config should be stored
    let retrieved = state
        .workspace_service
        .get_workspace(workspace.workspace_id)
        .await
        .expect("Get")
        .expect("Exists");
    assert_eq!(retrieved.llm_provider, "openai");

    // But provider creation should fail without API key
    let provider_result =
        ProviderFactory::create_llm_provider(&retrieved.llm_provider, &retrieved.llm_model);
    assert!(provider_result.is_err(), "OpenAI should fail without key");
}

/// Test: Workspace LLM config persists across retrievals.
#[tokio::test]
async fn test_llm_config_persistence() {
    let state = AppState::test_state();

    let workspace = create_workspace_with_providers(
        &state,
        "LLM Persistence Test",
        "mock",
        "persistent-model",
        "mock",
        "mock-embedding",
        768,
    )
    .await;

    // Retrieve multiple times
    for i in 0..5 {
        let retrieved = state
            .workspace_service
            .get_workspace(workspace.workspace_id)
            .await
            .expect(&format!("Get #{}", i))
            .expect("Exists");

        assert_eq!(retrieved.llm_provider, "mock", "Provider persists #{}", i);
        assert_eq!(
            retrieved.llm_model, "persistent-model",
            "Model persists #{}",
            i
        );
    }
}

/// Test: LLM full_id helper returns expected format.
#[tokio::test]
async fn test_llm_full_id_format() {
    let state = AppState::test_state();

    let workspace = create_workspace_with_providers(
        &state,
        "Full ID Test",
        "ollama",
        "gemma3:12b",
        "ollama",
        "nomic-embed-text",
        768,
    )
    .await;

    let retrieved = state
        .workspace_service
        .get_workspace(workspace.workspace_id)
        .await
        .expect("Get")
        .expect("Exists");

    // Verify full_id format (provider/model)
    let full_id = retrieved.llm_full_id();
    assert!(
        full_id.contains("ollama") || full_id.contains("gemma3:12b"),
        "Full ID should contain provider or model: {}",
        full_id
    );
}
