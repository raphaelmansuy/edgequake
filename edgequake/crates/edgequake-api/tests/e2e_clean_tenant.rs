//! Clean tenant E2E test helper and tests.
//!
//! OODA-10: Provides test isolation via unique tenants per test run.
//!
//! # Design
//!
//! Each test creates a fresh `AppState::test_state()` (in-memory storage),
//! ensuring complete data isolation between tests. Additionally, a unique
//! tenant + workspace is created per test to prove the multi-tenancy API.
//!
//! ## Isolation strategy
//!
//! - **In-memory mode**: Each `TestContext` gets its own `AppState`, so all
//!   data (documents, graph, vectors) is isolated by construction.
//! - **Tenant creation**: Proves the tenant/workspace API works and creates
//!   unique slugs per test run.
//! - **Document operations**: Use the global mock pipeline (no workspace
//!   headers) since the mock provider doesn't need real LLM connectivity.
//!   WHY: Workspace-scoped pipelines try to create provider-specific
//!   clients (e.g., ollama/embeddinggemma) which fail without a real server.
//!
//! For production-like testing with real providers and workspace-scoped
//! pipelines, see the E2E tests under `e2e_ollama_integration.rs`.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use serde_json::{json, Value};
use std::time::Duration;
use tower::ServiceExt;
use uuid::Uuid;

// ============================================================================
// Test Context
// ============================================================================

/// Test context with isolated state and tenant.
///
/// OODA-10: Each test gets fresh in-memory state + unique tenant.
struct TestContext {
    app: axum::Router,
    /// Tenant ID created for this test run.
    tenant_id: String,
    /// Default workspace ID auto-created with tenant.
    workspace_id: String,
}

impl TestContext {
    /// Create a test context with fresh in-memory storage and a unique tenant.
    ///
    /// WHY: Each test gets its own AppState for data isolation (OODA-10).
    /// A tenant is also created to prove the multi-tenancy API works.
    async fn new_isolated() -> Self {
        let state = AppState::test_state();
        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            enable_cors: false,
            enable_compression: false,
            enable_swagger: true,
        };
        let server = Server::new(config, state);
        let app = server.build_router();

        // Create a unique tenant to prove multi-tenancy works
        let unique_slug = format!("test-{}", Uuid::new_v4());
        let tenant_name = format!("Test Tenant {}", &unique_slug[5..13]);

        let create_req = json!({
            "name": tenant_name,
            "slug": unique_slug,
            "plan": "free"
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/tenants")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&create_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::CREATED,
            "Failed to create test tenant"
        );

        let body = extract_json(response).await;
        // WHY: TenantResponse serializes Uuid field as "id" (not "tenant_id")
        let tenant_id = body["id"].as_str().unwrap().to_string();

        // WHY: list_workspaces uses path param /api/v1/tenants/{tenant_id}/workspaces
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/tenants/{}/workspaces", tenant_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Failed to list workspaces for tenant"
        );

        // WHY: WorkspaceListResponse has "items" array, each with "id" field
        let ws_list = extract_json(response).await;
        let workspace_id = ws_list["items"][0]["id"].as_str().unwrap().to_string();

        Self {
            app,
            tenant_id,
            workspace_id,
        }
    }

    /// Upload a text document using the global mock pipeline.
    ///
    /// WHY: Document operations do NOT send X-Workspace-ID headers because
    /// the workspace-specific pipeline would try to create real LLM providers
    /// (ollama/openai) which are unavailable in test mode. The global mock
    /// pipeline handles extraction correctly.
    async fn upload_text(&self, content: &str, title: &str) -> Value {
        let request = json!({
            "content": content,
            "title": title,
            "metadata": {"test": true, "tenant_isolated": true}
        });

        let response = self
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/documents")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let status = response.status();
        let body = extract_json(response).await;

        // WHY: POST /documents returns 201 Created per REST semantics (UC0001)
        assert_eq!(
            status,
            StatusCode::CREATED,
            "Expected 201 Created, got {}. Response: {}",
            status,
            serde_json::to_string_pretty(&body).unwrap()
        );
        body
    }

    /// Get document by ID.
    async fn get_document(&self, document_id: &str) -> Value {
        let response = self
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/documents/{}", document_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        extract_json(response).await
    }

    /// Get graph data.
    async fn get_graph(&self) -> Value {
        let response = self
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/graph")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        extract_json(response).await
    }

    /// Query RAG.
    async fn query_rag(&self, query: &str) -> Value {
        let request = json!({
            "query": query
        });

        let response = self
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/query")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        extract_json(response).await
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

async fn extract_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("Failed to read response body");
    serde_json::from_slice(&bytes).expect("Failed to parse JSON")
}

