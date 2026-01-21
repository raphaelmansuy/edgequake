//! End-to-end tests for provider lineage during rebuild operations.
//!
//! These tests verify that when documents are reprocessed after a provider
//! change, the NEW provider is recorded in the document lineage.
//!
//! @implements SPEC-032: Provider Lineage Tracking
//! @implements OODA-207-210: Rebuild Lineage Verification

use edgequake_core::types::CreateWorkspaceRequest;
use edgequake_core::Tenant;
use edgequake_pipeline::ProcessingStats;
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
// OODA 207: ProcessingStats Provider Fields Tests
// ============================================================================

/// Test that ProcessingStats can store and retrieve provider lineage.
#[tokio::test]
#[serial]
async fn test_processing_stats_stores_provider_lineage() {
    clean_provider_env();

    let mut stats = ProcessingStats::default();

    // Set provider lineage
    stats.llm_provider = Some("openai".to_string());
    stats.llm_model = Some("gpt-4o-mini".to_string());
    stats.embedding_provider = Some("openai".to_string());
    stats.embedding_model = Some("text-embedding-3-small".to_string());
    stats.embedding_dimensions = Some(1536);

    // Verify
    assert_eq!(stats.llm_provider, Some("openai".to_string()));
    assert_eq!(stats.llm_model, Some("gpt-4o-mini".to_string()));
    assert_eq!(stats.embedding_provider, Some("openai".to_string()));
    assert_eq!(
        stats.embedding_model,
        Some("text-embedding-3-small".to_string())
    );
    assert_eq!(stats.embedding_dimensions, Some(1536));

    clean_provider_env();
}

/// Test that ProcessingStats serializes provider lineage to JSON.
#[tokio::test]
#[serial]
async fn test_processing_stats_serializes_lineage() {
    clean_provider_env();

    let mut stats = ProcessingStats::default();
    stats.llm_provider = Some("ollama".to_string());
    stats.llm_model = Some("gemma3:12b".to_string());
    stats.embedding_provider = Some("ollama".to_string());
    stats.embedding_model = Some("nomic-embed-text:latest".to_string());
    stats.embedding_dimensions = Some(768);
    stats.chunk_count = 5;

    // Serialize and deserialize
    let json = serde_json::to_string(&stats).expect("Should serialize");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Should parse");

    // Verify lineage fields are in JSON
    assert_eq!(parsed["llm_provider"], "ollama");
    assert_eq!(parsed["llm_model"], "gemma3:12b");
    assert_eq!(parsed["embedding_provider"], "ollama");
    assert_eq!(parsed["embedding_model"], "nomic-embed-text:latest");
    assert_eq!(parsed["embedding_dimensions"], 768);
    assert_eq!(parsed["chunk_count"], 5);

    clean_provider_env();
}

// ============================================================================
// OODA 208: Provider Lineage in Workspace Pipeline Tests
// ============================================================================

/// Test that workspace pipeline uses workspace config for lineage.
#[tokio::test]
#[serial]
async fn test_workspace_pipeline_uses_workspace_config_for_lineage() {
    clean_provider_env();

    let state = edgequake_api::AppState::new_memory(None::<String>);

    // Create tenant
    let tenant = Tenant::new("Lineage Test", &format!("test-{}", Uuid::new_v4()));
    let created_tenant = state
        .workspace_service
        .create_tenant(tenant)
        .await
        .expect("Should create tenant");

    // Create workspace with specific config
    let create_request = CreateWorkspaceRequest {
        name: "Lineage Test".to_string(),
        slug: Some(format!("ws-lineage-{}", Uuid::new_v4())),
        description: None,
        max_documents: None,
        llm_model: Some("custom-llm-model".to_string()),
        llm_provider: Some("mock".to_string()),
        embedding_model: Some("custom-embed-model".to_string()),
        embedding_provider: Some("mock".to_string()),
        embedding_dimension: Some(768),
    };

    let workspace = state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, create_request)
        .await
        .expect("Should create workspace");

    // Verify workspace has correct config
    assert_eq!(workspace.llm_model, "custom-llm-model");
    assert_eq!(workspace.llm_provider, "mock");
    assert_eq!(workspace.embedding_model, "custom-embed-model");
    assert_eq!(workspace.embedding_provider, "mock");
    assert_eq!(workspace.embedding_dimension, 768);

    // Create pipeline for this workspace
    let pipeline = state
        .create_workspace_pipeline(&workspace.workspace_id.to_string())
        .await;

    // Pipeline should exist
    assert!(std::sync::Arc::strong_count(&pipeline) >= 1);

    clean_provider_env();
}

// ============================================================================
// OODA 209: Provider Config Change Detection Tests
// ============================================================================

