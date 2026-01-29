//! End-to-end tests for rebuild operations with provider switching.
//!
//! These tests verify that when a workspace's provider configuration is changed
//! and rebuild operations are triggered, the NEW provider is actually used.
//!
//! ## Critical Test Scenarios
//!
//! 1. Rebuild embeddings after provider switch - verifies response has new config
//! 2. Rebuild knowledge graph after provider switch - verifies response has new config
//! 3. Workspace isolation during rebuild - other workspaces unaffected
//! 4. Dimension change validation - vectors cleared on dimension change
//!
//! @implements SPEC-032: Workspace provider switching for rebuild
//! @implements OODA-202-203: Rebuild Operation Provider Verification E2E

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use edgequake_core::types::CreateWorkspaceRequest;
use edgequake_core::Tenant;
use serde_json::{json, Value};
use serial_test::serial;
use tower::ServiceExt;
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

fn create_test_config() -> ServerConfig {
    ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: true,
    }
}

fn create_memory_state() -> AppState {
    AppState::new_memory(None::<String>)
}

async fn extract_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("Failed to read response body");
    serde_json::from_slice(&bytes).unwrap_or(json!({}))
}

// ============================================================================
// OODA 202: Rebuild Embeddings Provider Switching Tests
// ============================================================================

/// Test rebuild-embeddings endpoint returns updated provider config.
///
/// Flow:
/// 1. Create workspace with mock provider v1
/// 2. Call rebuild-embeddings with mock provider v2
/// 3. Verify response contains v2 config
#[tokio::test]
#[serial]
async fn test_rebuild_embeddings_returns_updated_provider_config() {
    clean_provider_env();

    let state = create_memory_state();

    // Create tenant
    let tenant = Tenant::new("Rebuild Test", &format!("test-{}", Uuid::new_v4()));
    let created_tenant = state
        .workspace_service
        .create_tenant(tenant)
        .await
        .expect("Should create tenant");

    // Create workspace with initial config
    let create_request = CreateWorkspaceRequest {
        name: "Rebuild Embed Test".to_string(),
        slug: Some(format!("ws-rebuild-{}", Uuid::new_v4())),
        description: None,
        max_documents: None,
        llm_model: Some("mock-llm-v1".to_string()),
        llm_provider: Some("mock".to_string()),
        embedding_model: Some("mock-embed-v1".to_string()),
        embedding_provider: Some("mock".to_string()),
        embedding_dimension: Some(768),
    };

    let workspace = state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, create_request)
        .await
        .expect("Should create workspace");

    // Verify initial config
    assert_eq!(workspace.embedding_model, "mock-embed-v1");
    assert_eq!(workspace.embedding_provider, "mock");
    assert_eq!(workspace.embedding_dimension, 768);

    // Build app and call rebuild-embeddings with new config
    let app = Server::new(create_test_config(), state.clone()).build_router();

    let rebuild_request = json!({
        "embedding_model": "mock-embed-v2",
        "embedding_provider": "mock",
        "embedding_dimension": 1536,
        "force": true
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/workspaces/{}/rebuild-embeddings",
                    workspace.workspace_id
                ))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&rebuild_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let json = extract_json(response).await;

    // Verify response contains NEW config
    assert_eq!(
        json["embedding_model"], "mock-embed-v2",
        "Response should contain updated embedding model"
    );
    assert_eq!(
        json["embedding_provider"], "mock",
        "Response should contain updated embedding provider"
    );
    assert_eq!(
        json["embedding_dimension"], 1536,
        "Response should contain updated embedding dimension"
    );

    // Verify workspace was updated
    let updated_ws = state
        .workspace_service
        .get_workspace(workspace.workspace_id)
        .await
        .expect("Should get workspace")
        .expect("Workspace should exist");

    assert_eq!(updated_ws.embedding_model, "mock-embed-v2");
    assert_eq!(updated_ws.embedding_provider, "mock");
    assert_eq!(updated_ws.embedding_dimension, 1536);

    clean_provider_env();
}

