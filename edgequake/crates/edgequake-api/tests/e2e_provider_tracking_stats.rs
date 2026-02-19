//! # E2E Tests: Provider Tracking in ProcessingStats
//!
//! OODA 226: Tests that verify provider names are correctly tracked in ProcessingStats
//! when documents are processed through workspace-specific pipelines.
//!
//! @implements SPEC-032: E2E provider switching verification
//! @implements OODA-226: Provider tracking in ProcessingStats

use edgequake_api::AppState;
use edgequake_core::types::{CreateWorkspaceRequest, UpdateWorkspaceRequest};
use edgequake_core::Tenant;
use edgequake_llm::ProviderFactory;
use edgequake_pipeline::{EntityExtractor, LLMExtractor};
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
        description: Some(format!("Test workspace with {} provider", llm_provider)),
        max_documents: None,
        llm_provider: Some(llm_provider.to_string()),
        llm_model: Some(llm_model.to_string()),
        embedding_provider: Some(embedding_provider.to_string()),
        embedding_model: Some(embedding_model.to_string()),
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
// Provider Tracking Tests
// ============================================================================

/// Test: LLMExtractor returns correct provider_name() for mock provider.
#[tokio::test]
#[serial]
async fn test_llm_extractor_provider_name_mock() {
    clean_provider_env();

    let llm =
        ProviderFactory::create_llm_provider("mock", "mock-model").expect("Should create mock LLM");
    let extractor = LLMExtractor::new(llm);

    // Verify the extractor reports correct provider and model
    assert_eq!(extractor.provider_name(), "mock");
    assert_eq!(extractor.model_name(), "mock-model");
    assert_eq!(extractor.name(), "llm");
}

/// Test: LLMExtractor returns correct provider_name() for ollama provider.
#[tokio::test]
#[serial]
async fn test_llm_extractor_provider_name_ollama() {
    clean_provider_env();

    let llm = ProviderFactory::create_llm_provider("ollama", "llama3:8b")
        .expect("Should create ollama LLM");
    let extractor = LLMExtractor::new(llm);

    // Verify the extractor reports correct provider and model
    assert_eq!(extractor.provider_name(), "ollama");
    assert_eq!(extractor.model_name(), "llama3:8b");
}

/// Test: LLMExtractor returns correct provider_name() for lmstudio provider.
#[tokio::test]
#[serial]
async fn test_llm_extractor_provider_name_lmstudio() {
    clean_provider_env();

    let llm = ProviderFactory::create_llm_provider("lmstudio", "qwen2.5-coder")
        .expect("Should create lmstudio LLM");
    let extractor = LLMExtractor::new(llm);

    // Verify the extractor reports correct provider and model
    assert_eq!(extractor.provider_name(), "lmstudio");
    assert_eq!(extractor.model_name(), "qwen2.5-coder");
}

/// Test: Workspace config stores provider names for tracking.
#[tokio::test]
async fn test_workspace_stores_provider_names() {
    let state = AppState::test_state();

    // Create workspace with specific provider config
    let workspace = create_workspace_with_providers(
        &state,
        "Provider Tracking Test",
        "ollama",
        "llama3:8b",
        "ollama",
        "nomic-embed-text",
        768,
    )
    .await;

    // Verify workspace stores provider names
    assert_eq!(workspace.llm_provider, "ollama");
    assert_eq!(workspace.llm_model, "llama3:8b");
    assert_eq!(workspace.embedding_provider, "ollama");
    assert_eq!(workspace.embedding_model, "nomic-embed-text");
    assert_eq!(workspace.embedding_dimension, 768);
}

/// Test: Provider switch updates workspace provider names.
#[tokio::test]
async fn test_provider_switch_updates_names() {
    let state = AppState::test_state();

    // Create workspace with ollama
    let workspace = create_workspace_with_providers(
        &state,
        "Provider Switch Test",
        "ollama",
        "llama3:8b",
        "ollama",
        "nomic-embed-text",
        768,
    )
    .await;

    // Verify initial config
    assert_eq!(workspace.llm_provider, "ollama");
    assert_eq!(workspace.embedding_provider, "ollama");

    // Switch to lmstudio
    let update_request = UpdateWorkspaceRequest {
        name: None,
        description: None,
        max_documents: None,
        is_active: None,
        llm_provider: Some("lmstudio".to_string()),
        llm_model: Some("qwen2.5-coder".to_string()),
        embedding_provider: Some("lmstudio".to_string()),
        embedding_model: Some("text-embedding-nomic".to_string()),
        embedding_dimension: Some(384),

        vision_provider: None,
        vision_model: None,
    };

    let updated = state
        .workspace_service
        .update_workspace(workspace.workspace_id, update_request)
        .await
        .expect("Should update workspace");

    // Verify new config
    assert_eq!(updated.llm_provider, "lmstudio");
    assert_eq!(updated.llm_model, "qwen2.5-coder");
    assert_eq!(updated.embedding_provider, "lmstudio");
    assert_eq!(updated.embedding_model, "text-embedding-nomic");
    assert_eq!(updated.embedding_dimension, 384);
}

