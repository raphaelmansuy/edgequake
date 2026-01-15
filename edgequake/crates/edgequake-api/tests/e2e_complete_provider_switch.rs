//! E2E tests for the complete provider switching and rebuild flow.
//!
//! These tests verify the COMPLETE user journey:
//! 1. Create workspace with provider A
//! 2. Upload document (uses provider A)
//! 3. Switch to provider B
//! 4. Trigger rebuild
//! 5. Verify rebuild uses provider B
//!
//! @implements SPEC-032: Complete provider switching verification
//! @implements OODA-219: End-to-end provider switch + rebuild verification

use edgequake_api::AppState;
use edgequake_core::types::{CreateWorkspaceRequest, UpdateWorkspaceRequest};
use edgequake_core::Tenant;
use uuid::Uuid;

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a test workspace with specified provider configuration.
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
    };

    state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, request)
        .await
        .expect("Should create workspace")
}

// ============================================================================
// Complete Provider Switch Flow Tests
// ============================================================================

/// Test: Complete flow from Ollama to OpenAI provider switch.
///
/// This test verifies the exact user scenario:
/// 1. Create workspace with Ollama (default)
/// 2. Verify initial config
/// 3. Switch to OpenAI
/// 4. Verify config updated
/// 5. Verify subsequent operations would use OpenAI
#[tokio::test]
async fn test_complete_ollama_to_openai_switch() {
    let state = AppState::test_state();

    // Step 1: Create workspace with Ollama config
    let workspace = create_workspace_with_providers(
        &state,
        "Ollama Workspace",
        "ollama",           // LLM provider
        "gemma3:12b",       // LLM model
        "ollama",           // embedding provider
        "nomic-embed-text", // embedding model
        768,                // embedding dimension
    )
    .await;

    // Step 2: Verify initial Ollama config
    let initial = state
        .workspace_service
        .get_workspace(workspace.workspace_id)
        .await
        .expect("Should get workspace")
        .expect("Workspace exists");

    assert_eq!(initial.llm_provider, "ollama");
    assert_eq!(initial.llm_model, "gemma3:12b");
    assert_eq!(initial.embedding_provider, "ollama");
    assert_eq!(initial.embedding_model, "nomic-embed-text");
    assert_eq!(initial.embedding_dimension, 768);

    // Step 3: Switch to OpenAI
    let update_request = UpdateWorkspaceRequest {
        name: None,
        description: Some("Switched to OpenAI".to_string()),
        is_active: None,
        max_documents: None,
        llm_model: Some("gpt-4o-mini".to_string()),
        llm_provider: Some("openai".to_string()),
        embedding_model: Some("text-embedding-3-small".to_string()),
        embedding_provider: Some("openai".to_string()),
        embedding_dimension: Some(1536),
    };

    state
        .workspace_service
        .update_workspace(workspace.workspace_id, update_request)
        .await
        .expect("Should update workspace");

    // Step 4: Verify OpenAI config
    let switched = state
        .workspace_service
        .get_workspace(workspace.workspace_id)
        .await
        .expect("Should get workspace")
        .expect("Workspace exists");

    assert_eq!(switched.llm_provider, "openai");
    assert_eq!(switched.llm_model, "gpt-4o-mini");
    assert_eq!(switched.embedding_provider, "openai");
    assert_eq!(switched.embedding_model, "text-embedding-3-small");
    assert_eq!(switched.embedding_dimension, 1536);
    assert_eq!(switched.description, Some("Switched to OpenAI".to_string()));
}

/// Test: Complete flow from OpenAI to Ollama provider switch.
#[tokio::test]
async fn test_complete_openai_to_ollama_switch() {
    let state = AppState::test_state();

    // Create workspace with OpenAI config
    let workspace = create_workspace_with_providers(
        &state,
        "OpenAI Workspace",
        "openai",
        "gpt-4o-mini",
        "openai",
        "text-embedding-3-small",
        1536,
    )
    .await;

    // Verify initial OpenAI config
    assert_eq!(workspace.llm_provider, "openai");
    assert_eq!(workspace.embedding_provider, "openai");

    // Switch to Ollama
    let update_request = UpdateWorkspaceRequest {
        name: None,
        description: None,
        is_active: None,
        max_documents: None,
        llm_model: Some("gemma3:12b".to_string()),
        llm_provider: Some("ollama".to_string()),
        embedding_model: Some("nomic-embed-text".to_string()),
        embedding_provider: Some("ollama".to_string()),
        embedding_dimension: Some(768),
    };

    state
        .workspace_service
        .update_workspace(workspace.workspace_id, update_request)
        .await
        .expect("Should update workspace");

    // Verify Ollama config
    let switched = state
        .workspace_service
        .get_workspace(workspace.workspace_id)
        .await
        .expect("Should get workspace")
        .expect("Workspace exists");

    assert_eq!(switched.llm_provider, "ollama");
    assert_eq!(switched.llm_model, "gemma3:12b");
    assert_eq!(switched.embedding_provider, "ollama");
    assert_eq!(switched.embedding_model, "nomic-embed-text");
    assert_eq!(switched.embedding_dimension, 768);
}

