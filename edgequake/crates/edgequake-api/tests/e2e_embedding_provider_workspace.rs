//! E2E tests for embedding provider workspace integration.
//!
//! These tests verify that embedding providers are correctly created
//! based on workspace configuration.
//!
//! @implements SPEC-032: Workspace-specific embedding providers
//! @implements OODA-225: Embedding provider integration tests

use edgequake_api::AppState;
use edgequake_core::types::{CreateWorkspaceRequest, UpdateWorkspaceRequest};
use edgequake_core::Tenant;
use edgequake_llm::ProviderFactory;
use serial_test::serial;
use uuid::Uuid;

// ============================================================================
// Helper Functions
// ============================================================================

fn clean_provider_env() {
    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("OLLAMA_HOST");
    std::env::remove_var("LMSTUDIO_HOST");
}

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
        description: Some(format!(
            "Test workspace with {} embedding",
            embedding_provider
        )),
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
// Embedding Provider Configuration Tests
// ============================================================================

/// Test: Workspace embedding config is stored correctly.
#[tokio::test]
async fn test_workspace_embedding_config_stored() {
    let state = AppState::test_state();

    let workspace = create_workspace_with_providers(
        &state,
        "Embedding Config Test",
        "mock",
        "mock-llm",
        "ollama",
        "nomic-embed-text",
        768,
    )
    .await;

    let retrieved = state
        .workspace_service
        .get_workspace(workspace.workspace_id)
        .await
        .expect("Should get workspace")
        .expect("Workspace exists");

    assert_eq!(retrieved.embedding_provider, "ollama");
    assert_eq!(retrieved.embedding_model, "nomic-embed-text");
    assert_eq!(retrieved.embedding_dimension, 768);
}

/// Test: ProviderFactory can create embedding provider from workspace config.
#[tokio::test]
#[serial]
async fn test_provider_factory_creates_workspace_embedding() {
    clean_provider_env();

    let state = AppState::test_state();

    let workspace = create_workspace_with_providers(
        &state,
        "Factory Embed Test",
        "mock",
        "mock-llm",
        "mock",
        "mock-embedding",
        768,
    )
    .await;

    // Create embedding provider from workspace config
    let provider_result = ProviderFactory::create_embedding_provider(
        &workspace.embedding_provider,
        &workspace.embedding_model,
        workspace.embedding_dimension,
    );

    assert!(
        provider_result.is_ok(),
        "Should create mock embedding provider"
    );
    let provider = provider_result.unwrap();
    assert_eq!(provider.name(), "mock");
}

/// Test: Embedding provider switch updates workspace config.
#[tokio::test]
async fn test_embedding_provider_switch_updates_config() {
    let state = AppState::test_state();

    // Create with Ollama
    let workspace = create_workspace_with_providers(
        &state,
        "Embed Switch Test",
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
        .expect("Get")
        .expect("Exists");
    assert_eq!(initial.embedding_provider, "ollama");

    // Switch to LMStudio
    let update = UpdateWorkspaceRequest {
        name: None,
        description: None,
        max_documents: None,
        is_active: None,
        llm_model: None,
        llm_provider: None,
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

    // Verify switch
    let updated = state
        .workspace_service
        .get_workspace(workspace.workspace_id)
        .await
        .expect("Get")
        .expect("Exists");
    assert_eq!(updated.embedding_provider, "lmstudio");
    assert_eq!(updated.embedding_model, "text-embedding-nomic");
}

/// Test: Multiple workspaces have independent embedding configs.
#[tokio::test]
async fn test_independent_embedding_configs_per_workspace() {
    let state = AppState::test_state();

    // Workspace 1 with Ollama embedding
    let ws1 = create_workspace_with_providers(
        &state,
        "WS1 Ollama Embed",
        "mock",
        "mock-llm",
        "ollama",
        "nomic-embed-text",
        768,
    )
    .await;

    // Workspace 2 with LMStudio embedding
    let ws2 = create_workspace_with_providers(
        &state,
        "WS2 LMStudio Embed",
        "mock",
        "mock-llm",
        "lmstudio",
        "text-embedding-nomic",
        768,
    )
    .await;

    // Workspace 3 with mock embedding
    let ws3 = create_workspace_with_providers(
        &state,
        "WS3 Mock Embed",
        "mock",
        "mock-llm",
        "mock",
        "mock-embed",
        512,
    )
    .await;

    // Verify each has independent config
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

    assert_eq!(config1.embedding_provider, "ollama");
    assert_eq!(config2.embedding_provider, "lmstudio");
    assert_eq!(config3.embedding_provider, "mock");
}

/// Test: OpenAI embedding config stored but creation fails without key.
#[tokio::test]
#[serial]
async fn test_openai_embedding_config_creation_fails() {
    clean_provider_env();

    let state = AppState::test_state();

    // Create workspace with OpenAI config
    let workspace = create_workspace_with_providers(
        &state,
        "OpenAI Embed Test",
        "mock",
        "mock-llm",
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
    assert_eq!(retrieved.embedding_provider, "openai");

    // But provider creation should fail without API key
    let provider_result = ProviderFactory::create_embedding_provider(
        &retrieved.embedding_provider,
        &retrieved.embedding_model,
        retrieved.embedding_dimension,
    );
    assert!(
        provider_result.is_err(),
        "OpenAI embedding should fail without key"
    );
}

/// Test: Ollama embedding provider created successfully.
#[tokio::test]
#[serial]
async fn test_ollama_embedding_provider_creation() {
    clean_provider_env();

    let state = AppState::test_state();

    let workspace = create_workspace_with_providers(
        &state,
        "Ollama Embed Create Test",
        "ollama",
        "gemma3:12b",
        "ollama",
        "nomic-embed-text",
        768,
    )
    .await;

    // Should create Ollama embedding provider
    let provider_result = ProviderFactory::create_embedding_provider(
        &workspace.embedding_provider,
        &workspace.embedding_model,
        workspace.embedding_dimension,
    );

    assert!(
        provider_result.is_ok(),
        "Should create Ollama embedding provider"
    );
    let provider = provider_result.unwrap();
    assert_eq!(provider.name(), "ollama");
}

/// Test: LMStudio embedding provider created successfully.
#[tokio::test]
#[serial]
async fn test_lmstudio_embedding_provider_creation() {
    clean_provider_env();

    let state = AppState::test_state();

    let workspace = create_workspace_with_providers(
        &state,
        "LMStudio Embed Create Test",
        "lmstudio",
        "qwen2.5-coder",
        "lmstudio",
        "text-embedding-nomic",
        768,
    )
    .await;

    // Should create LMStudio embedding provider
    let provider_result = ProviderFactory::create_embedding_provider(
        &workspace.embedding_provider,
        &workspace.embedding_model,
        workspace.embedding_dimension,
    );

    assert!(
        provider_result.is_ok(),
        "Should create LMStudio embedding provider"
    );
    let provider = provider_result.unwrap();
    assert_eq!(provider.name(), "lmstudio");
}

/// Test: Embedding config persists across retrievals.
#[tokio::test]
async fn test_embedding_config_persistence() {
    let state = AppState::test_state();

    let workspace = create_workspace_with_providers(
        &state,
        "Embed Persistence Test",
        "mock",
        "mock-llm",
        "mock",
        "persistent-embed",
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

        assert_eq!(
            retrieved.embedding_provider, "mock",
            "Provider persists #{}",
            i
        );
        assert_eq!(
            retrieved.embedding_model, "persistent-embed",
            "Model persists #{}",
            i
        );
    }
}