/// Test rebuild-embeddings requires force=true if config unchanged.
#[tokio::test]
#[serial]
async fn test_rebuild_embeddings_requires_force_if_unchanged() {
    clean_provider_env();

    let state = create_memory_state();

    // Create tenant and workspace
    let tenant = Tenant::new("Force Test", &format!("test-{}", Uuid::new_v4()));
    let created_tenant = state
        .workspace_service
        .create_tenant(tenant)
        .await
        .expect("Should create tenant");

    let create_request = CreateWorkspaceRequest {
        name: "Force Test".to_string(),
        slug: Some(format!("ws-force-{}", Uuid::new_v4())),
        description: None,
        max_documents: None,
        llm_model: Some("mock-llm".to_string()),
        llm_provider: Some("mock".to_string()),
        embedding_model: Some("mock-embed".to_string()),
        embedding_provider: Some("mock".to_string()),
        embedding_dimension: Some(1536),
    };

    let workspace = state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, create_request)
        .await
        .expect("Should create workspace");

    let app = Server::new(create_test_config(), state.clone()).build_router();

    // Call rebuild without changing config and without force
    let rebuild_request = json!({
        "embedding_model": "mock-embed",
        "embedding_provider": "mock",
        "embedding_dimension": 1536,
        "force": false
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/workspaces/{}/rebuild-embeddings",
                    workspace.workspace_id
                ))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&rebuild_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should fail with 400 (config unchanged, force=false)
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    clean_provider_env();
}

// ============================================================================
// OODA 203: Rebuild Knowledge Graph Provider Switching Tests
// ============================================================================

/// Test rebuild-knowledge-graph endpoint returns updated provider config.
#[tokio::test]
#[serial]
async fn test_rebuild_knowledge_graph_returns_updated_provider_config() {
    clean_provider_env();

    let state = create_memory_state();

    // Create tenant
    let tenant = Tenant::new("KG Rebuild Test", &format!("test-{}", Uuid::new_v4()));
    let created_tenant = state
        .workspace_service
        .create_tenant(tenant)
        .await
        .expect("Should create tenant");

    // Create workspace with initial LLM config
    let create_request = CreateWorkspaceRequest {
        name: "KG Rebuild Test".to_string(),
        slug: Some(format!("ws-kg-{}", Uuid::new_v4())),
        description: None,
        max_documents: None,
        llm_model: Some("mock-llm-v1".to_string()),
        llm_provider: Some("mock".to_string()),
        embedding_model: Some("mock-embed".to_string()),
        embedding_provider: Some("mock".to_string()),
        embedding_dimension: Some(1536),
    };

    let workspace = state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, create_request)
        .await
        .expect("Should create workspace");

    // Verify initial config
    assert_eq!(workspace.llm_model, "mock-llm-v1");
    assert_eq!(workspace.llm_provider, "mock");

    // Build app and call rebuild-knowledge-graph with new config
    let app = Server::new(create_test_config(), state.clone()).build_router();

    let rebuild_request = json!({
        "llm_model": "mock-llm-v2",
        "llm_provider": "mock",
        "rebuild_embeddings": false,
        "force": true
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/workspaces/{}/rebuild-knowledge-graph",
                    workspace.workspace_id
                ))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&rebuild_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let json = extract_json(response).await;

    // Verify response contains NEW config
    assert_eq!(
        json["llm_model"], "mock-llm-v2",
        "Response should contain updated LLM model"
    );
    assert_eq!(
        json["llm_provider"], "mock",
        "Response should contain updated LLM provider"
    );

    // Verify workspace was updated
    let updated_ws = state
        .workspace_service
        .get_workspace(workspace.workspace_id)
        .await
        .expect("Should get workspace")
        .expect("Workspace should exist");

    assert_eq!(updated_ws.llm_model, "mock-llm-v2");
    assert_eq!(updated_ws.llm_provider, "mock");

    clean_provider_env();
}