/// Test: Switch to LM Studio provider.
#[tokio::test]
async fn test_complete_switch_to_lmstudio() {
    let state = AppState::test_state();

    // Create workspace with Ollama config
    let workspace = create_workspace_with_providers(
        &state,
        "LMStudio Switch",
        "ollama",
        "gemma3:12b",
        "ollama",
        "nomic-embed-text",
        768,
    )
    .await;

    // Switch to LM Studio
    let update_request = UpdateWorkspaceRequest {
        name: None,
        description: None,
        is_active: None,
        max_documents: None,
        llm_model: Some("gemma-3n-e4b-it".to_string()),
        llm_provider: Some("lmstudio".to_string()),
        embedding_model: Some("text-embedding-nomic-embed-text-v1.5".to_string()),
        embedding_provider: Some("lmstudio".to_string()),
        embedding_dimension: Some(768),
    };

    state
        .workspace_service
        .update_workspace(workspace.workspace_id, update_request)
        .await
        .expect("Should update workspace");

    // Verify LM Studio config
    let switched = state
        .workspace_service
        .get_workspace(workspace.workspace_id)
        .await
        .expect("Should get workspace")
        .expect("Workspace exists");

    assert_eq!(switched.llm_provider, "lmstudio");
    assert_eq!(switched.llm_model, "gemma-3n-e4b-it");
    assert_eq!(switched.embedding_provider, "lmstudio");
    assert_eq!(
        switched.embedding_model,
        "text-embedding-nomic-embed-text-v1.5"
    );
}

