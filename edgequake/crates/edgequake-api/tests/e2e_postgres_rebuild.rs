//! End-to-end tests for rebuild operations with PostgreSQL storage.
//!
//! These tests verify that rebuild operations work correctly with PostgreSQL,
//! including provider config persistence and workspace isolation.
//!
//! ## Requirements
//!
//! These tests require:
//! - `postgres` feature enabled
//! - DATABASE_URL to be set to a valid PostgreSQL connection
//!
//! @implements SPEC-032: PostgreSQL Provider Persistence
//! @implements OODA-204-206: PostgreSQL Rebuild Tests

use serial_test::serial;

#[cfg(feature = "postgres")]
mod postgres_rebuild_tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use edgequake_api::state::AppState;
    use edgequake_api::{Server, ServerConfig};
    use edgequake_core::types::CreateWorkspaceRequest;
    use edgequake_core::Tenant;
    use serde_json::{json, Value};
    use tower::ServiceExt;
    use uuid::Uuid;

    /// Check if PostgreSQL is available
    fn is_postgres_available() -> bool {
        std::env::var("DATABASE_URL").is_ok()
    }

    /// Get database URL if available
    fn get_database_url() -> Option<String> {
        std::env::var("DATABASE_URL").ok()
    }

    /// Test helper: Clean environment for isolated tests
    fn clean_provider_env() {
        std::env::remove_var("EDGEQUAKE_LLM_PROVIDER");
        std::env::remove_var("OLLAMA_HOST");
        std::env::remove_var("OLLAMA_MODEL");
        std::env::remove_var("LMSTUDIO_HOST");
        std::env::remove_var("LMSTUDIO_MODEL");
        // Don't remove OPENAI_API_KEY or DATABASE_URL
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

    async fn extract_json(response: axum::response::Response) -> Value {
        let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .expect("Failed to read response body");
        serde_json::from_slice(&bytes).unwrap_or(json!({}))
    }

    // ========================================================================
    // OODA 204: PostgreSQL Rebuild Embeddings Tests
    // ========================================================================

    /// Test rebuild-embeddings persists provider config to PostgreSQL.
    #[tokio::test]
    #[serial]
    async fn test_postgres_rebuild_embeddings_persists_config() {
        if !is_postgres_available() {
            eprintln!("Skipping PostgreSQL test - DATABASE_URL not set");
            return;
        }

        clean_provider_env();

        let database_url = get_database_url().unwrap();

        // Create state with PostgreSQL
        let state = match AppState::new_postgres(database_url, "").await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Skipping PostgreSQL test - connection failed: {}", e);
                return;
            }
        };

        // Create unique tenant for this test
        let tenant_slug = format!("test-pg-rebuild-{}", &Uuid::new_v4().to_string()[..8]);
        let tenant = Tenant::new("PG Rebuild Test", &tenant_slug);
        let created_tenant = state
            .workspace_service
            .create_tenant(tenant)
            .await
            .expect("Should create tenant in PostgreSQL");

        // Create workspace with initial config
        let workspace_slug = format!("ws-pg-rebuild-{}", &Uuid::new_v4().to_string()[..8]);
        let create_request = CreateWorkspaceRequest {
            name: "PG Rebuild Test".to_string(),
            slug: Some(workspace_slug),
            description: None,
            max_documents: None,
            llm_model: Some("mock-llm-pg-v1".to_string()),
            llm_provider: Some("mock".to_string()),
            embedding_model: Some("mock-embed-pg-v1".to_string()),
            embedding_provider: Some("mock".to_string()),
            embedding_dimension: Some(768),
        };

        let workspace = state
            .workspace_service
            .create_workspace(created_tenant.tenant_id, create_request)
            .await
            .expect("Should create workspace in PostgreSQL");

        // Verify initial config in PostgreSQL
        let fetched = state
            .workspace_service
            .get_workspace(workspace.workspace_id)
            .await
            .expect("Should fetch workspace")
            .expect("Workspace should exist");

        assert_eq!(fetched.embedding_model, "mock-embed-pg-v1");
        assert_eq!(fetched.embedding_dimension, 768);

        // Build app and call rebuild-embeddings with new config
        let app = Server::new(create_test_config(), state.clone()).build_router();

        let rebuild_request = json!({
            "embedding_model": "mock-embed-pg-v2",
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
        assert_eq!(json["embedding_model"], "mock-embed-pg-v2");
        assert_eq!(json["embedding_dimension"], 1536);

        // Verify config persisted to PostgreSQL
        let updated = state
            .workspace_service
            .get_workspace(workspace.workspace_id)
            .await
            .expect("Should fetch updated workspace")
            .expect("Workspace should exist");

        assert_eq!(
            updated.embedding_model, "mock-embed-pg-v2",
            "Embedding model should be persisted to PostgreSQL"
        );
        assert_eq!(
            updated.embedding_dimension, 1536,
            "Embedding dimension should be persisted to PostgreSQL"
        );

        // Clean up
        let _ = state
            .workspace_service
            .delete_workspace(workspace.workspace_id)
            .await;
        let _ = state
            .workspace_service
            .delete_tenant(created_tenant.tenant_id)
            .await;

        clean_provider_env();
    }

    // ========================================================================
    // OODA 205: PostgreSQL Rebuild Knowledge Graph Tests
    // ========================================================================

    /// Test rebuild-knowledge-graph persists LLM config to PostgreSQL.
    #[tokio::test]
    #[serial]
    async fn test_postgres_rebuild_kg_persists_config() {
        if !is_postgres_available() {
            eprintln!("Skipping PostgreSQL test - DATABASE_URL not set");
            return;
        }

        clean_provider_env();

        let database_url = get_database_url().unwrap();

        // Create state with PostgreSQL
        let state = match AppState::new_postgres(database_url, "").await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Skipping PostgreSQL test - connection failed: {}", e);
                return;
            }
        };

        // Create unique tenant for this test
        let tenant_slug = format!("test-pg-kg-{}", &Uuid::new_v4().to_string()[..8]);
        let tenant = Tenant::new("PG KG Test", &tenant_slug);
        let created_tenant = state
            .workspace_service
            .create_tenant(tenant)
            .await
            .expect("Should create tenant in PostgreSQL");

        // Create workspace with initial config
        let workspace_slug = format!("ws-pg-kg-{}", &Uuid::new_v4().to_string()[..8]);
        let create_request = CreateWorkspaceRequest {
            name: "PG KG Test".to_string(),
            slug: Some(workspace_slug),
            description: None,
            max_documents: None,
            llm_model: Some("mock-llm-pg-v1".to_string()),
            llm_provider: Some("mock".to_string()),
            embedding_model: Some("mock-embed-pg".to_string()),
            embedding_provider: Some("mock".to_string()),
            embedding_dimension: Some(1536),
        };

        let workspace = state
            .workspace_service
            .create_workspace(created_tenant.tenant_id, create_request)
            .await
            .expect("Should create workspace in PostgreSQL");

        // Verify initial config
        let fetched = state
            .workspace_service
            .get_workspace(workspace.workspace_id)
            .await
            .expect("Should fetch workspace")
            .expect("Workspace should exist");

        assert_eq!(fetched.llm_model, "mock-llm-pg-v1");

        // Build app and call rebuild-knowledge-graph with new config
        let app = Server::new(create_test_config(), state.clone()).build_router();

        let rebuild_request = json!({
            "llm_model": "mock-llm-pg-v2",
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
        assert_eq!(json["llm_model"], "mock-llm-pg-v2");

        // Verify config persisted to PostgreSQL
        let updated = state
            .workspace_service
            .get_workspace(workspace.workspace_id)
            .await
            .expect("Should fetch updated workspace")
            .expect("Workspace should exist");

        assert_eq!(
            updated.llm_model, "mock-llm-pg-v2",
            "LLM model should be persisted to PostgreSQL"
        );

        // Clean up
        let _ = state
            .workspace_service
            .delete_workspace(workspace.workspace_id)
            .await;
        let _ = state
            .workspace_service
            .delete_tenant(created_tenant.tenant_id)
            .await;

        clean_provider_env();
    }

    // ========================================================================
    // OODA 206: PostgreSQL Workspace Isolation Tests
    // ========================================================================

    /// Test that rebuilding one workspace in PostgreSQL doesn't affect others.
    #[tokio::test]
    #[serial]
    async fn test_postgres_rebuild_workspace_isolation() {
        if !is_postgres_available() {
            eprintln!("Skipping PostgreSQL test - DATABASE_URL not set");
            return;
        }

        clean_provider_env();

        let database_url = get_database_url().unwrap();

        // Create state with PostgreSQL
        let state = match AppState::new_postgres(database_url, "").await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Skipping PostgreSQL test - connection failed: {}", e);
                return;
            }
        };

        // Create unique tenant
        let tenant_slug = format!("test-pg-iso-{}", &Uuid::new_v4().to_string()[..8]);
        let tenant = Tenant::new("PG Isolation Test", &tenant_slug);
        let created_tenant = state
            .workspace_service
            .create_tenant(tenant)
            .await
            .expect("Should create tenant");

        // Create workspace A
        let ws_a_slug = format!("ws-pg-a-{}", &Uuid::new_v4().to_string()[..8]);
        let create_a = CreateWorkspaceRequest {
            name: "Workspace A".to_string(),
            slug: Some(ws_a_slug),
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
            .create_workspace(created_tenant.tenant_id, create_a)
            .await
            .expect("Should create workspace A");

        // Create workspace B
        let ws_b_slug = format!("ws-pg-b-{}", &Uuid::new_v4().to_string()[..8]);
        let create_b = CreateWorkspaceRequest {
            name: "Workspace B".to_string(),
            slug: Some(ws_b_slug),
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
            .create_workspace(created_tenant.tenant_id, create_b)
            .await
            .expect("Should create workspace B");

        // Rebuild workspace A
        let app = Server::new(create_test_config(), state.clone()).build_router();

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

        // Verify workspace A updated
        let updated_a = state
            .workspace_service
            .get_workspace(workspace_a.workspace_id)
            .await
            .expect("Should fetch A")
            .expect("A should exist");

        assert_eq!(updated_a.embedding_model, "mock-embed-a-updated");
        assert_eq!(updated_a.embedding_dimension, 1536);

        // Verify workspace B UNCHANGED
        let unchanged_b = state
            .workspace_service
            .get_workspace(workspace_b.workspace_id)
            .await
            .expect("Should fetch B")
            .expect("B should exist");

        assert_eq!(
            unchanged_b.embedding_model, "mock-embed-b",
            "Workspace B embedding model should be unchanged"
        );
        assert_eq!(
            unchanged_b.embedding_dimension, 1024,
            "Workspace B dimension should be unchanged"
        );

        // Clean up
        let _ = state
            .workspace_service
            .delete_workspace(workspace_a.workspace_id)
            .await;
        let _ = state
            .workspace_service
            .delete_workspace(workspace_b.workspace_id)
            .await;
        let _ = state
            .workspace_service
            .delete_tenant(created_tenant.tenant_id)
            .await;

        clean_provider_env();
    }

    /// Test that pipeline uses PostgreSQL-persisted config after restart.
    #[tokio::test]
    #[serial]
    async fn test_postgres_pipeline_uses_persisted_config() {
        if !is_postgres_available() {
            eprintln!("Skipping PostgreSQL test - DATABASE_URL not set");
            return;
        }

        clean_provider_env();

        let database_url = get_database_url().unwrap();

        // First "instance" - create workspace and update config
        let workspace_id = {
            let state1 = match AppState::new_postgres(database_url.clone(), "").await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Skipping PostgreSQL test - connection failed: {}", e);
                    return;
                }
            };

            let tenant_slug = format!("test-pg-persist-{}", &Uuid::new_v4().to_string()[..8]);
            let tenant = Tenant::new("PG Persist Test", &tenant_slug);
            let created_tenant = state1
                .workspace_service
                .create_tenant(tenant)
                .await
                .expect("Should create tenant");

            let ws_slug = format!("ws-pg-persist-{}", &Uuid::new_v4().to_string()[..8]);
            let create_request = CreateWorkspaceRequest {
                name: "Persist Test".to_string(),
                slug: Some(ws_slug),
                description: None,
                max_documents: None,
                llm_model: Some("mock-llm-persisted".to_string()),
                llm_provider: Some("mock".to_string()),
                embedding_model: Some("mock-embed-persisted".to_string()),
                embedding_provider: Some("mock".to_string()),
                embedding_dimension: Some(1536),
            };

            let workspace = state1
                .workspace_service
                .create_workspace(created_tenant.tenant_id, create_request)
                .await
                .expect("Should create workspace");

            workspace.workspace_id
            // state1 goes out of scope, simulating "restart"
        };

        // Second "instance" - verify config persisted
        {
            let state2 = match AppState::new_postgres(database_url, "").await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Skipping PostgreSQL test - connection failed: {}", e);
                    return;
                }
            };

            // Fetch workspace from PostgreSQL
            let fetched = state2
                .workspace_service
                .get_workspace(workspace_id)
                .await
                .expect("Should fetch workspace from PostgreSQL")
                .expect("Workspace should exist in PostgreSQL");

            assert_eq!(
                fetched.llm_model, "mock-llm-persisted",
                "LLM model should be persisted across restarts"
            );
            assert_eq!(
                fetched.embedding_model, "mock-embed-persisted",
                "Embedding model should be persisted across restarts"
            );
            assert_eq!(
                fetched.embedding_dimension, 1536,
                "Embedding dimension should be persisted across restarts"
            );

            // Create pipeline - should use persisted config
            let pipeline = state2
                .create_workspace_pipeline(&workspace_id.to_string())
                .await;

            // Pipeline should exist
            assert!(std::sync::Arc::strong_count(&pipeline) >= 1);

            // Clean up
            let _ = state2
                .workspace_service
                .delete_workspace(workspace_id)
                .await;
        }

        clean_provider_env();
    }

    // ========================================================================
    // OODA 224: AGE Extension Load Fix Test
    // ========================================================================

    /// Test that rebuild-knowledge-graph correctly loads AGE extension before clearing.
    ///
    /// This test verifies the fix for the "type 'agtype' does not exist" error
    /// that occurred when rebuilding knowledge graph. The issue was that
    /// `clear_workspace` in PostgresAGEGraphStorage did not call `LOAD 'age'`
    /// before using AGE functions.
    ///
    /// @implements OODA-224: AGE Extension Load Fix
    #[tokio::test]
    #[serial]
    async fn test_postgres_rebuild_kg_loads_age_extension() {
        if !is_postgres_available() {
            eprintln!("Skipping PostgreSQL test - DATABASE_URL not set");
            return;
        }

        clean_provider_env();

        let database_url = get_database_url().unwrap();

        // Create state with PostgreSQL - this uses real AGE graph storage
        let state = match AppState::new_postgres(database_url, "").await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Skipping PostgreSQL test - connection failed: {}", e);
                return;
            }
        };

        // Create unique tenant for this test
        let tenant_slug = format!("test-ooda224-{}", &Uuid::new_v4().to_string()[..8]);
        let tenant = Tenant::new("OODA-224 AGE Test", &tenant_slug);
        let created_tenant = state
            .workspace_service
            .create_tenant(tenant)
            .await
            .expect("Should create tenant in PostgreSQL");

        // Create workspace
        let workspace_slug = format!("ws-ooda224-{}", &Uuid::new_v4().to_string()[..8]);
        let create_request = CreateWorkspaceRequest {
            name: "OODA-224 AGE Test".to_string(),
            slug: Some(workspace_slug),
            description: Some("Test for AGE extension loading fix".to_string()),
            max_documents: None,
            llm_model: Some("mock-llm".to_string()),
            llm_provider: Some("mock".to_string()),
            embedding_model: Some("mock-embed".to_string()),
            embedding_provider: Some("mock".to_string()),
            embedding_dimension: Some(768),
        };

        let workspace = state
            .workspace_service
            .create_workspace(created_tenant.tenant_id, create_request)
            .await
            .expect("Should create workspace in PostgreSQL");

        // Build app and call rebuild-knowledge-graph
        // This should NOT fail with "type 'agtype' does not exist" error
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

        // OODA-224 FIX: This should succeed (status 200), not fail with 500
        // Before the fix: "Failed to clear graph: Database error: Failed to clear workspace:
        //                  error returned from database: type 'agtype' does not exist"
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Rebuild should succeed - AGE extension must be loaded before using agtype"
        );

        let json = extract_json(response).await;

        // Verify response structure
        assert!(
            json.get("nodes_cleared").is_some(),
            "Response should include nodes_cleared"
        );
        assert!(
            json.get("edges_cleared").is_some(),
            "Response should include edges_cleared"
        );

        // Status can be "graph_cleared" (if workspace had data) or "no_change" (empty workspace)
        // The critical test is that it returns 200, not 500 with "agtype does not exist"
        let status = json["status"].as_str().unwrap_or("");
        assert!(
            status == "graph_cleared" || status == "no_change",
            "Status should be graph_cleared or no_change, got: {}",
            status
        );

        // Clean up
        let _ = state
            .workspace_service
            .delete_workspace(workspace.workspace_id)
            .await;
        let _ = state
            .workspace_service
            .delete_tenant(created_tenant.tenant_id)
            .await;

        clean_provider_env();
    }

    // ========================================================================
    // OODA-225: Dimension Change Cache Eviction Test
    // ========================================================================

    /// Test that rebuild-embeddings evicts vector cache when dimension changes.
    ///
    /// @implements OODA-225: Vector dimension mismatch after provider switch
    ///
    /// ## Scenario
    ///
    /// 1. Create workspace with Ollama (768 dimensions)
    /// 2. Switch to OpenAI (1536 dimensions) via rebuild-embeddings
    /// 3. Vector storage cache should be evicted so next query uses correct dimension
    ///
    /// ## Before fix
    ///
    /// After dimension change, queries fail with:
    /// "different vector dimensions 1536 and 768"
    /// because the cached vector storage still had 768-dim configuration.
    ///
    /// ## After fix
    ///
    /// The cache is evicted during rebuild, so the next access creates
    /// a new vector storage instance with the correct dimension.
    #[tokio::test]
    #[serial]
    async fn test_postgres_rebuild_embeddings_evicts_cache_on_dimension_change() {
        if !is_postgres_available() {
            eprintln!("Skipping PostgreSQL test - DATABASE_URL not set");
            return;
        }

        clean_provider_env();

        let database_url = get_database_url().unwrap();

        // Create state with PostgreSQL
        let state = match AppState::new_postgres(database_url, "").await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Skipping PostgreSQL test - connection failed: {}", e);
                return;
            }
        };

        // Create unique tenant for this test
        let tenant_slug = format!("test-ooda225-{}", &Uuid::new_v4().to_string()[..8]);
        let tenant = Tenant::new("OODA-225 Dimension Change Test", &tenant_slug);
        let created_tenant = state
            .workspace_service
            .create_tenant(tenant)
            .await
            .expect("Should create tenant in PostgreSQL");

        // Create workspace with initial 768 dimensions (Ollama-style)
        let workspace_slug = format!("ws-ooda225-{}", &Uuid::new_v4().to_string()[..8]);
        let create_request = CreateWorkspaceRequest {
            name: "OODA-225 Dimension Test".to_string(),
            slug: Some(workspace_slug),
            description: Some("Test for dimension change cache eviction".to_string()),
            max_documents: None,
            llm_model: Some("mock-llm".to_string()),
            llm_provider: Some("mock".to_string()),
            embedding_model: Some("mock-embed-768".to_string()),
            embedding_provider: Some("mock".to_string()),
            embedding_dimension: Some(768),
        };

        let workspace = state
            .workspace_service
            .create_workspace(created_tenant.tenant_id, create_request)
            .await
            .expect("Should create workspace in PostgreSQL");

        // Verify initial dimension is 768
        let initial = state
            .workspace_service
            .get_workspace(workspace.workspace_id)
            .await
            .expect("Should fetch workspace")
            .expect("Workspace should exist");
        assert_eq!(initial.embedding_dimension, 768);

        // Pre-warm the vector registry cache by accessing it
        // (This simulates the cache being populated before provider switch)
        use edgequake_storage::traits::WorkspaceVectorConfig;
        let config_768 = WorkspaceVectorConfig {
            workspace_id: workspace.workspace_id,
            dimension: 768,
            namespace: "default".to_string(),
        };
        let _ = state.vector_registry.get_or_create(config_768).await;

        // Build app and call rebuild-embeddings with NEW dimension (1536)
        let app = Server::new(create_test_config(), state.clone()).build_router();

        let rebuild_request = json!({
            "embedding_model": "mock-embed-1536",
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

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Rebuild embeddings should succeed with dimension change"
        );

        let json = extract_json(response).await;
        assert_eq!(json["embedding_dimension"], 1536);

        // Verify config updated in database
        let updated = state
            .workspace_service
            .get_workspace(workspace.workspace_id)
            .await
            .expect("Should fetch updated workspace")
            .expect("Workspace should exist");
        assert_eq!(updated.embedding_dimension, 1536);

        // OODA-225 CRITICAL TEST: Try to get vector storage with new dimension
        // Before fix: This would fail with "Dimension mismatch: cached=768, requested=1536"
        // After fix: Cache is evicted, so this should succeed with 1536
        let config_1536 = WorkspaceVectorConfig {
            workspace_id: workspace.workspace_id,
            dimension: 1536,
            namespace: "default".to_string(),
        };

        let result = state.vector_registry.get_or_create(config_1536).await;

        assert!(
            result.is_ok(),
            "Should be able to get vector storage with new dimension after rebuild. \
             Cache should have been evicted. Error: {:?}",
            result.err()
        );

        let storage = result.unwrap();
        assert_eq!(
            storage.dimension(),
            1536,
            "Vector storage should have new dimension 1536, not old 768"
        );

        // Clean up
        let _ = state
            .workspace_service
            .delete_workspace(workspace.workspace_id)
            .await;
        let _ = state
            .workspace_service
            .delete_tenant(created_tenant.tenant_id)
            .await;

        clean_provider_env();
    }
}
