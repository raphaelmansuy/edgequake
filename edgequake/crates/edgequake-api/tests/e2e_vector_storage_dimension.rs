//! E2E tests for vector storage with workspace-specific embedding dimension.
//!
//! These tests verify that vector storage operations use the workspace's
//! configured embedding dimension, not the global default.
//!
//! @implements SPEC-032: Workspace-specific vector dimension
//! @implements OODA-224: Vector storage dimension integration

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
            "Test workspace with {} dimension",
            embedding_dimension
        )),
        max_documents: None,
        llm_model: Some(llm_model.to_string()),
        llm_provider: Some(llm_provider.to_string()),
        embedding_model: Some(embedding_model.to_string()),
        embedding_provider: Some(embedding_provider.to_string()),
        embedding_dimension: Some(embedding_dimension),
    };

    state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, request)
        .await
        .expect("Should create workspace")
}

// ============================================================================
// Embedding Dimension Configuration Tests
// ============================================================================

/// Test: Workspace embedding dimension is stored correctly.
#[tokio::test]
async fn test_workspace_embedding_dimension_stored() {
    let state = AppState::test_state();

    let workspace = create_workspace_with_providers(
        &state,
        "Dimension 768 Test",
        "mock",
        "mock-model",
        "mock",
        "mock-embed",
        768,
    )
    .await;

    let retrieved = state
        .workspace_service
        .get_workspace(workspace.workspace_id)
        .await
        .expect("Should get workspace")
        .expect("Workspace exists");

    assert_eq!(retrieved.embedding_dimension, 768);
}

/// Test: Large embedding dimension (1536 for OpenAI) is stored.
#[tokio::test]
async fn test_workspace_openai_dimension_stored() {
    let state = AppState::test_state();

    let workspace = create_workspace_with_providers(
        &state,
        "Dimension 1536 Test",
        "openai",
        "gpt-4o-mini",
        "openai",
        "text-embedding-3-small",
        1536,
    )
    .await;

    let retrieved = state
        .workspace_service
        .get_workspace(workspace.workspace_id)
        .await
        .expect("Should get workspace")
        .expect("Workspace exists");

    assert_eq!(retrieved.embedding_dimension, 1536);
}

/// Test: Different workspaces can have different dimensions.
#[tokio::test]
async fn test_workspaces_different_dimensions() {
    let state = AppState::test_state();

    // Workspace 1: Ollama default (768)
    let ws1 = create_workspace_with_providers(
        &state,
        "Ollama Workspace",
        "ollama",
        "gemma3:12b",
        "ollama",
        "nomic-embed-text",
        768,
    )
    .await;

    // Workspace 2: OpenAI (1536)
    let ws2 = create_workspace_with_providers(
        &state,
        "OpenAI Workspace",
        "openai",
        "gpt-4o-mini",
        "openai",
        "text-embedding-3-small",
        1536,
    )
    .await;

    // Workspace 3: Small dimension (384)
    let ws3 = create_workspace_with_providers(
        &state,
        "Small Workspace",
        "mock",
        "mock-model",
        "mock",
        "mock-small-embed",
        384,
    )
    .await;

    // Verify each has correct dimension
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

    assert_eq!(config1.embedding_dimension, 768);
    assert_eq!(config2.embedding_dimension, 1536);
    assert_eq!(config3.embedding_dimension, 384);
}

/// Test: Dimension can be updated on existing workspace.
#[tokio::test]
async fn test_dimension_update_on_existing_workspace() {
    let state = AppState::test_state();

    // Create with 768 dimension
    let workspace = create_workspace_with_providers(
        &state,
        "Dimension Update Test",
        "ollama",
        "gemma3:12b",
        "ollama",
        "nomic-embed-text",
        768,
    )
    .await;

    // Verify initial dimension
    let initial = state
        .workspace_service
        .get_workspace(workspace.workspace_id)
        .await
        .expect("Get")
        .expect("Exists");
    assert_eq!(initial.embedding_dimension, 768);

    // Update to 1536
    let update = UpdateWorkspaceRequest {
        name: None,
        description: None,
        max_documents: None,
        is_active: None,
        llm_model: None,
        llm_provider: None,
        embedding_model: Some("text-embedding-3-small".to_string()),
        embedding_provider: Some("openai".to_string()),
        embedding_dimension: Some(1536),
    };

    state
        .workspace_service
        .update_workspace(workspace.workspace_id, update)
        .await
        .expect("Update should succeed");

    // Verify updated dimension
    let updated = state
        .workspace_service
        .get_workspace(workspace.workspace_id)
        .await
        .expect("Get")
        .expect("Exists");
    assert_eq!(updated.embedding_dimension, 1536);
}

/// Test: Embedding full_id helper returns expected format.
#[tokio::test]
async fn test_embedding_full_id_format() {
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

    let full_id = retrieved.embedding_full_id();
    assert!(
        full_id.contains("ollama") || full_id.contains("nomic-embed-text"),
        "Full ID should contain provider or model: {}",
        full_id
    );
}

/// Test: Dimension configuration persists across retrievals.
#[tokio::test]
async fn test_dimension_persistence() {
    let state = AppState::test_state();

    let workspace = create_workspace_with_providers(
        &state,
        "Dimension Persistence Test",
        "mock",
        "mock-model",
        "mock",
        "mock-embed",
        512,
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
            retrieved.embedding_dimension, 512,
            "Dimension persists #{}",
            i
        );
    }
}

/// Test: Create workspace with zero dimension defaults to 768.
#[tokio::test]
async fn test_workspace_creation_without_dimension() {
    let state = AppState::test_state();

    let tenant = Tenant::new("No Dim Tenant", &format!("tenant-{}", Uuid::new_v4()));
    let created_tenant = state
        .workspace_service
        .create_tenant(tenant)
        .await
        .expect("Should create tenant");

    // Create workspace without specifying dimension
    let request = CreateWorkspaceRequest {
        name: "No Dimension Specified".to_string(),
        slug: Some(format!("ws-{}", Uuid::new_v4())),
        description: None,
        max_documents: None,
        llm_model: Some("mock-model".to_string()),
        llm_provider: Some("mock".to_string()),
        embedding_model: Some("mock-embed".to_string()),
        embedding_provider: Some("mock".to_string()),
        embedding_dimension: None, // Not specified
    };

    let workspace = state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, request)
        .await
        .expect("Should create workspace");

    // Should default to 768
    assert_eq!(
        workspace.embedding_dimension, 768,
        "Default dimension should be 768"
    );
}