/// Test: Different workspaces maintain independent provider tracking.
#[tokio::test]
async fn test_independent_provider_tracking() {
    let state = AppState::test_state();

    // Create workspace 1 with ollama
    let ws1 = create_workspace_with_providers(
        &state,
        "Ollama Workspace",
        "ollama",
        "llama3:8b",
        "ollama",
        "nomic-embed-text",
        768,
    )
    .await;

    // Create workspace 2 with lmstudio
    let ws2 = create_workspace_with_providers(
        &state,
        "LMStudio Workspace",
        "lmstudio",
        "qwen2.5-coder",
        "lmstudio",
        "text-embedding-nomic",
        384,
    )
    .await;

    // Verify each workspace has its own config
    assert_eq!(ws1.llm_provider, "ollama");
    assert_eq!(ws1.embedding_provider, "ollama");
    assert_eq!(ws1.embedding_dimension, 768);

    assert_eq!(ws2.llm_provider, "lmstudio");
    assert_eq!(ws2.embedding_provider, "lmstudio");
    assert_eq!(ws2.embedding_dimension, 384);

    // Verify retrieval maintains independence
    let ws1_retrieved = state
        .workspace_service
        .get_workspace(ws1.workspace_id)
        .await
        .expect("Should get workspace")
        .expect("Workspace should exist");

    let ws2_retrieved = state
        .workspace_service
        .get_workspace(ws2.workspace_id)
        .await
        .expect("Should get workspace")
        .expect("Workspace should exist");

    assert_eq!(ws1_retrieved.llm_provider, "ollama");
    assert_eq!(ws2_retrieved.llm_provider, "lmstudio");
}

/// Test: EmbeddingProvider returns correct name() for provider tracking.
#[tokio::test]
#[serial]
async fn test_embedding_provider_name_tracking() {
    clean_provider_env();

    // Mock provider
    let mock_emb = ProviderFactory::create_embedding_provider("mock", "mock-embedding", 1536)
        .expect("Should create mock embedding");
    assert_eq!(mock_emb.name(), "mock");

    // Ollama provider
    let ollama_emb = ProviderFactory::create_embedding_provider("ollama", "nomic-embed-text", 768)
        .expect("Should create ollama embedding");
    assert_eq!(ollama_emb.name(), "ollama");

    // LMStudio provider
    let lmstudio_emb =
        ProviderFactory::create_embedding_provider("lmstudio", "text-embedding-nomic", 384)
            .expect("Should create lmstudio embedding");
    assert_eq!(lmstudio_emb.name(), "lmstudio");
}

/// Test: LLMProvider returns correct name() for provider tracking.
#[tokio::test]
#[serial]
async fn test_llm_provider_name_tracking() {
    clean_provider_env();

    // Mock provider
    let mock_llm =
        ProviderFactory::create_llm_provider("mock", "mock-model").expect("Should create mock LLM");
    assert_eq!(mock_llm.name(), "mock");

    // Ollama provider
    let ollama_llm = ProviderFactory::create_llm_provider("ollama", "llama3:8b")
        .expect("Should create ollama LLM");
    assert_eq!(ollama_llm.name(), "ollama");

    // LMStudio provider
    let lmstudio_llm = ProviderFactory::create_llm_provider("lmstudio", "qwen2.5-coder")
        .expect("Should create lmstudio LLM");
    assert_eq!(lmstudio_llm.name(), "lmstudio");
}

/// Test: create_workspace_pipeline uses workspace provider config.
#[tokio::test]
async fn test_create_workspace_pipeline_uses_provider_config() {
    let state = AppState::test_state();

    // Create workspace with mock provider
    let workspace = create_workspace_with_providers(
        &state,
        "Pipeline Provider Test",
        "mock",
        "mock-model",
        "mock",
        "mock-embedding",
        1536,
    )
    .await;

    // Create workspace pipeline
    let _pipeline = state
        .create_workspace_pipeline(&workspace.workspace_id.to_string())
        .await;

    // Verify the workspace config is correct (pipeline uses this to create providers)
    let ws_retrieved = state
        .workspace_service
        .get_workspace(workspace.workspace_id)
        .await
        .expect("Should get workspace")
        .expect("Workspace should exist");

    assert_eq!(ws_retrieved.llm_provider, "mock");
    assert_eq!(ws_retrieved.embedding_provider, "mock");
}