// ============================================================================
// Test Documents
// ============================================================================

/// Simple test document for quick tests.
const SIMPLE_DOCUMENT: &str = r#"
Sarah Chen is a senior AI researcher at TechCorp Labs. She leads the Natural Language Processing team.
Sarah collaborates closely with Dr. James Wilson on transformer architectures.
"#;

/// Medium document for entity extraction testing.
const ENTITY_DOCUMENT: &str = r#"
EdgeQuake Corporation is headquartered in San Francisco. Founded by Michael Roberts and Lisa Chang
in 2020, the company specializes in knowledge graph technologies.

Dr. Emily Watson serves as the CTO, leading a team of 150 engineers. She previously worked
at Google Brain. EdgeQuake's partnership with Stanford University involves Professor David Kim
who serves as an advisor.

The company raised $100 million in Series C funding from Sequoia Capital and Andreessen Horowitz.
"#;

// ============================================================================
// Tests: Tenant Isolation (OODA-10)
// ============================================================================

/// OODA-10: Test that each test run gets a fresh isolated tenant.
#[tokio::test]
async fn test_clean_tenant_isolation() {
    let ctx1 = TestContext::new_isolated().await;
    let ctx2 = TestContext::new_isolated().await;

    // WHY: Each test must get unique IDs to prevent data contamination
    assert_ne!(
        ctx1.tenant_id, ctx2.tenant_id,
        "Each test should get a unique tenant"
    );
    assert_ne!(
        ctx1.workspace_id, ctx2.workspace_id,
        "Each test should get a unique workspace"
    );
}

/// OODA-10: Test document upload with clean tenant.
#[tokio::test]
async fn test_document_upload_clean_tenant() {
    let ctx = TestContext::new_isolated().await;

    // Upload document
    let result = ctx.upload_text(SIMPLE_DOCUMENT, "Clean Tenant Test").await;

    // Verify document created
    assert!(
        result["document_id"].is_string(),
        "Upload should return a document_id"
    );
    assert_eq!(
        result["status"], "processed",
        "Document should be processed"
    );

    // Verify we can retrieve it
    let doc_id = result["document_id"].as_str().unwrap();
    let doc = ctx.get_document(doc_id).await;
    assert_eq!(doc["title"], "Clean Tenant Test");
}

/// OODA-10: Test entity extraction with clean tenant.
#[tokio::test]
async fn test_entity_extraction_clean_tenant() {
    let ctx = TestContext::new_isolated().await;

    // Upload entity-rich document
    let result = ctx.upload_text(ENTITY_DOCUMENT, "Entity Test").await;

    assert!(result["document_id"].is_string());
    assert_eq!(result["status"], "processed");

    // WHY: Mock provider extracts entities deterministically
    assert!(
        result.get("entity_count").is_some(),
        "Response should include entity_count"
    );
    assert!(
        result.get("relationship_count").is_some(),
        "Response should include relationship_count"
    );

    // Check graph has data
    let graph = ctx.get_graph().await;
    assert!(
        graph.get("nodes").is_some(),
        "Graph response should contain nodes"
    );
}

/// OODA-10: Test query with clean tenant.
#[tokio::test]
async fn test_query_clean_tenant() {
    let ctx = TestContext::new_isolated().await;

    // Upload document first
    ctx.upload_text(ENTITY_DOCUMENT, "Query Test Document")
        .await;

    // Query - mock provider returns deterministic results
    let result = ctx.query_rag("What is EdgeQuake Corporation?").await;

    // WHY: QueryResponse has "answer" field (not "response")
    assert!(
        result.get("answer").is_some(),
        "Query should return an answer field. Got: {}",
        serde_json::to_string_pretty(&result).unwrap()
    );
}

// ============================================================================
// Tests: Timeout enforcement (OODA-11)
// ============================================================================