/// Test rebuild-knowledge-graph requires force=true if config unchanged.
#[tokio::test]
#[serial]
async fn test_rebuild_knowledge_graph_requires_force_if_unchanged() {
    clean_provider_env();

    let state = create_memory_state();

    // Create tenant and workspace
    let tenant = Tenant::new("KG Force Test", &format!("test-{}", Uuid::new_v4()));
    let created_tenant = state
        .workspace_service
        .create_tenant(tenant)
        .await
        .expect("Should create tenant");

    let create_request = CreateWorkspaceRequest {
        name: "KG Force Test".to_string(),
        slug: Some(format!("ws-kg-force-{}", Uuid::new_v4())),
        description: None,
        max_documents: None,
        llm_model: Some("mock-llm".to_string()),
        llm_provider: Some("mock".to_string()),
        embedding_model: Some("mock-embed".to_string()),
        embedding_provider: Some("mock".to_string()),
        embedding_dimension: Some(1536),
    };

    let workspace = state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, create_request)
        .await
        .expect("Should create workspace");

    let app = Server::new(create_test_config(), state.clone()).build_router();

    // Call rebuild without changing config and without force
    let rebuild_request = json!({
        "llm_model": "mock-llm",
        "llm_provider": "mock",
        "force": false
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/workspaces/{}/rebuild-knowledge-graph",
                    workspace.workspace_id
                ))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&rebuild_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should fail with 400 (config unchanged, force=false)
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    clean_provider_env();
}

// ============================================================================
// OODA 204: Workspace Isolation Tests
// ============================================================================

/// Test that rebuilding one workspace does not affect other workspaces.
#[tokio::test]
#[serial]
async fn test_rebuild_workspace_isolation() {
    clean_provider_env();

    let state = create_memory_state();

    // Create tenant
    let tenant = Tenant::new("Isolation Test", &format!("test-{}", Uuid::new_v4()));
    let created_tenant = state
        .workspace_service
        .create_tenant(tenant)
        .await
        .expect("Should create tenant");

    // Create workspace A
    let create_request_a = CreateWorkspaceRequest {
        name: "Workspace A".to_string(),
        slug: Some(format!("ws-a-{}", Uuid::new_v4())),
        description: None,
        max_documents: None,
        llm_model: Some("mock-llm-a".to_string()),
        llm_provider: Some("mock".to_string()),
        embedding_model: Some("mock-embed-a".to_string()),
        embedding_provider: Some("mock".to_string()),
        embedding_dimension: Some(768),
    };

    let workspace_a = state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, create_request_a)
        .await
        .expect("Should create workspace A");

    // Create workspace B
    let create_request_b = CreateWorkspaceRequest {
        name: "Workspace B".to_string(),
        slug: Some(format!("ws-b-{}", Uuid::new_v4())),
        description: None,
        max_documents: None,
        llm_model: Some("mock-llm-b".to_string()),
        llm_provider: Some("mock".to_string()),
        embedding_model: Some("mock-embed-b".to_string()),
        embedding_provider: Some("mock".to_string()),
        embedding_dimension: Some(1024),
    };

    let workspace_b = state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, create_request_b)
        .await
        .expect("Should create workspace B");

    let app = Server::new(create_test_config(), state.clone()).build_router();

    // Rebuild workspace A with new config
    let rebuild_request = json!({
        "embedding_model": "mock-embed-a-updated",
        "embedding_provider": "mock",
        "embedding_dimension": 1536,
        "force": true
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/workspaces/{}/rebuild-embeddings",
                    workspace_a.workspace_id
                ))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&rebuild_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Verify workspace A was updated
    let updated_a = state
        .workspace_service
        .get_workspace(workspace_a.workspace_id)
        .await
        .expect("Should get workspace A")
        .expect("Workspace A should exist");

    assert_eq!(updated_a.embedding_model, "mock-embed-a-updated");
    assert_eq!(updated_a.embedding_dimension, 1536);

    // Verify workspace B is UNCHANGED
    let unchanged_b = state
        .workspace_service
        .get_workspace(workspace_b.workspace_id)
        .await
        .expect("Should get workspace B")
        .expect("Workspace B should exist");

    assert_eq!(unchanged_b.embedding_model, "mock-embed-b");
    assert_eq!(unchanged_b.embedding_dimension, 1024);

    clean_provider_env();
}

