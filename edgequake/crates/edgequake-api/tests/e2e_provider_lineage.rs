//! E2E Tests for Provider Lineage Tracking
//!
//! These tests verify that when documents are processed:
//! 1. The provider lineage (which provider was used) is correctly captured
//! 2. The lineage is stored in document metadata
//! 3. The lineage can be retrieved via API
//!
//! @implements SPEC-032: Provider lineage tracking
//! @implements OODA-198: Provider lineage implementation

use edgequake_core::types::CreateWorkspaceRequest;
use edgequake_core::Tenant;
use edgequake_pipeline::pipeline::ProcessingStats;
use serial_test::serial;
use uuid::Uuid;

// ============================================================================
// Test Fixtures
// ============================================================================

/// Create a test workspace with specified provider configuration.
async fn create_test_workspace(
    state: &edgequake_api::AppState,
    name: &str,
    llm_provider: &str,
    llm_model: &str,
    embedding_provider: &str,
    embedding_model: &str,
    embedding_dimension: usize,
) -> edgequake_core::Workspace {
    let tenant = Tenant::new(
        &format!("Lineage Test Tenant {}", name),
        &format!("lineage-{}", Uuid::new_v4()),
    );
    let created_tenant = state
        .workspace_service
        .create_tenant(tenant)
        .await
        .expect("Should create tenant");

    let request = CreateWorkspaceRequest {
        name: name.to_string(),
        slug: Some(format!("lineage-{}", Uuid::new_v4())),
        description: Some("Provider lineage test workspace".to_string()),
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
// ProcessingStats Provider Field Tests
// ============================================================================

/// Test that ProcessingStats correctly stores provider fields.
#[test]
fn test_processing_stats_has_provider_fields() {
    let mut stats = ProcessingStats::default();

    // Set provider fields
    stats.llm_provider = Some("openai".to_string());
    stats.llm_model = Some("gpt-4o-mini".to_string());
    stats.embedding_provider = Some("openai".to_string());
    stats.embedding_model = Some("text-embedding-3-small".to_string());
    stats.embedding_dimensions = Some(1536);

    // Verify they're stored
    assert_eq!(stats.llm_provider, Some("openai".to_string()));
    assert_eq!(stats.llm_model, Some("gpt-4o-mini".to_string()));
    assert_eq!(stats.embedding_provider, Some("openai".to_string()));
    assert_eq!(
        stats.embedding_model,
        Some("text-embedding-3-small".to_string())
    );
    assert_eq!(stats.embedding_dimensions, Some(1536));
}

/// Test that ProcessingStats serializes provider fields correctly.
#[test]
fn test_processing_stats_serialization() {
    let mut stats = ProcessingStats::default();
    stats.llm_provider = Some("ollama".to_string());
    stats.llm_model = Some("gemma3:12b".to_string());
    stats.embedding_provider = Some("ollama".to_string());
    stats.embedding_model = Some("nomic-embed-text".to_string());
    stats.embedding_dimensions = Some(768);
    stats.chunk_count = 5;
    stats.entity_count = 10;

    // Serialize to JSON
    let json = serde_json::to_value(&stats).expect("Should serialize");

    // Verify provider fields are present
    assert_eq!(
        json.get("llm_provider").and_then(|v| v.as_str()),
        Some("ollama"),
        "llm_provider should serialize"
    );
    assert_eq!(
        json.get("llm_model").and_then(|v| v.as_str()),
        Some("gemma3:12b"),
        "llm_model should serialize"
    );
    assert_eq!(
        json.get("embedding_provider").and_then(|v| v.as_str()),
        Some("ollama"),
        "embedding_provider should serialize"
    );
    assert_eq!(
        json.get("embedding_model").and_then(|v| v.as_str()),
        Some("nomic-embed-text"),
        "embedding_model should serialize"
    );
}

/// Test that ProcessingStats deserializes provider fields correctly.
#[test]
fn test_processing_stats_deserialization() {
    let json = serde_json::json!({
        "chunk_count": 3,
        "entity_count": 7,
        "relationship_count": 2,
        "processing_time_ms": 1500,
        "llm_calls": 1,
        "total_tokens": 500,
        "llm_provider": "lmstudio",
        "llm_model": "gemma-3n",
        "embedding_provider": "lmstudio",
        "embedding_model": "text-embed-local",
        "embedding_dimensions": 512
    });

    let stats: ProcessingStats = serde_json::from_value(json).expect("Should deserialize");

    assert_eq!(stats.llm_provider, Some("lmstudio".to_string()));
    assert_eq!(stats.llm_model, Some("gemma-3n".to_string()));
    assert_eq!(stats.embedding_provider, Some("lmstudio".to_string()));
    assert_eq!(stats.embedding_model, Some("text-embed-local".to_string()));
    assert_eq!(stats.embedding_dimensions, Some(512));
}

/// Test that old JSON without provider fields still deserializes (backward compat).
#[test]
fn test_processing_stats_backward_compatibility() {
    // Legacy JSON without provider fields
    let json = serde_json::json!({
        "chunk_count": 10,
        "entity_count": 20,
        "relationship_count": 5,
        "processing_time_ms": 2000,
        "llm_calls": 3,
        "total_tokens": 1000
    });

    let stats: ProcessingStats =
        serde_json::from_value(json).expect("Should deserialize legacy JSON");

    // Provider fields should be None
    assert!(
        stats.llm_provider.is_none(),
        "llm_provider should be None for legacy data"
    );
    assert!(
        stats.embedding_provider.is_none(),
        "embedding_provider should be None for legacy data"
    );

    // Other fields should be present
    assert_eq!(stats.chunk_count, 10);
    assert_eq!(stats.entity_count, 20);
}

// ============================================================================
// DocumentLineage Provider Field Tests
// ============================================================================

/// Test that DocumentLineage correctly stores provider fields.
#[test]
fn test_document_lineage_has_provider_fields() {
    use edgequake_pipeline::lineage::DocumentLineage;

    let mut lineage = DocumentLineage::new("doc-123", "test.txt", "job-456");

    // Initially should be None
    assert!(lineage.extraction_provider.is_none());
    assert!(lineage.extraction_model.is_none());

    // Set provider lineage
    lineage.set_provider_lineage(
        "openai",
        "gpt-4o-mini",
        "openai",
        "text-embedding-3-small",
        1536,
    );

    // Verify they're stored
    assert_eq!(lineage.extraction_provider, Some("openai".to_string()));
    assert_eq!(lineage.extraction_model, Some("gpt-4o-mini".to_string()));
    assert_eq!(lineage.embedding_provider, Some("openai".to_string()));
    assert_eq!(
        lineage.embedding_model,
        Some("text-embedding-3-small".to_string())
    );
    assert_eq!(lineage.embedding_dimension, Some(1536));
}

/// Test DocumentLineage serialization includes provider fields.
#[test]
fn test_document_lineage_serialization() {
    use edgequake_pipeline::lineage::DocumentLineage;

    let mut lineage = DocumentLineage::new("doc-789", "document.pdf", "job-abc");
    lineage.set_provider_lineage("mock", "mock-llm", "mock", "mock-embed", 768);

    let json = serde_json::to_value(&lineage).expect("Should serialize");

    assert_eq!(
        json.get("extraction_provider").and_then(|v| v.as_str()),
        Some("mock")
    );
    assert_eq!(
        json.get("embedding_dimension").and_then(|v| v.as_u64()),
        Some(768)
    );
}

// ============================================================================
// ProviderLineage Struct Tests
// ============================================================================

/// Test the ProviderLineage struct in processor module.
#[test]
fn test_provider_lineage_default() {
    use edgequake_api::processor::ProviderLineage;

    let lineage = ProviderLineage::default();

    // Default should be empty strings
    assert!(lineage.extraction_provider.is_empty());
    assert!(lineage.extraction_model.is_empty());
    assert!(lineage.embedding_provider.is_empty());
    assert!(lineage.embedding_model.is_empty());
    assert_eq!(lineage.embedding_dimension, 0);
}

/// Test ProviderLineage with values.
#[test]
fn test_provider_lineage_with_values() {
    use edgequake_api::processor::ProviderLineage;

    let lineage = ProviderLineage {
        extraction_provider: "openai".to_string(),
        extraction_model: "gpt-4o-mini".to_string(),
        embedding_provider: "openai".to_string(),
        embedding_model: "text-embedding-3-small".to_string(),
        embedding_dimension: 1536,
    };

    assert_eq!(lineage.extraction_provider, "openai");
    assert_eq!(lineage.extraction_model, "gpt-4o-mini");
    assert_eq!(lineage.embedding_provider, "openai");
    assert_eq!(lineage.embedding_model, "text-embedding-3-small");
    assert_eq!(lineage.embedding_dimension, 1536);
}

// ============================================================================
// Integration: Workspace Provider Lineage Retrieval
// ============================================================================

/// Test that workspace configuration provides correct provider lineage.
#[tokio::test]
#[serial]
async fn test_workspace_provides_correct_lineage() {
    // Clean environment
    std::env::remove_var("OPENAI_API_KEY");

    let state = edgequake_api::AppState::new_memory(None::<String>);

    // Create workspace with specific provider config
    let workspace = create_test_workspace(
        &state,
        "Lineage Test Workspace",
        "mock",
        "mock-extractor",
        "mock",
        "mock-embedder",
        768,
    )
    .await;

    // Verify workspace has correct provider config
    assert_eq!(workspace.llm_provider, "mock");
    assert_eq!(workspace.llm_model, "mock-extractor");
    assert_eq!(workspace.embedding_provider, "mock");
    assert_eq!(workspace.embedding_model, "mock-embedder");
    assert_eq!(workspace.embedding_dimension, 768);

    // Retrieve workspace and verify config persists
    let retrieved = state
        .workspace_service
        .get_workspace(workspace.workspace_id)
        .await
        .expect("Should get workspace")
        .expect("Workspace should exist");

    assert_eq!(retrieved.llm_provider, "mock");
    assert_eq!(retrieved.embedding_provider, "mock");
}

/// Test that different workspaces have isolated provider lineage.
#[tokio::test]
#[serial]
async fn test_workspace_lineage_isolation() {
    std::env::remove_var("OPENAI_API_KEY");

    let state = edgequake_api::AppState::new_memory(None::<String>);

    // Create two workspaces with different providers
    let ws1 = create_test_workspace(
        &state,
        "Workspace A",
        "mock",
        "model-a",
        "mock",
        "embed-a",
        512,
    )
    .await;

    let ws2 = create_test_workspace(
        &state,
        "Workspace B",
        "mock",
        "model-b",
        "mock",
        "embed-b",
        1024,
    )
    .await;

    // Verify isolation
    assert_eq!(ws1.llm_model, "model-a");
    assert_eq!(ws1.embedding_model, "embed-a");
    assert_eq!(ws1.embedding_dimension, 512);

    assert_eq!(ws2.llm_model, "model-b");
    assert_eq!(ws2.embedding_model, "embed-b");
    assert_eq!(ws2.embedding_dimension, 1024);
}