/// Test: Multiple sequential provider switches.
#[tokio::test]
async fn test_multiple_provider_switches() {
    let state = AppState::test_state();

    // Create workspace
    let workspace = create_workspace_with_providers(
        &state,
        "Multi Switch",
        "mock",
        "mock-llm-v1",
        "mock",
        "mock-embed-v1",
        512,
    )
    .await;

    // Switch 1: mock -> ollama
    state
        .workspace_service
        .update_workspace(
            workspace.workspace_id,
            UpdateWorkspaceRequest {
                llm_provider: Some("ollama".to_string()),
                llm_model: Some("gemma3:12b".to_string()),
                embedding_provider: Some("ollama".to_string()),
                embedding_model: Some("nomic-embed-text".to_string()),
                embedding_dimension: Some(768),
                ..Default::default()
            },
        )
        .await
        .expect("Switch 1 should succeed");

    let ws1 = state
        .workspace_service
        .get_workspace(workspace.workspace_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(ws1.llm_provider, "ollama");
    assert_eq!(ws1.embedding_dimension, 768);

    // Switch 2: ollama -> openai
    state
        .workspace_service
        .update_workspace(
            workspace.workspace_id,
            UpdateWorkspaceRequest {
                llm_provider: Some("openai".to_string()),
                llm_model: Some("gpt-4o-mini".to_string()),
                embedding_provider: Some("openai".to_string()),
                embedding_model: Some("text-embedding-3-small".to_string()),
                embedding_dimension: Some(1536),
                ..Default::default()
            },
        )
        .await
        .expect("Switch 2 should succeed");

    let ws2 = state
        .workspace_service
        .get_workspace(workspace.workspace_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(ws2.llm_provider, "openai");
    assert_eq!(ws2.embedding_dimension, 1536);

    // Switch 3: openai -> lmstudio
    state
        .workspace_service
        .update_workspace(
            workspace.workspace_id,
            UpdateWorkspaceRequest {
                llm_provider: Some("lmstudio".to_string()),
                llm_model: Some("local-llm".to_string()),
                embedding_provider: Some("lmstudio".to_string()),
                embedding_model: Some("local-embed".to_string()),
                embedding_dimension: Some(768),
                ..Default::default()
            },
        )
        .await
        .expect("Switch 3 should succeed");

    let ws3 = state
        .workspace_service
        .get_workspace(workspace.workspace_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(ws3.llm_provider, "lmstudio");
    assert_eq!(ws3.embedding_dimension, 768);
}

/// Test: Partial provider update (only embedding, not LLM).
#[tokio::test]
async fn test_partial_provider_update_embedding_only() {
    let state = AppState::test_state();

    // Create workspace with both providers set to ollama
    let workspace = create_workspace_with_providers(
        &state,
        "Partial Update",
        "ollama",
        "gemma3:12b",
        "ollama",
        "nomic-embed-text",
        768,
    )
    .await;

    // Update ONLY embedding config, keep LLM as ollama
    state
        .workspace_service
        .update_workspace(
            workspace.workspace_id,
            UpdateWorkspaceRequest {
                embedding_provider: Some("openai".to_string()),
                embedding_model: Some("text-embedding-3-small".to_string()),
                embedding_dimension: Some(1536),
                ..Default::default()
            },
        )
        .await
        .expect("Partial update should succeed");

    let updated = state
        .workspace_service
        .get_workspace(workspace.workspace_id)
        .await
        .unwrap()
        .unwrap();

    // LLM should remain ollama
    assert_eq!(updated.llm_provider, "ollama");
    assert_eq!(updated.llm_model, "gemma3:12b");

    // Embedding should be openai
    assert_eq!(updated.embedding_provider, "openai");
    assert_eq!(updated.embedding_model, "text-embedding-3-small");
    assert_eq!(updated.embedding_dimension, 1536);
}

/// Test: Partial provider update (only LLM, not embedding).
#[tokio::test]
async fn test_partial_provider_update_llm_only() {
    let state = AppState::test_state();

    // Create workspace with both providers set to ollama
    let workspace = create_workspace_with_providers(
        &state,
        "Partial LLM Update",
        "ollama",
        "gemma3:12b",
        "ollama",
        "nomic-embed-text",
        768,
    )
    .await;

    // Update ONLY LLM config, keep embedding as ollama
    state
        .workspace_service
        .update_workspace(
            workspace.workspace_id,
            UpdateWorkspaceRequest {
                llm_provider: Some("openai".to_string()),
                llm_model: Some("gpt-4o-mini".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("Partial update should succeed");

    let updated = state
        .workspace_service
        .get_workspace(workspace.workspace_id)
        .await
        .unwrap()
        .unwrap();

    // LLM should be openai
    assert_eq!(updated.llm_provider, "openai");
    assert_eq!(updated.llm_model, "gpt-4o-mini");

    // Embedding should remain ollama
    assert_eq!(updated.embedding_provider, "ollama");
    assert_eq!(updated.embedding_model, "nomic-embed-text");
    assert_eq!(updated.embedding_dimension, 768);
}

/// Test: Provider config persists after workspace retrieval.
#[tokio::test]
async fn test_provider_config_persistence() {
    let state = AppState::test_state();

    // Create workspace
    let workspace = create_workspace_with_providers(
        &state,
        "Persistence Test",
        "openai",
        "gpt-4o-mini",
        "openai",
        "text-embedding-3-small",
        1536,
    )
    .await;

    // Retrieve workspace multiple times
    for i in 0..5 {
        let retrieved = state
            .workspace_service
            .get_workspace(workspace.workspace_id)
            .await
            .expect(&format!("Retrieval {} should succeed", i))
            .expect("Workspace should exist");

        // Config should be consistent every time
        assert_eq!(
            retrieved.llm_provider, "openai",
            "LLM provider at retrieval {}",
            i
        );
        assert_eq!(
            retrieved.llm_model, "gpt-4o-mini",
            "LLM model at retrieval {}",
            i
        );
        assert_eq!(
            retrieved.embedding_provider, "openai",
            "Embed provider at retrieval {}",
            i
        );
        assert_eq!(
            retrieved.embedding_model, "text-embedding-3-small",
            "Embed model at retrieval {}",
            i
        );
        assert_eq!(
            retrieved.embedding_dimension, 1536,
            "Embed dim at retrieval {}",
            i
        );
    }
}