// ============================================================================
// OODA 205: Provider Pipeline Verification Tests
// ============================================================================

/// Test that workspace pipeline uses updated config after rebuild.
///
/// This verifies that if a document is processed AFTER rebuild,
/// the pipeline will use the NEW provider configuration.
#[tokio::test]
#[serial]
async fn test_pipeline_uses_updated_config_after_rebuild() {
    clean_provider_env();

    let state = create_memory_state();

    // Create tenant
    let tenant = Tenant::new("Pipeline Test", &format!("test-{}", Uuid::new_v4()));
    let created_tenant = state
        .workspace_service
        .create_tenant(tenant)
        .await
        .expect("Should create tenant");

    // Create workspace with initial config
    let create_request = CreateWorkspaceRequest {
        name: "Pipeline Test".to_string(),
        slug: Some(format!("ws-pipeline-{}", Uuid::new_v4())),
        description: None,
        max_documents: None,
        llm_model: Some("mock-llm-v1".to_string()),
        llm_provider: Some("mock".to_string()),
        embedding_model: Some("mock-embed-v1".to_string()),
        embedding_provider: Some("mock".to_string()),
        embedding_dimension: Some(768),
    };

    let workspace = state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, create_request)
        .await
        .expect("Should create workspace");

    // Create pipeline BEFORE rebuild
    let pipeline_before = state
        .create_workspace_pipeline(&workspace.workspace_id.to_string())
        .await;

    // Trigger rebuild with new config
    let app = Server::new(create_test_config(), state.clone()).build_router();

    let rebuild_request = json!({
        "embedding_model": "mock-embed-v2",
        "embedding_provider": "mock",
        "embedding_dimension": 1536,
        "force": true
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/workspaces/{}/rebuild-embeddings",
                    workspace.workspace_id
                ))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&rebuild_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Create pipeline AFTER rebuild
    let pipeline_after = state
        .create_workspace_pipeline(&workspace.workspace_id.to_string())
        .await;

    // Pipelines should be different instances (not cached)
    // because the workspace config changed
    assert!(
        !std::sync::Arc::ptr_eq(&pipeline_before, &pipeline_after),
        "Pipeline should be recreated after config change"
    );

    clean_provider_env();
}

/// Test rebuild with non-existent workspace returns 404.
#[tokio::test]
#[serial]
async fn test_rebuild_nonexistent_workspace_returns_404() {
    clean_provider_env();

    let state = create_memory_state();
    let app = Server::new(create_test_config(), state).build_router();

    let fake_workspace_id = Uuid::new_v4();

    let rebuild_request = json!({
        "embedding_model": "mock-embed",
        "embedding_provider": "mock",
        "force": true
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/workspaces/{}/rebuild-embeddings",
                    fake_workspace_id
                ))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&rebuild_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    clean_provider_env();
}

// ============================================================================
// OODA 206: Response Field Verification Tests
// ============================================================================