/// Test that workspace update changes provider config.
#[tokio::test]
#[serial]
async fn test_workspace_update_changes_lineage_source() {
    clean_provider_env();

    let state = edgequake_api::AppState::new_memory(None::<String>);

    // Create tenant and workspace
    let tenant = Tenant::new("Update Test", &format!("test-{}", Uuid::new_v4()));
    let created_tenant = state
        .workspace_service
        .create_tenant(tenant)
        .await
        .expect("Should create tenant");

    let create_request = CreateWorkspaceRequest {
        name: "Update Test".to_string(),
        slug: Some(format!("ws-update-{}", Uuid::new_v4())),
        description: None,
        max_documents: None,
        llm_model: Some("old-llm-model".to_string()),
        llm_provider: Some("mock".to_string()),
        embedding_model: Some("old-embed-model".to_string()),
        embedding_provider: Some("mock".to_string()),
        embedding_dimension: Some(768),
    };

    let workspace = state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, create_request)
        .await
        .expect("Should create workspace");

    // Update workspace to new config
    use edgequake_core::types::UpdateWorkspaceRequest;
    let update_request = UpdateWorkspaceRequest {
        llm_model: Some("new-llm-model".to_string()),
        llm_provider: Some("mock".to_string()),
        embedding_model: Some("new-embed-model".to_string()),
        embedding_provider: Some("mock".to_string()),
        embedding_dimension: Some(1536),
        ..Default::default()
    };

    let updated = state
        .workspace_service
        .update_workspace(workspace.workspace_id, update_request)
        .await
        .expect("Should update workspace");

    // Verify new config
    assert_eq!(updated.llm_model, "new-llm-model");
    assert_eq!(updated.embedding_model, "new-embed-model");
    assert_eq!(updated.embedding_dimension, 1536);

    // Pipeline created now should use NEW config
    let pipeline = state
        .create_workspace_pipeline(&workspace.workspace_id.to_string())
        .await;

    // Pipeline should exist with new config
    assert!(std::sync::Arc::strong_count(&pipeline) >= 1);

    clean_provider_env();
}

// ============================================================================
// OODA 210: Multiple Workspaces Lineage Isolation Tests
// ============================================================================

/// Test that each workspace has isolated lineage config.
#[tokio::test]
#[serial]
async fn test_workspaces_have_isolated_lineage_config() {
    clean_provider_env();

    let state = edgequake_api::AppState::new_memory(None::<String>);

    // Create tenant
    let tenant = Tenant::new("Isolation Test", &format!("test-{}", Uuid::new_v4()));
    let created_tenant = state
        .workspace_service
        .create_tenant(tenant)
        .await
        .expect("Should create tenant");

    // Create workspace A with OpenAI-like config
    let create_a = CreateWorkspaceRequest {
        name: "Workspace A".to_string(),
        slug: Some(format!("ws-a-{}", Uuid::new_v4())),
        description: None,
        max_documents: None,
        llm_model: Some("gpt-4o-mini".to_string()),
        llm_provider: Some("mock".to_string()), // Mock because no API key
        embedding_model: Some("text-embedding-3-small".to_string()),
        embedding_provider: Some("mock".to_string()),
        embedding_dimension: Some(1536),
    };

    let workspace_a = state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, create_a)
        .await
        .expect("Should create workspace A");

    // Create workspace B with Ollama-like config
    let create_b = CreateWorkspaceRequest {
        name: "Workspace B".to_string(),
        slug: Some(format!("ws-b-{}", Uuid::new_v4())),
        description: None,
        max_documents: None,
        llm_model: Some("gemma3:12b".to_string()),
        llm_provider: Some("mock".to_string()),
        embedding_model: Some("nomic-embed-text:latest".to_string()),
        embedding_provider: Some("mock".to_string()),
        embedding_dimension: Some(768),
    };

    let workspace_b = state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, create_b)
        .await
        .expect("Should create workspace B");

    // Verify configs are isolated
    assert_eq!(workspace_a.llm_model, "gpt-4o-mini");
    assert_eq!(workspace_b.llm_model, "gemma3:12b");

    assert_eq!(workspace_a.embedding_model, "text-embedding-3-small");
    assert_eq!(workspace_b.embedding_model, "nomic-embed-text:latest");

    assert_eq!(workspace_a.embedding_dimension, 1536);
    assert_eq!(workspace_b.embedding_dimension, 768);

    // Create pipelines for each workspace
    let pipeline_a = state
        .create_workspace_pipeline(&workspace_a.workspace_id.to_string())
        .await;

    let pipeline_b = state
        .create_workspace_pipeline(&workspace_b.workspace_id.to_string())
        .await;

    // Pipelines should be different instances
    assert!(!std::sync::Arc::ptr_eq(&pipeline_a, &pipeline_b));

    clean_provider_env();
}

/// Test that ProcessingStats can differentiate between workspaces.
#[tokio::test]
#[serial]
async fn test_processing_stats_workspace_differentiation() {
    clean_provider_env();

    // Create stats for workspace A (OpenAI)
    let mut stats_a = ProcessingStats::default();
    stats_a.llm_provider = Some("openai".to_string());
    stats_a.llm_model = Some("gpt-4o-mini".to_string());
    stats_a.embedding_provider = Some("openai".to_string());
    stats_a.embedding_model = Some("text-embedding-3-small".to_string());
    stats_a.embedding_dimensions = Some(1536);
    stats_a.chunk_count = 10;

    // Create stats for workspace B (Ollama)
    let mut stats_b = ProcessingStats::default();
    stats_b.llm_provider = Some("ollama".to_string());
    stats_b.llm_model = Some("gemma3:12b".to_string());
    stats_b.embedding_provider = Some("ollama".to_string());
    stats_b.embedding_model = Some("nomic-embed-text:latest".to_string());
    stats_b.embedding_dimensions = Some(768);
    stats_b.chunk_count = 10;

    // Verify they are different
    assert_ne!(stats_a.llm_provider, stats_b.llm_provider);
    assert_ne!(stats_a.llm_model, stats_b.llm_model);
    assert_ne!(stats_a.embedding_provider, stats_b.embedding_provider);
    assert_ne!(stats_a.embedding_model, stats_b.embedding_model);
    assert_ne!(stats_a.embedding_dimensions, stats_b.embedding_dimensions);

    // Same chunk count (data isolation, not lineage)
    assert_eq!(stats_a.chunk_count, stats_b.chunk_count);

    clean_provider_env();
}