/// OODA-11: Verify document upload completes within 30s.
#[tokio::test]
async fn test_document_upload_timeout_30s() {
    let timeout = Duration::from_secs(30);

    let result = tokio::time::timeout(timeout, async {
        let ctx = TestContext::new_isolated().await;
        ctx.upload_text(SIMPLE_DOCUMENT, "Timeout Test").await
    })
    .await;

    assert!(result.is_ok(), "Document upload should complete within 30s");
    let body = result.unwrap();
    assert!(body["document_id"].is_string());
}

/// OODA-11: Verify query completes within 30s.
#[tokio::test]
async fn test_query_timeout_30s() {
    let timeout = Duration::from_secs(30);

    let result = tokio::time::timeout(timeout, async {
        let ctx = TestContext::new_isolated().await;
        ctx.upload_text(ENTITY_DOCUMENT, "Timeout Query Test").await;
        ctx.query_rag("Tell me about EdgeQuake").await
    })
    .await;

    assert!(result.is_ok(), "Query should complete within 30s");
}

// ============================================================================
// Tests: Multiple documents in same tenant (OODA-10)
// ============================================================================

/// OODA-10: Test multiple documents in same clean tenant.
#[tokio::test]
async fn test_multiple_documents_same_tenant() {
    let ctx = TestContext::new_isolated().await;

    // Upload multiple documents
    let doc1 = ctx.upload_text(SIMPLE_DOCUMENT, "Doc 1").await;
    let doc2 = ctx.upload_text(ENTITY_DOCUMENT, "Doc 2").await;

    assert!(doc1["document_id"].is_string());
    assert!(doc2["document_id"].is_string());

    // Documents should have different IDs
    assert_ne!(
        doc1["document_id"].as_str().unwrap(),
        doc2["document_id"].as_str().unwrap(),
        "Each document should get a unique ID"
    );

    // Both should be processed
    assert_eq!(doc1["status"], "processed");
    assert_eq!(doc2["status"], "processed");
}

/// OODA-10: Test tenant creation with model configuration (SPEC-032).
#[tokio::test]
async fn test_tenant_with_model_config() {
    let state = AppState::test_state();
    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: true,
    };
    let server = Server::new(config, state);
    let app = server.build_router();

    let unique_slug = format!("test-openai-{}", Uuid::new_v4());

    // WHY: Tenant creation can specify default LLM + embedding config (SPEC-032)
    let create_req = json!({
        "name": "OpenAI Tenant",
        "slug": unique_slug,
        "plan": "pro",
        "default_llm_model": "gpt-4o-mini",
        "default_llm_provider": "openai",
        "default_embedding_model": "text-embedding-3-small",
        "default_embedding_provider": "openai",
        "default_embedding_dimension": 1536
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tenants")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&create_req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = extract_json(response).await;

    // Verify model config propagated
    assert_eq!(body["default_llm_model"], "gpt-4o-mini");
    assert_eq!(body["default_llm_provider"], "openai");
    assert_eq!(body["default_embedding_model"], "text-embedding-3-small");
    assert_eq!(body["default_embedding_provider"], "openai");
    assert_eq!(body["default_embedding_dimension"], 1536);

    // Verify auto-created workspace inherits model config
    let tenant_id = body["id"].as_str().unwrap();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/tenants/{}/workspaces", tenant_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let ws_list = extract_json(response).await;
    let workspace = &ws_list["items"][0];

    assert_eq!(
        workspace["llm_model"], "gpt-4o-mini",
        "Workspace should inherit tenant LLM model"
    );
    assert_eq!(
        workspace["embedding_model"], "text-embedding-3-small",
        "Workspace should inherit tenant embedding model"
    );
    assert_eq!(
        workspace["embedding_dimension"], 1536,
        "Workspace should inherit tenant embedding dimension"
    );
}

/// OODA-10: Test data isolation between independent contexts.
#[tokio::test]
async fn test_data_isolation_between_contexts() {
    // Create two independent contexts
    let ctx1 = TestContext::new_isolated().await;
    let ctx2 = TestContext::new_isolated().await;

    // Upload to ctx1 only
    let doc = ctx1.upload_text(SIMPLE_DOCUMENT, "Only in Context 1").await;
    let doc_id = doc["document_id"].as_str().unwrap();

    // ctx1 should find it
    let found = ctx1.get_document(doc_id).await;
    assert_eq!(found["title"], "Only in Context 1");

    // ctx2 should NOT find it (404) because it has separate in-memory state
    let response = ctx2
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/documents/{}", doc_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Document should NOT exist in a different test context"
    );
}