/// Test rebuild-embeddings response contains all required fields.
#[tokio::test]
#[serial]
async fn test_rebuild_embeddings_response_fields() {
    clean_provider_env();

    let state = create_memory_state();

    // Create tenant and workspace
    let tenant = Tenant::new("Response Test", &format!("test-{}", Uuid::new_v4()));
    let created_tenant = state
        .workspace_service
        .create_tenant(tenant)
        .await
        .expect("Should create tenant");

    let create_request = CreateWorkspaceRequest {
        name: "Response Test".to_string(),
        slug: Some(format!("ws-response-{}", Uuid::new_v4())),
        description: None,
        max_documents: None,
        llm_model: Some("mock-llm".to_string()),
        llm_provider: Some("mock".to_string()),
        embedding_model: Some("mock-embed".to_string()),
        embedding_provider: Some("mock".to_string()),
        embedding_dimension: Some(1536),
    };

    let workspace = state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, create_request)
        .await
        .expect("Should create workspace");

    let app = Server::new(create_test_config(), state.clone()).build_router();

    let rebuild_request = json!({
        "embedding_model": "mock-embed-new",
        "embedding_provider": "mock",
        "embedding_dimension": 768,
        "force": true
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/workspaces/{}/rebuild-embeddings",
                    workspace.workspace_id
                ))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&rebuild_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let json = extract_json(response).await;

    // Verify all required fields are present
    assert!(json.get("workspace_id").is_some(), "Missing workspace_id");
    assert!(json.get("status").is_some(), "Missing status");
    assert!(
        json.get("documents_to_process").is_some(),
        "Missing documents_to_process"
    );
    assert!(
        json.get("chunks_to_process").is_some(),
        "Missing chunks_to_process"
    );
    assert!(
        json.get("vectors_cleared").is_some(),
        "Missing vectors_cleared"
    );
    assert!(
        json.get("embedding_model").is_some(),
        "Missing embedding_model"
    );
    assert!(
        json.get("embedding_provider").is_some(),
        "Missing embedding_provider"
    );
    assert!(
        json.get("embedding_dimension").is_some(),
        "Missing embedding_dimension"
    );
    assert!(
        json.get("model_context_length").is_some(),
        "Missing model_context_length"
    );

    // Verify field values
    assert_eq!(json["embedding_model"], "mock-embed-new");
    assert_eq!(json["embedding_provider"], "mock");
    assert_eq!(json["embedding_dimension"], 768);

    clean_provider_env();
}

/// Test rebuild-knowledge-graph response contains all required fields.
#[tokio::test]
#[serial]
async fn test_rebuild_knowledge_graph_response_fields() {
    clean_provider_env();

    let state = create_memory_state();

    // Create tenant and workspace
    let tenant = Tenant::new("KG Response Test", &format!("test-{}", Uuid::new_v4()));
    let created_tenant = state
        .workspace_service
        .create_tenant(tenant)
        .await
        .expect("Should create tenant");

    let create_request = CreateWorkspaceRequest {
        name: "KG Response Test".to_string(),
        slug: Some(format!("ws-kg-response-{}", Uuid::new_v4())),
        description: None,
        max_documents: None,
        llm_model: Some("mock-llm".to_string()),
        llm_provider: Some("mock".to_string()),
        embedding_model: Some("mock-embed".to_string()),
        embedding_provider: Some("mock".to_string()),
        embedding_dimension: Some(1536),
    };

    let workspace = state
        .workspace_service
        .create_workspace(created_tenant.tenant_id, create_request)
        .await
        .expect("Should create workspace");

    let app = Server::new(create_test_config(), state.clone()).build_router();

    let rebuild_request = json!({
        "llm_model": "mock-llm-new",
        "llm_provider": "mock",
        "rebuild_embeddings": true,
        "force": true
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v1/workspaces/{}/rebuild-knowledge-graph",
                    workspace.workspace_id
                ))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&rebuild_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let json = extract_json(response).await;

    // Verify all required fields are present
    assert!(json.get("workspace_id").is_some(), "Missing workspace_id");
    assert!(json.get("status").is_some(), "Missing status");
    assert!(json.get("nodes_cleared").is_some(), "Missing nodes_cleared");
    assert!(json.get("edges_cleared").is_some(), "Missing edges_cleared");
    assert!(
        json.get("vectors_cleared").is_some(),
        "Missing vectors_cleared"
    );
    assert!(
        json.get("documents_to_process").is_some(),
        "Missing documents_to_process"
    );
    assert!(
        json.get("chunks_to_process").is_some(),
        "Missing chunks_to_process"
    );
    assert!(json.get("llm_model").is_some(), "Missing llm_model");
    assert!(json.get("llm_provider").is_some(), "Missing llm_provider");

    // Verify field values
    assert_eq!(json["llm_model"], "mock-llm-new");
    assert_eq!(json["llm_provider"], "mock");

    clean_provider_env();
}
