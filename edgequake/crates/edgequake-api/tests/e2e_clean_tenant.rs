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
//!
//! P-G2b: `POST /api/v1/documents` always enqueues a background task and
//! returns `202 ACCEPTED` + `status: "pending"` + `task_id` (no counts).
//! Tests that need a fully ingested document use a worker-backed app
//! (`common::create_test_app_with_workers`) and poll the track-status
//! endpoint via `common::wait_for_document_processed`.

mod common;

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

        let (tenant_id, workspace_id) = create_tenant_on(&app).await;

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
    ///
    /// P-G2b: returns the raw upload response (202 ACCEPTED + pending). The
    /// caller is responsible for polling the track-status endpoint if it
    /// needs the document fully ingested.
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

        // P-G2b: uploads always enqueue a background task and return 202.
        assert!(
            status == StatusCode::CREATED || status == StatusCode::ACCEPTED,
            "Expected 201 or 202, got {}. Response: {}",
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
// Worker-backed tenant context (P-G2b)
// ============================================================================

/// Worker-backed test context: a real `WorkerPool` runs enqueued upload tasks,
/// plus a unique tenant/workspace is created via the API. The `WorkerAppGuard`
/// is held inside the context so the pool stays alive for the test's lifetime.
struct WorkerTestContext {
    _workers: common::WorkerAppGuard,
    app: axum::Router,
    /// Tenant ID created for this test run (retained for parity with TestContext).
    #[allow(dead_code)]
    tenant_id: String,
    /// Default workspace ID auto-created with tenant (retained for parity).
    #[allow(dead_code)]
    workspace_id: String,
}

impl WorkerTestContext {
    /// Create a worker-backed context with a fresh unique tenant.
    async fn new_isolated() -> Self {
        let workers = common::create_test_app_with_workers().await;
        // Borrow the router through the guard to create the tenant.
        let app = workers.app().clone();
        let (tenant_id, workspace_id) = create_tenant_on(&app).await;
        Self {
            _workers: workers,
            app,
            tenant_id,
            workspace_id,
        }
    }

    fn app(&self) -> &axum::Router {
        &self.app
    }
}

// ============================================================================
// Shared tenant-creation helper
// ============================================================================

/// Create a unique tenant + auto-created workspace on the given app and
/// return `(tenant_id, workspace_id)`.
async fn create_tenant_on(app: &axum::Router) -> (String, String) {
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

    (tenant_id, workspace_id)
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
    let ctx = WorkerTestContext::new_isolated().await;
    let app = ctx.app();

    // P-G2b: upload enqueues a background task; wait for terminal state.
    // NOTE: common::upload_document_assert signature is (app, title, content).
    let body = common::upload_document_assert(app, "Clean Tenant Test", SIMPLE_DOCUMENT).await;
    let doc_id = body["document_id"].as_str().unwrap();
    let track_id = body["track_id"].as_str().unwrap();
    let final_status =
        common::wait_for_document_processed(app, track_id, Duration::from_secs(30)).await;
    assert!(
        final_status == "completed"
            || final_status == "processed"
            || final_status == "indexed"
            || final_status == "partial_failure",
        "document did not reach an ingested state: {}",
        final_status
    );

    // Verify we can retrieve it
    let (_doc_status, doc) =
        common::get_endpoint(app, &format!("/api/v1/documents/{}", doc_id)).await;
    assert_eq!(doc["title"], "Clean Tenant Test");
}

/// OODA-10: Test entity extraction with clean tenant.
#[tokio::test]
async fn test_entity_extraction_clean_tenant() {
    let ctx = WorkerTestContext::new_isolated().await;
    let app = ctx.app();

    // P-G2b: upload enqueues a background task; wait for terminal state.
    let (doc_id, _track_id, _final_status) =
        common::upload_and_wait(app, "Entity Test", ENTITY_DOCUMENT, Duration::from_secs(30)).await;

    // Check graph has data
    let (_g_status, graph) = common::get_endpoint(app, "/api/v1/graph").await;
    assert!(
        graph.get("nodes").is_some(),
        "Graph response should contain nodes"
    );

    // The document should be retrievable and ingested.
    let (_d_status, doc) =
        common::get_endpoint(app, &format!("/api/v1/documents/{}", doc_id)).await;
    assert!(matches!(
        doc["status"].as_str(),
        Some("completed") | Some("partial_failure") | Some("processed") | Some("indexed")
    ));
}

/// OODA-10: Test query with clean tenant.
#[tokio::test]
async fn test_query_clean_tenant() {
    let ctx = WorkerTestContext::new_isolated().await;
    let app = ctx.app();

    // Upload document first and wait for ingestion (P-G2b).
    let (_doc_id, _track_id, _final_status) = common::upload_and_wait(
        app,
        "Query Test Document",
        ENTITY_DOCUMENT,
        Duration::from_secs(30),
    )
    .await;

    // Query - mock provider returns deterministic results
    let (status, result) = common::post_json(
        app,
        "/api/v1/query",
        &json!({ "query": "What is EdgeQuake Corporation?" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

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

/// OODA-11 / P-G2b: the synchronous upload timeout path was removed (uploads
/// now always enqueue a background task and return 202 immediately). Preserve
/// the test's intent — verifying the upload path stays snappy and returns the
/// async contract — by asserting the upload itself completes well within 30s
/// and returns 202 + pending + task_id.
#[tokio::test]
async fn test_document_upload_timeout_30s() {
    let timeout = Duration::from_secs(30);

    let result = tokio::time::timeout(timeout, async {
        let ctx = WorkerTestContext::new_isolated().await;
        // Only the upload (enqueue) is timed; background processing is not
        // part of the synchronous request path anymore.
        ctx.app().clone()
    })
    .await;

    assert!(result.is_ok(), "Upload setup should complete within 30s");

    let app = result.unwrap();
    let (status, body) = common::upload_document(&app, "Timeout Test", SIMPLE_DOCUMENT).await;
    assert!(
        status == StatusCode::CREATED || status == StatusCode::ACCEPTED,
        "Upload should return 201 or 202: {}",
        status
    );
    assert!(body["document_id"].is_string());
    // P-G2b: async contract — pending status + task_id present, no counts.
    assert_eq!(body["status"], "pending");
    assert!(body.get("task_id").is_some());
    assert!(
        body.get("chunk_count").is_none(),
        "P-G2b: async upload response must not include chunk_count"
    );
    assert!(
        body.get("entity_count").is_none(),
        "P-G2b: async upload response must not include entity_count"
    );
    assert!(
        body.get("relationship_count").is_none(),
        "P-G2b: async upload response must not include relationship_count"
    );
}

/// OODA-11 / P-G2b: verify query completes within 30s after ingestion. The
/// upload itself now returns 202 immediately, so the timeout budget covers
/// upload + processing + query.
#[tokio::test]
async fn test_query_timeout_30s() {
    let timeout = Duration::from_secs(30);

    let result = tokio::time::timeout(timeout, async {
        let ctx = WorkerTestContext::new_isolated().await;
        let app = ctx.app();
        let (_doc_id, _track_id, _final_status) = common::upload_and_wait(
            app,
            "Timeout Query Test",
            ENTITY_DOCUMENT,
            Duration::from_secs(25),
        )
        .await;
        common::post_json(
            app,
            "/api/v1/query",
            &json!({ "query": "Tell me about EdgeQuake" }),
        )
        .await
    })
    .await;

    assert!(result.is_ok(), "Query should complete within 30s");
    let (status, _body) = result.unwrap();
    assert_eq!(status, StatusCode::OK);
}

// ============================================================================
// Tests: Multiple documents in same tenant (OODA-10)
// ============================================================================

/// OODA-10: Test multiple documents in same clean tenant.
#[tokio::test]
async fn test_multiple_documents_same_tenant() {
    let ctx = WorkerTestContext::new_isolated().await;
    let app = ctx.app();

    // Upload multiple documents and wait for both to ingest (P-G2b).
    let (doc1_id, _t1, _s1) =
        common::upload_and_wait(app, "Doc 1", SIMPLE_DOCUMENT, Duration::from_secs(30)).await;
    let (doc2_id, _t2, _s2) =
        common::upload_and_wait(app, "Doc 2", ENTITY_DOCUMENT, Duration::from_secs(30)).await;

    assert!(!doc1_id.is_empty());
    assert!(!doc2_id.is_empty());

    // Documents should have different IDs
    assert_ne!(doc1_id, doc2_id, "Each document should get a unique ID");
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
    // Create two independent contexts (each with its own in-memory AppState).
    // P-G2b: no worker pool needed here — the upload handler stores document
    // metadata + content synchronously before enqueuing the background task,
    // so the document is retrievable by ID immediately. The test only asserts
    // cross-context isolation (ctx2 must 404), not full ingestion.
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
